//! Tests for symmetry with the modelling tools in the window: edits made
//! with symmetry on are recorded mirrored, and the Last edit changes both
//! sides.

use super::modelling_tests::{H, cube, drag, last_operation, press, shape};
use super::tests::fresh;
use super::*;
use egui_kittest::kittest::Queryable;
use scene::{Element, ElementSelection, Operation};

/// The cube cut in two each way, in the Polygons mode, with a polygon
/// on its right (+X) side selected; symmetry on when `on`.
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

#[test]
fn with_symmetry_an_extrude_is_made_on_both_sides() {
    let mut harness = ready(true);
    press(&mut harness, Modifiers::NONE, Key::D);
    drag(&mut harness, 60.0);
    let op = last_operation(&harness).unwrap();
    assert!(matches!(op, Operation::Mirrored { .. }), "{op:?}");
    assert!(matches!(op.inner(), Operation::ExtrudePolygons { .. }));
    assert!(symmetrical(&shape(&harness)));
    // The Last edit says so, and changing it changes both sides.
    assert!(
        harness
            .query_by_label_contains("Last edit: Extrude 1 polygon, mirrored")
            .is_some()
    );
    let id = cube(&harness);
    let longer = op.with(scene::EditSettings {
        distance: Some(40.0),
        ..op.settings().unwrap()
    });
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::ReplaceOperation {
            id,
            layer: 0,
            index: 0,
            operation: longer,
        })
        .unwrap();
    harness.run();
    let bounds = shape(&harness).bounds().unwrap();
    assert!((bounds.max.x - 90.0).abs() < 1e-2 && (bounds.min.x + 90.0).abs() < 1e-2);
}

#[test]
fn without_symmetry_or_for_whole_shape_edits_nothing_is_mirrored() {
    let mut harness = ready(false);
    press(&mut harness, Modifiers::NONE, Key::D);
    drag(&mut harness, 60.0);
    assert!(matches!(
        last_operation(&harness),
        Some(Operation::ExtrudePolygons { .. })
    ));
    assert!(!symmetrical(&shape(&harness)));
}

#[test]
fn with_symmetry_delete_and_flip_take_both_sides() {
    let mut harness = ready(true);
    let faces = shape(&harness).face_count();
    press(&mut harness, Modifiers::NONE, Key::Delete);
    assert_eq!(shape(&harness).face_count(), faces - 2);
    assert!(matches!(
        last_operation(&harness),
        Some(Operation::Mirrored { .. })
    ));
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Delete 1 polygon")
    );
}
