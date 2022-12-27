use bevy::gltf::{Gltf, GltfMesh};
use bevy::prelude::*;
use bevy::utils::hashbrown::HashMap;
use bevy_extrude_mesh::bezier::{BezierCurve, OrientedPoint};
use bevy_extrude_mesh::extrude;
use bevy_extrude_mesh::extrude::ExtrudeShape;
use crate::assets::{AssetLoadingState, ModelAssets};
use crate::{noise, NoiseSettings};
use crate::world::route_gen::Route;
use crate::world::terrain::TERRAIN_CHUNK_SIZE;

pub(crate) struct TrackPlacementPlugin;

#[derive(Clone)]
struct TrackSegment {
    id: u64,
    points: Vec<OrientedPoint>,
    world_translation: Vec3,
}

impl TrackSegment {
    fn get_closest_point_at_t(&self, t: f32) -> (usize, &OrientedPoint) {
        let mut closest_point: Option<&OrientedPoint> = None;
        let mut closest_index = 0;
        let mut smallest_diff = 1.;
        for (i, point) in self.points.iter().enumerate() {
            let diff = (t - point.t).abs();
            if diff < smallest_diff {
                closest_index = i;
                closest_point = Some(point);
                smallest_diff = diff;
            }
        }

        (closest_index, closest_point.unwrap())
    }

    pub fn get_interpolated_position_at_t(&self, t: f32) -> (Vec3, Quat) {
        let (closest_index, closest_point) = self.get_closest_point_at_t(t);

        // Interpolate the position and rotation
        if closest_point.t > t {
            let prev = &self.points[closest_index - 1];
            let lerp_factor = t * (self.points.len() as f32 - 1.) - (closest_index as f32 - 1.);
            let position = Vec3::lerp(prev.position, closest_point.position, lerp_factor);
            let rotation = Quat::lerp(prev.rotation, closest_point.rotation, lerp_factor);
            (position, rotation)
        } else {
            let next = &self.points[closest_index + 1];
            let lerp_factor = t * (self.points.len() as f32 - 1.) - closest_index as f32;
            let position = Vec3::lerp(closest_point.position, next.position, lerp_factor);
            let rotation = Quat::lerp(closest_point.rotation, next.rotation, lerp_factor);
            (position, rotation)
        }
    }

    pub fn get_slope_angle_at_t(&self, t: f32) -> f32 {
        let (closest_index, closest_point) = self.get_closest_point_at_t(t);

        // Interpolate the position and rotation
        if closest_point.t > t {
            let prev = &self.points[closest_index - 1];
            let sine = (closest_point.position.y - prev.position.y) / Vec3::distance(prev.position, closest_point.position);
            sine.asin()
        } else {
            let next = &self.points[closest_index + 1];
            let sine = (next.position.y - closest_point.position.y) / Vec3::distance(closest_point.position, next.position);
            sine.asin()
        }
    }
}

#[derive(Resource, Default)]
struct PlacementData {
    track_shape: Option<ExtrudeShape>,
    track_material: Option<Handle<StandardMaterial>>,

    segments: Vec<TrackSegment>,
    last_placed_segment_id: u64,
    last_used_node_id: u64,
}

#[derive(Component, Default)]
pub(crate) struct Track {
    /// Used to sample the t value (used by train bogies).
    segments: HashMap<u32, TrackSegment>,
}

impl Track {
    fn get_segment_at_t(&self, t: f32) -> Option<&TrackSegment> {
        assert!(t >= 0., "t wasn't a positive number (shouldn't actually happen)");
        let lower_bound = t.floor() as u32;

        self.segments.get(&lower_bound)
    }

    pub fn get_interpolated_position_at_t(&self, t: f32) -> Option<(Vec3, Quat)> {
        let segment = self.get_segment_at_t(t);
        if segment.is_some() {
            let segment = segment.unwrap();
            let lower_bound = t.floor() as u32;
            let (mut position, rotation) = segment.get_interpolated_position_at_t(t - lower_bound as f32).clone();
            position += segment.world_translation;
            Some((position, rotation))
        } else {
            None
        }
    }

    pub fn get_slope_angle_at_t(&self, t: f32) -> Option<f32> {
        let segment = self.get_segment_at_t(t);
        let lower_bound = t.floor() as u32;
        if segment.is_some() {
            let segment = segment.unwrap();
            Some(segment.get_slope_angle_at_t(t - lower_bound as f32))
        } else {
            None
        }
    }
}

impl PlacementData {
    fn current_segment_id(&self) -> u64 {
        if self.segments.is_empty() {
            0
        } else {
            self.segments[self.segments.len() - 1].id
        }
    }
}

impl Plugin for TrackPlacementPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(PlacementData::default())

            // startup
            .add_system_set(
                SystemSet::on_enter(AssetLoadingState::AssetsLoaded)
                    .with_system(spawn_track_entity)
                    .with_system(setup_track_data)
                    .with_system(setup_track_material)
            )

            // update
            .add_system_set(
                SystemSet::on_update(AssetLoadingState::AssetsLoaded)
                    .with_system(update_placement_data)
                    .with_system(update_track_entity)
                    .with_system(place_tracks)
            );
    }
}

fn spawn_track_entity(
    mut commands: Commands,
) {
    commands.spawn_empty()
        .insert(Track::default());
}

fn setup_track_data(
    mut data_res: ResMut<PlacementData>,
    model_assets: Res<ModelAssets>,

    meshes: Res<Assets<Mesh>>,
    gltf_assets: Res<Assets<Gltf>>,
    gltf_mesh_assets: Res<Assets<GltfMesh>>,
) {
    if let Some(gltf) = gltf_assets.get(&model_assets.track_cross_section) {
        let track_gltf_mesh = gltf_mesh_assets.get(&gltf.named_meshes["TrackCrossSection"]).unwrap();
        let track_mesh = meshes.get(&track_gltf_mesh.primitives[0].mesh).unwrap();
        let extrude_shape = ExtrudeShape::from_mesh(track_mesh);
        data_res.track_shape = Some(extrude_shape);
    }
}

fn setup_track_material(
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut data_res: ResMut<PlacementData>,
) {
    let material = StandardMaterial {
        ..default()
    };
    let handle = materials.add(material);

    data_res.track_material = Some(handle);
}

fn update_track_entity(
    mut track_query: Query<&mut Track>,
    placement_data_res: Res<PlacementData>,
) {
    // TODO: add support for multiple tracks
    let mut track = track_query.single_mut();

    // TODO: (IMPORTANT) optimize this
    track.segments = HashMap::new();
    for segment in &placement_data_res.segments {
        track.segments.insert(segment.id as u32, segment.clone());
    }
}

// Updates the placement data one TrackSegment per run.
fn update_placement_data(
    mut data_res: ResMut<PlacementData>,
    route_res: Res<Route>,
    noise_settings: Res<NoiseSettings>,
) {
    let id_to_add = if data_res.last_used_node_id == 0 { 2 } else { data_res.last_used_node_id + 1 };
    if route_res.get_last_id() <= 2 || route_res.get_last_id() == data_res.last_used_node_id || route_res.get_next_point(id_to_add).is_none() {
        return;
    }

    // We need at least four points to get the direction stuff right.
    let next_node = route_res.get_next_point(id_to_add).unwrap();
    let new_node = route_res.points.get(&id_to_add).unwrap();
    let last_node = route_res.points.get(&(id_to_add - 1)).unwrap();
    let previous_node = route_res.points.get(&(id_to_add - 2)).unwrap();

    // Calculate the bezier points (the vertices will be positioned relative to zero)
    let bezier_start = Vec3::ZERO;
    let bezier_end = new_node.clone() - last_node.clone();
    let (bezier_control1, bezier_control2) = find_control_points(last_node.clone(), new_node.clone(), Some(previous_node.clone()), Some(next_node.clone()), last_node.clone());
    let bezier_curve = BezierCurve::new(vec![bezier_start, bezier_control1, bezier_control2, bezier_end]);

    // Generate the path using the noise function as the height function
    let height_fn = noise::get_heightmap_function(TERRAIN_CHUNK_SIZE as f32, noise_settings.clone(), Vec3::new(last_node.x, -last_node.y + 0.3, last_node.z));
    let path = bezier_curve.generate_path_with_custom_height_function(20, height_fn);

    // Push the new segment to the resource
    let segment = TrackSegment {
        id: data_res.current_segment_id() + 1,
        points: path,
        world_translation: last_node.clone(),
    };
    data_res.segments.push(segment);
    data_res.last_used_node_id = id_to_add;
}

fn place_tracks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut placement_data: ResMut<PlacementData>,
) {
    if placement_data.track_shape.is_none() || placement_data.track_material.is_none() {
        return;
    }
    if placement_data.last_placed_segment_id + 1 > placement_data.current_segment_id() {
        return;
    }

    let id_to_place = placement_data.last_placed_segment_id + 1;

    let segment = placement_data.segments.iter().find(|seg| seg.id == id_to_place).unwrap();
    let points = &segment.points;

    let translation = segment.world_translation;

    let mesh = extrude::extrude(placement_data.track_shape.as_ref().unwrap(), points);
    let handle = meshes.add(mesh);
    commands.spawn(PbrBundle {
        mesh: handle,
        material: placement_data.track_material.clone().unwrap(),
        transform: Transform::from_translation(translation),
        ..default()
    });

    placement_data.last_placed_segment_id = id_to_place;
}

fn find_control_points(start: Vec3, end: Vec3, previous: Option<Vec3>, next: Option<Vec3>, centered_at: Vec3) -> (Vec3, Vec3) {
    let first = find_control_point(previous, start, Some(end), false) - centered_at;
    let second = find_control_point(Some(start), end, next, true) - centered_at;

    (first, second)
}

fn find_control_point(previous_point: Option<Vec3>, current_point: Vec3, next_point: Option<Vec3>, is_reverse: bool) -> Vec3 {
    let previous = if previous_point.is_none() { current_point } else { previous_point.unwrap() };
    let next = if next_point.is_none() { current_point } else { next_point.unwrap() };

    let smoothing = 0.2;

    let opposing_line_properties = line_properties( Vec2::new(previous.x, previous.z), Vec2::new(next.x, next.z));
    let opp_length = opposing_line_properties.0;
    let opp_angle = opposing_line_properties.1;

    let angle = opp_angle + if is_reverse { std::f32::consts::PI } else { 0. };
    let length = opp_length * smoothing;

    let x = current_point.x + angle.cos() * length;
    let y = current_point.y;
    let z = current_point.z + angle.sin() * length;

    Vec3::new(x, y, z)
}

fn line_properties(point1: Vec2, point2: Vec2) -> (f32, f32) {
    let length_x = point2.x - point1.x;
    let length_y = point2.y - point1.y;

    (
        (length_x * length_x + length_y * length_y).sqrt(), // length
        f32::atan2(length_y, length_x), //angle
    )
}
