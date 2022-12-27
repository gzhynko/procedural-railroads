mod noise;
mod lines;
mod assets;
mod rolling_stock;
mod world;

use std::ops::RangeInclusive;
use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};
use bevy::prelude::*;
use bevy_atmosphere::prelude::*;
use bevy::render::camera::Projection;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{AddressMode, SamplerDescriptor};

use bevy::window::{PresentMode, WindowPlugin};
use bevy_egui::{egui, EguiContext, EguiPlugin};
use bevy_flycam::{FlyCam, MovementSettings, NoCameraPlayerPlugin};
use crate::assets::AssetsPlugin;
use world::route_gen::RouteGenerationPlugin;

use world::terrain::{Terrain, TerrainPlugin};
use world::train_tracks::TrackPlacementPlugin;
use crate::rolling_stock::{RollingStockPlugin};

const SEED: u32 = 1354251456;

#[derive(Copy, Clone, Resource)]
struct NoiseSettings {
    amplitude: f64,
    frequency: f32,
    scale: (f64, f64),
    seed: u32,
}

impl Default for NoiseSettings {
    fn default() -> Self {
        Self {
            amplitude: 25.,
            frequency: 1.0,
            scale: (700., 700.),
            seed: SEED
        }
    }
}

#[derive(Default, Resource)]
struct UiState {

}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                present_mode: PresentMode::AutoVsync,
                ..default()
            },
            ..default()
        }))
        .add_plugin(WireframePlugin)
        .add_plugin(NoCameraPlayerPlugin)
        .add_plugin(AtmospherePlugin)
        .add_plugin(EguiPlugin)

        .add_plugin(AssetsPlugin)
        .add_plugin(TerrainPlugin)
        .add_plugin(RouteGenerationPlugin)
        .add_plugin(TrackPlacementPlugin)
        .add_plugin(RollingStockPlugin)

        .insert_resource(Msaa { samples: 4 })
        .insert_resource(MovementSettings {
            sensitivity: 0.00012, // default: 0.00012
            speed: 100.0, // default: 12.0
        })
        .insert_resource(NoiseSettings::default())
        .insert_resource(WireframeConfig::default())

        .add_startup_system(setup)
        .add_system(move_sun)
        .add_system(move_cam)
        //.add_system(ui)

        .run();
}

/// Marker for updating the position of the global light
#[derive(Component)]
struct Sun;

/// Marker to track Player position
#[derive(Component)]
struct Player;

fn setup(
    mut commands: Commands,
    mut wireframe_config: ResMut<WireframeConfig>,
) {
    // Enable/Disable the wireframe globally
    wireframe_config.global = true;

    // The Sun
    let dir_light = DirectionalLight{
        color: Color::WHITE,
        illuminance: 32000.,
        shadows_enabled: true,
        ..default()
    };
    commands
        .spawn(DirectionalLightBundle {
            transform: Transform::from_rotation(Quat::from_rotation_x(-1.)),
            directional_light: dir_light,
            ..Default::default()
        })
        .insert(Sun); // Marks the light as the Sun

    // The camera
    let mut perspective_proj = PerspectiveProjection::default();
    perspective_proj.far = 5000.; // set the far projection to be high to avoid clipping by the skybox
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        projection: Projection::Perspective(perspective_proj),
        ..default()
    })
        .insert(FlyCam)
        .insert(AtmosphereCamera::default())
        .insert(Player);
}

// Move the sun to make sure shadows get applied near the player.
// TODO: Maybe remove this when cascaded shadow maps get merged?
fn move_sun(
    mut sun_query: Query<&mut Transform, With<Sun>>,
    player_query: Query<&Transform, (With<Player>, Without<Sun>)>,
) {
    let player_transform = player_query.single();
    let mut sun_transform = sun_query.single_mut();
    sun_transform.translation = player_transform.translation;
}

fn move_cam(
    mut cam_query: Query<&mut Transform, (With<Camera>, Without<rolling_stock::components::Bogie>)>,
    bogies_query: Query<&Transform, (With<rolling_stock::components::Bogie>, Without<Camera>)>,
) {
    if bogies_query.is_empty() {
        return;
    }

    let mut cam = cam_query.single_mut();
    //cam.translation = bogies_query.iter().next().unwrap().translation;
    //cam.translation.y += 10.;
}

fn ui(
    mut egui_context: ResMut<EguiContext>,
    mut noise: ResMut<NoiseSettings>,
    mut terrain_res: ResMut<Terrain>,
) {
    let mut any_changed = false;
    egui::SidePanel::right("right_panel").show(egui_context.ctx_mut(), |ui| {
        ui.heading("Terrain Gen Settings");

        ui.horizontal(|ui| {
            ui.label("Amplitude");
            let modified = ui.add(egui::Slider::new(&mut noise.amplitude, RangeInclusive::new(0., 15.))).changed();
            if modified {
                any_changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Frequency");
            let modified = ui.add(egui::Slider::new(&mut noise.frequency, RangeInclusive::new(0., 15.))).changed();
            if modified {
                any_changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Scale (x, y)");
            let modified_x = ui.add(egui::Slider::new(&mut noise.scale.0, RangeInclusive::new(0.01, 1000.))).changed();
            let modified_y = ui.add(egui::Slider::new(&mut noise.scale.1, RangeInclusive::new(0.01, 1000.))).changed();
            if modified_x || modified_y {
                any_changed = true;
            }
        });
    });
    if any_changed {
        terrain_res.loaded_chunks_pos.clear();
    }
}
