use bevy::gltf::Gltf;
use bevy::prelude::*;
use bevy_asset_loader::prelude::*;

pub(crate) struct AssetsPlugin;

impl Plugin for AssetsPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_state::<AssetLoadingState>()
            .add_loading_state(
                LoadingState::new(AssetLoadingState::AssetsLoading)
                    .continue_to_state(AssetLoadingState::AssetsLoaded)
                    .with_dynamic_assets_file::<StandardDynamicAssetCollection>("textures.assets.ron")
                    .with_dynamic_assets_file::<StandardDynamicAssetCollection>("models.assets.ron")
                    .load_collection::<TextureAssets>()
                    .load_collection::<ModelAssets>()
            );
    }
}

#[derive(AssetCollection, Resource)]
pub(crate) struct TextureAssets {
    #[asset(key = "textures.terrain.temp")]
    pub(crate) terrain_temp: Handle<Image>,
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
