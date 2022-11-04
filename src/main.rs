mod terrain_generator;
mod noise;
mod lines;
mod road_generator;

use std::ops::RangeInclusive;
use bevy::pbr::StandardMaterialUniform;
use bevy::pbr::wireframe::{Wireframe, WireframeConfig, WireframePlugin};
use bevy::prelude::*;
use bevy_atmosphere::prelude::*;
use bevy::prelude::shape::Cube;
use bevy::reflect::{TypeUuid, Uuid};
use bevy::render::camera::Projection;
use bevy::render::mesh::{Indices, MeshVertexAttribute, PrimitiveTopology};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{AddressMode, AsBindGroup, AsBindGroupError, BindGroupLayout, PreparedBindGroup, SamplerDescriptor, ShaderRef};
use bevy::render::renderer::RenderDevice;
use bevy::render::texture::{FallbackImage, ImageSettings};
use bevy::window::{PresentMode, WindowPlugin};
use bevy_egui::{egui, EguiContext, EguiPlugin};
use bevy_flycam::{FlyCam, MovementSettings, NoCameraPlayerPlugin, PlayerPlugin};
use bevy::render::extract_resource::ExtractResource;
use bevy::render::render_resource::ShaderType;
use crate::road_generator::RoadPlugin;

use crate::terrain_generator::{Terrain, TerrainPlugin};

const SEED: u32 = 1354251456;

#[derive(Copy, Clone)]
struct NoiseSettings {
    octaves: usize,
    amplitude: f64,
    frequency: f32,
    persistence: f64,
    lacunarity: f32,
    scale: (f64, f64),
    bias: f64,
    seed: u32,
}

impl Default for NoiseSettings {
    fn default() -> Self {
        Self {
            octaves: 2,
            amplitude: 10.,
            frequency: 1.0,
            persistence: 1.0,
            lacunarity: 1.0,
            scale: (350., 350.),
            bias: 1.0,
            seed: SEED
        }
    }
}

#[derive(Default)]
struct UiState {

}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(WireframePlugin)
        .add_plugin(NoCameraPlayerPlugin)
        .add_plugin(AtmospherePlugin)
        .add_plugin(TerrainPlugin)
        .add_plugin(RoadPlugin)
        .add_plugin(EguiPlugin)
        .add_plugin(MaterialPlugin::<TerrainMaterial>::default())

        .insert_resource(Msaa { samples: 4 })
        .insert_resource(WindowDescriptor {
            present_mode: PresentMode::AutoVsync,
            ..default()
        })
        .insert_resource(MovementSettings {
            sensitivity: 0.00012, // default: 0.00012
            speed: 100.0, // default: 12.0
        })
        .insert_resource(NoiseSettings::default())
        .insert_resource(Atmosphere::default()) // Default Atmosphere material, we can edit it to simulate another planet
        .insert_resource(CycleTimer(Timer::new(
            bevy::utils::Duration::from_millis(10), // Update our atmosphere every 50ms (in a real game, this would be much slower, but for the sake of an example we use a faster update)
            true,
        )))

        .add_startup_system(setup)
        .add_system(ui)
        .add_system(daylight_cycle)

        .run();
}

/// Marker for updating the position of the global light
#[derive(Component)]
struct Sun;

/// Marker to track Player position
#[derive(Component)]
struct Player;

// Timer for updating the daylight cycle (updating the atmosphere every frame is slow, so it's better to do incremental changes)
struct CycleTimer(Timer);

// We can edit the Atmosphere resource and it will be updated automatically
fn daylight_cycle(
    mut atmosphere: ResMut<Atmosphere>,
    mut query: Query<(&mut Transform, &mut DirectionalLight), With<Sun>>,
    mut timer: ResMut<CycleTimer>,
    time: Res<Time>,
) {
    timer.0.tick(time.delta());

    if timer.0.finished() {
        let t: f32 = 40.;
        atmosphere.sun_position = Vec3::new(0., t.sin(), t.cos());

        if let Some((mut light_trans, mut directional)) = query.single_mut().into() {
            light_trans.rotation = Quat::from_rotation_x(-t.sin().atan2(t.cos()));
            directional.illuminance = t.sin().max(0.0).powf(2.0) * 50000.0;
        }
    }
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut wireframe_config: ResMut<WireframeConfig>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    //wireframe_config.global = true;

    // Our Sun
    let dir_light = DirectionalLight{
        color: Color::WHITE,
        illuminance: 32000.,
        shadows_enabled: true,
        ..default()
    };
    commands
        .spawn_bundle(DirectionalLightBundle {
            transform: Transform::from_rotation(Quat::from_rotation_x(-1.)),
            directional_light: dir_light,
            ..Default::default()
        })
        .insert(Sun); // Marks the light as Sun

    // cube
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(Cube::new(1.0))),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });

    // camera
    let mut perspective_proj = PerspectiveProjection::default();
    perspective_proj.far = 5000.;
    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        projection: Projection::Perspective(perspective_proj),
        ..default()
    })
        .insert(FlyCam)
        .insert(AtmosphereCamera(None))
        .insert(Player);
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

impl Material for TerrainMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/terrain_texturing.wgsl".into()
    }
}

#[derive(AsBindGroup, Debug, Clone, Default, ExtractResource, ShaderType)]
struct Fog {
    color: Vec4,
    density_or_start: f32,
    end: f32,
}

#[uuid = "b62bb455-a72c-4b56-87bb-81e0554e234f"]
#[derive(AsBindGroup, Clone, TypeUuid)]
pub struct TerrainMaterial {
    #[uniform(0)]
    fog: Fog,

    #[uniform(1)]
    grass_pbr_material: StandardMaterialUniform,
    #[uniform(2)]
    rock_pbr_material: StandardMaterialUniform,

    #[texture(3)]
    #[sampler(4)]
    grass_albedo_texture: Option<Handle<Image>>,
    #[texture(5)]
    #[sampler(6)]
    rock_albedo_texture: Option<Handle<Image>>,

    #[texture(7)]
    #[sampler(8)]
    grass_normal_texture: Option<Handle<Image>>,
    #[texture(9)]
    #[sampler(10)]
    rock_normal_texture: Option<Handle<Image>>,
}
