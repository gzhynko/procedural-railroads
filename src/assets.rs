use bevy::gltf::Gltf;
use bevy::prelude::*;
use bevy_asset_loader::prelude::*;

pub(crate) struct AssetsPlugin;

impl Plugin for AssetsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_state(AssetLoadingState::AssetsLoading)
            .add_loading_state(
            LoadingState::new(AssetLoadingState::AssetsLoading)
                .continue_to_state(AssetLoadingState::AssetsLoaded)
                .with_dynamic_collections::<StandardDynamicAssetCollection>(vec![
                    "textures.assets",
                    "models.assets",
                ])
                .with_collection::<TextureAssets>()
                .with_collection::<ModelAssets>()
            );
    }
}

#[derive(AssetCollection, Resource)]
pub(crate) struct TextureAssets {
    #[asset(key = "textures.terrain.grass")]
    pub(crate) terrain_grass: Handle<Image>,
    #[asset(key = "textures.terrain.rock")]
    pub(crate) terrain_rock: Handle<Image>,
}

#[derive(AssetCollection, Resource)]
pub(crate) struct ModelAssets {
    #[asset(key = "models.track.cross_section")]
    pub(crate) track_cross_section: Handle<Gltf>,
}

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
pub(crate) enum AssetLoadingState {
    AssetsLoading,
    AssetsLoaded,
}