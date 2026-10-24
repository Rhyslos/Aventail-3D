//! Tests for the Decimate modifier in the window (decision 0145): the
//! palette button, the Mesh menu and command search add it; the
//! Attributes panel shows its settings and the triangles shown.

use super::modelling_tests::{H, cube};
use super::tests::{fresh, mesh_group};
use egui_kittest::kittest::{NodeT, Queryable};
use scene::{DecimateOptions, Modifier, ModifierKind, ObjectId};

fn modifiers(harness: &H, id: ObjectId) -> Vec<Modifier> {
    harness
        .state()
        .workbench
        .document()
        .scene()
        .modifiers(id)
        .to_vec()
}

/// The fresh window's cube, selected.
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
fn the_palette_button_adds_a_decimate_with_its_settings_in_attributes() {
    let (mut harness, id) = selected_cube();
    harness.get_by_label("Decimate").click();
    harness.run();
    let list = modifiers(&harness, id);
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].kind,
        ModifierKind::Decimate(DecimateOptions::default())
    );
    for name in [
        "Reduce by",
        "Ratio",
        "Keep UV seams",
        "Keep hard edges",
        "Keep material borders",
        "Keep open borders",
        "Quads where flat",
    ] {
        assert!(harness.query_all_by_label(name).count() >= 1, "{name}");
    }
    // The cube's corners all stay: its 12 triangles are shown.
    assert!(
        harness.query_all_by_label("12").count() >= 1,
        "triangles shown"
    );
    // A switch is one undo step.
    let switch = harness
        .get_all_by_label("Quads where flat")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::CheckBox)
        .expect("the switch");
    switch.scroll_to_me();
    harness.run();
    harness
        .get_all_by_label("Quads where flat")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::CheckBox)
        .expect("the switch")
        .click();
    harness.run();
    let ModifierKind::Decimate(options) = modifiers(&harness, id)[0].kind else {
        panic!("decimate")
    };
    assert!(options.quads_where_flat);
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    assert_eq!(
        modifiers(&harness, id)[0].kind,
        ModifierKind::Decimate(DecimateOptions::default())
    );
}

#[test]
fn the_mesh_menu_and_command_search_offer_it_under_clean_up() {
    let (mut harness, id) = selected_cube();
    mesh_group(&mut harness, super::mesh_menu::MESH_MODIFIERS_MENU);
    harness
        .get_all_by_label("Decimate")
        .find(|n| !n.accesskit_node().is_disabled())
        .expect("Decimate can be chosen")
        .click();
    harness.run();
    assert_eq!(modifiers(&harness, id).len(), 1);
    let entries = harness.state().workbench.search_entries(&harness.ctx);
    let entry = entries.iter().find(|e| e.name == "Decimate").unwrap();
    assert_eq!(entry.place, "Mesh");
    assert!(entry.not_now.is_none());
}
