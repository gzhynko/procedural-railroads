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
use bevy::render::mesh::{PrimitiveTopology};
use bevy::render::render_asset::RenderAssets;

use bevy::window::{PresentMode, WindowPlugin};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use bevy_egui::egui::emath;
use bevy_flycam::{FlyCam, MovementSettings, NoCameraPlayerPlugin};
use crate::assets::AssetsPlugin;

use world::WorldPlugin;
use crate::noise::NoiseSettings;
use crate::rolling_stock::components::Wagon;
use crate::world::terrain::types::Terrain;

#[derive(Default, Resource)]
struct ControlsUiState {
    wireframe_enabled: bool,
    cam_follows_wagon: bool,
}

const PHYSICS_TIMESTEP: f32 = 1. / 60.;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::AutoVsync,
                ..default()
            }),
            ..default()
        }))
        .add_plugins((WireframePlugin, NoCameraPlayerPlugin, AtmospherePlugin, EguiPlugin))

        .add_plugins((
            AssetsPlugin,
            WorldPlugin,
            //RollingStockPlugin
        ))

        .insert_resource(MovementSettings {
            sensitivity: 0.00012, // default: 0.00012
            speed: 100.0, // default: 12.0
        })
        .insert_resource(NoiseSettings::default())
        .insert_resource(WireframeConfig::default())
        .insert_resource(AtmosphereModel::default())

        .insert_resource(ControlsUiState::default())

        .add_systems(Startup, setup)
        .add_systems(Update, apply_controls_settings)
        .add_systems(Update, controls_ui)
        .add_systems(Update, terrain_gen_ui)


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
    wireframe_config.global = false;

    // The Sun
    let dir_light = DirectionalLight {
        color: Color::WHITE,
        illuminance: 32000.,
        shadows_enabled: true,
        ..default()
    };
    commands.spawn(
        DirectionalLightBundle {
            transform: Transform::from_rotation(Quat::from_rotation_x(-1.)),
            directional_light: dir_light,
            ..Default::default()
        })
        .insert(Sun); // Marks the light as the Sun

    // The camera
    let mut perspective_proj = PerspectiveProjection::default();
    perspective_proj.far = 10000.; // set the far projection to be high to avoid clipping by the skybox
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 50.0, 0.0).with_rotation(Quat::from_rotation_x(-std::f32::consts::PI / 2.0)),
            projection: Projection::Perspective(perspective_proj),
            ..default()
        },
        FogSettings {
            color: Color::srgba(1.0, 1.0, 1.0, 1.0),
            falloff: FogFalloff::Linear {
                start: 4000.0,
                end: 4500.0,
            },
            ..default()
        },
    ))
        .insert(FlyCam)
        .insert(AtmosphereCamera::default())
        .insert(Player);
}

fn apply_controls_settings(
    controls_res: Res<ControlsUiState>,
    mut wireframe_config: ResMut<WireframeConfig>,

    mut cam_query: Query<&mut Transform, (With<Camera>, Without<rolling_stock::components::Bogie>)>,
    tracked_wagon_query: Query<&Transform, (With<Wagon>, With<rolling_stock::components::TrackedWagon>, Without<Camera>)>,
) {
    if controls_res.cam_follows_wagon {
        if let Ok(wagon_transform) = tracked_wagon_query.get_single() {
            let mut cam_transform = cam_query.single_mut();
            cam_transform.translation = wagon_transform.translation;
            cam_transform.translation.y += 10.0;
        }
    }

    wireframe_config.global = controls_res.wireframe_enabled;
}

fn controls_ui(
    mut egui_contexts: EguiContexts,
    mut controls_res: ResMut<ControlsUiState>,
) {
    egui::Window::new("Controls").show(egui_contexts.ctx_mut(), |ui| {
        ui.allocate_space(emath::Vec2::new(250., 0.));
        ui.set_max_width(250.0);

        ui.checkbox(&mut controls_res.wireframe_enabled, "Enable wireframe");
        ui.checkbox(&mut controls_res.cam_follows_wagon, "Camera follows wagon");
    });
}

fn terrain_gen_ui(
    mut egui_contexts: EguiContexts,
    mut noise: ResMut<NoiseSettings>,
    mut terrain_res: ResMut<Terrain>,
) {
    let mut any_changed = false;
    egui::SidePanel::right("right_panel").show(egui_contexts.ctx_mut(), |ui| {
        ui.heading("Terrain Gen Settings");

        ui.horizontal(|ui| {
            ui.label("Amplitude");
            let lost_focus = ui.add(egui::Slider::new(&mut noise.amplitude, RangeInclusive::new(0., 15.))).lost_focus();
            if lost_focus {
                any_changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Frequency");
            let lost_focus = ui.add(egui::Slider::new(&mut noise.frequency, RangeInclusive::new(0., 15.))).lost_focus();
            if lost_focus {
                any_changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Number Octaves");
            let lost_focus = ui.add(egui::Slider::new(&mut noise.num_octaves, RangeInclusive::new(1, 10))).lost_focus();
            if lost_focus {
                any_changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Terrain Horizontal Scale");
            let lost_focus = ui.add(egui::Slider::new(&mut noise.scale, RangeInclusive::new(1.0, 10.0))).lost_focus();
            if lost_focus {
                any_changed = true;
            }
        });
        if ui.button("Update").clicked() {
            any_changed = true;
        }
    });
    if any_changed {
        terrain_res.invalidate_all_chunks();
    }
}
