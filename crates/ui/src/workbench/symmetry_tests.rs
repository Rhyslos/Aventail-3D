//! Tests for the live mirror with the modelling tools in the window
//! (decision 0088): edits are made once, on the shape itself, and its
//! mirrored twin follows.

use super::modelling_tests::{H, cube, drag, last_operation, press, shape};
use super::tests::fresh;
use super::*;
use egui_kittest::kittest::Queryable;
use scene::{Element, ElementSelection, Operation};

/// The cube cut in two each way, in the Polygons mode, with a polygon
/// on its right (+X) side selected; a live mirror on when `on`.
fn ready(on: bool) -> H {
    let mut harness = fresh();
    let id = cube(&harness);
    let doc = harness.state_mut().workbench.document_mut();
    doc.set_selection([id]);
    doc.execute(scene::Command::SetPrimitive {
        ids: vec![id],
        primitive: scene::Primitive::Cube(mesh::CubeParams {
            segments: [2, 2, 2],
            ..Default::default()
        }),
    })
    .unwrap();
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::Num3);
    let mesh = shape(&harness);
    let right = mesh
        .faces()
        .map(|(f, _)| f)
        .find(|f| mesh.face_normal(*f).x > 0.9)
        .unwrap();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements(
            [(id, ElementSelection::of([Element::Polygon(right)]))],
            "Select",
            None,
        );
    if on {
        harness.state_mut().workbench.toggle_symmetry();
    }
    harness.run();
    harness
}

fn symmetrical(mesh: &scene::Mesh) -> bool {
    mesh.vertices().all(|(_, p)| {
        let m = glam::Vec3::new(-p.x, p.y, p.z);
        mesh.vertices().any(|(_, q)| q.distance(m) < 1e-2)
    })
}

/// The cube's shape as shown, with its live mirror.
fn shown(harness: &H) -> std::sync::Arc<scene::Mesh> {
    let doc = harness.state().workbench.document();
    doc.scene().shown_shape(cube(harness)).unwrap()
}

#[test]
fn with_a_mirror_an_extrude_is_made_once_and_shown_on_both_sides() {
    let mut harness = ready(true);
    press(&mut harness, Modifiers::NONE, Key::D);
    drag(&mut harness, 60.0);
    let op = last_operation(&harness).unwrap();
    assert!(matches!(op, Operation::ExtrudePolygons { .. }), "{op:?}");
    // The shape itself has one extrude, on the right.
    assert!(!symmetrical(&shape(&harness)));
    // As shown (drawn and exported), both sides have it.
    assert!(symmetrical(&shown(&harness)));
    assert!(
        harness
            .query_by_label_contains("Last edit: Extrude 1 polygon")
            .is_some()
    );
}

#[test]
fn without_a_mirror_nothing_is_mirrored() {
    let mut harness = ready(false);
    press(&mut harness, Modifiers::NONE, Key::D);
    drag(&mut harness, 60.0);
    assert!(matches!(
        last_operation(&harness),
        Some(Operation::ExtrudePolygons { .. })
    ));
    assert!(!symmetrical(&shown(&harness)));
}

#[test]
fn with_a_mirror_delete_is_made_once_and_the_twin_follows() {
    let mut harness = ready(true);
    let faces = shape(&harness).face_count();
    let shown_faces = shown(&harness).face_count();
    press(&mut harness, Modifiers::NONE, Key::Delete);
    assert_eq!(shape(&harness).face_count(), faces - 1);
    assert_eq!(shown(&harness).face_count(), shown_faces - 2);
    assert!(matches!(
        last_operation(&harness),
        Some(Operation::DeleteElements { .. })
    ));
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Delete 1 polygon")
    );
}
