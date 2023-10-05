use bevy::prelude::*;
use crate::rolling_stock::components::{AttachedToWagon, BogiePhysics, WagonPhysics};

pub(crate) fn get_carried_mass(
    attached_to: Option<&AttachedToWagon>,
    physics: &BogiePhysics,
    wagons_query: &Query<&WagonPhysics>,
) -> f32 {
    if let Some(attached_to) = attached_to {
        let wagon = wagons_query.get(attached_to.0).unwrap();
        let carried_wagon_mass = wagon.mass / 2.;
        carried_wagon_mass + physics.mass
    } else {
        physics.mass
    }
}

pub(crate) fn get_attached_bogies(
    wagon: &Entity,
    bogies_query: &Query<(Entity, &AttachedToWagon)>,
) -> Vec<Entity> {
    let mut result = Vec::new();
    for (entity, attached_to) in bogies_query {
        if &attached_to.0 == wagon {
            result.push(entity);
        }
    }

    result
}
