use bevy::gltf::Gltf;
use bevy::prelude::*;
use bevy_asset_loader::prelude::*;

pub(crate) struct AssetsPlugin;

impl Plugin for AssetsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_state::<AssetLoadingState>()
            .add_loading_state(
            LoadingState::new(AssetLoadingState::AssetsLoading)
                .continue_to_state(AssetLoadingState::AssetsLoaded)
            )

            .add_dynamic_collection_to_loading_state::<_, StandardDynamicAssetCollection>(AssetLoadingState::AssetsLoading, "textures.assets.ron")
            .add_dynamic_collection_to_loading_state::<_, StandardDynamicAssetCollection>(AssetLoadingState::AssetsLoading, "models.assets.ron")
            .add_collection_to_loading_state::<_, TextureAssets>(AssetLoadingState::AssetsLoading)
            .add_collection_to_loading_state::<_, ModelAssets>(AssetLoadingState::AssetsLoading);

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
    #[asset(key = "models.train.wagon_bogie")]
    pub(crate) train_wagon_bogie: Handle<Scene>,
    #[asset(key = "models.train.gondola")]
    pub(crate) train_gondola: Handle<Scene>,
}

#[derive(States, Default, Clone, Eq, PartialEq, Debug, Hash)]
pub(crate) enum AssetLoadingState {
    #[default]
    AssetsLoading,
    AssetsLoaded,
}
