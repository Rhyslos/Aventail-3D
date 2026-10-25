//! Tests for the game bevel in the window (decision 0156): the palette
//! button, the Mesh menu and command search give the selected shapes a
//! Live bevel with Soft bevels and Weighted normals last, in one undo
//! step; the Attributes panel shows both, with the Soft bevels switch;
//! the bake check finds no hard edges to warn about.

use super::modelling_tests::{H, cube};
use super::tests::{fresh, mesh_group};
use crate::attributes::live_bevel_rows::SOFT_BEVELS;
use egui_kittest::kittest::{NodeT, Queryable};
use scene::{LiveBevelOptions, Modifier, ModifierKind, ObjectId};

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

fn names(harness: &H, id: ObjectId) -> Vec<&'static str> {
    modifiers(harness, id)
        .iter()
        .map(|m| m.kind.name())
        .collect()
}

#[test]
fn the_palette_button_adds_a_soft_live_bevel_and_weighted_normals_in_one_step() {
    let (mut harness, id) = selected_cube();
    let steps = harness.state().workbench.document().undo_steps();
    // The last button: the palette scrolls to it when the window is short.
    harness.get_by_label("Game bevel").scroll_to_me();
    harness.run_steps(4);
    harness.get_by_label("Game bevel").click();
    harness.run();
    assert_eq!(names(&harness, id), ["Live bevel", "Weighted normals"]);
    assert_eq!(
        modifiers(&harness, id)[0].kind,
        ModifierKind::LiveBevel(LiveBevelOptions {
            soften: true,
            ..LiveBevelOptions::default()
        })
    );
    let doc = harness.state().workbench.document();
    assert_eq!(doc.undo_steps(), steps + 1);
    assert_eq!(doc.undo_label(), Some("Game bevel"));
    assert_eq!(
        doc.scene().object(id).unwrap().hardness,
        scene::Hardness::Marked
    );
    // Both are listed under the cube, and each shows its settings, with
    // the switch it turned on.
    for name in ["Live bevel", "Weighted normals"] {
        let row = format!("{}{name}", crate::modifier_list::ROW_PREFIX);
        assert!(harness.query_by_label(&row).is_some(), "{row}");
    }
    super::tests::open_modifier(&mut harness, id, 0);
    for name in ["Width", "Segments", SOFT_BEVELS] {
        assert!(harness.query_all_by_label(name).count() >= 1, "{name}");
    }
    super::tests::open_modifier(&mut harness, id, 1);
    for name in ["Weight by", "Keep hard edges"] {
        assert!(harness.query_all_by_label(name).count() >= 1, "{name}");
    }
    let note = harness.state().workbench.note.clone().unwrap_or_default();
    assert!(note.starts_with("Game bevel:"), "{note}");
    // One Ctrl + Z takes it all back.
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    harness.run();
    assert!(modifiers(&harness, id).is_empty());
    assert!(matches!(
        harness
            .state()
            .workbench
            .document()
            .scene()
            .object(id)
            .unwrap()
            .hardness,
        scene::Hardness::ByAngle(_)
    ));
}

#[test]
fn the_soft_bevels_switch_is_one_undo_step() {
    let (mut harness, id) = selected_cube();
    harness.get_by_label("Live bevel").click();
    harness.run();
    let steps = harness.state().workbench.document().undo_steps();
    harness
        .get_all_by_label(SOFT_BEVELS)
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::CheckBox)
        .expect("the Soft bevels switch")
        .scroll_to_me();
    harness.run();
    harness
        .get_all_by_label(SOFT_BEVELS)
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::CheckBox)
        .expect("the Soft bevels switch")
        .click();
    harness.run();
    let ModifierKind::LiveBevel(o) = modifiers(&harness, id)[0].kind else {
        panic!("a live bevel")
    };
    assert!(o.soften);
    assert_eq!(harness.state().workbench.document().undo_steps(), steps + 1);
}

#[test]
fn the_mesh_menu_and_command_search_offer_it() {
    let (mut harness, id) = selected_cube();
    mesh_group(&mut harness, super::mesh_menu::MESH_MODIFIERS_MENU);
    harness
        .get_all_by_label("Game bevel")
        .find(|n| !n.accesskit_node().is_disabled())
        .expect("Game bevel can be chosen")
        .click();
    harness.run();
    assert_eq!(names(&harness, id), ["Live bevel", "Weighted normals"]);
    let entries = harness.state().workbench.search_entries(&harness.ctx);
    let entry = entries.iter().find(|e| e.name == "Game bevel").unwrap();
    assert_eq!(entry.place, "Mesh");
    assert!(entry.not_now.is_none());
}

#[test]
fn with_nothing_selected_it_says_why() {
    let (mut harness, id) = selected_cube();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .clear_selection();
    harness.run();
    let entries = harness.state().workbench.search_entries(&harness.ctx);
    let entry = entries.iter().find(|e| e.name == "Game bevel").unwrap();
    assert!(entry.not_now.is_some());
    harness.state_mut().workbench.run_game_bevel();
    assert!(modifiers(&harness, id).is_empty());
    assert!(harness.state().workbench.note.is_some());
}

#[test]
fn the_bake_check_finds_no_hard_edges_left() {
    let (mut harness, _) = selected_cube();
    harness.get_by_label("Game bevel").click();
    harness.run();
    let problems = harness.state_mut().workbench.bake_problems();
    assert!(
        !problems
            .iter()
            .any(|p| matches!(p.kind, super::baking::Kind::HardJoined(_))),
        "the bevels shade round: no hard edges for a normal map to hold"
    );
}
