use bevy::color::palettes::css::RED;
use crate::{Assets, Component, Commands, default, MaterialMeshBundle, Mesh, noise, NoiseSettings, Player, Query, Res, ResMut, Transform, Vec2, Vec3, With, Entity, Resource};
use crate::lines::{LineMaterial, LineStrip};
use crate::world::terrain;
use crate::world::terrain::{FAR_GRID_CHUNK_SIZE, is_within_far_render_distance};

/// The distance between each route node
const NODE_LENGTH: f32 = 50.;
/// The maximum allowed turn angle between each successive nodes in degrees
const MAX_TURN_ANGLE: i32 = 5;

#[derive(Component)]
pub(crate) struct RouteNode;

#[derive(Resource)]
pub(crate) struct Route {
    pub id_counter: usize,
    points: Vec<Vec3>,
    points_changed: bool,
}

impl Route {
    pub fn get_last_id(&self) -> usize {
        self.id_counter - 1
    }

    pub fn get_point(&self, id: usize) -> Option<&Vec3> {
        if id >= self.points.len() { None }
        else { Some(&self.points[id]) }
    }

    pub fn get_cloned_points(&self) -> Vec<Vec3> {
        self.points.clone()
    }
}

impl Default for Route {
    fn default() -> Self {
        Self {
            id_counter: 0,
            points: Vec::new(),
            points_changed: false,
        }
    }
}

/// Determine and set the first point of the route.
pub(crate) fn init_line_points(
    mut route_res: ResMut<Route>,
    noise_settings: Res<NoiseSettings>,
) {
    let noise_fn = noise::get_heightmap_function(FAR_GRID_CHUNK_SIZE as f32, noise_settings.clone(), Vec3::ZERO);
    let starting_point_2d = Vec2::new(0., 0.);
    let starting_height = noise_fn(starting_point_2d.x as f64, starting_point_2d.y as f64) as f32;
    let starting_point = Vec3::new(starting_point_2d.x, starting_height + 1., starting_point_2d.y);

    let next_point = find_next_path_node(noise_fn, starting_point, 0, 180, 5);

    route_res.points.insert(0, starting_point);
    route_res.points.insert(1, next_point);
    route_res.id_counter = 2;
    route_res.points_changed = true;
}

pub(crate) fn update_polyline_points(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<LineMaterial>>,
    mut route_res: ResMut<Route>,

    route_node_query: Query<Entity, With<RouteNode>>,
) {
    if !route_res.points_changed { return; }

    for entity in &route_node_query {
        commands.entity(entity).despawn();
        //TODO: also remove associated meshes, this might be causing a memory leak.
    }

    let point_array: Vec<Vec3> = route_res.get_cloned_points();

    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(LineStrip {
            points: point_array,
        })),
        material: materials.add(LineMaterial { color: RED.into() }),
        ..default()
    }).insert(RouteNode);

    route_res.points_changed = false;
}

/// Builds the track route for the generated chunks.
pub(crate) fn build_route_path(
    mut route_res: ResMut<Route>,

    player_query: Query<&Transform, With<Player>>,
    noise_settings: Res<NoiseSettings>,
) {
    let noise_fn = noise::get_heightmap_function(FAR_GRID_CHUNK_SIZE as f32, noise_settings.clone(), Vec3::ZERO);
    let current_node_id = route_res.id_counter;

    let player_transform = player_query.single();
    let player_world_position = Vec2::new(player_transform.translation.x, player_transform.translation.z);

    let player_chunk_pos = terrain::get_far_chunk_position(player_world_position);
    let last_route_point = route_res.get_point(current_node_id - 1).unwrap().clone();
    // Do not proceed if outside of render distance
    if !is_within_far_render_distance(&Vec2::new(last_route_point.x, last_route_point.z), &player_chunk_pos) {
        return;
    }

    let route_point_before_last = route_res.get_point(current_node_id - 2).unwrap();
    let route_vector = Vec2::new(last_route_point.x - route_point_before_last.x, last_route_point.z - route_point_before_last.z);
    let world_vector = Vec2::new(1.0, 0.0);
    let angle = (route_vector.dot(world_vector) / (route_vector.length() * 1.0)).acos().to_degrees() as i32;

    let next_route_point = find_next_path_node(noise_fn, last_route_point, angle, MAX_TURN_ANGLE, 1);
    route_res.points.insert(current_node_id, next_route_point);
    route_res.id_counter += 1;
    route_res.points_changed = true;
}

/// Calculates the next node in the route path by taking the route with lowest slope
pub(crate) fn find_next_path_node<F>(noise_fn: F, starting_point: Vec3, starting_absolute_angle_deg: i32, max_angle_deg: i32, angle_step_deg: usize) -> Vec3
    where F: Fn(f64, f64) -> f64 {
    let mut result = Vec3::ZERO;
    let mut current_min_slope = 1000.; // arbitrarily large number
    let starting_point_2d = Vec2::new(starting_point.x, starting_point.z);
    for angle_deg in ((starting_absolute_angle_deg - max_angle_deg)..(starting_absolute_angle_deg + max_angle_deg + 1)).step_by(angle_step_deg) {
        let angle_rad = f32::to_radians(angle_deg as f32);
        let x = NODE_LENGTH * angle_rad.cos();
        let y = NODE_LENGTH * angle_rad.sin();
        let this_pos = Vec2::new(x, y) + starting_point_2d;

        let height_here = noise_fn(this_pos.x as f64, this_pos.y as f64) as f32;
        //if height_here <= WATER_LEVEL {
        //    continue;
        //}
        let slope = calc_absolute_slope(this_pos.distance(starting_point_2d), starting_point.y, height_here);
        if slope < current_min_slope {
            current_min_slope = slope;
            result = Vec3::new(this_pos.x, height_here, this_pos.y);
        }
    }

    result
}

fn calc_absolute_slope(dist: f32, height1: f32, height2: f32) -> f32 {
    ((height2 - height1) / dist).abs()
}
