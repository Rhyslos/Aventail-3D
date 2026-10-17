//! Tests for Snap to and Align in the window (decision 0093).

use super::aligning::TargetUse;
use super::modelling_tests::{H, cube, note};
use super::tests::{fresh, main_menu};
use egui_kittest::kittest::Queryable;
use glam::Vec3;
use scene::{ObjectId, PrimitiveKind, Transform};

fn place(harness: &mut H, id: ObjectId, at: Vec3, turn: f32) {
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::SetTransforms(vec![(
            id,
            Transform {
                position: at,
                rotation: Vec3::new(turn, 0.0, 0.0),
                ..Transform::default()
            },
        )]))
        .unwrap();
    harness.run();
}

/// The cube (selected) at the origin, and a sphere (the target) at
/// (300, 100, 50).
fn two_objects() -> (H, ObjectId, ObjectId) {
    let mut harness = fresh();
    let id = cube(&harness);
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(scene::Command::Create(scene::ObjectKind::Primitive(
        PrimitiveKind::Sphere.standard(),
    )))
    .unwrap();
    let target = doc.selection()[0];
    place(&mut harness, target, Vec3::new(300.0, 100.0, 50.0), 0.0);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    (harness, id, target)
}

fn pivot(harness: &H, id: ObjectId) -> Vec3 {
    let scene = harness.state().workbench.document().scene();
    scene.world_matrix(id).w_axis.truncate()
}

fn bounds(harness: &H, id: ObjectId) -> scene::Bounds {
    let scene = harness.state().workbench.document().scene();
    scene.bounds_of([id]).unwrap()
}

/// Opens Align on `target` as a click on it would.
fn open_align(harness: &mut H, target: ObjectId) {
    harness
        .state_mut()
        .workbench
        .target_picked(TargetUse::Align, target);
    harness.run();
}

/// One of the window's two rows of points: 0 for this object's, 1 for
/// the target's.
fn point(harness: &mut H, row: usize, name: &str) {
    let mut all: Vec<_> = harness.get_all_by_label(name).collect();
    all.sort_by(|a, b| a.rect().min.y.total_cmp(&b.rect().min.y));
    all[row].click();
    harness.run();
}

#[test]
fn snap_to_puts_the_pivot_on_the_targets_and_esc_cancels_the_wait() {
    let (mut harness, id, target) = two_objects();
    main_menu(&harness, "Tools").click();
    harness.run();
    harness.get_by_label(super::aligning::SNAP_TO_LABEL).click();
    harness.run();
    assert!(note(&harness).starts_with("Snap to: click"));
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(note(&harness).contains("cancelled"));
    let size = bounds(&harness, id).size();
    harness.state_mut().workbench.snap_to(target);
    harness.run();
    assert!(pivot(&harness, id).abs_diff_eq(Vec3::new(300.0, 100.0, 50.0), 1e-3));
    // The shape came along.
    assert!(bounds(&harness, id).size().abs_diff_eq(size, 1e-3));
    assert!(bounds(&harness, id).min.z > 49.0);
    let label = harness.state().workbench.document().undo_label().unwrap();
    assert_eq!(label, "Snap to Sphere");
}

#[test]
fn align_lines_up_middles_live_and_keeps_one_step() {
    let (mut harness, id, target) = two_objects();
    let steps = harness.state().workbench.document().undo_steps();
    open_align(&mut harness, target);
    assert!(harness.state().workbench.is_aligning());
    // Middle to middle on every axis, shown at once.
    let centre = bounds(&harness, target).centre();
    assert!(bounds(&harness, id).centre().abs_diff_eq(centre, 1e-2));
    // Stand it on top: Z only, lowest to highest.
    harness.get_by_label("Along X").click();
    harness.get_by_label("Along Y").click();
    harness.run();
    point(&mut harness, 0, "Lowest");
    point(&mut harness, 1, "Highest");
    let top = bounds(&harness, target).max.z;
    assert!((bounds(&harness, id).min.z - top).abs() < 1e-2);
    assert!(pivot(&harness, id).x.abs() < 1e-3, "X is left alone now");
    dialog_button(&mut harness, "Align");
    assert!(!harness.state().workbench.is_aligning());
    let doc = harness.state().workbench.document();
    assert_eq!(doc.undo_steps(), steps + 1, "one step");
    assert_eq!(doc.undo_label(), Some("Align to Sphere"));
}

/// A button at the foot of the window (the title says Align too).
fn dialog_button(harness: &mut H, name: &str) {
    harness
        .get_all_by_label(name)
        .max_by(|a, b| a.rect().min.y.total_cmp(&b.rect().min.y))
        .unwrap()
        .click();
    harness.run();
}

#[test]
fn cancel_puts_everything_back() {
    let (mut harness, id, target) = two_objects();
    let steps = harness.state().workbench.document().undo_steps();
    open_align(&mut harness, target);
    point(&mut harness, 0, "Highest");
    dialog_button(&mut harness, "Cancel");
    assert!(pivot(&harness, id).abs_diff_eq(Vec3::ZERO, 1e-4));
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
    // Esc too.
    open_align(&mut harness, target);
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(!harness.state().workbench.is_aligning());
    assert!(pivot(&harness, id).abs_diff_eq(Vec3::ZERO, 1e-4));
}

#[test]
fn align_can_turn_them_to_match() {
    let (mut harness, id, target) = two_objects();
    place(&mut harness, target, Vec3::new(300.0, 100.0, 50.0), 0.5);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    open_align(&mut harness, target);
    harness.get_by_label("Also turn them to match").click();
    harness.run();
    let turn = harness
        .state()
        .workbench
        .document()
        .scene()
        .object(id)
        .unwrap()
        .transform
        .rotation;
    assert!((turn.x - 0.5).abs() < 1e-4, "{turn}");
    dialog_button(&mut harness, "Align");
    // Middles still line up after the turn.
    same_middles(&harness, id, target);
}

/// After aligning middles, the boxes share their middle.
fn same_middles(harness: &H, id: ObjectId, target: ObjectId) {
    let a = bounds(harness, id).centre();
    let b = bounds(harness, target).centre();
    assert!(a.abs_diff_eq(b, 1e-2), "{a} {b}");
}

#[test]
fn the_target_cant_be_one_of_the_selected() {
    let (mut harness, id, _) = two_objects();
    open_align(&mut harness, id);
    assert!(!harness.state().workbench.is_aligning());
    assert!(note(&harness).contains("other than"));
}
