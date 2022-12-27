use bevy::prelude::*;
use bevy::prelude::shape::Cube;
use bevy::utils::HashMap;
use crate::rolling_stock::BogieBundle;
use crate::rolling_stock::components::{AttachedTo, Bogie, BogiePhysics, Wagon};

pub(crate) fn spawn_wagon(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube_handle = meshes.add(Mesh::from(Cube::new(1.)));
    let material_handle = materials.add(StandardMaterial::default());

    let wagon = commands.spawn_empty()
        .insert(Wagon {
            mass: 0.,
            distance_between_bogies: 15.,
        })
        .id();
    
    commands.spawn(BogieBundle {
        bogie: Bogie {
            is_leading: Some(true),
            current_track: 0,
            position_on_track: 4.,
        },
        physics: BogiePhysics {
            mass: 4700.0,
            velocity: 0.,
            horizontal_force: 15000.,
            vertical_force: 0.,
        },
        pbr: PbrBundle {
            mesh: cube_handle.clone(),
            material: material_handle.clone(),
            ..default()
        },
    });
        //.insert(AttachedTo(wagon.clone()));
    commands.spawn(BogieBundle {
        bogie: Bogie {
            is_leading: Some(false),
            current_track: 0,
            position_on_track: 4.,
        },
        physics: BogiePhysics {
            mass: 4700.0,
            velocity: 0.,
            horizontal_force: 15000.,
            vertical_force: 0.,
        },
        pbr: PbrBundle {
            mesh: cube_handle.clone(),
            material: material_handle.clone(),
            ..default()
        },
    });
        //.insert(AttachedTo(wagon.clone()));
}

pub(crate) fn constrain_attached_bogies(
    mut bogies_query: Query<(&mut Bogie, &Transform, &AttachedTo)>,
    wagons_query: Query<&Wagon>
) {
    let mut bogie_pairs = HashMap::<Entity, Vec<(Mut<Bogie>, &Transform)>>::new();
    for (mut bogie, bogie_transform, attached_to) in &mut bogies_query {
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
            trailing_bogie.position_on_track -= 0.01 * (current_distance - distance_between_bogies).abs();
        } else {
            trailing_bogie.position_on_track += 0.01 * (current_distance - distance_between_bogies).abs();
        }
    }
}
