//! Tests for seams in the window (decision 0098): Mark and Clear seams on
//! edges and around polygons, Seams from hard edges, Suggest seams,
//! Select seams and the UV menu.

use super::modelling_tests::{H, cube, note, press};
use super::tests::{fresh, main_menu};
use super::*;
use crate::commands::CommandId;
use egui_kittest::kittest::Queryable;
use scene::{EdgeKey, Element, ElementSelection};
use std::collections::BTreeSet;

/// Adds `primitive` to the fresh window's scene, selected alone.
fn add(harness: &mut H, primitive: scene::Primitive) -> scene::ObjectId {
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(scene::Command::Create(scene::ObjectKind::Primitive(
        primitive,
    )))
    .unwrap();
    let id = *doc.selection().first().unwrap();
    harness.run();
    id
}

/// The first selected object's shape.
fn shape(harness: &H) -> std::sync::Arc<scene::Mesh> {
    let doc = harness.state().workbench.document();
    let id = *doc.selection().first().unwrap();
    doc.scene().evaluated(id).unwrap().mesh
}

/// A plane cut in two each way, selected, in `mode_key`'s mode, with
/// `elements` selected.
fn plane(mode_key: Key, elements: impl Fn(&scene::Mesh) -> Vec<Element>) -> H {
    let mut harness = fresh();
    let id = add(
        &mut harness,
        scene::Primitive::Plane(mesh::PlaneParams {
            segments: [2, 2],
            ..Default::default()
        }),
    );
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

fn inner(mesh: &scene::Mesh) -> Vec<EdgeKey> {
    let topology = mesh.topology();
    topology
        .edges()
        .filter(|e| topology.edge_uses(*e).len() == 2)
        .collect()
}

fn seams(harness: &H) -> BTreeSet<EdgeKey> {
    mesh::uvs::seams(&shape(harness)).into_iter().collect()
}

fn steps(harness: &H) -> usize {
    harness.state().workbench.document().undo_steps()
}

#[test]
fn mark_and_clear_seams_on_the_selected_edges_as_one_step_each() {
    let mut harness = plane(Key::Num2, |m| vec![Element::Edge(inner(m)[0])]);
    let edge = inner(&shape(&harness))[0];
    assert!(seams(&harness).is_empty(), "a plane has no seams");
    let before = steps(&harness);
    run(&mut harness, CommandId::MarkSeam);
    assert_eq!(seams(&harness), [edge].into_iter().collect());
    assert_eq!(steps(&harness), before + 1);
    assert!(note(&harness).contains("Ctrl + Z"), "{}", note(&harness));
    // Again: already a seam, and nothing is recorded.
    run(&mut harness, CommandId::MarkSeam);
    assert_eq!(steps(&harness), before + 1);
    assert!(note(&harness).contains("already"));
    run(&mut harness, CommandId::ClearSeam);
    assert!(seams(&harness).is_empty());
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    assert_eq!(seams(&harness).len(), 1);
}

#[test]
fn with_polygons_selected_the_edges_around_them_become_seams() {
    let mut harness = plane(Key::Num3, |m| {
        vec![Element::Polygon(m.faces().next().unwrap().0)]
    });
    run(&mut harness, CommandId::MarkSeam);
    // A corner polygon of the plane: two of its sides are open edges,
    // the other two border its neighbours.
    let marked = seams(&harness);
    assert_eq!(marked.len(), 2);
    let mesh = shape(&harness);
    let (face, _) = mesh.faces().next().unwrap();
    for e in &marked {
        assert!(mesh.topology().face_edges(face).contains(e));
    }
    // Clearing takes every edge of the polygon.
    run(&mut harness, CommandId::ClearSeam);
    assert!(seams(&harness).is_empty());
}

#[test]
fn seams_need_an_editing_mode() {
    let mut harness = plane(Key::Num2, |_| Vec::new());
    harness.state_mut().workbench.mode = Mode::Model;
    let before = steps(&harness);
    run(&mut harness, CommandId::MarkSeam);
    assert_eq!(steps(&harness), before);
    assert!(note(&harness).contains("Edges mode"), "{}", note(&harness));
    // Nothing selected in the Edges mode says so.
    harness.state_mut().workbench.mode = Mode::Edges;
    run(&mut harness, CommandId::MarkSeam);
    assert!(
        note(&harness).contains("select some edges"),
        "{}",
        note(&harness)
    );
}

#[test]
fn seams_from_hard_edges_puts_back_a_cubes_twelve() {
    let mut harness = fresh();
    // Only the marks here: unwrapping a seamless cube would cut it.
    harness.state_mut().workbench.uv_settings.live_unwrap = false;
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    assert_eq!(
        seams(&harness).len(),
        12,
        "a cube comes cut along its edges"
    );
    // Clear every seam: all polygons, Clear seams.
    press(&mut harness, Modifiers::NONE, Key::Num3);
    let all: Vec<Element> = shape(&harness)
        .faces()
        .map(|(f, _)| Element::Polygon(f))
        .collect();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements([(id, ElementSelection::of(all))], "Select", None);
    run(&mut harness, CommandId::ClearSeam);
    assert!(seams(&harness).is_empty());
    harness.state_mut().workbench.run_seams_from_hard_edges();
    harness.run();
    assert_eq!(seams(&harness).len(), 12);
    // Again: nothing to do.
    let before = steps(&harness);
    harness.state_mut().workbench.run_seams_from_hard_edges();
    assert_eq!(steps(&harness), before);
    assert!(note(&harness).contains("already"));
}

#[test]
fn suggest_seams_replaces_the_seams_so_every_piece_lies_flat() {
    let mut harness = fresh();
    add(&mut harness, scene::Primitive::Sphere(Default::default()));
    let before = seams(&harness);
    let steps_before = steps(&harness);
    harness.state_mut().workbench.run_suggest_seams();
    harness.run();
    let after = seams(&harness);
    assert_ne!(after, before);
    assert_eq!(steps(&harness), steps_before + 1, "one step");
    let mesh = shape(&harness);
    for piece in uv::seams::pieces(&mesh, &after) {
        assert!(uv::seams::is_disk(&mesh, &piece, &after));
    }
    assert!(note(&harness).contains("suggested"), "{}", note(&harness));
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    assert_eq!(seams(&harness), before);
}

#[test]
fn select_seams_selects_them_in_the_edges_mode() {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    harness.state_mut().workbench.run_select_seams();
    harness.run();
    harness.run();
    let bench = &harness.state().workbench;
    assert_eq!(bench.mode, Mode::Edges);
    let selected = bench.document().elements(id).unwrap();
    assert_eq!(selected.count(scene::ElementKind::Edges), 12);
}

#[test]
fn the_uv_menu_holds_the_seam_tools() {
    let mut harness = plane(Key::Num2, |m| vec![Element::Edge(inner(m)[1])]);
    // The UV menu shows its groups (decision 0113).
    main_menu(&harness, "UV").click();
    harness.run();
    for group in [
        super::UV_SEAMS_MENU,
        super::UV_TURN_MENU,
        super::UV_LAY_OUT_MENU,
        super::UV_PIECES_MENU,
        super::UV_SETTINGS_MENU,
    ] {
        assert!(harness.query_by_label_contains(group).is_some(), "{group}");
    }
    harness.key_press(Key::Escape);
    harness.run();
    super::tests::uv_group(&mut harness, super::UV_SEAMS_MENU);
    for label in [
        "Mark seams",
        "Clear seams",
        super::seams::SEAMS_FROM_HARD_LABEL,
        super::seams::SUGGEST_SEAMS_LABEL,
        super::seams::SELECT_SEAMS_LABEL,
    ] {
        assert!(harness.query_by_label(label).is_some(), "{label}");
    }
    harness.get_by_label("Mark seams").click();
    harness.run();
    assert_eq!(seams(&harness).len(), 1);
}
