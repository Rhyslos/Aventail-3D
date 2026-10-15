//! Tests for Extrude's shape in the window (decision 0070): its rows in
//! the Attributes panel, the next drag and the last extrude following
//! them, and Ctrl + wheel changing its steps.

use super::modelling_tests::{H, ctrl_wheel, cube, drag, last_operation, on_screen, ready, shape};
use super::*;
use egui_kittest::kittest::{NodeT, Queryable};
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

/// Gives the cube `scale`.
fn scale_cube(harness: &mut H, scale: glam::Vec3) {
    let id = cube(harness);
    let doc = harness.state_mut().workbench.document_mut();
    let mut transform = doc.scene().object(id).unwrap().transform;
    transform.scale = scale;
    doc.execute(scene::Command::SetTransforms(vec![(id, transform)]))
        .unwrap();
    harness.run();
}

fn last_stretch(harness: &H) -> glam::Vec3 {
    match last_operation(harness).map(|op| op.inner().clone()) {
        Some(Operation::ExtrudeBent { stretch, .. }) => stretch,
        other => panic!("not a bent extrude: {other:?}"),
    }
}

#[test]
fn a_bend_on_a_stretched_object_keeps_its_shape_as_seen() {
    // The cube stretched to twice its height (decision 0081).
    let mut harness = ready(Key::D);
    scale_cube(&mut harness, glam::Vec3::new(1.0, 1.0, 2.0));
    harness.state_mut().workbench.attributes.bend = scene::Bend {
        angle: 90.0,
        radius: 20.0,
        segments: 4,
        ..scene::Bend::default()
    };
    harness.run();
    drag(&mut harness, 40.0);
    let stretch = last_stretch(&harness);
    assert!(
        (stretch - glam::Vec3::new(0.75, 0.75, 1.5)).length() < 1e-4,
        "{stretch}"
    );
    let report = mesh::MeshReport::new(&shape(&harness));
    assert!(report.is_closed() && !report.has_problems(), "{report:?}");
    // Scaled evenly afterwards, changing the bend bends it as the object
    // is now: no stretch.
    scale_cube(&mut harness, glam::Vec3::splat(2.0));
    let at = on_screen(&harness, glam::Vec3::new(0.0, 0.0, 200.0));
    ctrl_wheel(&mut harness, at, true);
    assert_eq!(last_stretch(&harness), glam::Vec3::ONE);
}

#[test]
fn an_evenly_scaled_object_bends_with_no_stretch() {
    let mut harness = ready(Key::D);
    scale_cube(&mut harness, glam::Vec3::splat(3.0));
    harness.state_mut().workbench.attributes.bend = scene::Bend {
        angle: 45.0,
        ..scene::Bend::default()
    };
    harness.run();
    drag(&mut harness, 40.0);
    assert_eq!(last_stretch(&harness), glam::Vec3::ONE);
}

/// Where the Last edit's bend Segments field is, top to bottom.
fn segments_top(harness: &H) -> f32 {
    harness
        .get_all_by_label("Bend segments")
        .map(|n| n.rect().min.y)
        .fold(f32::INFINITY, f32::min)
}

#[test]
fn the_bend_rows_stay_put_when_the_extrude_starts_bending() {
    // Regression: when the Angle left 0, the shape rows above the bend
    // vanished and the bend rows jumped up under the mouse.
    let mut harness = ready(Key::D);
    drag(&mut harness, 60.0);
    let straight = segments_top(&harness);
    assert!(straight.is_finite());
    let op = last_operation(&harness).unwrap();
    let bent = op.with(scene::EditSettings {
        bend: Some(scene::Bend {
            angle: 1.0,
            ..scene::Bend::default()
        }),
        ..op.settings().unwrap()
    });
    assert!(matches!(bent, Operation::ExtrudeBent { .. }));
    let id = cube(&harness);
    let doc = harness.state_mut().workbench.document_mut();
    let layer = doc.scene().object(id).unwrap().stack.layers().len() - 1;
    let index = doc.scene().object(id).unwrap().stack.layers()[layer]
        .entries
        .len()
        - 1;
    doc.execute(scene::Command::ReplaceOperation {
        id,
        layer,
        index,
        operation: bent,
    })
    .unwrap();
    harness.run();
    assert!(
        harness
            .query_by_label_contains("Last edit: Extrude 1 polygon with a bend")
            .is_some()
    );
    assert!((segments_top(&harness) - straight).abs() < 0.5);
    // The shape rows are still there, greyed out.
    let direction = harness.get_by_label("Extrude direction");
    assert!(direction.accesskit_node().is_disabled());
}

#[test]
fn insets_and_straight_extrudes_on_a_stretched_object_record_its_stretch() {
    // Decision 0082: every edit that measures across the shape.
    let tall = glam::Vec3::new(0.75, 0.75, 1.5);
    for key in [Key::I, Key::D] {
        let mut harness = ready(key);
        scale_cube(&mut harness, glam::Vec3::new(1.0, 1.0, 2.0));
        drag(&mut harness, 30.0);
        let op = last_operation(&harness).unwrap();
        assert!(
            matches!(
                op,
                Operation::InsetPolygons { .. } | Operation::ExtrudePolygons { .. }
            ),
            "{op:?}"
        );
        assert!((op.stretch() - tall).length() < 1e-4, "{op:?}");
        // Scaled evenly afterwards, the next change of the edit takes the
        // new (no) stretch.
        scale_cube(&mut harness, glam::Vec3::splat(2.0));
        let changed = op.with(scene::EditSettings {
            distance: Some(1.0),
            ..op.settings().unwrap()
        });
        assert_eq!(changed.stretch(), op.stretch(), "kept until changed here");
    }
}
