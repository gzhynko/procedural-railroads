use bevy::prelude::*;
use crate::assets::AssetLoadingState;
use crate::lines::LineMaterial;

use crate::world::route_gen::*;
use crate::world::terrain::*;
use crate::world::train_tracks::*;

pub mod terrain;
pub mod route_gen;
pub mod train_tracks;
mod utils;

/// Responsible for routing through terrain, generating terrain mesh, and placing rail tracks.
pub(crate) struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(MaterialPlugin::<LineMaterial>::default())
            .add_plugins(MaterialPlugin::<TerrainMaterial>::default())

            .insert_resource(Route::default())
            .insert_resource(Terrain::default())
            .insert_resource(PlacementData::default())

            // startup systems
            .add_systems(Startup, init_line_points)
            .add_systems(OnEnter(AssetLoadingState::AssetsLoaded), (setup_terrain, setup_water))
            .add_systems(OnEnter(AssetLoadingState::AssetsLoaded),
                         (spawn_track_entity, setup_track_data, setup_track_material))

            // update systems
            .add_systems(Update, update_polyline_points)
            .add_systems(Update, build_route_path)
            .add_systems(Update,
                         (spawn_generated_chunks, generate_terrain, remove_unused_terrain, update_water_plane, configure_terrain_images)
                             .run_if(in_state(AssetLoadingState::AssetsLoaded)))
            .add_systems(Update,
                         (update_placement_data, update_track_entity, place_tracks)
                             .run_if(in_state(AssetLoadingState::AssetsLoaded)));
    }
}

