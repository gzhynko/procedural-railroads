use bevy::prelude::*;

#[derive(Component)]
pub struct Bogie {
    /// Whether this bogie is the leading bogie of the wagon.
    /// Should be set to None if not attached to any wagon.
    pub is_leading: Option<bool>,
    /// The entity index of the track this bogie is currently on.
    pub current_track: u32,
    /// The position of this bogie on the current track (the "t" value).
    /// The integer part of the number is the index of the track segment,
    /// the decimal part of the number is the position inside the segment.
    pub position_on_track: f32,
}

#[derive(Component)]
pub struct BogiePhysics {
    /// The mass of the bogie in kg.
    pub mass: f32,
    pub velocity: f32,
    /// The force that should be applied without taking into account the slope of the track.
    pub horizontal_force: f32,
    /// The force that should be applied depending on the slope of the track.
    pub vertical_force: f32,
}

/// Specifies the wagon entity this part is attached to.
#[derive(Component)]
pub struct AttachedTo(pub Entity);

#[derive(Component)]
pub struct Wagon {
    pub mass: f32,
    pub distance_between_bogies: f32,
}