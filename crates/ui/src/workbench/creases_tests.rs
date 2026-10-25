//! Tests for creases and the Subdivide modifier in the window (decision
//! 0143): Crease edges and Uncrease from the menus and the Edges mode's
//! tool menu, the Attributes panel's Crease field, and the Subdivide
//! modifier's palette button and settings.

use super::modelling_tests::{H, cube, note, press, shape};
use super::tests::fresh;
use super::*;
use crate::commands::{CommandId, EDGES_ONLY, menu_for};
use crate::edge_values::CREASE_LABEL;
use egui::{Event, PointerButton};
use egui_kittest::kittest::Queryable;
use scene::{Element, ElementSelection, ModifierKind, SubdivideOptions};

/// The fresh window's cube selected, in `mode_key`'s mode, with its
/// first `n` edges selected.
pub(super) fn ready(mode_key: Key, n: usize) -> H {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    press(&mut harness, Modifiers::NONE, mode_key);
    let mesh = shape(&harness);
    let chosen: Vec<Element> = mesh.edges()[..n]
        .iter()
        .map(|e| Element::Edge(*e))
        .collect();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements([(id, ElementSelection::of(chosen))], "Select", None);
    harness.run();
    harness
}

fn run(harness: &mut H, command: CommandId) {
    let ctx = harness.ctx.clone();
    harness
        .state_mut()
        .workbench
        .run_command(command, &ctx, &mut Vec::new());
    harness.run();
}

fn creases(harness: &H) -> Vec<f32> {
    mesh::crease::creases(&shape(harness))
        .into_iter()
        .map(|(_, c)| c)
        .collect()
}

/// Drags a value field sideways by `dx` points.
pub(super) fn drag_field(harness: &mut H, label: &str, dx: f32) {
    // The field, right of its row's label.
    let from = harness
        .get_all_by_label(label)
        .max_by(|a, b| a.rect().min.x.total_cmp(&b.rect().min.x))
        .unwrap()
        .rect()
        .center();
    harness.event(Event::PointerMoved(from));
    harness.step();
    harness.event(Event::PointerButton {
        pos: from,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    for i in 1..=10 {
        harness.event(Event::PointerMoved(
            from + egui::vec2(dx * i as f32 / 10.0, 0.0),
        ));
        harness.step();
    }
    harness.event(Event::PointerButton {
        pos: from + egui::vec2(dx, 0.0),
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
}

#[test]
fn crease_edges_and_uncrease_change_the_selected_edges() {
    let mut harness = ready(Key::Num2, 2);
    run(&mut harness, CommandId::CreaseEdges);
    assert_eq!(creases(&harness), vec![1.0, 1.0]);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Crease 2 edges")
    );
    let steps = harness.state().workbench.document().undo_steps();
    run(&mut harness, CommandId::CreaseEdges);
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
    assert_eq!(
        note(&harness),
        "Crease edges: the selected edges are already creased."
    );
    run(&mut harness, CommandId::Uncrease);
    assert!(creases(&harness).is_empty());
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Uncrease 2 edges")
    );
    run(&mut harness, CommandId::Uncrease);
    assert_eq!(
        note(&harness),
        "Uncrease: the selected edges have no crease."
    );
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert_eq!(creases(&harness).len(), 2, "undo brings them back");
}

#[test]
fn creases_work_in_the_edges_mode_only() {
    let mut harness = ready(Key::Num3, 0);
    run(&mut harness, CommandId::CreaseEdges);
    assert!(creases(&harness).is_empty());
    assert!(
        note(&harness).starts_with("Crease edges works on edges"),
        "{}",
        note(&harness)
    );
    assert_eq!(harness.query_all_by_label(CREASE_LABEL).count(), 0);
}

#[test]
fn the_edges_ring_holds_the_creases_beside_hard_and_soft_edges() {
    let edges = menu_for(crate::palette::Mode::Edges);
    let (place, group) = EDGES_ONLY[0];
    assert_eq!(group, CommandId::Creases);
    assert!(edges.iter().any(|s| s.place == place && s.command == group));
    let hard = edges
        .iter()
        .find(|s| s.command == CommandId::HardEdges)
        .unwrap();
    assert_eq!(hard.place, place + 1);
    // Not in the Polygons mode, where Flip normals keeps its place.
    let polygons = menu_for(crate::palette::Mode::Polygons);
    assert!(polygons.iter().all(|s| s.command != group));
    assert!(
        polygons
            .iter()
            .any(|s| s.place == place && s.command == CommandId::FlipNormals)
    );
}

#[test]
fn the_crease_field_sets_a_partial_crease_in_one_step() {
    let mut harness = ready(Key::Num2, 3);
    assert!(harness.query_by_label("Selected edges: 3").is_some());
    let steps = harness.state().workbench.document().undo_steps();
    let edits = |h: &H| {
        h.state()
            .workbench
            .document()
            .scene()
            .object(cube(h))
            .unwrap()
            .stack
            .operation_count()
    };
    let before = edits(&harness);
    drag_field(&mut harness, CREASE_LABEL, 40.0);
    let found = creases(&harness);
    assert_eq!(found.len(), 3);
    assert!(found.iter().all(|c| *c > 0.0 && *c < 1.0), "{found:?}");
    assert!(found.windows(2).all(|w| w[0] == w[1]));
    assert_eq!(
        harness.state().workbench.document().undo_steps(),
        steps + 1,
        "one drag, one step"
    );
    assert_eq!(edits(&harness), before + 1, "one drag, one edit");
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert!(creases(&harness).is_empty());
}

#[test]
fn the_palette_adds_a_subdivide_with_its_settings() {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    harness.get_by_label("Subdivide").click();
    harness.run();
    let doc = harness.state().workbench.document();
    let list = doc.scene().modifiers(id);
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].kind,
        ModifierKind::Subdivide(SubdivideOptions::default())
    );
    assert_eq!(doc.scene().shown_shape(id).unwrap().face_count(), 24);
    for row in ["Levels", "Levels for export", "Hard edges stay sharp"] {
        assert!(harness.query_all_by_label(row).count() >= 1, "{row}");
    }
}
