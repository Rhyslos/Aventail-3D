//! Tests for the Struts modifier in the window (decision 0159): the
//! palette button, the Mesh menu and command search add it, and its
//! settings show in the Attributes panel.

use super::modelling_tests::H;
use super::tests::{empty, mesh_group};
use egui_kittest::kittest::{NodeT, Queryable};
use scene::{Command, ModifierKind, ObjectId, ObjectKind, StrutsOptions};

/// A window with a cube, selected, in the Model mode.
fn cube() -> (H, ObjectId) {
    let mut harness = empty();
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(Command::Create(ObjectKind::Primitive(
        scene::PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let id = doc.selection()[0];
    harness.run();
    (harness, id)
}

fn options(harness: &H, id: ObjectId) -> StrutsOptions {
    match &harness.state().workbench.document().scene().modifiers(id)[0].kind {
        ModifierKind::Struts(options) => *options,
        other => panic!("{other:?}"),
    }
}

fn shows(harness: &H, name: &str) -> bool {
    harness.query_all_by_label(name).count() >= 1
}

#[test]
fn the_palette_button_turns_the_cube_into_a_frame() {
    let (mut harness, id) = cube();
    harness.get_by_label("Struts").click();
    harness.run();
    assert_eq!(options(&harness, id), StrutsOptions::default());
    let doc = harness.state().workbench.document();
    let input = doc.scene().unmodified_shape(id).unwrap();
    let shown = doc.scene().shown_shape(id).unwrap();
    assert_eq!(
        shown.topology().pieces().len(),
        input.edges().len() + input.vertex_count()
    );
    for name in ["Thickness", "Sides", "Even thickness", "Keep the faces"] {
        assert!(shows(&harness, name), "{name}");
    }
}

#[test]
fn keep_the_faces_is_one_undo_step() {
    let (mut harness, id) = cube();
    harness.get_by_label("Struts").click();
    harness.run();
    let toggle = harness
        .get_all_by_label("Keep the faces")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::CheckBox)
        .expect("the Keep the faces switch");
    toggle.scroll_to_me();
    harness.run_steps(4);
    harness
        .get_all_by_label("Keep the faces")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::CheckBox)
        .unwrap()
        .click();
    harness.run();
    assert!(options(&harness, id).keep_faces);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Change Struts")
    );
}

#[test]
fn the_mesh_menu_and_command_search_offer_it_under_generate() {
    let (mut harness, id) = cube();
    let entries = harness.state().workbench.search_entries(&harness.ctx);
    let entry = entries.iter().find(|e| e.name == "Struts").unwrap();
    assert_eq!(entry.place, "Mesh");
    assert!(entry.not_now.is_none());
    mesh_group(&mut harness, super::mesh_menu::MESH_MODIFIERS_MENU);
    harness
        .get_all_by_label("Struts")
        .find(|n| !n.accesskit_node().is_disabled())
        .expect("Struts can be chosen")
        .click();
    harness.run();
    let doc = harness.state().workbench.document();
    assert_eq!(doc.scene().modifiers(id).len(), 1);
    assert_eq!(doc.undo_label(), Some("Add Struts"));
}
