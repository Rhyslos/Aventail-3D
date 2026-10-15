//! Tests for Extrude's shape in the window (decision 0070): its rows in
//! the Attributes panel, the next drag and the last extrude following
//! them, and Ctrl + wheel changing its steps.

use super::modelling_tests::{H, ctrl_wheel, cube, drag, last_operation, on_screen, ready, shape};
use super::*;
use egui_kittest::kittest::Queryable;
use scene::{ExtrudeDirection, ExtrudeShape, Operation};

fn last_shape(harness: &H) -> ExtrudeShape {
    match last_operation(harness) {
        Some(Operation::ExtrudePolygons { shape, .. }) => shape,
        other => panic!("not a straight extrude: {other:?}"),
    }
}

#[test]
fn the_tool_shows_its_shape_rows_and_a_drag_goes_straight_out() {
    let mut harness = ready(Key::D);
    for name in ["Extrude direction", "End size", "Extrude steps", "Flat end"] {
        assert!(harness.query_all_by_label(name).count() >= 1, "{name}");
    }
    drag(&mut harness, 60.0);
    assert_eq!(last_shape(&harness), ExtrudeShape::default());
}

#[test]
fn the_direction_list_sets_the_next_extrude() {
    let mut harness = ready(Key::D);
    harness.get_by_label("Extrude direction").click();
    harness.run();
    harness.get_by_label("Along Z").click();
    harness.run();
    assert_eq!(
        harness.state().workbench.attributes.extrude.direction,
        ExtrudeDirection::AlongZ
    );
    harness.state_mut().workbench.attributes.extrude.end_size = 0.5;
    harness.state_mut().workbench.attributes.extrude.steps = 2;
    drag(&mut harness, 60.0);
    let made = last_shape(&harness);
    assert_eq!(made.direction, ExtrudeDirection::AlongZ);
    assert_eq!((made.end_size, made.steps), (0.5, 2));
    // Two rows of four walls, the top a quarter of the size.
    assert_eq!(shape(&harness).face_count(), 6 + 8);
}

#[test]
fn ctrl_and_the_wheel_change_the_extrudes_steps() {
    let mut harness = ready(Key::D);
    let at = on_screen(&harness, glam::Vec3::new(0.0, 0.0, 100.0));
    // Before: the tool's steps.
    ctrl_wheel(&mut harness, at, true);
    assert_eq!(harness.state().workbench.attributes.extrude.steps, 2);
    drag(&mut harness, 60.0);
    assert_eq!(last_shape(&harness).steps, 2);
    // After: the last extrude, one undo step per notch.
    let steps = harness.state().workbench.document().undo_steps();
    ctrl_wheel(&mut harness, at, true);
    assert_eq!(last_shape(&harness).steps, 3);
    assert_eq!(harness.state().workbench.document().undo_steps(), steps + 1);
    assert_eq!(shape(&harness).face_count(), 6 + 12);
    // With a bend set, the wheel changes the bend's segments instead.
    harness.state_mut().workbench.attributes.bend = scene::Bend {
        angle: 90.0,
        radius: 20.0,
        ..scene::Bend::default()
    };
    let before = harness.state().workbench.attributes.bend.segments;
    harness
        .state_mut()
        .workbench
        .document_mut()
        .clear_selection();
    harness.run();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    ctrl_wheel(&mut harness, at, true);
    // The last edit is still a straight extrude: its steps go up.
    assert_eq!(last_shape(&harness).steps, 4);
    assert_eq!(harness.state().workbench.attributes.bend.segments, before);
}

#[test]
fn the_last_extrudes_shape_can_be_changed_and_bending_takes_over() {
    let mut harness = ready(Key::D);
    drag(&mut harness, 60.0);
    let op = last_operation(&harness).unwrap();
    let settings = op.settings().unwrap();
    assert_eq!(settings.extrude, Some(ExtrudeShape::default()));
    // The Last edit's rows are there.
    assert!(harness.query_all_by_label("End size").count() >= 1);
    // Bent, then straight again: shaped as asked.
    let bent = op.with(scene::EditSettings {
        bend: Some(scene::Bend {
            angle: 45.0,
            ..scene::Bend::default()
        }),
        ..settings
    });
    assert!(matches!(bent, Operation::ExtrudeBent { .. }));
    assert_eq!(bent.settings().unwrap().extrude, None);
    let flat = ExtrudeShape {
        flat_end: true,
        ..ExtrudeShape::default()
    };
    let back = bent.with(scene::EditSettings {
        bend: Some(scene::Bend::default()),
        extrude: Some(flat),
        ..bent.settings().unwrap()
    });
    assert!(
        matches!(back, Operation::ExtrudePolygons { shape, .. } if shape == flat),
        "{back:?}"
    );
}
