use futures_lite::future;
use bevy::prelude::{Query, RunCriteriaDescriptorCoercion};
use bevy::render::render_resource::AsBindGroupShaderType;
use bevy::render::texture::ImageSampler::Descriptor;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy::utils::HashMap;
use noisy_bevy::simplex_noise_2d_seeded;
use crate::{App, Mesh, Plugin, Vec2, Component, Indices, Vec3, PrimitiveTopology, Player, Transform, Commands, Assets, ResMut, Res, StandardMaterial, Color, default, PbrBundle, TerrainMaterial, MaterialMeshBundle, Handle, With, Entity, NoiseSettings, AssetServer, ImageSettings, SamplerDescriptor, AddressMode, EventReader, Image, AssetEvent, Fog, Vec4, RenderAssets};

const TERRAIN_CHUNK_SIZE: u32 = 256; // in meters
const RENDER_DISTANCE_CHUNKS: u32 = 5;

/// Includes systems for procedural terrain generation
pub(crate) struct TerrainPlugin;

/// The main terrain resource
pub(crate) struct Terrain {
    /// The ID counter for unique chunk IDs.
    id_counter: u64,

    /// Stores positions of chunks in a hashmap with the key being the unique ID of the chunk.
    /// The positions of chunks here are relative to center.
    /// So (0, 0) will mean a chunk at position `(-TERRAIN_CHUNK_SIZE / 2., -TERRAIN_CHUNK_SIZE / 2.)`.
    pub(crate) loaded_chunks_pos: HashMap<u64, Vec2>,
    /// Stores handles to chunks meshes in a hashmap with the key being the unique ID of the chunk.
    loaded_chunks_meshes: HashMap<u64, Handle<Mesh>>,

    /// Stores a handle to the main terrain material.
    terrain_material_handle: Option<Handle<TerrainMaterial>>,
    grass_texture_handle: Option<Handle<Image>>,
    rock_texture_handle: Option<Handle<Image>>,
}

impl Default for Terrain {
    fn default() -> Self {
        Self {
            id_counter: 0,
            loaded_chunks_pos: HashMap::new(),
            loaded_chunks_meshes: HashMap::new(),
            terrain_material_handle: None,
            grass_texture_handle: None,
            rock_texture_handle: None,
        }
    }
}

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(Terrain::default())

            .add_startup_system(setup)
            .add_system(configure_terrain_images)
            .add_system(generate_terrain)
            .add_system(spawn_generated_chunks)
            .add_system(remove_unused_terrain);
    }
}

#[derive(Component)]
pub(crate) struct TerrainChunk(u64);

#[derive(Component)]
struct GenerateChunkMeshTask(Task<(u64, Vec2, Mesh)>);

fn setup(
    mut terrain_materials: ResMut<Assets<TerrainMaterial>>,
    mut terrain_res: ResMut<Terrain>,
    asset_server: Res<AssetServer>,
) {
    let grass_albedo_handle = asset_server.load("textures/grass1-albedo3.png");
    let grass_normal_handle = asset_server.load("textures/grass1-normal1-ogl.png");
    let rock_albedo_handle = asset_server.load("textures/rock.jpg");

    terrain_res.grass_texture_handle = Some(grass_handle.clone());
    terrain_res.rock_texture_handle = Some(rock_handle.clone());

    let pbr = StandardMaterial {
        perceptual_roughness: 0.5,
        metallic: 0.0,
        reflectance: 0.5,
        ..default()
    };

    let fog = Fog {
        color: Vec4::new(1., 1., 1., 1.),
        density_or_start: 0.001,
        end: 0.0
    };

    // TODO: Add normal map support (pass them to RenderAssets)
    let terrain_material_handle = terrain_materials.add(TerrainMaterial {
        fog,
        grass_pbr_material: pbr.clone().as_bind_group_shader_type(&RenderAssets::new()),
        rock_pbr_material: pbr.clone().as_bind_group_shader_type(&RenderAssets::new()),
        grass_albedo_texture: Some(grass_albedo_handle),
        rock_albedo_texture: Some(rock_albedo_handle),
        grass_normal_texture: Some(grass_normal_handle),
        rock_normal_texture: None
    });
    terrain_res.terrain_material_handle = Some(terrain_material_handle);
}

/// A messy workaround to set sampler address modes for the terrain textures
fn configure_terrain_images(
    terrain_res: Res<Terrain>,
    mut images: ResMut<Assets<Image>>,
) {
    let mut descriptor = SamplerDescriptor::default();
    descriptor.address_mode_u = AddressMode::Repeat;
    descriptor.address_mode_v = AddressMode::Repeat;
    descriptor.address_mode_w = AddressMode::Repeat;

    let mut grass = images.get_mut(terrain_res.grass_texture_handle.as_ref().unwrap());
    if let Some(image) = grass {
        image.sampler_descriptor = Descriptor(descriptor.clone());
    }

    let mut rock = images.get_mut(&terrain_res.rock_texture_handle.as_ref().unwrap());
    if let Some(image) = rock {
        image.sampler_descriptor = Descriptor(descriptor.clone());
    }
}

/// Spawns threads to generate chunk meshes. The generated chunks are then spawned in `spawn_generated_chunks`
fn generate_terrain(
    player_query: Query<&Transform, With<Player>>,
    mut terrain_res: ResMut<Terrain>,

    mut commands: Commands,
    noise_settings: Res<NoiseSettings>,
) {
    // Get player position first since terrain gen will be based on it
    let player_transform = player_query.single();
    let player_position = Vec2::new(player_transform.translation.x, player_transform.translation.z);

    let player_chunk_x = ((player_position.x + TERRAIN_CHUNK_SIZE as f32 / 2.) / TERRAIN_CHUNK_SIZE as f32).floor() as i32;
    let player_chunk_y = ((player_position.y + TERRAIN_CHUNK_SIZE as f32 / 2.) / TERRAIN_CHUNK_SIZE as f32).floor() as i32;

    let thread_pool = AsyncComputeTaskPool::get();
    for x in (player_chunk_x - RENDER_DISTANCE_CHUNKS as i32)..(player_chunk_x + RENDER_DISTANCE_CHUNKS as i32) {
        for y in (player_chunk_y - RENDER_DISTANCE_CHUNKS as i32)..(player_chunk_y + RENDER_DISTANCE_CHUNKS as i32) {
            let chunk = Vec2::new(x as f32, y as f32);
            let chunk_position = (chunk * TERRAIN_CHUNK_SIZE as f32) - Vec2::splat(TERRAIN_CHUNK_SIZE as f32 / 2.);
            // check first if the chunk is already loaded
            if terrain_res.loaded_chunks_pos.values().any(|pos| pos == &chunk) { continue }

            let current_id = terrain_res.id_counter.clone();

            // Calculate meshes asynchronously
            let noise_settings = noise_settings.clone();
            let task = thread_pool.spawn(async move {
                let noise_fn = move |x: f64, y: f64| -> f64 {
                    //let noise = noise::Perlin::new(noise_settings.seed);
                    noise_settings.amplitude * simplex_noise_2d_seeded(Vec2::new((x as f32 - TERRAIN_CHUNK_SIZE as f32 / 2.) / noise_settings.scale.0 as f32, (y as f32 - TERRAIN_CHUNK_SIZE as f32 / 2.)  / noise_settings.scale.1 as f32), noise_settings.seed as f32) as f64
                        + 5. * simplex_noise_2d_seeded(Vec2::new((x as f32 - TERRAIN_CHUNK_SIZE as f32 / 2.) / 100., (y as f32 - TERRAIN_CHUNK_SIZE as f32 / 2.)  / 100.), noise_settings.seed as f32 + 1.) as f64

                    //noise_settings.amplitude * noise.get([(x - TERRAIN_CHUNK_SIZE as f64 / 2.) / noise_settings.scale.0, (y - TERRAIN_CHUNK_SIZE as f64 / 2.) / noise_settings.scale.0])
                };

                let (vertices, indices) = mesh_data_from_perlin(noise_fn, TERRAIN_CHUNK_SIZE + 1, TERRAIN_CHUNK_SIZE + 1, chunk_position);
                let mut normals = calculate_normals(&vertices, &indices);

                (current_id.clone(), chunk_position, build_mesh(vertices, indices, normals))
            });

            commands.spawn().insert(GenerateChunkMeshTask(task));
            terrain_res.loaded_chunks_pos.insert(current_id, chunk);
            terrain_res.id_counter += 1;
        }
    }
}

/// Collects the results from threads spawned in `generate_terrain` and spawns the chunks.
fn spawn_generated_chunks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut terrain_res: ResMut<Terrain>,
    mut mesh_gen_tasks: Query<(Entity, &mut GenerateChunkMeshTask)>,
) {
    let terrain_material = terrain_res.terrain_material_handle.clone().unwrap();

    for (entity, mut task) in &mut mesh_gen_tasks {
        if let Some((id, chunk_position, mesh)) = future::block_on(future::poll_once(&mut task.0)) {
            let mesh_handle = meshes.add(mesh);

            // Add the chunk to the world and tag it with the TerrainChunk component
            commands.entity(entity)
                .insert_bundle(
                    MaterialMeshBundle {
                        transform: Transform::from_xyz(chunk_position.x, 0., chunk_position.y),
                        mesh: mesh_handle.clone(),
                        material: terrain_material.clone(),
                        ..default()
                    }
                )
                .insert(TerrainChunk(id));
            // Remove the task component
            commands.entity(entity).remove::<GenerateChunkMeshTask>();

            terrain_res.loaded_chunks_meshes.insert(id, mesh_handle);
        }
    }
}

fn remove_unused_terrain(
    mut commands: Commands,
    mut terrain_res: ResMut<Terrain>,
    mut meshes: ResMut<Assets<Mesh>>,
    player_query: Query<&Transform, With<Player>>,
    chunks: Query<(Entity, &Transform, &TerrainChunk)>,
) {
    let player_transform = player_query.single();
    let player_position = Vec2::new(player_transform.translation.x, player_transform.translation.z);

    let player_chunk_x = ((player_position.x + TERRAIN_CHUNK_SIZE as f32 / 2.) / TERRAIN_CHUNK_SIZE as f32).floor() as i32;
    let player_chunk_y = ((player_position.y + TERRAIN_CHUNK_SIZE as f32 / 2.) / TERRAIN_CHUNK_SIZE as f32).floor() as i32;

    // Only retain chunks that are inside the render distance
    // The newly removed chunks will be cleaned in remove_extra_chunks.
    terrain_res.loaded_chunks_pos.retain(|_, pos| {
        if pos.x < player_chunk_x as f32 - RENDER_DISTANCE_CHUNKS as f32 || pos.x > player_chunk_x as f32 + RENDER_DISTANCE_CHUNKS as f32 {
            false
        } else if pos.y < player_chunk_y as f32 - RENDER_DISTANCE_CHUNKS as f32 || pos.y > player_chunk_y as f32 + RENDER_DISTANCE_CHUNKS as f32 {
            false
        } else {
            true
        }
    });

    for (chunk_entity, chunk_transform, chunk) in &chunks {
        let chunk_x = ((chunk_transform.translation.x + TERRAIN_CHUNK_SIZE as f32 / 2.) / TERRAIN_CHUNK_SIZE as f32).floor() as f32;
        let chunk_y = ((chunk_transform.translation.z + TERRAIN_CHUNK_SIZE as f32 / 2.) / TERRAIN_CHUNK_SIZE as f32).floor() as f32;
        let chunk_pos = Vec2::new(chunk_x, chunk_y);

        if terrain_res.loaded_chunks_pos.values().any(|&pos| pos == chunk_pos) { continue }

        commands.entity(chunk_entity).despawn();

        let mesh_handle = terrain_res.loaded_chunks_meshes.get(&chunk.0);
        if let Some(handle) = mesh_handle {
            meshes.remove(handle);
        }
        terrain_res.loaded_chunks_meshes.remove(&chunk.0);
    }
}

/// Builds the terrain mesh from pre-calculated vertices, indices, and normals.
fn build_mesh(vertices: Vec<[f32; 3]>, indices: Vec<u32>, normals: Vec<[f32; 3]>) -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.set_indices(Some(Indices::U32(indices)));
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

        // B - A
        let edge_ab = Vec3::new(vertex_b[0] - vertex_a[0], vertex_b[1] - vertex_a[1], vertex_b[2] - vertex_a[2]);
        // C - A
        let edge_ac = Vec3::new(vertex_c[0] - vertex_a[0], vertex_c[1] - vertex_a[1], vertex_c[2] - vertex_a[2]);

        // AB cross AC
        let mut cross = Vec3::cross(edge_ab, edge_ac);
        normals[index_a] += cross;
        normals[index_b] += cross;
        normals[index_c] += cross;
    }

    for i in 0..vertices.len() {
        normals[i] = normals[i].normalize();
    }

    normals.iter().map(|v| [v.x, v.y, v.z]).collect()
}

/// Generates mesh data (vertices, indices) from a perlin noise function
fn mesh_data_from_perlin<F>(perlin_fn: F, mesh_width: u32, mesh_height: u32, offset: Vec2) -> (Vec<[f32; 3]>, Vec<u32>)
    where F: Fn(f64, f64) -> f64 {
    let mut vertices = Vec::with_capacity((mesh_width * mesh_height) as usize);
    let mut indices = Vec::with_capacity(((mesh_width - 1)*(mesh_height - 1)*6) as usize);

    let mut vertex_index = 0;
    for y in 0..mesh_height {
        for x in 0..mesh_width {
            //let vertex_elevation = 15. * perlin_fn((x as f64 + offset.x as f64) * 0.01, (y as f64 + offset.y as f64) * 0.01) as f32
            //    + 5. * perlin_fn((x as f64 + offset.x as f64 + 50.) * 0.02, (y as f64 + offset.y as f64 + 50.) * 0.02) as f32
            //    + 2. * perlin_fn((x as f64 + offset.x as f64 + 100.) * 0.004, (y as f64 + offset.y as f64  + 100.) * 0.004) as f32;

            let vertex_elevation = perlin_fn(x as f64 + offset.x as f64, y as f64 + offset.y as f64) as f32;

            let position = [x as f32, vertex_elevation, y as f32];
            vertices.push(position);

            if x < mesh_width - 1 && y < mesh_height - 1 {
                indices.push(vertex_index);
                indices.push(vertex_index + mesh_width + 1);
                indices.push(vertex_index + mesh_width);
                indices.push(vertex_index + mesh_width + 1);
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