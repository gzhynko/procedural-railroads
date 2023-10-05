pub(crate) mod components;
mod bogie_systems;
mod wagon_systems;
mod utils;
mod ui_systems;

use bevy::prelude::*;

use crate::assets::AssetLoadingState;

use crate::rolling_stock::components::{Bogie, BogiePhysics, Wagon, WagonPhysics};
use crate::rolling_stock::bogie_systems::*;
use crate::rolling_stock::ui_systems::*;
use crate::rolling_stock::wagon_systems::*;

pub struct RollingStockPlugin;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
enum WagonPhysicsSet {
    SetForces,
    ApplyForces,
}

impl Plugin for RollingStockPlugin {
    fn build(&self, app: &mut App) {
        app
            .configure_set(Update, WagonPhysicsSet::ApplyForces.after(WagonPhysicsSet::SetForces))

            .add_systems(OnEnter(AssetLoadingState::AssetsLoaded), spawn_wagon)

            .add_systems(Update,
                         (
                             update_bogie_current_slope_angle,
                             set_bogie_static_kinetic_forces.after(update_bogie_current_slope_angle),
                             set_bogie_vertical_forces.after(update_bogie_current_slope_angle),
                             set_bogie_horizontal_forces.after(update_bogie_current_slope_angle)
                         )
                             .in_set(WagonPhysicsSet::SetForces)
                             .run_if(in_state(AssetLoadingState::AssetsLoaded))
            )
            .add_systems(FixedUpdate,
                         (
                             apply_bogie_forces,
                             apply_bogie_velocities,
                             sync_bogie_velocities,
                             constrain_attached_bogies
                         )
                             .chain()
                             .in_set(WagonPhysicsSet::ApplyForces)
                             .run_if(in_state(AssetLoadingState::AssetsLoaded))
            )

            .add_systems(Update, (update_bogie_transforms, sync_wagons_with_bogies).chain())
            .add_systems(Update, tracked_wagon_status_ui);
    }
}

#[derive(Bundle)]
pub struct BogieBundle {
    bogie: Bogie,
    physics: BogiePhysics,
    scene: SceneBundle,
}

#[derive(Bundle)]
pub struct WagonBundle {
    wagon: Wagon,
    physics: WagonPhysics,
    scene: SceneBundle,
}
