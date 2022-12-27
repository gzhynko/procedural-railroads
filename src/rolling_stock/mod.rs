pub(crate) mod components;
mod bogie_systems;
mod wagon_systems;

use bevy::prelude::*;
use bevy::time::FixedTimestep;

use crate::assets::AssetLoadingState;

use crate::rolling_stock::components::{Bogie, BogiePhysics};
use crate::rolling_stock::bogie_systems::*;
use crate::rolling_stock::wagon_systems::*;

pub struct RollingStockPlugin;

const PHYSICS_TIMESTEP: f32 = 1. / 60.;

impl Plugin for RollingStockPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_startup_system(spawn_bogie)
            .add_system_set(
                SystemSet::new()
                    .with_run_criteria(FixedTimestep::step(PHYSICS_TIMESTEP as f64))
                    //.with_system(set_bogie_vertical_forces)
                    //.with_system(apply_bogie_forces.after(set_bogie_vertical_forces))
                    .with_system(apply_bogie_velocities)
                    //.with_system(constrain_attached_bogies.after(apply_bogie_velocities))
                    .with_system(update_bogie_transforms.after(apply_bogie_velocities))
            );
    }
}

#[derive(Bundle)]
pub struct BogieBundle {
    bogie: Bogie,
    physics: BogiePhysics,
    #[bundle]
    pbr: PbrBundle,
}
