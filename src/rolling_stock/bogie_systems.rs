use bevy::prelude::*;
use bevy::prelude::shape::Cube;
use crate::rolling_stock::{BogieBundle, PHYSICS_TIMESTEP};

use crate::rolling_stock::components::{AttachedTo, Bogie, BogiePhysics, Wagon};
use crate::world::train_tracks::Track;

const GRAV_ACCELERATION: f32 = -9.81;
const T_COEFFICIENT: f32 = 100.;

pub(crate) fn spawn_bogie(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube_handle = meshes.add(Mesh::from(Cube::new(1.)));
    let material_handle = materials.add(StandardMaterial::default());
    commands.spawn(BogieBundle {
        bogie: Bogie {
            is_leading: None,
            current_track: 0,
            position_on_track: 2.,
        },
        physics: BogiePhysics {
            mass: 4700.0,
            velocity: 50.0,
            horizontal_force: 0.0,
            vertical_force: 0.0,
        },
        pbr: PbrBundle {
            mesh: cube_handle,
            material: material_handle,
            ..default()
        },
    });
}

pub(crate) fn apply_bogie_velocities(
    time: Res<Time>,
    mut bogies_query: Query<(&BogiePhysics, &mut Bogie)>,
) {
    for (physics , mut bogie) in &mut bogies_query {
        bogie.position_on_track += physics.velocity * PHYSICS_TIMESTEP / T_COEFFICIENT;
    }
}

pub(crate) fn apply_bogie_forces(
    time: Res<Time>,
    mut bogies_query: Query<(&mut BogiePhysics, Option<&AttachedTo>)>,
    wagons_query: Query<&Wagon>,
) {
    for (mut bogie_physics, attached_to) in &mut bogies_query {
        let mass;
        if let Some(attached_to) = attached_to {
            let wagon = wagons_query.get(attached_to.0).unwrap();
            mass = wagon.mass + bogie_physics.mass;
        } else {
            mass = bogie_physics.mass;
        }
        bogie_physics.velocity += (bogie_physics.vertical_force + bogie_physics.horizontal_force) / mass * PHYSICS_TIMESTEP;
    }
}

pub(crate) fn set_bogie_vertical_forces(
    mut bogies_query: Query<(&mut BogiePhysics, &Bogie, Option<&AttachedTo>)>,
    wagons_query: Query<&Wagon>,
    track_query: Query<&Track>,
) {
    if track_query.is_empty() {
        return;
    }

    let track = track_query.single();
    for (mut bogie_physics, bogie, attached_to) in &mut bogies_query {
        let slope_angle = track.get_slope_angle_at_t(bogie.position_on_track);
        if slope_angle.is_none() {
            println!("slope angle is none, skipping this bogie");
            continue;
        }
        let slope_sin = slope_angle.unwrap().sin();

        let mass;
        if let Some(attached_to) = attached_to {
            let wagon = wagons_query.get(attached_to.0).unwrap();
            mass = wagon.mass + bogie_physics.mass;
        } else {
            mass = bogie_physics.mass;
        }

        bogie_physics.vertical_force = mass * GRAV_ACCELERATION * slope_sin;
    }
}

pub(crate) fn update_bogie_transforms(
    mut bogies_query: Query<(&mut Transform, &Bogie)>,
    track_query: Query<&Track>
) {
    if track_query.is_empty() {
        return;
    }

    let track = track_query.single();
    for (mut bogie_transform, bogie) in &mut bogies_query {
        let t = bogie.position_on_track;
        let point_option = track.get_interpolated_position_at_t(t);

        if let Some((position, rotation)) = point_option {
            bogie_transform.translation = position;
            bogie_transform.rotation = rotation;
        }
    }
}
