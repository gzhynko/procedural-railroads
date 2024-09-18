use bevy::color::palettes::basic::BLUE;
use bevy::pbr::{StandardMaterialUniform};
use futures_lite::future;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroupShaderType, ShaderRef, AsBindGroup};
use bevy::render::texture::ImageSampler::Descriptor;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy::utils::HashMap;
use bevy_mod_picking::PickableBundle;
use bevy::render::extract_resource::ExtractResource;
use bevy::render::render_resource::ShaderType;
use bevy::reflect::{TypePath};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::texture::{ImageAddressMode, ImageSamplerDescriptor};

use crate::{Mesh, Vec2, Component, Indices, Vec3, PrimitiveTopology, Player, Transform, Commands, Assets, ResMut, Res, StandardMaterial, default, MaterialMeshBundle, Handle, With, Entity, NoiseSettings, Image, Vec4, RenderAssets, noise};
use crate::assets::{TextureAssets};

pub const FAR_GRID_CHUNK_SIZE: u32 = 1000; // in meters
pub const FAR_GRID_RENDER_DISTANCE: u32 = 5; // far grid chunks

pub const NEAR_GRID_CHUNK_SIZE: u32 = FAR_GRID_CHUNK_SIZE / 10; // in meters
pub const NEAR_GRID_RENDER_DISTANCE: u32 = 2; // far grid chunks

pub const WATER_LEVEL: f32 = -23.;

#[derive(Default)]
pub(crate) struct FarChunkData {
    /// The position of the chunk here are relative to center.
    /// So (0, 0) will mean a chunk at position `(-TERRAIN_CHUNK_SIZE / 2., -TERRAIN_CHUNK_SIZE / 2.)`.
    pub(crate) pos: Vec2,

    pub(crate) mesh_handle: Handle<Mesh>,
    pub(crate) near_mesh_handles: Vec<Handle<Mesh>>,

    /// A boolean that tracks whether this far chunk has been flagged for near chunk generation.
    /// As soon as the track midline has exited a far chunk, the chunk is flagged for it to have its near grid generated.
    /// Then, if generate_near_terrain sees a chunk that has been flagged and is within the near chunk render distance, it starts generating near grid for that chunk.
    pub(crate) flagged: bool,
    pub(crate) generating_near_chunks: bool,

    pub(crate) midline_entry_node_id: Option<usize>,
    pub(crate) midline_exit_node_id: Option<usize>,
}

/// The main terrain resource
#[derive(Resource)]
pub(crate) struct Terrain {
    /// The ID counter for unique chunk IDs.
    id_counter: u64,

    /// Stores the far-grid chunks' data and maps them by ID
    pub(crate) loaded_chunks: HashMap<u64, FarChunkData>,

    /// Stores a handle to the main terrain material.
    terrain_material_handle: Option<Handle<TerrainMaterial>>,

    /// Texture handle for grass.
    grass_texture_handle: Option<Handle<Image>>,
    /// Texture handle for rock.
    rock_texture_handle: Option<Handle<Image>>,
}

impl Default for Terrain {
    fn default() -> Self {
        Self {
            id_counter: 0,

            loaded_chunks: HashMap::new(),

            terrain_material_handle: None,
            grass_texture_handle: None,
            rock_texture_handle: None,
        }
    }
}

impl Terrain {
    pub(crate) fn get_new_chunk_id(&mut self) -> u64 {
        let result = self.id_counter.clone();
        self.id_counter += 1;

        result
    }
}

#[derive(Component)]
pub(crate) struct FarGridTerrainChunk(u64);

#[derive(Component)]
pub(crate) struct NearGridTerrainChunk(u64);

enum GenerateChunkMeshTaskType { FarGrid, NearGrid, }

#[derive(Component)]
pub(crate) struct GenerateChunkMeshTask(Task<(u64, GenerateChunkMeshTaskType, Vec2, Mesh)>);

/// Marker to update water plane position
#[derive(Component)]
pub(crate) struct WaterPlane;

pub(crate) fn setup_terrain(
    mut terrain_materials: ResMut<Assets<TerrainMaterial>>,
    mut terrain_res: ResMut<Terrain>,
    texture_assets: Res<TextureAssets>
) {
    let terrain_temp_albedo_handle = texture_assets.terrain_temp.clone();
    //let grass_albedo_handle = texture_assets.terrain_grass.clone();
    let rock_albedo_handle = texture_assets.terrain_rock.clone();

    terrain_res.grass_texture_handle = Some(terrain_temp_albedo_handle.clone());
    terrain_res.rock_texture_handle = Some(rock_albedo_handle.clone());

    let pbr = StandardMaterial {
        perceptual_roughness: 0.9,
        metallic: 0.0,
        reflectance: 0.2,
        ..default()
    };

    // TODO: Add normal map support (pass them to RenderAssets)
    let terrain_material_handle = terrain_materials.add(TerrainMaterial {
        grass_pbr_material: pbr.clone().as_bind_group_shader_type(&RenderAssets::default()),
        rock_pbr_material: pbr.clone().as_bind_group_shader_type(&RenderAssets::default()),
        grass_albedo_texture: Some(terrain_temp_albedo_handle),
        rock_albedo_texture: Some(rock_albedo_handle),
    });
    terrain_res.terrain_material_handle = Some(terrain_material_handle);
}

pub(crate) fn setup_water(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn the water plane
    let plane_material = StandardMaterial {
        base_color: Color::from(BLUE),
        perceptual_roughness: 0.0,
        metallic: 0.0,
        reflectance: 0.6,
        ..StandardMaterial::default()
    };
    let plane_pos = Vec3::new(-1. * FAR_GRID_CHUNK_SIZE as f32 / 2., WATER_LEVEL, -1. * FAR_GRID_CHUNK_SIZE as f32 / 2.);
    let plane_scale = Vec3::new((FAR_GRID_CHUNK_SIZE * FAR_GRID_RENDER_DISTANCE * 2) as f32 - FAR_GRID_CHUNK_SIZE as f32 * 0., 1., (FAR_GRID_CHUNK_SIZE * FAR_GRID_RENDER_DISTANCE * 2) as f32 - FAR_GRID_CHUNK_SIZE as f32 * 0.);
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(Plane3d::default())),
        material: standard_materials.add(plane_material),
        transform: Transform::default().with_translation(plane_pos).with_scale(plane_scale),
        ..default()
    })
        .insert(WaterPlane);
}

pub(crate) fn update_water_plane(
    mut water_plane_transform_query: Query<&mut Transform, (With<WaterPlane>, Without<Player>)>,
    player_transform_query: Query<&Transform, (With<Player>, Without<WaterPlane>)>,
) {
    let mut water_plane_transform = water_plane_transform_query.single_mut();
    let player_translation = player_transform_query.single().translation;
    water_plane_transform.translation = Vec3::new(player_translation.x - FAR_GRID_CHUNK_SIZE as f32 / 2., WATER_LEVEL, player_translation.z - FAR_GRID_CHUNK_SIZE as f32 / 2.);
}

/// A messy workaround to set sampler address modes for the terrain textures (needed to sample without UVs)
pub(crate) fn configure_terrain_images(
    terrain_res: Res<Terrain>,
    mut images: ResMut<Assets<Image>>,
) {
    let mut descriptor = ImageSamplerDescriptor::default();
    descriptor.address_mode_u = ImageAddressMode::Repeat;
    descriptor.address_mode_v = ImageAddressMode::Repeat;
    descriptor.address_mode_w = ImageAddressMode::Repeat;

    let image_handles = [terrain_res.rock_texture_handle.clone(), terrain_res.grass_texture_handle.clone()];
    for handle in image_handles {
        if handle.is_none() { continue }
        let texture = images.get_mut(handle.as_ref().unwrap());
        if let Some(image) = texture {
            image.sampler = Descriptor(descriptor.clone());
        }
    }
}

/// Spawns threads to generate far-grid chunk meshes. The generated chunks are then spawned into the world in `spawn_generated_chunks`.
pub(crate) fn generate_far_terrain(
    player_query: Query<&Transform, With<Player>>,
    mut terrain_res: ResMut<Terrain>,

    mut commands: Commands,
    noise_settings: Res<NoiseSettings>,
) {
    // Get player position first since terrain gen will be based on it
    let player_transform = player_query.single();
    let player_world_position = Vec2::new(player_transform.translation.x, player_transform.translation.z);
    let player_chunk = get_far_chunk_position(player_world_position);

    // Spawn threads for the chunks that need to be generated
    let thread_pool = AsyncComputeTaskPool::get();
    for x in (player_chunk.x - FAR_GRID_RENDER_DISTANCE as i32)..(player_chunk.x + FAR_GRID_RENDER_DISTANCE as i32) {
        for y in (player_chunk.y - FAR_GRID_RENDER_DISTANCE as i32)..(player_chunk.y + FAR_GRID_RENDER_DISTANCE as i32) {
            let chunk = Vec2::new(x as f32, y as f32);
            let chunk_world_position = (chunk * FAR_GRID_CHUNK_SIZE as f32) - Vec2::splat(FAR_GRID_CHUNK_SIZE as f32 / 2.);
            // check first if the chunk is already loaded
            if terrain_res.loaded_chunks.values().any(|d| &d.pos == &chunk) { continue }

            let current_id = terrain_res.get_new_chunk_id();

            // Calculate meshes asynchronously
            let noise_settings = noise_settings.clone();
            let task = thread_pool.spawn(async move {
                let noise_fn = noise::get_heightmap_function(FAR_GRID_CHUNK_SIZE as f32, noise_settings, Vec3::ZERO);

                let (vertices, indices) = mesh_data_from_noise(noise_fn, FAR_GRID_CHUNK_SIZE + 1, FAR_GRID_CHUNK_SIZE + 1, 200, chunk_world_position);
                let normals = calculate_normals(&vertices, &indices);
                let mesh = build_mesh(vertices, indices, normals);

                (current_id.clone(), GenerateChunkMeshTaskType::FarGrid, chunk_world_position, mesh)
            });

            commands.spawn_empty().insert(GenerateChunkMeshTask(task));
            terrain_res.loaded_chunks.insert(current_id, FarChunkData { pos: chunk, ..default() });
        }
    }
}

/// Spawns threads to generate near-grid chunk meshes. The generated chunks are then spawned into the world in `spawn_generated_chunks`.
pub(crate) fn generate_near_terrain(
    player_query: Query<&Transform, With<Player>>,
    mut terrain_res: ResMut<Terrain>,

    mut commands: Commands,
    noise_settings: Res<NoiseSettings>,
) {
    // Get player position first since terrain gen will be based on it
    let player_transform = player_query.single();
    let player_world_position = Vec2::new(player_transform.translation.x, player_transform.translation.z);
    let player_chunk = get_far_chunk_position(player_world_position);

    // Spawn threads for the chunks that need to be generated
    let thread_pool = AsyncComputeTaskPool::get();
    for (far_chunk_id, mut far_chunk_data) in terrain_res.loaded_chunks.iter_mut() {
        // do not generate near grid for this chunk if it's not ready yet
        if !far_chunk_data.flagged {
            continue;
        }

        // do not generate near grid for this chunk if it is outside the far grid render distance (counting from the player chunk)
        let far_chunk_pos = far_chunk_data.pos * FAR_GRID_CHUNK_SIZE as f32 - Vec2::splat(FAR_GRID_CHUNK_SIZE as f32 / 2.);
        //if !is_within_far_render_distance(&far_chunk_pos, &player_chunk) {
        //    continue;
        //}

        // begin near grid generation process for this far chunk
        far_chunk_data.flagged = false;
        far_chunk_data.generating_near_chunks = true;

        let num_near_chunks = 10;
        let near_chunk_size = FAR_GRID_CHUNK_SIZE as i32 / num_near_chunks;
        for x in 0..num_near_chunks {
            for y in 0..num_near_chunks {
                let near_chunk_world_position = far_chunk_pos + Vec2::new((x * near_chunk_size) as f32, (y * near_chunk_size) as f32);

                let noise_settings = noise_settings.clone();
                let chunk_id = far_chunk_id.clone();
                let task = thread_pool.spawn(async move {
                    let noise_fn = noise::get_heightmap_function(FAR_GRID_CHUNK_SIZE as f32, noise_settings, Vec3::ZERO);

                    let (vertices, indices) = mesh_data_from_noise(noise_fn, near_chunk_size as u32 + 1, near_chunk_size as u32 + 1, 20,near_chunk_world_position);
                    let normals = calculate_normals(&vertices, &indices);
                    let mesh = build_mesh(vertices, indices, normals);

                    (chunk_id, GenerateChunkMeshTaskType::NearGrid, near_chunk_world_position, mesh)
                });

                commands.spawn_empty().insert(GenerateChunkMeshTask(task));
            }
        }
        // TODO: implement the rest of near grid generation algorithm
    }
}

/// Collects the results from threads spawned in `generate_terrain` and spawns the chunks.
pub(crate) fn spawn_generated_chunks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut terrain_res: ResMut<Terrain>,
    mut mesh_gen_tasks: Query<(Entity, &mut GenerateChunkMeshTask)>,
) {
    let terrain_material = terrain_res.terrain_material_handle.clone().unwrap();

    if let Some(c) = terrain_res.loaded_chunks.values_mut().find(|a| a.pos == Vec2::new(0.0, 0.0)) {
        if c.near_mesh_handles.is_empty() && !c.flagged && !c.generating_near_chunks {
            c.flagged = true;
        }
    }

    for (entity, mut task) in &mut mesh_gen_tasks {
        if let Some((id, task_type, chunk_position, mesh)) = future::block_on(future::poll_once(&mut task.0)) {
            let mesh_handle = meshes.add(mesh);

            match task_type {
                GenerateChunkMeshTaskType::FarGrid => {
                    // Add the chunk to the world and tag it with the FarGridTerrainChunk component
                    commands.entity(entity)
                        .remove::<GenerateChunkMeshTask>() // Remove the task component
                        .insert(
                            MaterialMeshBundle {
                                transform: Transform::from_xyz(chunk_position.x, 0., chunk_position.y),
                                mesh: mesh_handle.clone(),
                                material: terrain_material.clone(),
                                ..default()
                            }
                        )
                        .insert(FarGridTerrainChunk(id))
                        .insert(PickableBundle::default());

                    let mut chunk = terrain_res.loaded_chunks.get_mut(&id);
                    if let Some(mut data) = chunk {
                        data.mesh_handle = mesh_handle;
                    }
                },
                GenerateChunkMeshTaskType::NearGrid => {
                    // Add the chunk to the world and tag it with the FarGridTerrainChunk component
                    commands.entity(entity)
                        .remove::<GenerateChunkMeshTask>() // Remove the task component
                        .insert(
                            MaterialMeshBundle {
                                transform: Transform::from_xyz(chunk_position.x, 0., chunk_position.y),
                                mesh: mesh_handle.clone(),
                                material: terrain_material.clone(),
                                ..default()
                            }
                        )
                        .insert(NearGridTerrainChunk(id))
                        .insert(PickableBundle::default());

                    let mut chunk = terrain_res.loaded_chunks.get_mut(&id);
                    if let Some(mut data) = chunk {
                        data.generating_near_chunks = false;
                        data.near_mesh_handles.push(mesh_handle);
                    }
                },
            }
        }
    }
}

pub(crate) fn remove_unused_terrain(
    mut commands: Commands,
    mut terrain_res: ResMut<Terrain>,
    mut meshes: ResMut<Assets<Mesh>>,
    player_query: Query<&Transform, With<Player>>,
    chunks: Query<(Entity, &FarGridTerrainChunk)>,
) {
    let player_transform = player_query.single();
    let player_position = Vec2::new(player_transform.translation.x, player_transform.translation.z);

    let player_chunk_x = ((player_position.x + FAR_GRID_CHUNK_SIZE as f32 / 2.) / FAR_GRID_CHUNK_SIZE as f32).floor() as i32;
    let player_chunk_y = ((player_position.y + FAR_GRID_CHUNK_SIZE as f32 / 2.) / FAR_GRID_CHUNK_SIZE as f32).floor() as i32;

    for (chunk_entity, chunk) in &chunks {
        let chunk_data = terrain_res.loaded_chunks.get(&chunk.0).unwrap();
        if (chunk_data.pos.x < player_chunk_x as f32 - FAR_GRID_RENDER_DISTANCE as f32 || chunk_data.pos.x > player_chunk_x as f32 + FAR_GRID_RENDER_DISTANCE as f32)
            || chunk_data.pos.y < player_chunk_y as f32 - FAR_GRID_RENDER_DISTANCE as f32 || chunk_data.pos.y > player_chunk_y as f32 + FAR_GRID_RENDER_DISTANCE as f32 {
            commands.entity(chunk_entity).despawn();

            let mesh_handle = &chunk_data.mesh_handle;
            meshes.remove(mesh_handle);

            terrain_res.loaded_chunks.remove(&chunk.0);
        }
    }
}

pub(crate) fn get_far_chunk_position(world_position: Vec2) -> IVec2 {
    let chunk_x = ((world_position.x + FAR_GRID_CHUNK_SIZE as f32 / 2.) / FAR_GRID_CHUNK_SIZE as f32).floor() as i32;
    let chunk_y = ((world_position.y + FAR_GRID_CHUNK_SIZE as f32 / 2.) / FAR_GRID_CHUNK_SIZE as f32).floor() as i32;

    IVec2::new(chunk_x, chunk_y)
}

pub(crate) fn is_within_far_render_distance(point: &Vec2, from_chunk_pos: &IVec2) -> bool {
    let min_x = (from_chunk_pos.x - FAR_GRID_RENDER_DISTANCE as i32) * FAR_GRID_CHUNK_SIZE as i32;
    let max_x = (from_chunk_pos.x + FAR_GRID_RENDER_DISTANCE as i32) * FAR_GRID_CHUNK_SIZE as i32;
    let min_y = (from_chunk_pos.y - FAR_GRID_RENDER_DISTANCE as i32) * FAR_GRID_CHUNK_SIZE as i32;
    let max_y = (from_chunk_pos.y + FAR_GRID_RENDER_DISTANCE as i32) * FAR_GRID_CHUNK_SIZE as i32;

    if point.x > max_x as f32 || point.x < min_x as f32 {
        false
    } else if point.y > max_y as f32 || point.y < min_y as f32 {
        false
    } else {
        true
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
    where F: Fn(f64, f64) -> f64 {
    let vertex_count_x = mesh_width / vertex_subdivision + 2;
    let vertex_count_z = mesh_height / vertex_subdivision + 2;

    let mut vertices = Vec::with_capacity((vertex_count_x * vertex_count_z) as usize);
    let mut indices = Vec::with_capacity(((vertex_count_x - 1) * (vertex_count_z - 1) * 6) as usize);

    let mut vertex_index = 0;
    for z in 0..vertex_count_z {
        for x in 0..vertex_count_x {
            let vertex_elevation = noise_fn((x * vertex_subdivision) as f64 + offset.x as f64, (z * vertex_subdivision) as f64 + offset.y as f64) as f32;

            let position = [(x * vertex_subdivision) as f32, vertex_elevation, (z * vertex_subdivision) as f32];
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

#[derive(AsBindGroup, Debug, Clone, Default, ExtractResource, ShaderType, Resource)]
struct Fog {
    color: Vec4,
    density_or_start: f32,
    end: f32,
}

#[derive(Asset, AsBindGroup, Clone, TypePath)]
pub(crate) struct TerrainMaterial {
    #[uniform(0)]
    grass_pbr_material: StandardMaterialUniform,
    #[uniform(1)]
    rock_pbr_material: StandardMaterialUniform,

    #[texture(2)]
    #[sampler(3)]
    grass_albedo_texture: Option<Handle<Image>>,
    #[texture(4)]
    #[sampler(5)]
    rock_albedo_texture: Option<Handle<Image>>,
}

impl Material for TerrainMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/terrain_texturing.wgsl".into()
    }
}
