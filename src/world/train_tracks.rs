use bevy::gltf::{Gltf, GltfMesh};
use bevy::prelude::*;
use bevy::utils::hashbrown::HashMap;
use bevy_extrude_mesh::bezier::{BezierCurve};
use bevy_extrude_mesh::extrude;
use bevy_extrude_mesh::extrude::ExtrudeShape;
use crate::assets::{ModelAssets};
use crate::{noise, NoiseSettings};
use crate::world::route_gen::Route;
use crate::world::terrain::FAR_GRID_CHUNK_SIZE;

const NUM_SUBDIVISIONS: u32 = 20;
const TRACK_ELEVATION: f32 = 1.;

#[derive(Clone)]
struct TrackSegment {
    id: usize,
    curve: BezierCurve,
    world_translation: Vec3,
}

#[derive(Resource, Default)]
pub(crate) struct PlacementData {
    track_shape: Option<ExtrudeShape>,
    track_material: Option<Handle<StandardMaterial>>,

    segments: Vec<TrackSegment>,
    last_placed_segment_id: usize,
    last_used_node_id: usize,
}

struct SampledTrackSegment {
    curve: BezierCurve,
    world_translation: Vec3,
}

#[derive(Component, Default)]
pub(crate) struct Track {
    /// Used to sample the t value (used by train bogies).
    segments: HashMap<u32, SampledTrackSegment>,
    last_used_segment_id: usize,
}

impl Track {
    fn get_segment_at_t(&self, t: f32) -> Option<&SampledTrackSegment> {
        assert!(t >= 0., "t wasn't a positive number (shouldn't actually happen)");
        let lower_bound = t.floor() as u32;

        self.segments.get(&lower_bound)
    }

    pub fn get_interpolated_position_at_t<F: Fn(f64, f64) -> f64>(&self, t: f32, height_fn: &F) -> Option<(Vec3, Quat)> {
        let segment = self.get_segment_at_t(t);
        if let Some(segment) = segment {
            let lower_bound = t.floor();
            let local_t = t - lower_bound;

            let actual_t = segment.curve.map(local_t);
            let mut point = segment.curve.get_oriented_point(actual_t);
            point.position += segment.world_translation;
            point.position.y = height_fn(point.position.x as f64, point.position.z as f64) as f32;
            point.position.y += TRACK_ELEVATION;

            Some((point.position, point.rotation))
        } else {
            None
        }
    }

    pub fn get_slope_angle_at_t<F: Fn(f64, f64) -> f64>(&self, t: f32, height_fn: &F) -> Option<f32> {
        let segment = self.get_segment_at_t(t);

        if let Some(segment) = segment {
            let lower_bound = t.floor();
            let local_t = t - lower_bound;

            let actual_t = segment.curve.map(local_t);
            let mut this_pos = segment.curve.get_oriented_point(actual_t).position;
            this_pos += segment.world_translation;
            this_pos.y = height_fn(this_pos.x as f64, this_pos.z as f64) as f32;

            let mut new_pos;
            let step = 1. / NUM_SUBDIVISIONS as f32;
            let new_t = t + step;
            if new_t.floor() == lower_bound {
                let new_local_t = new_t - new_t.floor();
                let new_actual_t = segment.curve.map(new_local_t);
                new_pos = segment.curve.get_oriented_point(new_actual_t).position;
                new_pos += segment.world_translation;
            } else {
                let new_segment = self.get_segment_at_t(new_t);
                if let Some(new_segment) = new_segment {
                    let new_local_t = new_t - new_t.floor();
                    let new_actual_t = new_segment.curve.map(new_local_t);
                    new_pos = new_segment.curve.get_oriented_point(new_actual_t).position;
                    new_pos += new_segment.world_translation;
                } else {
                    return None;
                }
            }
            new_pos.y = height_fn(new_pos.x as f64, new_pos.z as f64) as f32;

            let sine = (new_pos.y - this_pos.y) / Vec3::distance(this_pos, new_pos);
            Some(sine.asin())
        } else {
            None
        }
    }
}

impl PlacementData {
    fn current_segment_id(&self) -> usize {
        if self.segments.is_empty() {
            0
        } else {
            self.segments[self.segments.len() - 1].id
        }
    }
}

pub(crate) fn spawn_track_entity(
    mut commands: Commands,
) {
    commands.spawn_empty()
        .insert(Track::default());
}

pub(crate) fn setup_track_data(
    mut data_res: ResMut<PlacementData>,
    model_assets: Res<ModelAssets>,

    meshes: Res<Assets<Mesh>>,
    gltf_assets: Res<Assets<Gltf>>,
    gltf_mesh_assets: Res<Assets<GltfMesh>>,
) {
    if let Some(gltf) = gltf_assets.get(&model_assets.track_cross_section) {
        let track_gltf_mesh = gltf_mesh_assets.get(&gltf.meshes[0]).unwrap();
        let track_mesh = meshes.get(&track_gltf_mesh.primitives[0].mesh).unwrap();
        let extrude_shape = ExtrudeShape::from_mesh(track_mesh);
        data_res.track_shape = Some(extrude_shape);
    }
}

pub(crate) fn setup_track_material(
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut data_res: ResMut<PlacementData>,
) {
    let material = StandardMaterial {
        ..default()
    };
    let handle = materials.add(material);

    data_res.track_material = Some(handle);
}

pub(crate) fn update_track_entity(
    mut track_query: Query<&mut Track>,
    placement_data_res: Res<PlacementData>,
    noise_settings: Res<NoiseSettings>,
) {
    if placement_data_res.segments.is_empty() {
        return;
    }

    // TODO: add support for multiple tracks
    let mut track = track_query.single_mut();

    if placement_data_res.current_segment_id() <= track.last_used_segment_id {
        return;
    }

    let segment = placement_data_res.segments.iter().find(|seg| seg.id == track.last_used_segment_id + 1);
    if segment.is_none() {
        println!("update_track_entity: no segment with id of track.last_used_segment_id + 1");
        return;
    }
    let mut cloned_segment = segment.unwrap().clone();
    let world_pos = cloned_segment.world_translation;
    let height_fn = noise::get_heightmap_function(FAR_GRID_CHUNK_SIZE as f32, noise_settings.clone(), Vec3::new(world_pos.x, -world_pos.y + 0.3, world_pos.z));

    cloned_segment.curve.calculate_arc_lengths_with_custom_height_function(&height_fn);

    let sampled_segment = SampledTrackSegment {
        curve: cloned_segment.curve,
        world_translation: cloned_segment.world_translation,
    };
    track.segments.insert(cloned_segment.id as u32, sampled_segment);
    track.last_used_segment_id += 1;
}

// Updates the placement data one TrackSegment per run.
pub(crate) fn update_placement_data(
    mut data_res: ResMut<PlacementData>,
    route_res: Res<Route>,
) {
    let id_to_add = if data_res.last_used_node_id == 0 { 2 } else { data_res.last_used_node_id + 1 };
    if route_res.get_last_id() <= 2 || route_res.get_last_id() == data_res.last_used_node_id || route_res.get_point(id_to_add + 1).is_none() {
        return;
    }

    // We need at least four points to get the direction stuff right.
    let next_node = route_res.get_point(id_to_add + 1).unwrap();
    let new_node = route_res.get_point(id_to_add).unwrap();
    let last_node = route_res.get_point(id_to_add - 1).unwrap();
    let previous_node = route_res.get_point(id_to_add - 2).unwrap();

    // Calculate the bezier points (the vertices will be positioned relative to zero)
    let mut bezier_start = Vec3::ZERO;
    let mut bezier_end = new_node.clone() - last_node.clone();
    let (mut bezier_control1, mut bezier_control2) = find_control_points(last_node.clone(), new_node.clone(), Some(previous_node.clone()), Some(next_node.clone()), last_node.clone());

    bezier_start.y = 0.;
    bezier_end.y = 0.;
    bezier_control1.y = 0.;
    bezier_control2.y = 0.;
    let bezier_curve = BezierCurve::new(vec![bezier_start, bezier_control1, bezier_control2, bezier_end], None);

    // Push the new segment to the resource
    let segment = TrackSegment {
        id: data_res.current_segment_id() + 1,
        curve: bezier_curve,
        world_translation: last_node.clone(),
    };
    data_res.segments.push(segment);
    data_res.last_used_node_id = id_to_add;
}

pub(crate) fn place_tracks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut placement_data: ResMut<PlacementData>,
    noise_settings: Res<NoiseSettings>,
) {
    if placement_data.track_shape.is_none() || placement_data.track_material.is_none() {
        return;
    }
    if placement_data.last_placed_segment_id + 1 > placement_data.current_segment_id() {
        return;
    }

    let id_to_place = placement_data.last_placed_segment_id + 1;

    let segment = placement_data.segments.iter().find(|seg| seg.id == id_to_place).unwrap();

    // Generate the path using the noise function as the height function
    let world_pos = segment.world_translation;
    let height_fn = noise::get_heightmap_function(FAR_GRID_CHUNK_SIZE as f32, noise_settings.clone(), Vec3::new(world_pos.x, -world_pos.y, world_pos.z));
    let path = segment.curve.generate_path_with_custom_height_function(NUM_SUBDIVISIONS, height_fn);

    let mut translation = segment.world_translation;
    translation.y += TRACK_ELEVATION;

    let mesh = extrude::extrude(placement_data.track_shape.as_ref().unwrap(), &path);
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
