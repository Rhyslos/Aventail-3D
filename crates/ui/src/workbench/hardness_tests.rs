//! Tests for hard and soft edges in the window, and the Display menu's
//! Hard edges and Normals.

use super::modelling_tests::{H, cube, note, press, shape};
use super::selection::SelectRequest;
use super::tests::fresh;
use super::*;
use crate::commands::CommandId;
use egui_kittest::kittest::Queryable;
use scene::{Element, ElementSelection};

/// The fresh window's cube selected, in `mode_key`'s mode, with
/// `elements` selected.
fn ready(mode_key: Key, elements: impl Fn(&scene::Mesh) -> Vec<Element>) -> H {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    press(&mut harness, Modifiers::NONE, mode_key);
    let chosen = elements(&shape(&harness));
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

fn first_edge(mesh: &scene::Mesh) -> Vec<Element> {
    vec![Element::Edge(mesh.edges()[0])]
}

fn top(mesh: &scene::Mesh) -> Vec<Element> {
    mesh.faces()
        .map(|(f, _)| f)
        .filter(|f| mesh.face_normal(*f).z > 0.9)
        .map(Element::Polygon)
        .collect()
}

#[test]
fn soft_edges_softens_the_selected_edge_and_hard_edges_says_it_already_is() {
    let mut harness = ready(Key::Num2, first_edge);
    let steps = harness.state().workbench.document().undo_steps();
    run(&mut harness, CommandId::HardEdges);
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
    assert_eq!(
        note(&harness),
        "Mark hard edges: the selected edges are already hard."
    );
    run(&mut harness, CommandId::SoftEdges);
    assert_eq!(shape(&harness).hard_edge_count(), 11);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Soften 1 edge")
    );
}

#[test]
fn in_the_polygons_mode_the_polygons_edges_change() {
    let mut harness = ready(Key::Num3, top);
    run(&mut harness, CommandId::SoftEdges);
    assert_eq!(
        shape(&harness).hard_edge_count(),
        8,
        "the top's four went soft"
    );
    // In the Points mode it says where it works.
    press(&mut harness, Modifiers::NONE, Key::Num1);
    run(&mut harness, CommandId::HardEdges);
    assert!(note(&harness).starts_with("Mark hard edges works on edges or polygons"));
}

#[test]
fn hard_edges_by_angle_shows_its_angle_as_the_last_edit() {
    let mut harness = ready(Key::Num2, first_edge);
    run(&mut harness, CommandId::SoftEdges);
    harness.state_mut().workbench.run_hard_by_angle();
    harness.run();
    assert_eq!(
        shape(&harness).hard_edge_count(),
        12,
        "a cube's corners are sharp"
    );
    assert!(
        harness
            .query_by_label_contains("Last edit: Hard edges by angle")
            .is_some()
    );
    assert!(harness.query_all_by_label("Angle").count() >= 1);
    assert!(note(&harness).starts_with("Edges sharper than 30°"));
}

#[test]
fn select_hard_edges_selects_them_in_the_edges_mode() {
    let mut harness = ready(Key::Num3, |_| Vec::new());
    harness.state_mut().workbench.mode = crate::palette::Mode::Edges;
    harness
        .state_mut()
        .workbench
        .select_requests
        .push(SelectRequest::HardEdges);
    harness.run();
    let id = cube(&harness);
    let doc = harness.state().workbench.document();
    assert_eq!(doc.elements(id).unwrap().edges().count(), 12);
}

#[test]
fn the_display_menu_marks_hard_edges_and_shows_normals() {
    let mut harness = ready(Key::Num2, |_| Vec::new());
    let count = |harness: &H, style| {
        harness
            .state()
            .workbench
            .components
            .overlay()
            .lines
            .iter()
            .filter(|(_, s)| *s == style)
            .count()
    };
    assert_eq!(
        count(&harness, crate::OverlayStyle::Hard),
        12,
        "on as standard"
    );
    assert_eq!(
        count(&harness, crate::OverlayStyle::Normal),
        0,
        "off as standard"
    );
    let options = harness.state_mut().workbench.viewport.options_mut();
    options.show_normals = true;
    options.hide_hard_edges = true;
    harness.run();
    assert_eq!(count(&harness, crate::OverlayStyle::Hard), 0);
    assert_eq!(count(&harness, crate::OverlayStyle::Normal), 6);
    // Both switches are in the viewport's Display menu.
    harness.get_by_label("Display").click();
    harness.run();
    assert!(harness.query_by_label("Show normals").is_some());
    assert!(harness.query_by_label("Show hard edges").is_some());
}

#[test]
fn automatic_hard_edges_is_on_in_the_mesh_menu_and_hand_edits_say_so() {
    use crate::viewport::chrome::{AUTO_HARD_EDGES_ENTRY, HARD_EDGES_ENTRY};
    let mut harness = ready(Key::Num2, first_edge);
    let options = |h: &H| h.state().workbench.viewport.options();
    assert!(!options(&harness).hard_edges_by_hand, "on as standard");
    assert_eq!(
        options(&harness).auto_hard_angle(),
        Some(crate::AUTO_HARD_ANGLE)
    );
    // Softening an edge by hand is kept, and the note says the angle
    // decides the shading for now.
    run(&mut harness, CommandId::SoftEdges);
    assert_eq!(shape(&harness).hard_edge_count(), 11);
    assert!(
        note(&harness).contains("Automatic hard edges is on"),
        "{}",
        note(&harness)
    );
    // The switch is in the main Mesh menu, not the viewport's Display menu
    // (moved there at the project lead's request, decision 0065).
    harness.get_by_label("Display").click();
    harness.run();
    assert!(harness.query_by_label(HARD_EDGES_ENTRY).is_some());
    assert!(harness.query_by_label(AUTO_HARD_EDGES_ENTRY).is_none());
    harness.key_press(egui::Key::Escape);
    harness.run();
    crate::workbench::tests::main_menu(&harness, "Mesh").click();
    harness.run();
    harness.get_by_label(AUTO_HARD_EDGES_ENTRY).click();
    harness.run();
    assert!(options(&harness).hard_edges_by_hand);
    assert_eq!(options(&harness).auto_hard_angle(), None);
    // Switched off, hand edits say nothing more.
    run(&mut harness, CommandId::HardEdges);
    assert!(!note(&harness).contains("Automatic"), "{}", note(&harness));
}
