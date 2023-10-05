use bevy::prelude::*;
use crate::{noise, NoiseSettings, PHYSICS_TIMESTEP};
use crate::rolling_stock::{utils};

use crate::rolling_stock::components::{AttachedToWagon, Bogie, BogiePhysics, WagonPhysics};
use crate::world::terrain::TERRAIN_CHUNK_SIZE;
use crate::world::train_tracks::Track;

const GRAV_ACCELERATION: f32 = -9.8;
const T_COEFFICIENT: f32 = 100.;

const KINETIC_FRICTION_COEFFICIENT: f32 = 0.001;
const STATIC_FRICTION_COEFFICIENT: f32 = 0.01;

pub(crate) fn apply_bogie_velocities(
    mut bogies_query: Query<(&BogiePhysics, &mut Bogie)>,
) {
    for (physics , mut bogie) in &mut bogies_query {
        bogie.position_on_track += physics.velocity * PHYSICS_TIMESTEP / T_COEFFICIENT;
    }
}

pub(crate) fn apply_bogie_forces(
    mut bogies_query: Query<(&mut BogiePhysics, Option<&AttachedToWagon>)>,
    wagons_query: Query<&WagonPhysics>,
) {
    for (mut bogie_physics, attached_to) in &mut bogies_query {
        if bogie_physics.current_slope_angle.is_none() {
            continue;
        }

        // Set the velocity to 0 if it's small.
        if bogie_physics.velocity.abs() <= 0.001 {
            bogie_physics.velocity = 0.;
        }

        // Do not start moving the bogie if the sum of vertical and horizontal forces is less than the static force.
        if bogie_physics.velocity == 0. {
            if (bogie_physics.vertical_force + bogie_physics.horizontal_force).abs() <= bogie_physics.static_force {
                continue;
            }
        }

        let mass = utils::get_carried_mass(attached_to, &bogie_physics, &wagons_query);

        // Apply the kinetic force (opposite to velocity).
        bogie_physics.velocity += (-1. * bogie_physics.velocity.signum()) * (bogie_physics.kinetic_force / mass * PHYSICS_TIMESTEP);

        // Finally, apply the vertical and horizontal velocities.
        bogie_physics.velocity += (bogie_physics.vertical_force + bogie_physics.horizontal_force) / mass * PHYSICS_TIMESTEP;
    }
}

pub(crate) fn set_bogie_static_kinetic_forces(
    mut bogies_query: Query<(&mut BogiePhysics, Option<&AttachedToWagon>)>,
    wagons_query: Query<&WagonPhysics>,
) {
    for (mut bogie_physics, attached_to) in &mut bogies_query {
        let slope_angle = bogie_physics.current_slope_angle;
        if slope_angle.is_none() {
            println!("(static+kinetic forces) slope angle is none, skipping this bogie");
            continue;
        }
        let slope_cos = slope_angle.unwrap().cos();

        let wagon_braking_force = if let Some(attached_to) = attached_to {
            let wagon_physics = wagons_query.get(attached_to.0).unwrap();
            wagon_physics.braking_force
        } else {
            0.
        };

        let mass = utils::get_carried_mass(attached_to, &bogie_physics, &wagons_query);
        let static_friction = STATIC_FRICTION_COEFFICIENT * mass * GRAV_ACCELERATION * slope_cos;
        let kinetic_friction = KINETIC_FRICTION_COEFFICIENT * mass * GRAV_ACCELERATION * slope_cos;
        bogie_physics.static_force = wagon_braking_force / 2. + static_friction.abs();
        bogie_physics.kinetic_force = wagon_braking_force / 2. + kinetic_friction.abs();
    }
}

pub(crate) fn set_bogie_horizontal_forces(
    mut bogies_query: Query<(&mut BogiePhysics, Option<&AttachedToWagon>)>,
    wagons_query: Query<&WagonPhysics>,
) {
    for (mut bogie_physics, attached_to) in &mut bogies_query {
        let mut wagon_tractive_force = 0.;
        if let Some(attached_to) = attached_to {
            let wagon_physics = wagons_query.get(attached_to.0).unwrap();
            wagon_tractive_force = wagon_physics.tractive_force;
        }

        bogie_physics.horizontal_force = wagon_tractive_force / 2.;
    }
}

pub(crate) fn set_bogie_vertical_forces(
    mut bogies_query: Query<(&mut BogiePhysics, Option<&AttachedToWagon>)>,
    wagons_query: Query<&WagonPhysics>,
) {
    for (mut bogie_physics, attached_to) in &mut bogies_query {
        let slope_angle = bogie_physics.current_slope_angle;
        if slope_angle.is_none() {
            println!("(vertical forces) slope angle is none, skipping this bogie");
            continue;
        }
        let slope_sin = slope_angle.unwrap().sin();

        let mass = utils::get_carried_mass(attached_to, &bogie_physics, &wagons_query);
        bogie_physics.vertical_force = mass * GRAV_ACCELERATION * slope_sin;
    }
}

pub(crate) fn update_bogie_current_slope_angle(
    mut bogies_query: Query<(&mut BogiePhysics, &Bogie)>,
    track_query: Query<&Track>,
    noise_settings: Res<NoiseSettings>,
) {
    if track_query.is_empty() {
        return;
    }

    let track = track_query.single();
    for (mut bogie_physics, bogie) in &mut bogies_query {
        let height_fn = noise::get_heightmap_function(TERRAIN_CHUNK_SIZE as f32, noise_settings.clone(), Vec3::ZERO);
        let slope_angle = track.get_slope_angle_at_t(bogie.position_on_track, &height_fn);
        bogie_physics.current_slope_angle = slope_angle;
    }
}

pub(crate) fn update_bogie_transforms(
    mut bogies_query: Query<(&mut Transform, &BogiePhysics, &Bogie)>,
    track_query: Query<&Track>,
    noise_settings: Res<NoiseSettings>,
) {
    if track_query.is_empty() {
        return;
    }

    let track = track_query.single();
    for (mut bogie_transform, bogie_physics, bogie) in &mut bogies_query {
        let t = bogie.position_on_track;
        let height_fn = noise::get_heightmap_function(TERRAIN_CHUNK_SIZE as f32, noise_settings.clone(), Vec3::ZERO);
        let point_option = track.get_interpolated_position_at_t(t, &height_fn);
        let angle = bogie_physics.current_slope_angle;
        if angle.is_none() {
            return;
        }

        if let Some((position, rotation)) = point_option {
            let angle_rotation = Quat::from_rotation_x(angle.unwrap());
            bogie_transform.translation = position;
            bogie_transform.rotation = rotation * angle_rotation;
        }
    }
}
