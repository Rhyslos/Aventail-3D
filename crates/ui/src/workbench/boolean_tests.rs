//! Tests for booleans in the window (decision 0135): the Mesh menu, the
//! modifiers palette and command search start them; the first object
//! selected is kept; the Attributes panel shows them; cutters are drawn
//! as outlines.

use super::modelling_tests::{H, cube, note};
use super::tests::{fresh, mesh_group};
use crate::booleans::KIND_LABEL;
use crate::commands::CommandId;
use egui_kittest::kittest::{NodeT, Queryable};
use scene::{BooleanKind, Command, ObjectId, ObjectKind, PrimitiveKind, Transform};

/// The fresh window's cube and a second cube overlapping its corner,
/// nothing selected.
fn two_cubes() -> (H, ObjectId, ObjectId) {
    let mut harness = fresh();
    let kept = cube(&harness);
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let cutter = doc.selection()[0];
    doc.execute(Command::Rename {
        id: cutter,
        name: "Cutter".into(),
    })
    .unwrap();
    doc.execute(Command::SetTransforms(vec![(
        cutter,
        Transform {
            position: glam::Vec3::splat(50.0),
            ..Transform::default()
        },
    )]))
    .unwrap();
    doc.clear_selection();
    harness.run();
    (harness, kept, cutter)
}

/// Selects `first`, then adds `then` (as Ctrl + click does).
fn pick_in_order(harness: &mut H, first: ObjectId, then: ObjectId) {
    let doc = harness.state_mut().workbench.document_mut();
    doc.set_selection([first]);
    doc.set_selection([first, then]);
    harness.run();
}

fn booleans(harness: &H, id: ObjectId) -> Vec<scene::Boolean> {
    harness
        .state()
        .workbench
        .document()
        .scene()
        .booleans(id)
        .to_vec()
}

fn is_disabled(harness: &H, label: &str) -> bool {
    harness
        .get_all_by_label(label)
        .any(|n| n.accesskit_node().is_disabled())
}

#[test]
fn the_mesh_menu_subtracts_with_the_first_selected_kept() {
    let (mut harness, kept, cutter) = two_cubes();
    // One shape alone can't: the menu says why.
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([kept]);
    harness.run();
    mesh_group(&mut harness, super::mesh_menu::MESH_BOOLEANS_MENU);
    assert!(is_disabled(&harness, "Subtract"));
    harness.key_press(egui::Key::Escape);
    harness.run();
    // The cutter picked second: the first is kept.
    pick_in_order(&mut harness, kept, cutter);
    mesh_group(&mut harness, super::mesh_menu::MESH_BOOLEANS_MENU);
    harness
        .get_all_by_label("Subtract")
        .find(|n| !n.accesskit_node().is_disabled())
        .expect("Subtract can be chosen")
        .click();
    harness.run();
    let list = booleans(&harness, kept);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].kind, BooleanKind::Subtract);
    assert_eq!(list[0].cutters, [cutter]);
    assert_eq!(
        harness.state().workbench.document().selection(),
        [kept],
        "the kept object is selected"
    );
}

#[test]
fn the_order_of_picking_decides_which_is_kept() {
    let (mut harness, kept, cutter) = two_cubes();
    // Picked the other way round, the other one is kept.
    pick_in_order(&mut harness, cutter, kept);
    let mut requests = Vec::new();
    harness
        .state_mut()
        .workbench
        .run_boolean(BooleanKind::Union, &mut requests);
    harness.state_mut().workbench.carry_out(requests);
    assert_eq!(booleans(&harness, cutter)[0].cutters, [kept]);
    assert!(booleans(&harness, kept).is_empty());
}

#[test]
fn a_loop_is_refused_with_a_reason() {
    let (mut harness, kept, cutter) = two_cubes();
    pick_in_order(&mut harness, kept, cutter);
    let mut requests = Vec::new();
    harness
        .state_mut()
        .workbench
        .run_boolean(BooleanKind::Subtract, &mut requests);
    harness.state_mut().workbench.carry_out(requests);
    pick_in_order(&mut harness, cutter, kept);
    let ready = harness.state().workbench.readiness(CommandId::Union);
    assert!(matches!(ready, crate::tool_menu::Ready::NotNow(_)));
    let mut requests = Vec::new();
    harness
        .state_mut()
        .workbench
        .run_boolean(BooleanKind::Union, &mut requests);
    assert!(requests.is_empty());
    assert!(note(&harness).contains("wait"), "{}", note(&harness));
}

#[test]
fn the_modifiers_palette_has_the_three_booleans() {
    let (mut harness, kept, cutter) = two_cubes();
    for name in ["Union", "Subtract", "Intersect"] {
        assert!(harness.query_by_label(name).is_some(), "{name}");
    }
    pick_in_order(&mut harness, kept, cutter);
    harness.get_by_label("Intersect").click();
    harness.run();
    assert_eq!(booleans(&harness, kept)[0].kind, BooleanKind::Intersect);
}

#[test]
fn command_search_finds_them_and_says_when_they_cant_be_used() {
    let (mut harness, kept, cutter) = two_cubes();
    let entries = harness.state().workbench.search_entries(&harness.ctx);
    for name in ["Union", "Subtract", "Intersect"] {
        let entry = entries.iter().find(|e| e.name == name).expect(name);
        assert_eq!(entry.place, "Mesh");
        assert!(entry.not_now.is_some(), "nothing selected");
    }
    pick_in_order(&mut harness, kept, cutter);
    let entries = harness.state().workbench.search_entries(&harness.ctx);
    let subtract = entries.iter().find(|e| e.name == "Subtract").unwrap();
    assert!(subtract.not_now.is_none());
    harness.key_press(egui::Key::Space);
    harness.run();
    harness
        .get_by_label(crate::command_search::SEARCH_LABEL)
        .type_text("subtract");
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(booleans(&harness, kept)[0].cutters, [cutter]);
}

#[test]
fn the_attributes_panel_shows_the_boolean_and_the_cutter_what_it_cuts() {
    let (mut harness, kept, cutter) = two_cubes();
    pick_in_order(&mut harness, kept, cutter);
    let mut requests = Vec::new();
    harness
        .state_mut()
        .workbench
        .run_boolean(BooleanKind::Subtract, &mut requests);
    harness.state_mut().workbench.carry_out(requests);
    harness.run();
    assert!(harness.query_by_label(KIND_LABEL).is_some());
    // The cutter by name, beside its row in the Objects panel.
    assert!(harness.get_all_by_label("Cutter").count() >= 2);
    // Taking the cutter out ends the boolean.
    harness.get_by_label("Take out").click();
    harness.run();
    assert!(booleans(&harness, kept).is_empty());
    harness.state_mut().workbench.document_mut().undo().unwrap();
    // The cutter says what it cuts.
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([cutter]);
    harness.run();
    assert!(harness.query_by_label_contains("Cuts: Cube").is_some());
}

#[test]
fn a_cutter_is_drawn_as_an_outline_and_the_kept_shape_cut() {
    let (mut harness, kept, cutter) = two_cubes();
    pick_in_order(&mut harness, kept, cutter);
    let mut requests = Vec::new();
    harness
        .state_mut()
        .workbench
        .run_boolean(BooleanKind::Subtract, &mut requests);
    harness.state_mut().workbench.carry_out(requests);
    harness.run();
    let view = harness.state().renderer.drawn.last().unwrap().clone();
    let item = |id| view.scene.items.iter().find(|i| i.id == id).unwrap();
    assert!(item(cutter).outline);
    assert!(!item(kept).outline);
    assert_eq!(item(kept).shown.face_count(), 6, "edited uncut");
    assert_eq!(item(kept).drawn.face_count(), 9, "drawn cut");
}
