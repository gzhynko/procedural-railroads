use bevy::prelude::*;
use bevy::utils::HashMap;
use crate::assets::ModelAssets;
use crate::rolling_stock::{BogieBundle, WagonBundle};
use crate::rolling_stock::components::{AttachedToWagon, Bogie, BogiePhysics, TrackedWagon, Wagon, WagonPhysics};
use crate::world::train_tracks::Track;

pub(crate) fn spawn_wagon(
    mut commands: Commands,
    model_assets: Res<ModelAssets>,
) {
    let wagon = commands.spawn(WagonBundle {
        wagon: Wagon {
            distance_between_bogies: 12.,
        },
        physics: WagonPhysics {
            mass: 30000.,
            velocity: 0.0,
            tractive_force: 10000.,
            braking_force: 0.,
        },
        scene: SceneBundle {
            scene: model_assets.train_gondola.clone(),
            ..default()
        },
    })
        .insert(TrackedWagon)
        .id();
    
    commands.spawn(BogieBundle {
        bogie: Bogie {
            is_leading: Some(true),
            current_track: None,
            position_on_track: 2.,
        },
        physics: BogiePhysics {
            mass: 4700.0,
            ..default()
        },
        scene: SceneBundle {
            scene: model_assets.train_wagon_bogie.clone(),
            ..default()
        },
    })
        .insert(AttachedToWagon(wagon.clone()));
    commands.spawn(BogieBundle {
        bogie: Bogie {
            is_leading: Some(false),
            current_track: None,
            position_on_track: 2.,
        },
        physics: BogiePhysics {
            mass: 4700.0,
            ..default()
        },
        scene: SceneBundle {
            scene: model_assets.train_wagon_bogie.clone(),
            ..default()
        },
    })
        .insert(AttachedToWagon(wagon.clone()));
}

pub(crate) fn sync_bogie_velocities(
    mut bogies_query: Query<(&Bogie, &mut BogiePhysics, &AttachedToWagon), Without<Wagon>>,
) {
    let mut bogie_pairs = HashMap::<Entity, Vec<Mut<BogiePhysics>>>::new();
    for (_, bogie_physics, attached_to) in &mut bogies_query {
        if !bogie_pairs.contains_key(&attached_to.0) {
            bogie_pairs.insert(attached_to.0.clone(), Vec::new());
        }
        bogie_pairs.get_mut(&attached_to.0).unwrap().push(bogie_physics);
    }

    for (_, bogies) in &mut bogie_pairs {
        let total_velocity: f32 = bogies.iter().map(|physics| physics.velocity).sum();
        let avg_bogie_velocity = total_velocity / bogies.len() as f32;

        for bogie_physics in bogies {
            bogie_physics.velocity = avg_bogie_velocity;
        }
    }
}

pub(crate) fn sync_wagons_with_bogies(
    bogies_query: Query<(&Bogie, &BogiePhysics, &Transform, &AttachedToWagon), Without<Wagon>>,
    mut wagons_query: Query<(&mut WagonPhysics, &mut Transform), (With<Wagon>, Without<Bogie>)>,
) {
    //TODO: move the code for finding bogie pairs into a separate function

    let mut bogie_pairs = HashMap::<Entity, Vec<(&Bogie, &BogiePhysics, &Transform)>>::new();
    for (bogie, bogie_physics, bogie_transform, attached_to) in &bogies_query {
        if !bogie_pairs.contains_key(&attached_to.0) {
            bogie_pairs.insert(attached_to.0.clone(), Vec::new());
        }
        bogie_pairs.get_mut(&attached_to.0).unwrap().push((bogie, bogie_physics, bogie_transform));
    }

    for (wagon, bogies) in &bogie_pairs {
        let (mut wagon_physics, mut wagon_transform) = wagons_query.get_mut(wagon.clone()).unwrap();

        let mut total_bogie_velocity = 0.0;

        let mut leading_bogie_transform = None;
        let mut trailing_bogie_transform = None;
        for (bogie, bogie_physics, bogie_transform) in bogies {
            total_bogie_velocity += bogie_physics.velocity;
            if bogie.is_leading.unwrap() {
                leading_bogie_transform = Some(bogie_transform);
            } else {
                trailing_bogie_transform = Some(bogie_transform);
            }
        }

        let avg_bogie_velocity = total_bogie_velocity / bogies.len() as f32;
        wagon_physics.velocity = avg_bogie_velocity;

        if leading_bogie_transform.is_none() || trailing_bogie_transform.is_none() {
            warn!("Unable to update transform for wagon {:?}: unable to determine the leading or the trailing bogie.", wagon);
            continue;
        }
        let leading_transform = leading_bogie_transform.unwrap();
        let trailing_transform = trailing_bogie_transform.unwrap();

        wagon_transform.translation = trailing_transform.translation + (leading_transform.translation - trailing_transform.translation) / 2.;
        wagon_transform.look_at(leading_transform.translation, Vec3::Y);
        // TODO: Make this (and other similar stuff) configurable (via a .ron file, for instance)
        wagon_transform.translation.y += 0.75;
    }
}

pub(crate) fn constrain_attached_bogies(
    mut bogies_query: Query<(&mut Bogie, &Transform, &AttachedToWagon)>,
    wagons_query: Query<&Wagon>,
    track_query: Query<&Track>,
) {
    // Make sure this runs only if update_bogie_transforms can also run.
    // TODO: Make a more comprehensive check or rework this somehow (maybe run criteria?)
    if track_query.is_empty() {
        return;
    }

    let mut bogie_pairs = HashMap::<Entity, Vec<(Mut<Bogie>, &Transform)>>::new();
    for (bogie, bogie_transform, attached_to) in &mut bogies_query {
        if !bogie_pairs.contains_key(&attached_to.0) {
            bogie_pairs.insert(attached_to.0.clone(), Vec::new());
        }
        bogie_pairs.get_mut(&attached_to.0).unwrap().push((bogie, bogie_transform));
    }

    for (wagon, bogies) in &mut bogie_pairs {
        let wagon = wagons_query.get(wagon.clone()).unwrap();
        let distance_between_bogies = wagon.distance_between_bogies;

        let mut leading_bogie = None;
        let mut trailing_bogie = None;
        for bogie in bogies {
            if bogie.0.is_leading.unwrap() {
                leading_bogie = Some(bogie);
            } else {
                trailing_bogie = Some(bogie);
            }
        }

        let (_, leading_transform) = leading_bogie.unwrap();
        let (trailing_bogie, trailing_transform) = trailing_bogie.unwrap();

        let current_distance = leading_transform.translation.distance(trailing_transform.translation);
        if current_distance < distance_between_bogies {
            trailing_bogie.position_on_track -= 0.001 * (current_distance - distance_between_bogies).abs();
            //println!("current_distance < distance_between_bogies; curr dist: {}, trailing t: {}", current_distance, trailing_bogie.position_on_track);
        } else {
            trailing_bogie.position_on_track += 0.001 * (current_distance - distance_between_bogies).abs();
            //println!("current_distance > distance_between_bogies; curr dist: {}, trailing t: {}", current_distance, trailing_bogie.position_on_track);
        }
    }
}
