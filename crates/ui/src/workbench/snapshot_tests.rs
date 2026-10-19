//! Tests for layout snapshots in the window (decision 0114): the UV
//! Editor's Snapshots menu takes, restores, compares and removes them.

use super::modelling_tests::{H, cube, shape};
use super::tests::fresh;
use crate::uv_editor::snapshots::{SNAPSHOTS_MENU, TAKE_SNAPSHOT};
use egui_kittest::kittest::{NodeT, Queryable};
use scene::Operation;

fn uv_edit_with_the_cube() -> (H, scene::ObjectId) {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .switch_layout(super::LayoutChoice::UvEdit);
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    (harness, id)
}

fn open_menu(harness: &mut H) {
    harness.get_by_label(SNAPSHOTS_MENU).click();
    harness.run();
}

fn layout(harness: &H) -> scene::UvLayout {
    scene::UvLayout::of(&shape(harness)).unwrap()
}

#[test]
fn the_menu_takes_restores_compares_and_removes_snapshots() {
    let (mut harness, id) = uv_edit_with_the_cube();
    open_menu(&mut harness);
    assert!(harness.query_by_label("No snapshots yet.").is_some());
    harness.get_by_label(TAKE_SNAPSHOT).click();
    harness.run();
    let saved = layout(&harness);
    let doc = harness.state().workbench.document();
    assert_eq!(doc.scene().snapshots().len(), 1);
    assert_eq!(doc.scene().snapshots()[0].name, "Snapshot 1");
    assert_eq!(doc.undo_label(), Some("Take the snapshot “Snapshot 1”"));
    // Move the UVs, then put the snapshot back from the menu.
    let faces = shape(&harness).faces().map(|(f, _)| f).collect();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Record {
            id,
            operation: Operation::TransformUvs {
                faces,
                points: Vec::new(),
                transform: glam::Affine2::from_angle(0.5),
                anchors: None,
            },
        })
        .unwrap();
    harness.run();
    assert_ne!(layout(&harness), saved);
    harness.get_by_label("Restore").click();
    harness.run();
    assert_eq!(layout(&harness), saved);
    assert!(
        harness
            .query_by_label_contains("Restored “Snapshot 1”")
            .is_some()
    );
    // Compare draws it faintly, and says so.
    harness.get_by_label("Compare").click();
    harness.run();
    assert_eq!(
        harness.state().workbench.uv_editor.compared(),
        Some("Snapshot 1")
    );
    harness.get_by_label("Compare").click();
    harness.run();
    assert_eq!(harness.state().workbench.uv_editor.compared(), None);
    // Remove it: one more step, and the UVs stay.
    harness.get_by_label("×").click();
    harness.run();
    let doc = harness.state().workbench.document();
    assert!(doc.scene().snapshots().is_empty());
    assert_eq!(layout(&harness), saved);
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    assert_eq!(
        harness
            .state()
            .workbench
            .document()
            .scene()
            .snapshots()
            .len(),
        1
    );
}

#[test]
fn taking_needs_objects_with_uvs() {
    let (mut harness, _) = uv_edit_with_the_cube();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .clear_selection();
    harness.run();
    open_menu(&mut harness);
    assert!(
        harness
            .get_by_label(TAKE_SNAPSHOT)
            .accesskit_node()
            .is_disabled()
    );
}
