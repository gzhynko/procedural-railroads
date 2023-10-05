use bevy::prelude::*;

#[derive(Component, Default)]
pub struct Bogie {
    /// Whether this bogie is the leading bogie of the wagon.
    /// Should be set to None if not attached to any wagon.
    pub is_leading: Option<bool>,
    /// The track entity this bogie is currently on.
    pub current_track: Option<Entity>,
    /// The position of this bogie on the current track (the "t" value).
    /// The integer part of the number is the index of the track segment,
    /// the decimal part of the number is the position inside the segment.
    pub position_on_track: f32,
}

#[derive(Component, Default)]
pub struct BogiePhysics {
    /// The mass of the bogie in kg.
    pub mass: f32,
    pub velocity: f32,
    /// The force that should be applied regardless of the slope of the track,
    /// i.e. Tractive force.
    pub horizontal_force: f32,
    /// The force that should be applied depending on the slope of the track,
    /// i.e. Gravity.
    pub vertical_force: f32,
    /// The force that should be applied in the direction opposite to velocity,
    /// i.e. Braking force, kinetic friction.
    pub kinetic_force: f32,
    /// The force that, if greater than horizontal_force + vertical_force, prevents the bogie from moving,
    /// i.e. Braking force, static friction.
    pub static_force: f32,
    /// The angle of the current slope in radians.
    pub current_slope_angle: Option<f32>,
}

/// Specifies the wagon entity this part is attached to.
#[derive(Component)]
pub struct AttachedToWagon(pub Entity);

#[derive(Component, Default)]
pub struct Wagon {
    pub distance_between_bogies: f32,
}

#[derive(Component, Default)]
pub struct WagonPhysics {
    /// The mass of the wagon (excluding bogies) in kg.
    pub mass: f32,
    /// The averaged velocity of the bogies attached to the wagon. Updated each frame.
    pub velocity: f32,
    pub tractive_force: f32,
    pub braking_force: f32,
}

/// Used as a marker to track a single wagon for UI.
#[derive(Component)]
pub struct TrackedWagon;
