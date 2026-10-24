//! Tests for the Shell modifier in the window (decision 0154): the
//! palette button, the Mesh menu and command search add it, and its
//! settings show in the Attributes panel, the Direction list changing
//! it in one undo step.

use super::modelling_tests::H;
use super::tests::{empty, mesh_group};
use egui_kittest::kittest::{NodeT, Queryable};
use scene::{Command, ModifierKind, ObjectId, ObjectKind, ShellDirection, ShellOptions};

/// A window with a plane, selected, in the Model mode.
fn plane() -> (H, ObjectId) {
    let mut harness = empty();
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(Command::Create(ObjectKind::Primitive(
        scene::PrimitiveKind::Plane.standard(),
    )))
    .unwrap();
    let id = doc.selection()[0];
    harness.run();
    (harness, id)
}

fn options(harness: &H, id: ObjectId) -> ShellOptions {
    match &harness.state().workbench.document().scene().modifiers(id)[0].kind {
        ModifierKind::Shell(options) => *options,
        other => panic!("{other:?}"),
    }
}

fn shows(harness: &H, name: &str) -> bool {
    harness.query_all_by_label(name).count() >= 1
}

#[test]
fn the_palette_button_gives_the_plane_thickness() {
    let (mut harness, id) = plane();
    harness.get_by_label("Shell").click();
    harness.run();
    assert_eq!(options(&harness, id), ShellOptions::default());
    let doc = harness.state().workbench.document();
    let input = doc.scene().unmodified_shape(id).unwrap();
    let shown = doc.scene().shown_shape(id).unwrap();
    assert!(shown.face_count() > input.face_count() * 2);
    for name in ["Thickness", "Direction", "Even thickness", "Fill the rims"] {
        assert!(shows(&harness, name), "{name}");
    }
}

#[test]
fn the_direction_list_changes_it() {
    let (mut harness, id) = plane();
    harness.get_by_label("Shell").click();
    harness.run();
    let list = || {
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::ComboBox
    };
    harness
        .get_all_by_label("Direction")
        .find(list())
        .expect("the Direction list")
        .scroll_to_me();
    harness.run_steps(4);
    harness
        .get_all_by_label("Direction")
        .find(list())
        .expect("the Direction list")
        .click();
    harness.run_steps(4);
    harness.get_by_label("Both sides").click();
    harness.run();
    assert_eq!(options(&harness, id).direction, ShellDirection::Both);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Change Shell")
    );
}

#[test]
fn the_mesh_menu_and_command_search_offer_it_under_generate() {
    let (mut harness, id) = plane();
    let entries = harness.state().workbench.search_entries(&harness.ctx);
    let entry = entries.iter().find(|e| e.name == "Shell").unwrap();
    assert_eq!(entry.place, "Mesh");
    assert!(entry.not_now.is_none());
    mesh_group(&mut harness, super::mesh_menu::MESH_MODIFIERS_MENU);
    harness
        .get_all_by_label("Shell")
        .find(|n| !n.accesskit_node().is_disabled())
        .expect("Shell can be chosen")
        .click();
    harness.run();
    let doc = harness.state().workbench.document();
    assert_eq!(doc.scene().modifiers(id).len(), 1);
    assert_eq!(doc.undo_label(), Some("Add Shell"));
}
