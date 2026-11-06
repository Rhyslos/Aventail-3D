//! Tests for quicker seam work in the window (decision 0234): the
//! Fewer seams ↔ less stretch slider, Mark seam loop and ring, Mirror
//! seams and the seam brush.

use super::modelling_tests::{H, note, press};
use super::seams_tests::{add, inner, plane, seams, shape, steps};
use super::tests::{fresh, uv_group};
use super::*;
use egui_kittest::kittest::Queryable;
use scene::{EdgeKey, Element, ElementSelection};

fn select_edges(harness: &mut H, edges: Vec<EdgeKey>) {
    let doc = harness.state_mut().workbench.document_mut();
    let id = *doc.selection().first().unwrap();
    doc.select_elements(
        [(
            id,
            ElementSelection::of(edges.into_iter().map(Element::Edge)),
        )],
        "Select",
        None,
    );
    harness.run();
}

fn sphere(harness: &mut H) -> scene::ObjectId {
    add(harness, scene::Primitive::Sphere(Default::default()))
}

#[test]
fn the_slider_suggests_seams_live_in_one_undo_step() {
    let mut harness = fresh();
    sphere(&mut harness);
    let ready_made = seams(&harness);
    let before = steps(&harness);
    let w = &mut harness.state_mut().workbench;
    // A drag: the slider moves through several places before letting go.
    w.uv_settings.seam_balance = 0.0;
    w.apply_seam_balance();
    let fewest = mesh::uvs::seams(&shape_of(w)).len();
    assert!(fewest > 0, "a sphere needs a cut to lie flat");
    for t in [0.3, 0.6, 1.0] {
        w.uv_settings.seam_balance = t;
        w.apply_seam_balance();
    }
    let most = mesh::uvs::seams(&shape_of(w)).len();
    assert!(
        most > fewest,
        "less stretch, more seams: {fewest} then {most}"
    );
    assert_eq!(
        w.document().undo_steps(),
        before + 1,
        "one step for the drag"
    );
    // It set the settings Suggest seams uses.
    assert!(w.uv_settings.budget_on);
    assert!((w.uv_settings.seam_corner_angle - uv::seam_tools::TIGHTEST_CORNER).abs() < 1e-3);
    assert!(note(&harness).contains("Ctrl + Z"), "{}", note(&harness));
    // Undo takes the whole drag back.
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    assert_eq!(seams(&harness), ready_made);
    // Letting go ends the step: the next drag is a step of its own.
    harness.state_mut().workbench.seam_slider_key = None;
    let w = &mut harness.state_mut().workbench;
    w.uv_settings.seam_balance = 0.5;
    w.apply_seam_balance();
    w.seam_slider_key = None;
    w.uv_settings.seam_balance = 0.9;
    w.apply_seam_balance();
    assert_eq!(w.document().undo_steps(), before + 2);
}

fn shape_of(w: &Workbench) -> std::sync::Arc<scene::Mesh> {
    let doc = w.document();
    let id = *doc.selection().first().unwrap();
    doc.scene().evaluated(id).unwrap().mesh
}

#[test]
fn the_slider_with_nothing_selected_only_changes_the_settings() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .clear_selection();
    let before = steps(&harness);
    let w = &mut harness.state_mut().workbench;
    w.uv_settings.seam_balance = 0.0;
    w.apply_seam_balance();
    assert_eq!(w.uv_settings.seam_corner_angle, 180.0);
    assert!(!w.uv_settings.budget_on);
    assert_eq!(w.document().undo_steps(), before);
    assert!(
        note(&harness).contains("Select objects"),
        "{}",
        note(&harness)
    );
}

/// A cylinder's upright edge.
fn side_edge(mesh: &scene::Mesh) -> EdgeKey {
    mesh.topology()
        .edges()
        .find(|e| {
            let (a, b) = e.ends();
            let (pa, pb) = (mesh.position(a).unwrap(), mesh.position(b).unwrap());
            // Along the axis: the ends differ in one direction only.
            let d = (pa - pb).abs();
            let moving = [d.x, d.y, d.z].iter().filter(|c| **c > 1e-3).count();
            moving == 1
        })
        .unwrap()
}

#[test]
fn mark_seam_loop_and_ring_widen_the_selected_edge() {
    let mut harness = fresh();
    add(
        &mut harness,
        scene::Primitive::Cylinder(mesh::CylinderParams {
            height_segments: 3,
            ..Default::default()
        }),
    );
    press(&mut harness, Modifiers::NONE, Key::Num2);
    let edge = side_edge(&shape(&harness));
    select_edges(&mut harness, vec![edge]);
    let before = steps(&harness);
    harness.state_mut().workbench.run_seam_loops(false);
    harness.run();
    let along = seams(&harness);
    assert!(along.contains(&edge) && along.len() >= 3, "{}", along.len());
    assert_eq!(steps(&harness), before + 1);
    // Again: nothing more to mark, nothing recorded.
    harness.state_mut().workbench.run_seam_loops(false);
    assert_eq!(steps(&harness), before + 1);
    assert!(note(&harness).contains("already"), "{}", note(&harness));
    harness.state_mut().workbench.run_seam_loops(true);
    harness.run();
    assert!(seams(&harness).len() > along.len(), "the ring goes round");
}

#[test]
fn loops_and_rings_need_an_edge() {
    let mut harness = plane(Key::Num3, |_| Vec::new());
    harness.state_mut().workbench.run_seam_loops(false);
    assert!(note(&harness).contains("Edges mode"), "{}", note(&harness));
    harness.state_mut().workbench.mode = Mode::Edges;
    harness.state_mut().workbench.run_seam_loops(true);
    assert!(
        note(&harness).contains("select an edge"),
        "{}",
        note(&harness)
    );
}

#[test]
fn mirror_seams_copies_one_sides_seams_to_the_other() {
    let mut harness = fresh();
    let id = add(
        &mut harness,
        scene::Primitive::Cube(mesh::CubeParams {
            segments: [2, 2, 2],
            ..Default::default()
        }),
    );
    let mesh = shape(&harness);
    // The inner edges wholly on the right.
    let right: Vec<EdgeKey> = inner(&mesh)
        .into_iter()
        .filter(|e| {
            let (a, b) = e.ends();
            mesh.position(a).unwrap().x > 1e-3 && mesh.position(b).unwrap().x > 1e-3
        })
        .collect();
    let op = super::seams::seams_op(&mesh, right.clone(), true);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Record { id, operation: op })
        .unwrap();
    harness.run();
    let before = steps(&harness);
    harness.state_mut().workbench.run_mirror_seams();
    harness.run();
    // Each right-hand edge's mirror on the left is a seam now.
    let now = seams(&harness);
    let mesh = shape(&harness);
    let at = |v: scene::VertexId| mesh.position(v).unwrap();
    for e in &right {
        let (a, b) = e.ends();
        let (pa, pb) = (
            at(a) * glam::Vec3::new(-1.0, 1.0, 1.0),
            at(b) * glam::Vec3::new(-1.0, 1.0, 1.0),
        );
        let mirror = now.iter().find(|m| {
            let (c, d) = m.ends();
            (at(c).distance(pa) < 1e-4 && at(d).distance(pb) < 1e-4)
                || (at(c).distance(pb) < 1e-4 && at(d).distance(pa) < 1e-4)
        });
        assert!(mirror.is_some(), "{e:?}");
    }
    assert_eq!(steps(&harness), before + 1);
    harness.state_mut().workbench.run_mirror_seams();
    assert_eq!(steps(&harness), before + 1);
    assert!(note(&harness).contains("same seams"), "{}", note(&harness));
}

#[test]
fn the_seam_brush_marks_a_stroke_in_one_step_and_ctrl_clears() {
    let mut harness = plane(Key::Num1, |_| Vec::new());
    let id = *harness
        .state()
        .workbench
        .document()
        .selection()
        .first()
        .unwrap();
    let edges = inner(&shape(&harness));
    assert!(edges.len() >= 4);
    harness.state_mut().workbench.start_seam_brush();
    harness.run();
    let w = &mut harness.state_mut().workbench;
    assert_eq!(w.mode, Mode::Edges, "the brush paints edges");
    assert_eq!(
        w.viewport.select_tool(),
        crate::selecting::SelectTool::Brush
    );
    let before = w.document().undo_steps();
    // One stroke over two dabs.
    w.seam_dab(Some(1), vec![(id, edges[0]), (id, edges[1])]);
    w.seam_dab(Some(1), vec![(id, edges[1]), (id, edges[2])]);
    w.finish_seam_stroke();
    assert_eq!(mesh::uvs::seams(&shape_of(w)).len(), 3);
    assert_eq!(w.document().undo_steps(), before + 1, "one step per stroke");
    // A Ctrl stroke clears.
    w.seam_brush.as_mut().unwrap().ctrl = true;
    w.seam_dab(Some(2), vec![(id, edges[0])]);
    w.finish_seam_stroke();
    assert_eq!(mesh::uvs::seams(&shape_of(w)).len(), 2);
    assert_eq!(w.document().undo_steps(), before + 2);
    // Going over nothing records nothing.
    w.seam_brush.as_mut().unwrap().ctrl = false;
    w.seam_dab(Some(3), Vec::new());
    w.finish_seam_stroke();
    assert_eq!(w.document().undo_steps(), before + 2);
    // Undo takes back one stroke at a time.
    w.document_mut().undo();
    assert_eq!(mesh::uvs::seams(&shape_of(w)).len(), 3);
}

#[test]
fn brackets_size_the_seam_brush_and_esc_puts_it_down() {
    let mut harness = plane(Key::Num2, |_| Vec::new());
    harness
        .state_mut()
        .workbench
        .viewport
        .set_select_tool(crate::selecting::SelectTool::Box);
    harness.state_mut().workbench.start_seam_brush();
    harness.run();
    let radius = harness.state().workbench.viewport.brush_radius();
    press(&mut harness, Modifiers::NONE, Key::CloseBracket);
    assert!(harness.state().workbench.viewport.brush_radius() > radius);
    press(&mut harness, Modifiers::NONE, Key::OpenBracket);
    press(&mut harness, Modifiers::NONE, Key::OpenBracket);
    assert!(harness.state().workbench.viewport.brush_radius() < radius);
    press(&mut harness, Modifiers::NONE, Key::Escape);
    let w = &harness.state().workbench;
    assert!(!w.seam_brush_on());
    assert_eq!(
        w.viewport.select_tool(),
        crate::selecting::SelectTool::Box,
        "as before"
    );
    // Something still selected: Esc only put the brush down.
    assert!(!w.document().selection().is_empty());
}

#[test]
fn leaving_the_edges_mode_puts_the_seam_brush_down() {
    let mut harness = plane(Key::Num2, |_| Vec::new());
    harness.state_mut().workbench.start_seam_brush();
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::Num3);
    assert!(!harness.state().workbench.seam_brush_on());
}

#[test]
fn the_seam_brush_needs_an_object() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .clear_selection();
    harness.state_mut().workbench.start_seam_brush();
    assert!(!harness.state().workbench.seam_brush_on());
    assert!(
        note(&harness).contains("select the objects"),
        "{}",
        note(&harness)
    );
}

#[test]
fn the_uv_menus_seams_group_has_the_new_tools() {
    let mut harness = fresh();
    sphere(&mut harness);
    uv_group(&mut harness, "Seams");
    for label in [
        seam_tools::SEAM_LOOP_LABEL,
        seam_tools::SEAM_RING_LABEL,
        seam_tools::MIRROR_SEAMS_LABEL,
        seam_tools::SEAM_BRUSH_LABEL,
        seam_tools::BALANCE_LABEL,
    ] {
        assert!(
            harness.query_by_label_contains(label).is_some(),
            "{label} is in UV → Seams"
        );
    }
    harness.get_by_label(seam_tools::SEAM_BRUSH_LABEL).click();
    harness.run();
    assert!(harness.state().workbench.seam_brush_on());
}
