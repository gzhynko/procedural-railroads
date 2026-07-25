pub(crate) mod terrain_gen;
pub(crate) mod types;

use bevy::color::palettes::basic::BLUE;
use futures_lite::future;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroupShaderType, AsBindGroup};
use bevy::render::texture::ImageSampler::Descriptor;
use bevy_mod_picking::PickableBundle;
use bevy::render::extract_resource::ExtractResource;
use bevy::render::render_resource::ShaderType;
use bevy::reflect::{TypePath};
use bevy::render::texture::{ImageAddressMode, ImageSamplerDescriptor};
use crate::{Mesh, Vec2, Component, Vec3, Player, Transform, Commands, Assets, ResMut, Res, StandardMaterial, default, MaterialMeshBundle, With, Entity, Image, RenderAssets};
use crate::assets::{TextureAssets};
use crate::world::terrain::types::{ChunkData, TerrainChunk, GenerateChunkMeshTask, Terrain, WaterPlane, TerrainMaterial, NearChunkStatus};

pub const CHUNK_SIZE: u32 = 1000; // in meters
pub const FAR_GRID_RENDER_DISTANCE: u32 = 1; // far grid chunks
pub const NEAR_GRID_RENDER_DISTANCE: u32 = 1; // near grid chunks

pub const WATER_LEVEL: f32 = -23.;

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
        alpha_mode: AlphaMode::Blend,
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
        alpha_mode: AlphaMode::AlphaToCoverage,
        ..StandardMaterial::default()
    };
    let plane_pos = Vec3::new(-1. * CHUNK_SIZE as f32 / 2., WATER_LEVEL, -1. * CHUNK_SIZE as f32 / 2.);
    let plane_scale = Vec3::new((CHUNK_SIZE * FAR_GRID_RENDER_DISTANCE * 2) as f32 - CHUNK_SIZE as f32 * 0., 1., (CHUNK_SIZE * FAR_GRID_RENDER_DISTANCE * 2) as f32 - CHUNK_SIZE as f32 * 0.);
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
    water_plane_transform.translation = Vec3::new(player_translation.x - CHUNK_SIZE as f32 / 2., WATER_LEVEL, player_translation.z - CHUNK_SIZE as f32 / 2.);
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

/// Collects the results from threads spawned in `generate_terrain` and spawns the chunks.
pub(crate) fn spawn_generated_chunks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut terrain_res: ResMut<Terrain>,
    mut mesh_gen_tasks: Query<(Entity, &mut GenerateChunkMeshTask)>,
) {
    let terrain_material = terrain_res.terrain_material_handle.clone().unwrap();

    if let Some(c) = terrain_res.loaded_chunks.values_mut().find(|a| a.pos == Vec2::new(0.0, 0.0)) {
        if c.near_chunk_status.is_none() {
            c.near_chunk_status = Some(NearChunkStatus::Flagged);
        }
    }

    for (entity, mut task) in &mut mesh_gen_tasks {
        if let Some((id, chunk_pos, chunk_world_position, mesh)) = future::block_on(future::poll_once(&mut task.0)) {
            let chunk_pos_int = IVec2::new(chunk_pos.x as i32, chunk_pos.y as i32);
            if terrain_res.invalidated_chunks.contains(&id) || !terrain_res.generating_chunks.contains(&chunk_pos_int) {
                commands.entity(entity).despawn();
                terrain_res.generating_chunks.remove(&chunk_pos_int);
                continue;
            }

            let mesh_handle = meshes.add(mesh);

            // Add the chunk to the world and tag it with the FarGridTerrainChunk component
            commands.entity(entity)
                .remove::<GenerateChunkMeshTask>() // Remove the task component
                .insert(
                    MaterialMeshBundle {
                        transform: Transform::from_xyz(chunk_world_position.x, 0., chunk_world_position.y),
                        mesh: mesh_handle.clone(),
                        material: terrain_material.clone(),
                        ..default()
                    }
                )
                .insert(TerrainChunk(id))
                .insert(PickableBundle::default());

            let chunk_data = ChunkData {
                pos: chunk_pos,
                mesh_handle,
                ..default()
            };

            terrain_res.generating_chunks.remove(&chunk_pos_int);
            terrain_res.loaded_chunks.insert(id, chunk_data);
        }
    }
}

pub(crate) fn invalidate_unused_terrain(
    mut terrain_res: ResMut<Terrain>,
    player_query: Query<&Transform, With<Player>>,
    chunks: Query<(Entity, &TerrainChunk)>,
) {
    if !terrain_res.invalidated_chunks.is_empty() {
        return;
    }

    let player_transform = player_query.single();
    let player_position = Vec2::new(player_transform.translation.x, player_transform.translation.z);

    let player_chunk_x = ((player_position.x + CHUNK_SIZE as f32 / 2.) / CHUNK_SIZE as f32).floor() as i32;
    let player_chunk_y = ((player_position.y + CHUNK_SIZE as f32 / 2.) / CHUNK_SIZE as f32).floor() as i32;

    for (_, chunk) in &chunks {
        if let Some(chunk_data) = terrain_res.loaded_chunks.get(&chunk.0) {
            if (chunk_data.pos.x < player_chunk_x as f32 - FAR_GRID_RENDER_DISTANCE as f32
                || chunk_data.pos.x > player_chunk_x as f32 + FAR_GRID_RENDER_DISTANCE as f32)
                || chunk_data.pos.y < player_chunk_y as f32 - FAR_GRID_RENDER_DISTANCE as f32
                || chunk_data.pos.y > player_chunk_y as f32 + FAR_GRID_RENDER_DISTANCE as f32 {
                terrain_res.invalidated_chunks.insert(chunk.0);
            }
        }
    }
}

pub(crate) fn remove_invalidated_chunks(
    mut commands: Commands,
    mut terrain_res: ResMut<Terrain>,
    mut meshes: ResMut<Assets<Mesh>>,
    chunks: Query<(Entity, &TerrainChunk)>,
) {
    if terrain_res.invalidated_chunks.is_empty() {
        return;
    }

    for (chunk_entity, chunk) in &chunks {
        if !terrain_res.invalidated_chunks.contains(&chunk.0) {
            continue;
        }

        let chunk_data = terrain_res.loaded_chunks.get(&chunk.0).unwrap();
        commands.entity(chunk_entity).despawn();

        let mesh_handle = &chunk_data.mesh_handle;
        meshes.remove(mesh_handle);

        terrain_res.invalidated_chunks.remove(&chunk.0);
        terrain_res.loaded_chunks.remove(&chunk.0);
    }
}

pub(crate) fn is_within_near_render_distance(world_point: &Vec2, from_chunk_pos: &IVec2) -> bool {
    is_chunk_within_limit(world_point, from_chunk_pos, NEAR_GRID_RENDER_DISTANCE as i32)
}

pub(crate) fn is_within_far_render_distance(world_point: &Vec2, from_chunk_pos: &IVec2) -> bool {
    is_chunk_within_limit(world_point, from_chunk_pos, FAR_GRID_RENDER_DISTANCE as i32)
}

fn is_chunk_within_limit(world_point: &Vec2, from_chunk_pos: &IVec2, limit: i32) -> bool {
    let min_x = (from_chunk_pos.x - limit) * CHUNK_SIZE as i32;
    let max_x = (from_chunk_pos.x + limit) * CHUNK_SIZE as i32;
    let min_y = (from_chunk_pos.y - limit) * CHUNK_SIZE as i32;
    let max_y = (from_chunk_pos.y + limit) * CHUNK_SIZE as i32;

    if world_point.x > max_x as f32 || world_point.x < min_x as f32 {
        false
    } else if world_point.y > max_y as f32 || world_point.y < min_y as f32 {
        false
    } else {
        true
    }
}
