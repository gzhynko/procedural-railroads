use std::collections::HashSet;
use bevy::asset::{Asset, Handle};
use bevy::math::{IVec2, Vec2};
use bevy::pbr::{Material, StandardMaterialUniform};
use bevy::prelude::{AlphaMode, Component, Image, Mesh, Resource, TypePath};
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use bevy::tasks::Task;
use bevy::utils::HashMap;

/// The main terrain resource
#[derive(Resource)]
pub(crate) struct Terrain {
    /// The ID counter for unique chunk IDs.
    pub(crate) id_counter: u64,

    pub(crate) generating_chunks: HashSet<IVec2>,
    /// Stores the chunk data and maps them by ID
    pub(crate) loaded_chunks: HashMap<u64, ChunkData>,
    pub(crate) invalidated_chunks: HashSet<u64>,

    /// Stores a handle to the main terrain material.
    pub(crate) terrain_material_handle: Option<Handle<TerrainMaterial>>,

    /// Texture handle for grass.
    pub(crate) grass_texture_handle: Option<Handle<Image>>,
    /// Texture handle for rock.
    pub(crate) rock_texture_handle: Option<Handle<Image>>,
}

impl Default for Terrain {
    fn default() -> Self {
        Self {
            id_counter: 0,

            generating_chunks: HashSet::new(),
            loaded_chunks: HashMap::new(),
            invalidated_chunks: HashSet::new(),

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

    pub(crate) fn invalidate_all_chunks(&mut self) {
        for key in self.loaded_chunks.keys() {
            self.invalidated_chunks.insert(*key);
        }
        self.generating_chunks.clear();
    }
}

#[derive(PartialEq)]
pub(crate) enum NearChunkStatus { Flagged, Generating, Generated }

#[derive(Default)]
pub(crate) struct ChunkData {
    /// The position of the chunk here are relative to center.
    /// So (0, 0) will mean a chunk at position `(-TERRAIN_CHUNK_SIZE / 2., -TERRAIN_CHUNK_SIZE / 2.)`.
    pub(crate) pos: Vec2,

    pub(crate) mesh_handle: Handle<Mesh>,
    pub(crate) near_mesh_handles: Vec<Handle<Mesh>>,

    /// A boolean that tracks whether this far chunk has been flagged for near chunk generation.
    /// As soon as the track midline has exited a far chunk, the chunk is flagged for it to have its near grid generated.
    /// Then, if generate_near_terrain sees a chunk that has been flagged and is within the near chunk render distance, it starts generating near grid for that chunk.
    pub(crate) near_chunk_status: Option<NearChunkStatus>,

    pub(crate) midline_entry_node_id: Option<usize>,
    pub(crate) midline_exit_node_id: Option<usize>,
}

#[derive(Component)]
pub(crate) struct TerrainChunk(pub(crate) u64);

#[derive(Component)]
pub(crate) struct GenerateChunkMeshTask(pub(crate) Task<(u64, Vec2, Vec2, Mesh)>);

/// Marker to update water plane position
#[derive(Component)]
pub(crate) struct WaterPlane;

pub(crate) enum MeshSubdivisionLevel { Far = 1, Near1 = 4, Near2 = 8 }

#[derive(Asset, AsBindGroup, Clone, TypePath)]
pub(crate) struct TerrainMaterial {
    #[uniform(0)]
    pub(crate) grass_pbr_material: StandardMaterialUniform,
    #[uniform(1)]
    pub(crate) rock_pbr_material: StandardMaterialUniform,

    #[texture(2)]
    #[sampler(3)]
    pub(crate) grass_albedo_texture: Option<Handle<Image>>,
    #[texture(4)]
    #[sampler(5)]
    pub(crate) rock_albedo_texture: Option<Handle<Image>>,
}

impl Material for TerrainMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/terrain_texturing.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::AlphaToCoverage
    }
}
