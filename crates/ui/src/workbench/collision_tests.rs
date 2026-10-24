//! Tests for collision shapes in the window (decision 0151): the
//! Attributes panel's Wrapped in parts and its settings, the outlines over
//! the view, and Display → Show collision.

use super::modelling_tests::{H, cube};
use super::tests::fresh;
use egui_kittest::kittest::Queryable;
use scene::{Collision, ConvexPartsOptions, ObjectId};

/// How many collision outline lines the overlay holds.
fn outline_lines(harness: &H) -> usize {
    harness
        .state()
        .workbench
        .components
        .overlay()
        .lines
        .iter()
        .filter(|(_, s)| *s == crate::OverlayStyle::Collision)
        .count()
}

fn set_collision(harness: &mut H, id: ObjectId, collision: Collision) {
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::SetCollision {
            ids: vec![id],
            collision,
        })
        .unwrap();
    settle(harness);
}

/// Runs the window until no collision shapes are worked out elsewhere.
fn settle(harness: &mut H) {
    // A frame first, so the work is handed out.
    harness.run_steps(1);
    for _ in 0..600 {
        if !harness.state().workbench.components.busy() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
        harness.run_steps(1);
    }
    harness.run_steps(2);
}

fn selected_cube() -> (H, ObjectId) {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    (harness, id)
}

#[test]
fn wrapped_in_parts_shows_its_settings_and_changes_them_as_one_step() {
    let (mut harness, id) = selected_cube();
    assert!(harness.query_by_label("Max parts").is_none());
    harness.get_by_label("Collision shape").click();
    harness.run();
    harness.get_by_label("Wrapped in parts (convex)").click();
    settle(&mut harness);
    let doc = harness.state().workbench.document();
    assert_eq!(doc.scene().object(id).unwrap().collision, Collision::Parts);
    assert_eq!(
        doc.undo_label(),
        Some("Collision: wrapped in parts (convex)")
    );
    harness.get_by_label("Increase Max parts").click();
    settle(&mut harness);
    harness.get_by_label("Decrease Detail").click();
    settle(&mut harness);
    let doc = harness.state().workbench.document();
    let parts = doc.scene().object(id).unwrap().collision_parts;
    assert_eq!(parts.max_parts, ConvexPartsOptions::STANDARD_MAX_PARTS + 1);
    assert!((parts.detail - 0.49).abs() < 1e-4, "{}", parts.detail);
    assert_eq!(doc.undo_label(), Some("Change the collision parts"));
}

#[test]
fn the_selected_objects_collision_is_outlined() {
    let (mut harness, id) = selected_cube();
    assert_eq!(outline_lines(&harness), 0, "no collision, no outline");
    set_collision(&mut harness, id, Collision::Mesh);
    assert_eq!(outline_lines(&harness), 0, "the shape itself is the shape");
    set_collision(&mut harness, id, Collision::Box);
    assert_eq!(outline_lines(&harness), 12, "a box's twelve edges");
    set_collision(&mut harness, id, Collision::Convex);
    assert_eq!(outline_lines(&harness), 12, "a cube wrapped is itself");
    // Parts are worked out on another thread, then drawn.
    set_collision(&mut harness, id, Collision::Parts);
    assert_eq!(outline_lines(&harness), 12, "a cube is one part");
    // Not selected: no outline, until Show collision is on.
    harness
        .state_mut()
        .workbench
        .document_mut()
        .clear_selection();
    harness.run();
    assert_eq!(outline_lines(&harness), 0);
    harness
        .state_mut()
        .workbench
        .viewport
        .options_mut()
        .show_collision = true;
    harness.run();
    assert_eq!(outline_lines(&harness), 12);
}

#[test]
fn an_open_shape_says_why_it_isnt_split() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Create(scene::ObjectKind::Primitive(
            scene::PrimitiveKind::Plane.standard(),
        )))
        .unwrap();
    harness.run();
    let plane = harness.state().workbench.document().selection()[0];
    set_collision(&mut harness, plane, Collision::Parts);
    assert!(
        harness
            .query_by_label_contains("No collision shape: the shape is flat")
            .is_some()
    );
}
