use bevy::math::{IVec2, Vec2, Vec3};
use bevy::prelude::{Commands, Mesh, Query, Res, ResMut, Transform, With};
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::tasks::AsyncComputeTaskPool;
use crate::noise::NoiseSettings;
use crate::{noise, Player};
use crate::world::terrain::{is_within_near_render_distance, CHUNK_SIZE, FAR_GRID_RENDER_DISTANCE};
use crate::world::terrain::types::{GenerateChunkMeshTask, MeshSubdivisionLevel, NearChunkStatus, Terrain};

/// Spawns threads to generate far-grid (coarse terrain) chunk meshes.
/// The generated chunks are then spawned into the world in `spawn_generated_chunks`.
pub(crate) fn generate_far_terrain(
    player_query: Query<&Transform, With<Player>>,
    mut terrain_res: ResMut<Terrain>,

    mut commands: Commands,
    noise_settings: Res<NoiseSettings>,
) {
    // get player position first since terrain gen will be based on it
    let player_transform = player_query.single();
    let player_world_position = Vec2::new(player_transform.translation.x, player_transform.translation.z);
    let player_chunk = get_far_chunk_position(player_world_position);

    // spawn threads for the chunks that need to be generated
    let thread_pool = AsyncComputeTaskPool::get();
    for x in (player_chunk.x - FAR_GRID_RENDER_DISTANCE as i32)..(player_chunk.x + FAR_GRID_RENDER_DISTANCE as i32) {
        for y in (player_chunk.y - FAR_GRID_RENDER_DISTANCE as i32)..(player_chunk.y + FAR_GRID_RENDER_DISTANCE as i32) {
            let chunk = Vec2::new(x as f32, y as f32);
            let chunk_world_position = (chunk * CHUNK_SIZE as f32) - Vec2::splat(CHUNK_SIZE as f32 / 2.);
            // check first if the chunk is already loaded
            if terrain_res.loaded_chunks.values().any(|v| v.pos == chunk) ||
                terrain_res.generating_chunks.contains(&IVec2::new(chunk.x as i32, chunk.y as i32)) {
                continue;
            }

            let current_id = terrain_res.get_new_chunk_id();

            // Calculate meshes asynchronously
            let noise_settings = noise_settings.clone();
            let task = thread_pool.spawn(async move {
                let noise_fn = noise::get_heightmap_function(CHUNK_SIZE as f32, noise_settings, Vec3::ZERO);

                let (vertices, indices) = mesh_data_from_noise(noise_fn, CHUNK_SIZE + 1, CHUNK_SIZE + 1, 2, chunk_world_position);
                let normals = calculate_normals(&vertices, &indices);
                let mesh = build_mesh(vertices, indices, normals);

                (current_id.clone(), chunk, chunk_world_position, mesh)
            });

            commands.spawn_empty().insert(GenerateChunkMeshTask(task));
            terrain_res.generating_chunks.insert(IVec2::new(chunk.x as i32, chunk.y as i32));
        }
    }
}

pub(crate) fn generate_near_terrain(
    player_query: Query<&Transform, With<Player>>,
    mut terrain_res: ResMut<Terrain>,

    mut commands: Commands,
    noise_settings: Res<NoiseSettings>,
) {
    // get player position first since terrain gen will be based on it
    let player_transform = player_query.single();
    let player_world_position = Vec2::new(player_transform.translation.x, player_transform.translation.z);

    for (chunk_id, mut chunk_data) in &mut terrain_res.loaded_chunks {
        if chunk_data.near_chunk_status != Some(NearChunkStatus::Flagged)
            || !is_within_near_render_distance(&player_world_position, &IVec2::new(chunk_data.pos.x as i32, chunk_data.pos.y as i32)) {
            continue;
        }

        chunk_data.near_chunk_status = Some(NearChunkStatus::Generating);

    }
}

/// Builds the terrain mesh from pre-calculated vertices, indices, and normals.
fn build_mesh(vertices: Vec<[f32; 3]>, indices: Vec<u32>, normals: Vec<[f32; 3]>) -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_indices(Indices::U32(indices));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);

    mesh
}

/// Calculates smooth shading normals.
/// Reference: https://computergraphics.stackexchange.com/questions/4031/programmatically-generating-vertex-normals
fn calculate_normals(vertices: &Vec<[f32; 3]>, indices: &Vec<u32>) -> Vec<[f32; 3]> {
    let mut normals: Vec<Vec3> = Vec::new();

    for _ in vertices {
        normals.push(Vec3::new(0., 0., 0.));
    }

    for i in (0..indices.len()).step_by(3) {
        let index_a = indices[i] as usize;
        let index_b = indices[i + 1] as usize;
        let index_c = indices[i + 2] as usize;

        let vertex_a = vertices[index_a]; // A
        let vertex_b = vertices[index_b]; // B
        let vertex_c = vertices[index_c]; // C

        // AB
        let edge_ab = Vec3::new(vertex_b[0] - vertex_a[0], vertex_b[1] - vertex_a[1], vertex_b[2] - vertex_a[2]);
        // AC
        let edge_ac = Vec3::new(vertex_c[0] - vertex_a[0], vertex_c[1] - vertex_a[1], vertex_c[2] - vertex_a[2]);

        // AB cross AC
        let cross = Vec3::cross(edge_ab, edge_ac);
        normals[index_a] += cross;
        normals[index_b] += cross;
        normals[index_c] += cross;
    }

    for i in 0..vertices.len() {
        normals[i] = normals[i].normalize();
    }

    normals.iter().map(|v| [v.x, v.y, v.z]).collect()
}

/// Generates mesh data (vertices, indices) from a noise function
fn mesh_data_from_noise<F>(noise_fn: F, mesh_width: u32, mesh_height: u32, vertex_subdivision: u32, offset: Vec2) -> (Vec<[f32; 3]>, Vec<u32>)
where F: Fn(f32, f32) -> f32 {
    let vertex_count_x = vertex_subdivision + 1;
    let vertex_count_z = vertex_subdivision + 1;

    let mut vertices = Vec::with_capacity((vertex_count_x * vertex_count_z) as usize);
    let mut indices = Vec::with_capacity(((vertex_count_x - 1) * (vertex_count_z - 1) * 6) as usize);

    let mut vertex_index = 0;
    for z in 0..vertex_count_z {
        for x in 0..vertex_count_x {
            let world_x = x as f32 * mesh_width as f32 / vertex_subdivision as f32;
            let world_z = z as f32 * mesh_height as f32 / vertex_subdivision as f32;

            let vertex_elevation = noise_fn(world_x + offset.x as f32, world_z + offset.y as f32);

            let position = [world_x, vertex_elevation, world_z];
            vertices.push(position);

            if x < vertex_count_x - 1 && z < vertex_count_z - 1 {
                indices.push(vertex_index);
                indices.push(vertex_index + vertex_count_x + 1);
                indices.push(vertex_index + vertex_count_x);
                indices.push(vertex_index + vertex_count_x + 1);
                indices.push(vertex_index);
                indices.push(vertex_index + 1);
            }

            vertex_index += 1;
        }
    }

    // Flip the arrays (because of counterclockwise winding)
    vertices.reverse();
    indices.reverse();

    (vertices, indices)
}

pub(crate) fn subdivision_level_here(x: f32, z: f32) -> MeshSubdivisionLevel {
    println!("get subdivision level: {} {}", x, z);
    if x <= 500.0 && z <= 500.0 {
        MeshSubdivisionLevel::Near1
    } else if x <= 1000.0 && z <= 1000.0 {
        MeshSubdivisionLevel::Near2
    } else {
        MeshSubdivisionLevel::Far
    }
}

pub(crate) fn get_far_chunk_position(world_position: Vec2) -> IVec2 {
    let chunk_x = ((world_position.x + CHUNK_SIZE as f32 / 2.) / CHUNK_SIZE as f32).floor() as i32;
    let chunk_y = ((world_position.y + CHUNK_SIZE as f32 / 2.) / CHUNK_SIZE as f32).floor() as i32;

    IVec2::new(chunk_x, chunk_y)
}
