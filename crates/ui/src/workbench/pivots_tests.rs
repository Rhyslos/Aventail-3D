//! Tests for placing pivots in the window (decision 0068): the ready-made
//! placements, Pivot to object and its Esc, and the Object mode's tool
//! menu.

use super::modelling_tests::{H, cube, note};
use super::tests::fresh;
use super::*;
use crate::commands::{CommandId, menu_for};
use glam::Vec3;
use scene::{PrimitiveKind, Transform};

fn selected_cube() -> H {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.state_mut().workbench.mode = Mode::Object;
    harness.run();
    harness
}

fn run(harness: &mut H, command: CommandId) {
    let ctx = harness.ctx.clone();
    let mut requests = Vec::new();
    harness
        .state_mut()
        .workbench
        .run_command(command, &ctx, &mut requests);
    harness.state_mut().workbench.carry_out(requests);
    harness.run();
}

fn pivot_of(harness: &H, id: scene::ObjectId) -> Vec3 {
    harness
        .state()
        .workbench
        .document()
        .scene()
        .world_matrix(id)
        .w_axis
        .truncate()
}

fn bounds_of(harness: &H, id: scene::ObjectId) -> scene::Bounds {
    harness
        .state()
        .workbench
        .document()
        .scene()
        .bounds_of([id])
        .unwrap()
}

fn move_to(harness: &mut H, id: scene::ObjectId, x: f32, y: f32, z: f32) {
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::SetTransforms(vec![(
            id,
            Transform {
                position: Vec3::new(x, y, z),
                ..Transform::default()
            },
        )]))
        .unwrap();
    harness.run();
}

#[test]
fn pivot_to_centre_moves_the_pivot_into_the_middle_of_the_shape() {
    let mut harness = selected_cube();
    let id = cube(&harness);
    let before = bounds_of(&harness, id);
    // A new cube's pivot is at the middle of its base.
    assert!(!pivot_of(&harness, id).abs_diff_eq(before.centre(), 1e-3));
    run(&mut harness, CommandId::PivotToCentre);
    assert!(pivot_of(&harness, id).abs_diff_eq(before.centre(), 1e-3));
    let after = bounds_of(&harness, id);
    assert!(after.min.abs_diff_eq(before.min, 1e-3), "the shape stays");
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Pivot to centre")
    );
    // Again: nothing to do, and no undo step.
    let steps = harness.state().workbench.document().undo_steps();
    run(&mut harness, CommandId::PivotToCentre);
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
    assert!(note(&harness).contains("already there"));
}

#[test]
fn pivot_to_world_origin_and_object_to_world_origin() {
    let mut harness = selected_cube();
    let id = cube(&harness);
    move_to(&mut harness, id, 200.0, 50.0, 0.0);
    let before = bounds_of(&harness, id);
    run(&mut harness, CommandId::PivotToOrigin);
    assert!(pivot_of(&harness, id).abs_diff_eq(Vec3::ZERO, 1e-3));
    assert!(bounds_of(&harness, id).min.abs_diff_eq(before.min, 1e-3));
    // Undo, then move the whole object instead.
    harness.state_mut().workbench.document_mut().undo().unwrap();
    run(&mut harness, CommandId::ObjectToOrigin);
    assert!(pivot_of(&harness, id).abs_diff_eq(Vec3::ZERO, 1e-3));
    let moved = bounds_of(&harness, id);
    assert!(
        moved
            .min
            .abs_diff_eq(before.min - Vec3::new(200.0, 50.0, 0.0), 1e-3),
        "the shape came along"
    );
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Object to world origin")
    );
}

#[test]
fn pivot_to_object_uses_the_clicked_objects_pivot_and_esc_cancels() {
    let mut harness = selected_cube();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Create(scene::ObjectKind::Primitive(
            PrimitiveKind::Sphere.standard(),
        )))
        .unwrap();
    let target = harness.state().workbench.document().selection()[0];
    move_to(&mut harness, target, -300.0, 0.0, 40.0);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    // Esc cancels the wait.
    run(&mut harness, CommandId::PivotToObject);
    assert!(harness.state().workbench.is_picking_pivot_target());
    assert!(note(&harness).starts_with("Pivot to object: click"));
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(!harness.state().workbench.is_picking_pivot_target());
    assert_eq!(
        harness.state().workbench.document().selection(),
        vec![id],
        "Esc only cancelled"
    );
    // The click's target.
    let before = bounds_of(&harness, id);
    harness.state_mut().workbench.pivot_to_object(target);
    harness.run();
    assert!(pivot_of(&harness, id).abs_diff_eq(Vec3::new(-300.0, 0.0, 40.0), 1e-3));
    assert!(bounds_of(&harness, id).max.abs_diff_eq(before.max, 1e-3));
    let label = harness.state().workbench.document().undo_label().unwrap();
    assert!(label.starts_with("Pivot to "), "{label}");
}

#[test]
fn placing_pivots_needs_a_selection() {
    let mut harness = selected_cube();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .clear_selection();
    for command in [
        CommandId::PivotToCentre,
        CommandId::PivotToOrigin,
        CommandId::PivotToObject,
        CommandId::ObjectToOrigin,
    ] {
        run(&mut harness, command);
        assert!(
            note(&harness).starts_with("Nothing is selected"),
            "{command:?}: {}",
            note(&harness)
        );
    }
    assert!(!harness.state().workbench.is_picking_pivot_target());
}

#[test]
fn the_object_modes_tool_menu_holds_the_pivot_commands() {
    let commands: Vec<_> = menu_for(Mode::Object).iter().map(|s| s.command).collect();
    assert!(commands.contains(&CommandId::PivotToObject));
    assert!(!commands.contains(&CommandId::Join));
    let model: Vec<_> = menu_for(Mode::Model).iter().map(|s| s.command).collect();
    assert!(!model.contains(&CommandId::PivotToCentre));
    assert!(model.contains(&CommandId::Scale));
}
