//! Tests for the Live bevel modifier and bevel weights in the window
//! (decision 0149): the palette button, the Mesh menu and command search
//! add it, before any Subdivide or Weighted normals; the Attributes panel
//! shows its settings and says when nothing is marked; the Bevel weight
//! field marks the selected edges in one step.

use super::creases_tests::{drag_field, ready};
use super::modelling_tests::{H, cube, shape};
use super::tests::{fresh, mesh_group};
use super::*;
use crate::edge_values::{BEVEL_WEIGHT_LABEL, CREASE_LABEL};
use egui_kittest::kittest::{NodeT, Queryable};
use scene::{BevelEdges, Command, LiveBevelOptions, Modifier, ModifierKind, ObjectId};

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

fn run(harness: &mut H, command: Command) {
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(command)
        .unwrap();
    harness.run();
}

fn polygons_shown(harness: &H, id: ObjectId) -> usize {
    harness
        .state()
        .workbench
        .document()
        .scene()
        .shown_shape(id)
        .unwrap()
        .face_count()
}

#[test]
fn the_palette_button_adds_it_with_its_settings_in_attributes() {
    let (mut harness, id) = selected_cube();
    harness.get_by_label("Live bevel").click();
    harness.run();
    let list = modifiers(&harness, id);
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].kind,
        ModifierKind::LiveBevel(LiveBevelOptions::default())
    );
    assert_eq!(polygons_shown(&harness, id), 54);
    for name in [
        "Edges",
        "Angle",
        "Width",
        "Segments",
        "Profile",
        "Stop at neighbours",
    ] {
        assert!(harness.query_all_by_label(name).count() >= 1, "{name}");
    }
}

#[test]
fn it_goes_before_weighted_normals_and_says_when_nothing_is_marked() {
    let (mut harness, id) = selected_cube();
    harness.get_by_label("Weighted normals").click();
    harness.run();
    harness.get_by_label("Live bevel").click();
    harness.run();
    let names: Vec<&str> = modifiers(&harness, id)
        .iter()
        .map(|m| m.kind.name())
        .collect();
    assert_eq!(names, ["Live bevel", "Weighted normals"]);
    // Marked edges, with none marked: the panel says why, and the Angle
    // row goes.
    let mut marked = modifiers(&harness, id)[0].clone();
    marked.kind = ModifierKind::LiveBevel(LiveBevelOptions {
        edges: BevelEdges::Marked,
        ..LiveBevelOptions::default()
    });
    run(
        &mut harness,
        Command::ChangeModifier {
            id,
            index: 0,
            modifier: marked,
        },
    );
    assert_eq!(polygons_shown(&harness, id), 6);
    let why = "no edge has a bevel weight";
    assert_eq!(
        harness
            .query_all(egui_kittest::kittest::by().label_contains(why))
            .count(),
        1
    );
    assert_eq!(harness.query_all_by_label("Angle").count(), 0);
}

#[test]
fn the_bevel_weight_field_marks_the_selected_edges_in_one_step() {
    let mut harness = ready(Key::Num2, 2);
    let id = cube(&harness);
    // Both fields show for the selected edges.
    assert!(harness.query_all_by_label(CREASE_LABEL).count() >= 1);
    assert!(harness.query_all_by_label(BEVEL_WEIGHT_LABEL).count() >= 1);
    let steps = harness.state().workbench.document().undo_steps();
    drag_field(&mut harness, BEVEL_WEIGHT_LABEL, 40.0);
    let weights = mesh::bevel_weight::bevel_weights(&shape(&harness));
    assert_eq!(weights.len(), 2);
    assert!(weights.iter().all(|(_, w)| *w > 0.0 && *w <= 1.0));
    assert!(mesh::crease::creases(&shape(&harness)).is_empty());
    assert_eq!(
        harness.state().workbench.document().undo_steps(),
        steps + 1,
        "one drag, one step"
    );
    assert!(
        harness
            .state()
            .workbench
            .document()
            .undo_label()
            .is_some_and(|l| l.starts_with("Bevel weight 2 edges")),
        "{:?}",
        harness.state().workbench.document().undo_label()
    );
    // A Live bevel on marked edges bevels just those two.
    run(
        &mut harness,
        Command::AddModifier {
            ids: vec![id],
            kind: ModifierKind::LiveBevel(LiveBevelOptions {
                edges: BevelEdges::Marked,
                ..LiveBevelOptions::default()
            }),
        },
    );
    assert!(polygons_shown(&harness, id) > 6);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert!(mesh::bevel_weight::bevel_weights(&shape(&harness)).is_empty());
}

#[test]
fn the_mesh_menu_and_command_search_offer_it() {
    let (mut harness, id) = selected_cube();
    mesh_group(&mut harness, super::mesh_menu::MESH_MODIFIERS_MENU);
    harness
        .get_all_by_label("Live bevel")
        .find(|n| !n.accesskit_node().is_disabled())
        .expect("Live bevel can be chosen")
        .click();
    harness.run();
    assert_eq!(modifiers(&harness, id).len(), 1);
    let entries = harness.state().workbench.search_entries(&harness.ctx);
    let entry = entries.iter().find(|e| e.name == "Live bevel").unwrap();
    assert_eq!(entry.place, "Mesh");
    assert!(entry.not_now.is_none());
}
