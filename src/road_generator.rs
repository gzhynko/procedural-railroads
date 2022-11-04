use crate::{App, Assets, Color, Commands, default, MaterialMeshBundle, MaterialPlugin, Mesh, noise, NoiseSettings, Player, Plugin, Query, Res, ResMut, Transform, Vec2, Vec3, With};
use crate::lines::{LineMaterial, LineStrip};
use crate::terrain_generator::{RENDER_DISTANCE_CHUNKS, TERRAIN_CHUNK_SIZE};

/// The size of each road node
const NODE_LENGTH: f32 = 20.;
/// The maximum allowed turn angle between each successive nodes in degrees
const MAX_TURN_ANGLE: f32 = 45.; // pi / 4

pub(crate) struct RoadPlugin;

struct Road {
    polyline_points: Vec<Vec3>,
}

impl Default for Road {
    fn default() -> Self {
        Self {
            polyline_points: Vec::new(),
        }
    }
}

impl Plugin for RoadPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugin(MaterialPlugin::<LineMaterial>::default())
            .insert_resource(Road::default())

            .add_startup_system(init_line_points)
            .add_startup_system(init_render)
            .add_system(update_polyline_points);
            //.add_system(build_road_path);
    }
}

/// Setup the polyline entity
fn init_render(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<LineMaterial>>,
) {

}

/// Determine and set the first point of the road.
fn init_line_points(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<LineMaterial>>,

    mut road_res: ResMut<Road>,
    noise_settings: Res<NoiseSettings>,
) {
    let noise_fn = noise::get_heightmap_function(TERRAIN_CHUNK_SIZE as f32, noise_settings.clone());
    let starting_point_2d = Vec2::new(0., 0.);
    let starting_height = noise_fn(starting_point_2d.x as f64, starting_point_2d.y as f64) as f32;
    let starting_point = Vec3::new(starting_point_2d.x, starting_height + 1., starting_point_2d.y);

    let mut result = Vec3::ZERO;
    let mut current_min_slope = 1000.; // arbitrarily large number
    for angle_deg in (-180..181).step_by(5) {
        let angle_rad = f32::to_radians(angle_deg as f32);
        let x = NODE_LENGTH * angle_rad.cos();
        let y = NODE_LENGTH * angle_rad.sin();
        let this_pos = Vec2::new(x, y) + starting_point_2d;

        let height_here = noise_fn(this_pos.x as f64, this_pos.y as f64) as f32;
        let slope = calc_absolute_slope(this_pos.distance(starting_point_2d), starting_height, height_here);
        if slope < current_min_slope {
            current_min_slope = slope;
            result = Vec3::new(this_pos.x, height_here, this_pos.y);
        }
    }

    road_res.polyline_points.push(starting_point);
    road_res.polyline_points.push(result);

    commands.spawn_bundle(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(LineStrip {
            points: road_res.polyline_points.clone(),
        })),
        material: materials.add(LineMaterial { color: Color::RED }),
        ..default()
    });
}

fn update_polyline_points(
    mut commands: Commands,
    mut road_res: ResMut<Road>,
) {

}

/// Builds the rail road path for the generated chunks.
fn build_road_path(
    mut commands: Commands,
    mut road_res: ResMut<Road>,

    player_query: Query<&Transform, With<Player>>,
    noise_settings: Res<NoiseSettings>,
) {
    let last_road_point = road_res.polyline_points.last().unwrap();

    let player_transform = player_query.single();
    let player_position = Vec2::new(player_transform.translation.x, player_transform.translation.z);

    let player_chunk_x = ((player_position.x + TERRAIN_CHUNK_SIZE as f32 / 2.) / TERRAIN_CHUNK_SIZE as f32).floor() as i32;
    let player_chunk_y = ((player_position.y + TERRAIN_CHUNK_SIZE as f32 / 2.) / TERRAIN_CHUNK_SIZE as f32).floor() as i32;
    let player_chunk_pos = Vec2::new(player_chunk_x as f32, player_chunk_y as f32);
    // Do not proceed if outside of render distance
    if !is_within_render_distance(Vec2::new(last_road_point.x, last_road_point.z), RENDER_DISTANCE_CHUNKS as f32, player_chunk_pos, TERRAIN_CHUNK_SIZE as f32) {
        return;
    }

    if road_res.polyline_points.len() == 1 {
        
    }
}

fn calc_absolute_slope(dist: f32, height1: f32, height2: f32) -> f32 {
    ((height2 - height1) / dist).abs()
}

fn is_within_render_distance(pos: Vec2, render_distance_chunks: f32, player_chunk_pos: Vec2, chunk_size: f32) -> bool {
    let min_x = (player_chunk_pos.x - render_distance_chunks) * chunk_size;
    let max_x = (player_chunk_pos.x + render_distance_chunks) * chunk_size;
    let min_y = (player_chunk_pos.y - render_distance_chunks) * chunk_size;
    let max_y = (player_chunk_pos.y + render_distance_chunks) * chunk_size;

    if pos.x > max_x || pos.x < min_x {
        false
    } else if pos.y > max_y || pos.y < min_y {
        false
    } else {
        true
    }
}