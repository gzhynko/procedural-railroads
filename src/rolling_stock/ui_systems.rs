use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use bevy_egui::egui::emath;
use crate::rolling_stock::components::{AttachedToWagon, Bogie, BogiePhysics, TrackedWagon, WagonPhysics};
use crate::rolling_stock::utils;

pub(crate) fn tracked_wagon_status_ui(
    mut egui_contexts: EguiContexts,
    mut tracked_wagon_query: Query<(Entity, &mut WagonPhysics), (With<TrackedWagon>, Without<AttachedToWagon>)>,
    bogie_entity_query: Query<(Entity, &AttachedToWagon)>,
    bogie_query: Query<(&Bogie, &BogiePhysics)>,
) {
    if tracked_wagon_query.is_empty() {
        return;
    }

    let (wagon_entity, mut wagon_physics) = tracked_wagon_query.single_mut();
    let bogies = utils::get_attached_bogies(&wagon_entity, &bogie_entity_query);

    egui::Window::new("Tracked Wagon").show(egui_contexts.ctx_mut(), |ui| {
        ui.allocate_space(emath::Vec2::new(250., 0.));
        ui.set_max_width(250.0);

        // Display controls for tractive and braking force
        ui.add(egui::Slider::new(&mut wagon_physics.tractive_force, -300000.0..=300000.).text("Tractive force"));
        ui.add(egui::Slider::new(&mut wagon_physics.braking_force, 0.0..=300000.).text("Braking force"));

        ui.separator();

        // Display the status of the wagon.
        ui.label(format!("Mass: {}", wagon_physics.mass));
        ui.label(format!("Velocity: {}", wagon_physics.velocity));
        ui.label(format!("Tractive force: {}", wagon_physics.tractive_force));
        ui.label(format!("Braking force: {}", wagon_physics.braking_force));

        // Display status of each of the attached bogies.
        for bogie_entity in bogies {
            let (bogie, bogie_physics) = bogie_query.get(bogie_entity).unwrap();
            ui.collapsing(if bogie.is_leading.unwrap() {"Leading bogie"} else {"Trailing bogie"}, |collapsing_ui| {
                collapsing_ui.label(format!("Mass: {}", bogie_physics.mass));
                collapsing_ui.label(format!("Velocity: {}", bogie_physics.velocity));
                collapsing_ui.label(format!("Vertical force: {}", bogie_physics.vertical_force));
                collapsing_ui.label(format!("Horizontal force: {}", bogie_physics.horizontal_force));
                collapsing_ui.label(format!("Kinetic force: {}", bogie_physics.kinetic_force));
                collapsing_ui.label(format!("Static force: {}", bogie_physics.static_force));
            });
        }
    });
}
