//! Exploratory checks of the UV suite (editor): written by a test pass to look
//! for problems. Each test names what it tries.

#[allow(unused_imports)]
use super::tests;

use super::modelling_tests::{H, cube, note, press};
use super::tests::quick_harness;
use super::*;
use crate::text::tr;
use crate::uv_editor::gizmo::{UvHandle, handle_spot};
use egui::{Event, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;
use glam::Vec2;
use scene::{Element, ElementSelection};
use uv::brush::UvBrushKind;

// ------------------------------------------------------------ helpers ---

/// The cube, selected, in the UV Edit layout (the UV mode).
fn uv_edit_cube() -> H {
    let mut harness = quick_harness();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    harness
        .state_mut()
        .workbench
        .switch_layout(LayoutChoice::UvEdit);
    harness.run();
    harness.run();
    harness
}

fn mesh_of(harness: &H, id: scene::ObjectId) -> std::sync::Arc<scene::Mesh> {
    harness
        .state()
        .workbench
        .document()
        .scene()
        .evaluated(id)
        .unwrap()
        .mesh
}

/// Every corner's UV of object `id`.
fn uvs_of(harness: &H, id: scene::ObjectId) -> Vec<Vec2> {
    let mesh = mesh_of(harness, id);
    mesh::uvs::Uvs::of(&mesh)
        .unwrap()
        .into_values(mesh.face_layout().1.len())
}

fn cube_uvs(harness: &H) -> Vec<Vec2> {
    uvs_of(harness, cube(harness))
}

fn same(a: &[Vec2], b: &[Vec2]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (*x - *y).length() < 1e-4)
}

/// The middle of face `face`'s UVs.
fn face_middle(harness: &H, id: scene::ObjectId, face: scene::FaceId) -> Vec2 {
    let mesh = mesh_of(harness, id);
    let uvs = mesh::uvs::Uvs::of(&mesh).unwrap();
    let n = mesh.face(face).unwrap().len();
    (0..n)
        .map(|k| uvs.corner(&mesh, face, k).unwrap())
        .sum::<Vec2>()
        / n as f32
}

/// A spot just inside face `face`'s first corner (a brush moves corners).
fn near_corner(harness: &H, id: scene::ObjectId, face: scene::FaceId) -> Vec2 {
    let corner = face_uvs(harness, id, face)[0];
    corner + (face_middle(harness, id, face) - corner) * 0.1
}

fn face_uvs(harness: &H, id: scene::ObjectId, face: scene::FaceId) -> Vec<Vec2> {
    let mesh = mesh_of(harness, id);
    let uvs = mesh::uvs::Uvs::of(&mesh).unwrap();
    let n = mesh.face(face).unwrap().len();
    (0..n)
        .map(|k| uvs.corner(&mesh, face, k).unwrap())
        .collect()
}

/// The face whose normal points most along `way`.
fn face_facing(harness: &H, id: scene::ObjectId, way: glam::Vec3) -> scene::FaceId {
    let mesh = mesh_of(harness, id);
    mesh.faces()
        .map(|(f, _)| f)
        .max_by(|a, b| {
            mesh.face_normal(*a)
                .dot(way)
                .total_cmp(&mesh.face_normal(*b).dot(way))
        })
        .unwrap()
}

fn select(harness: &mut H, id: scene::ObjectId, elements: Vec<Element>) {
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements([(id, ElementSelection::of(elements))], "Select", None);
    harness.run();
}

fn screen(harness: &H, uv: Vec2) -> Pos2 {
    harness.state().workbench.uv_editor.screen_of(uv).unwrap()
}

fn gizmo_centre(harness: &H) -> Pos2 {
    let bench = &harness.state().workbench;
    bench
        .uv_editor
        .gizmo_centre(bench.document(), bench.mode)
        .expect("the gizmo shows")
}

fn over_editor(harness: &mut H) {
    let over = harness
        .get_by_label(crate::uv_editor::UV_EDITOR_LABEL)
        .rect()
        .left_top()
        + egui::vec2(30.0, 30.0);
    harness.event(Event::PointerMoved(over));
    harness.run();
}

fn button(harness: &mut H, at: Pos2, pressed: bool, modifiers: Modifiers) {
    harness.event(Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers,
    });
    harness.step();
}

/// Presses at `from` and moves by `by` in steps, without letting go.
fn press_and_move(harness: &mut H, from: Pos2, by: egui::Vec2, modifiers: Modifiers) {
    // Hover first, as a hand does: overlays over the editor show while the
    // mouse is over it, and take the press from the next frame.
    harness.event(Event::ModifiersChanged(modifiers));
    harness.event(Event::PointerMoved(from));
    harness.run();
    harness.step();
    button(harness, from, true, modifiers);
    for i in 1..=10 {
        harness.event(Event::PointerMoved(from + by * (i as f32 / 10.0)));
        harness.step();
    }
}

fn let_go(harness: &mut H, at: Pos2) {
    button(harness, at, false, Modifiers::NONE);
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    harness.run();
}

fn drag(harness: &mut H, from: Pos2, by: egui::Vec2, modifiers: Modifiers) {
    press_and_move(harness, from, by, modifiers);
    let_go(harness, from + by);
}

fn click(harness: &mut H, at: Pos2, modifiers: Modifiers) {
    harness.event(Event::ModifiersChanged(modifiers));
    harness.event(Event::PointerMoved(at));
    harness.run();
    button(harness, at, true, modifiers);
    button(harness, at, false, modifiers);
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    harness.run();
}

fn undo_key(harness: &mut H) {
    press(harness, Modifiers::COMMAND, Key::Z);
}

fn steps(harness: &H) -> usize {
    harness.state().workbench.document().undo_steps()
}

// ------------------------------------------------------------ lattice ---

/// Drags lattice handle `k` by `by` screen points, through the window.
fn drag_handle(harness: &mut H, k: usize, by: egui::Vec2) {
    let at = {
        let bench = &harness.state().workbench;
        let p = bench.lattice.as_ref().unwrap().points[k];
        bench.uv_editor.screen_of(p).unwrap()
    };
    drag(harness, at, by, Modifiers::NONE);
}

#[test]
#[ignore = "BUG: after Ctrl+Z of a lattice drag, the next handle drag brings the undone warp back"]
fn undoing_a_lattice_drag_stays_undone_after_the_next_drag() {
    // What dragging only the corner handle gives.
    let expected = {
        let mut harness = uv_edit_cube();
        harness.state_mut().workbench.start_lattice();
        harness.run();
        drag_handle(&mut harness, 0, egui::vec2(-20.0, 20.0));
        cube_uvs(&harness)
    };
    let mut harness = uv_edit_cube();
    let original = cube_uvs(&harness);
    harness.state_mut().workbench.start_lattice();
    harness.run();
    assert!(harness.state().workbench.warping(), "{}", note(&harness));
    drag_handle(&mut harness, 4, egui::vec2(40.0, 0.0));
    assert!(
        !same(&cube_uvs(&harness), &original),
        "the middle handle bent it"
    );
    undo_key(&mut harness);
    assert!(same(&cube_uvs(&harness), &original), "undone");
    drag_handle(&mut harness, 0, egui::vec2(-20.0, 20.0));
    assert!(
        same(&cube_uvs(&harness), &expected),
        "only the second drag's warp should be there"
    );
}

#[test]
#[ignore = "BUG: a lattice drag throws away UV edits made since the lattice went up (here a Turn left)"]
fn a_lattice_drag_keeps_uv_edits_made_while_it_is_up() {
    let mut harness = uv_edit_cube();
    let id = cube(&harness);
    harness.state_mut().workbench.start_lattice();
    harness.run();
    // Turn the top piece a quarter while the lattice is up (UV menu).
    harness.state_mut().workbench.mode = Mode::Polygons;
    let top = face_facing(&harness, id, glam::Vec3::Z);
    select(&mut harness, id, vec![Element::Polygon(top)]);
    let ctx = harness.ctx.clone();
    harness.state_mut().workbench.run_command(
        crate::commands::CommandId::UvTurnLeft,
        &ctx,
        &mut Vec::new(),
    );
    harness.run();
    let turned = face_uvs(&harness, id, top);
    // A tiny lattice drag of a far handle.
    drag_handle(&mut harness, 8, egui::vec2(1.0, 0.0));
    let after = face_uvs(&harness, id, top);
    // The top piece may bend a hair with the lattice, but not turn back.
    let back = turned
        .iter()
        .zip(&after)
        .map(|(a, b)| (*a - *b).length())
        .fold(0.0, f32::max);
    assert!(back < 0.02, "the turn was undone by the lattice: {back}");
}

// ------------------------------------------------------------ brushes ---

/// Isolates the cube's top piece in the editor.
fn isolate_top(harness: &mut H) -> (scene::ObjectId, scene::FaceId) {
    let id = cube(harness);
    harness.state_mut().workbench.mode = Mode::Polygons;
    let top = face_facing(harness, id, glam::Vec3::Z);
    select(harness, id, vec![Element::Polygon(top)]);
    let bench = &mut harness.state_mut().workbench;
    bench.uv_editor.set_isolating(true, &bench.document);
    harness.run();
    harness.run();
    (id, top)
}

#[test]
#[ignore = "BUG: a UV brush stroke moves pieces the editor hides while isolating"]
fn a_brush_leaves_pieces_hidden_by_isolate_alone() {
    let mut harness = uv_edit_cube();
    let (id, _) = isolate_top(&mut harness);
    let bottom = face_facing(&harness, id, -glam::Vec3::Z);
    let before = face_uvs(&harness, id, bottom);
    harness
        .state_mut()
        .workbench
        .start_uv_brush(UvBrushKind::Grab);
    harness.run();
    let at = screen(&harness, near_corner(&harness, id, bottom));
    drag(&mut harness, at, egui::vec2(25.0, 0.0), Modifiers::NONE);
    assert!(
        same(&face_uvs(&harness, id, bottom), &before),
        "the hidden bottom piece moved"
    );
}

#[test]
fn a_brush_stroke_through_the_window_is_one_undo_step() {
    let mut harness = uv_edit_cube();
    let id = cube(&harness);
    let top = face_facing(&harness, id, glam::Vec3::Z);
    let before = cube_uvs(&harness);
    let steps_before = steps(&harness);
    harness
        .state_mut()
        .workbench
        .start_uv_brush(UvBrushKind::Grab);
    harness.run();
    let at = screen(&harness, near_corner(&harness, id, top));
    drag(&mut harness, at, egui::vec2(25.0, 10.0), Modifiers::NONE);
    assert!(!same(&cube_uvs(&harness), &before));
    assert_eq!(steps(&harness), steps_before + 1);
    undo_key(&mut harness);
    assert!(same(&cube_uvs(&harness), &before));
}

#[test]
fn brackets_stop_at_the_brush_size_limits() {
    let mut harness = uv_edit_cube();
    harness
        .state_mut()
        .workbench
        .start_uv_brush(UvBrushKind::Relax);
    harness.run();
    let radius = |h: &H| h.state().workbench.uv_brushing.as_ref().unwrap().radius;
    for _ in 0..40 {
        press(&mut harness, Modifiers::NONE, Key::OpenBracket);
    }
    assert_eq!(radius(&harness), uv_brushes::UV_BRUSH_MIN);
    for _ in 0..60 {
        press(&mut harness, Modifiers::NONE, Key::CloseBracket);
    }
    assert_eq!(radius(&harness), uv_brushes::UV_BRUSH_MAX);
    // A huge brush still makes one step, and doesn't produce NaNs.
    let id = cube(&harness);
    let top = face_facing(&harness, id, glam::Vec3::Z);
    let at = screen(&harness, near_corner(&harness, id, top));
    drag(&mut harness, at, egui::vec2(15.0, 0.0), Modifiers::NONE);
    assert!(cube_uvs(&harness).iter().all(|uv| uv.is_finite()));
}

#[test]
#[ignore = "CONFUSING: with a UV brush up, W shows the gizmo but dragging it brushes instead"]
fn a_gizmo_key_with_a_uv_brush_up_lets_the_gizmo_be_dragged() {
    let mut harness = uv_edit_cube();
    let id = cube(&harness);
    harness.state_mut().workbench.mode = Mode::Polygons;
    let top = face_facing(&harness, id, glam::Vec3::Z);
    select(&mut harness, id, vec![Element::Polygon(top)]);
    harness
        .state_mut()
        .workbench
        .start_uv_brush(UvBrushKind::Grab);
    harness.run();
    over_editor(&mut harness);
    press(&mut harness, Modifiers::NONE, Key::W);
    let before = face_middle(&harness, id, top);
    let from = handle_spot(gizmo_centre(&harness), Tool::Move, UvHandle::MoveU);
    drag(&mut harness, from, egui::vec2(50.0, 0.0), Modifiers::NONE);
    let after = face_middle(&harness, id, top);
    assert!(
        (after.y - before.y).abs() < 1e-4 && after.x > before.x + 0.01,
        "the arrow across should move the piece across: {before} -> {after}"
    );
}

// ------------------------------------------------------ stitch and cut ---

/// A plane `n` × `m` polygons, selected alone, in the UV Edit layout and
/// the Edges mode.
fn plane_in_editor(n: u32, m: u32) -> (H, scene::ObjectId) {
    let mut harness = quick_harness();
    let id = super::seams_tests::add(
        &mut harness,
        scene::Primitive::Plane(mesh::PlaneParams {
            segments: [n, m],
            ..Default::default()
        }),
    );
    harness
        .state_mut()
        .workbench
        .switch_layout(LayoutChoice::UvEdit);
    harness.run();
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::Num2);
    assert_eq!(harness.state().workbench.mode, Mode::Edges);
    (harness, id)
}

fn islands_of(harness: &H, id: scene::ObjectId) -> usize {
    mesh::uvs::islands(&mesh_of(harness, id)).len()
}

#[test]
fn x_then_c_over_the_editor_puts_the_uvs_back() {
    let (mut harness, id) = plane_in_editor(2, 1);
    let before = uvs_of(&harness, id);
    let edge = super::seams_tests::inner(&mesh_of(&harness, id))[0];
    select(&mut harness, id, vec![Element::Edge(edge)]);
    over_editor(&mut harness);
    let steps_before = steps(&harness);
    press(&mut harness, Modifiers::NONE, Key::X);
    assert_eq!(islands_of(&harness, id), 2, "{}", note(&harness));
    // X over the editor didn't slit (the viewport's X).
    assert_eq!(harness.state().workbench.tool(), Tool::Select);
    press(&mut harness, Modifiers::NONE, Key::C);
    assert_eq!(islands_of(&harness, id), 1, "{}", note(&harness));
    assert!(
        same(&uvs_of(&harness, id), &before),
        "stitched back in place"
    );
    assert_eq!(steps(&harness), steps_before + 2);
    undo_key(&mut harness);
    undo_key(&mut harness);
    assert!(same(&uvs_of(&harness, id), &before));
    assert_eq!(steps(&harness), steps_before);
}

#[test]
fn cutting_every_edge_and_undoing_it() {
    let (mut harness, id) = plane_in_editor(3, 3);
    let before = uvs_of(&harness, id);
    let edges: Vec<Element> = mesh_of(&harness, id)
        .topology()
        .edges()
        .map(Element::Edge)
        .collect();
    select(&mut harness, id, edges);
    over_editor(&mut harness);
    press(&mut harness, Modifiers::NONE, Key::X);
    assert_eq!(islands_of(&harness, id), 9, "{}", note(&harness));
    assert!(uvs_of(&harness, id).iter().all(|uv| uv.is_finite()));
    // Again: nothing more to cut, and it says so.
    let steps_now = steps(&harness);
    press(&mut harness, Modifiers::NONE, Key::X);
    assert_eq!(steps(&harness), steps_now);
    assert!(note(&harness).contains("cut already"), "{}", note(&harness));
    // Stitch them all back.
    press(&mut harness, Modifiers::NONE, Key::C);
    assert_eq!(islands_of(&harness, id), 1, "{}", note(&harness));
    undo_key(&mut harness);
    undo_key(&mut harness);
    assert!(same(&uvs_of(&harness, id), &before));
}

#[test]
fn stitching_edges_of_two_objects_is_one_step() {
    let (mut harness, a) = plane_in_editor(2, 1);
    let b = super::seams_tests::add(
        &mut harness,
        scene::Primitive::Plane(mesh::PlaneParams {
            segments: [2, 1],
            ..Default::default()
        }),
    );
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([a, b]);
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::Num2);
    if harness.state().workbench.mode != Mode::Edges {
        press(&mut harness, Modifiers::NONE, Key::Num2);
    }
    let ea = super::seams_tests::inner(&mesh_of(&harness, a))[0];
    let eb = super::seams_tests::inner(&mesh_of(&harness, b))[0];
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements(
            [
                (a, ElementSelection::of([Element::Edge(ea)])),
                (b, ElementSelection::of([Element::Edge(eb)])),
            ],
            "Select",
            None,
        );
    harness.run();
    over_editor(&mut harness);
    press(&mut harness, Modifiers::NONE, Key::X);
    assert_eq!((islands_of(&harness, a), islands_of(&harness, b)), (2, 2));
    let s = steps(&harness);
    press(&mut harness, Modifiers::NONE, Key::C);
    assert_eq!((islands_of(&harness, a), islands_of(&harness, b)), (1, 1));
    assert_eq!(steps(&harness), s + 1);
    undo_key(&mut harness);
    assert_eq!((islands_of(&harness, a), islands_of(&harness, b)), (2, 2));
}

#[test]
fn x_and_c_over_the_editor_with_nothing_selected_say_what_to_do() {
    let (mut harness, id) = plane_in_editor(2, 1);
    select(&mut harness, id, vec![]);
    over_editor(&mut harness);
    let s = steps(&harness);
    press(&mut harness, Modifiers::NONE, Key::X);
    assert!(
        note(&harness).contains("select the edges"),
        "{}",
        note(&harness)
    );
    press(&mut harness, Modifiers::NONE, Key::C);
    assert!(
        note(&harness).contains("select edges"),
        "{}",
        note(&harness)
    );
    assert_eq!(steps(&harness), s);
    assert_eq!(harness.state().workbench.tool(), Tool::Select);
}

// ------------------------------------------------------------- gizmo ---

/// The cube in the UV mode with its top piece selected (through the
/// document, as a click does).
fn top_piece_selected() -> (H, scene::ObjectId, scene::FaceId) {
    let mut harness = uv_edit_cube();
    let id = cube(&harness);
    let top = face_facing(&harness, id, glam::Vec3::Z);
    select(&mut harness, id, vec![Element::Polygon(top)]);
    assert_eq!(harness.state().workbench.mode, Mode::Uv);
    (harness, id, top)
}

#[test]
fn a_gizmo_drag_of_two_objects_is_one_undo_step() {
    let (mut harness, a) = plane_in_editor(1, 1);
    let b = super::seams_tests::add(
        &mut harness,
        scene::Primitive::Plane(mesh::PlaneParams::default()),
    );
    let fa: Vec<Element> = mesh_of(&harness, a)
        .faces()
        .map(|(f, _)| Element::Polygon(f))
        .collect();
    let fb: Vec<Element> = mesh_of(&harness, b)
        .faces()
        .map(|(f, _)| Element::Polygon(f))
        .collect();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([a, b]);
    harness.state_mut().workbench.mode = Mode::Polygons;
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements(
            [(a, ElementSelection::of(fa)), (b, ElementSelection::of(fb))],
            "Select",
            None,
        );
    harness.run();
    let (ua, ub) = (uvs_of(&harness, a), uvs_of(&harness, b));
    over_editor(&mut harness);
    press(&mut harness, Modifiers::NONE, Key::W);
    let s = steps(&harness);
    let from = handle_spot(gizmo_centre(&harness), Tool::Move, UvHandle::MoveFree);
    drag(&mut harness, from, egui::vec2(30.0, -20.0), Modifiers::NONE);
    assert!(!same(&uvs_of(&harness, a), &ua) && !same(&uvs_of(&harness, b), &ub));
    assert_eq!(steps(&harness), s + 1);
    undo_key(&mut harness);
    assert!(same(&uvs_of(&harness, a), &ua) && same(&uvs_of(&harness, b), &ub));
}

#[test]
#[ignore = "INCONSISTENT: Tab during a UV gizmo drag cancels the drag and leaves the UV mode; in the viewport it waits"]
fn tab_during_a_uv_gizmo_drag_waits_like_the_viewport() {
    let (mut harness, id, top) = top_piece_selected();
    over_editor(&mut harness);
    press(&mut harness, Modifiers::NONE, Key::W);
    let before = face_middle(&harness, id, top);
    let from = handle_spot(gizmo_centre(&harness), Tool::Move, UvHandle::MoveFree);
    press_and_move(&mut harness, from, egui::vec2(40.0, 0.0), Modifiers::NONE);
    assert!(harness.state().workbench.uv_editor.dragging_gizmo());
    press(&mut harness, Modifiers::NONE, Key::Tab);
    let mode = harness.state().workbench.mode;
    let_go(&mut harness, from + egui::vec2(40.0, 0.0));
    let after = face_middle(&harness, id, top);
    // The viewport's rule: "Let go of the drag (or press Esc) first, then
    // Tab." Either way, the move and the undo list must agree.
    eprintln!(
        "mode after Tab {mode:?}, note {}, moved {}",
        note(&harness),
        (after - before).length() > 1e-4
    );
    assert_eq!(mode, Mode::Uv, "Tab mid-drag left the UV mode");
    assert!(after.x > before.x + 0.01);
}

#[test]
fn ctrl_z_during_a_uv_gizmo_drag_leaves_the_uvs_as_undone() {
    let (mut harness, id, top) = top_piece_selected();
    over_editor(&mut harness);
    press(&mut harness, Modifiers::NONE, Key::W);
    let before = face_middle(&harness, id, top);
    let s = steps(&harness);
    let from = handle_spot(gizmo_centre(&harness), Tool::Move, UvHandle::MoveFree);
    press_and_move(&mut harness, from, egui::vec2(40.0, 0.0), Modifiers::NONE);
    undo_key(&mut harness);
    for i in 1..=5 {
        harness.event(Event::PointerMoved(
            from + egui::vec2(40.0 + 4.0 * i as f32, 0.0),
        ));
        harness.step();
    }
    let_go(&mut harness, from + egui::vec2(60.0, 0.0));
    let after = face_middle(&harness, id, top);
    eprintln!(
        "gizmo undo mid-drag: before {before} after {after}, steps {} -> {}",
        s,
        steps(&harness)
    );
    // Whatever the undo took back stays taken back: the stack and the UVs
    // agree with each other.
    let now = steps(&harness);
    if (after - before).length() < 1e-4 {
        assert!(now <= s + 1);
    } else {
        assert_eq!(now, s + 1, "a move shown must be an undo step");
    }
}

#[test]
fn ctrl_z_during_a_uv_brush_stroke() {
    let (mut harness, id) = plane_in_editor(4, 4);
    let before = uvs_of(&harness, id);
    let s = steps(&harness);
    harness
        .state_mut()
        .workbench
        .start_uv_brush(UvBrushKind::Grab);
    harness.run();
    let at = screen(&harness, Vec2::splat(0.5));
    press_and_move(&mut harness, at, egui::vec2(20.0, 0.0), Modifiers::NONE);
    assert!(!same(&uvs_of(&harness, id), &before));
    undo_key(&mut harness);
    let undone = same(&uvs_of(&harness, id), &before);
    for i in 1..=5 {
        harness.event(Event::PointerMoved(
            at + egui::vec2(20.0 + 2.0 * i as f32, 0.0),
        ));
        harness.step();
    }
    let_go(&mut harness, at + egui::vec2(30.0, 0.0));
    eprintln!(
        "brush undo mid-stroke: undone at once {undone}, steps {} -> {}, back to start at the end {}",
        s,
        steps(&harness),
        same(&uvs_of(&harness, id), &before)
    );
    // Whatever happens, one Ctrl + Z more must give the start back.
    undo_key(&mut harness);
    assert!(same(&uvs_of(&harness, id), &before));
}

// ------------------------------------------------------------- pivots ---

#[test]
fn ctrl_click_places_your_own_pivot_and_keeps_the_selection() {
    let (mut harness, id, top) = top_piece_selected();
    *harness.state_mut().workbench.uv_editor.pivot_mut().0 =
        crate::uv_editor::placing::UvPivot::Own;
    over_editor(&mut harness);
    press(&mut harness, Modifiers::NONE, Key::R);
    // With no point placed yet, the gizmo still shows (at the middle).
    let middle = face_middle(&harness, id, top);
    let centre = gizmo_centre(&harness);
    assert!(centre.distance(screen(&harness, middle)) < 1.0);
    let selected = harness.state().workbench.document().elements(id).cloned();
    let spot = Vec2::new(0.05, 0.05);
    let s = steps(&harness);
    let at = screen(&harness, spot);
    click(&mut harness, at, Modifiers::COMMAND);
    assert_eq!(
        harness.state().workbench.document().elements(id).cloned(),
        selected
    );
    assert_eq!(steps(&harness), s, "placing the pivot isn't an edit");
    let placed = harness
        .state_mut()
        .workbench
        .uv_editor
        .pivot_mut()
        .1
        .unwrap();
    assert!((placed - spot).length() < 0.01, "{placed}");
    // S, then the middle square dragged at the placed point: the piece
    // grows about it.
    press(&mut harness, Modifiers::NONE, Key::S);
    let was = face_middle(&harness, id, top) - spot;
    let from = screen(&harness, spot);
    drag(&mut harness, from, egui::vec2(40.0, 0.0), Modifiers::NONE);
    let now = face_middle(&harness, id, top) - spot;
    assert!(now.length() > was.length() * 1.2, "{was} -> {now}");
    assert!(
        now.normalize().dot(was.normalize()) > 0.999,
        "grown about the point"
    );
}

#[test]
#[ignore = "DOCS: with a box-corner pivot, grid snapping lands the corner on the grid, not the selection's middle as editing-uvs.md says"]
fn grid_snapping_puts_the_selections_middle_on_the_grid() {
    let (mut harness, id, top) = top_piece_selected();
    *harness.state_mut().workbench.uv_editor.pivot_mut().0 =
        crate::uv_editor::placing::UvPivot::BoxCorner(0);
    {
        let snap = harness.state_mut().workbench.uv_editor.snap_to_mut();
        snap.corners = false;
        snap.edges = false;
        snap.trim = false;
        snap.grid = true;
    }
    harness.state_mut().workbench.viewport.options_mut().snap.on = true;
    over_editor(&mut harness);
    press(&mut harness, Modifiers::NONE, Key::W);
    // The gizmo sits on the pivot: the selection's bottom left.
    let low = face_uvs(&harness, id, top)
        .into_iter()
        .fold(Vec2::splat(f32::INFINITY), Vec2::min);
    let from = screen(&harness, low);
    let before = face_middle(&harness, id, top);
    drag(&mut harness, from, egui::vec2(37.0, 0.0), Modifiers::NONE);
    let corner_now = face_uvs(&harness, id, top)
        .into_iter()
        .fold(Vec2::splat(f32::INFINITY), Vec2::min);
    eprintln!(
        "snapped: corner {low} -> {corner_now}, middle {before} -> {}",
        face_middle(&harness, id, top)
    );
    assert!(
        (face_middle(&harness, id, top) - before).length() > 1e-3,
        "it moved"
    );
    let m = face_middle(&harness, id, top);
    let on_grid = |x: f32| ((x * 10.0) - (x * 10.0).round()).abs() < 1e-3;
    assert!(on_grid(m.x), "the middle {m} isn't on the grid of tenths");
}

#[test]
fn snapping_with_only_corners_and_none_near_moves_freely() {
    let (mut harness, id, top) = top_piece_selected();
    {
        let snap = harness.state_mut().workbench.uv_editor.snap_to_mut();
        snap.corners = true;
        snap.edges = false;
        snap.trim = false;
        snap.grid = false;
        snap.pixels = false;
    }
    harness.state_mut().workbench.viewport.options_mut().snap.on = true;
    over_editor(&mut harness);
    press(&mut harness, Modifiers::NONE, Key::W);
    let before = face_middle(&harness, id, top);
    let from = handle_spot(gizmo_centre(&harness), Tool::Move, UvHandle::MoveU);
    drag(&mut harness, from, egui::vec2(-3.0, 0.0), Modifiers::NONE);
    let after = face_middle(&harness, id, top);
    assert!(after.is_finite());
    assert!(
        (after.y - before.y).abs() < 1e-5,
        "the arrow across moved it up"
    );
}

// ------------------------------------------------------ no UVs at all ---

fn wedge_without_uvs() -> (H, scene::ObjectId) {
    let mut harness = tests::fresh();
    let text = "o Wedge\nv 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nv 0 0 1\nv 1 0 1\n\
                f 1 4 3 2\nf 1 2 6 5\nf 2 3 6\nf 1 5 4\nf 4 5 6 3\n";
    let import = fileio::obj::read_obj(
        text,
        fileio::export::FileUnits::Metres,
        fileio::export::UpAxis::Z,
    )
    .unwrap();
    harness
        .state_mut()
        .workbench
        .imported(std::path::Path::new("/in/wedge.obj"), import);
    harness.run();
    let id = *harness
        .state()
        .workbench
        .document()
        .selection()
        .first()
        .unwrap();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness
        .state_mut()
        .workbench
        .switch_layout(LayoutChoice::UvEdit);
    harness.run();
    harness.run();
    assert!(!mesh::uvs::has_uvs(&mesh_of(&harness, id)));
    (harness, id)
}

#[test]
fn every_uv_key_on_a_shape_without_uvs_is_harmless() {
    let (mut harness, id) = wedge_without_uvs();
    let s = steps(&harness);
    let all: Vec<Element> = mesh_of(&harness, id)
        .faces()
        .map(|(f, _)| Element::Polygon(f))
        .collect();
    harness.state_mut().workbench.mode = Mode::Polygons;
    select(&mut harness, id, all);
    over_editor(&mut harness);
    for key in [
        Key::W,
        Key::R,
        Key::S,
        Key::T,
        Key::Z,
        Key::H,
        Key::X,
        Key::C,
    ] {
        press(&mut harness, Modifiers::NONE, key);
    }
    press(&mut harness, Modifiers::ALT, Key::A);
    harness.state_mut().workbench.start_lattice();
    harness.run();
    assert!(!harness.state().workbench.warping());
    harness
        .state_mut()
        .workbench
        .start_uv_brush(UvBrushKind::Pinch);
    harness.run();
    let at = harness
        .get_by_label(crate::uv_editor::UV_EDITOR_LABEL)
        .rect()
        .center();
    drag(&mut harness, at, egui::vec2(20.0, 0.0), Modifiers::NONE);
    eprintln!("no-UV note: {}", note(&harness));
    let ctx = harness.ctx.clone();
    for command in [
        crate::commands::CommandId::UvTurnLeft,
        crate::commands::CommandId::UvFlipU,
    ] {
        harness
            .state_mut()
            .workbench
            .run_command(command, &ctx, &mut Vec::new());
        harness.run();
    }
    // X over the editor cut (marked seams) even though there are no UVs:
    // allowed, but nothing may have broken.
    eprintln!(
        "no-UV steps {} -> {}, note {}",
        s,
        steps(&harness),
        note(&harness)
    );
    let mesh = mesh_of(&harness, id);
    assert_eq!(mesh.faces().count(), 5, "no polygons lost");
}

// ------------------------------------------------ keys mid-stroke/drag ---

#[test]
#[ignore = "BUG: Delete (and Ctrl+Z) go through in the middle of a UV brush stroke, splitting it into several undo steps"]
fn delete_during_a_uv_brush_stroke_doesnt_break_anything() {
    let (mut harness, id) = plane_in_editor(4, 4);
    press(&mut harness, Modifiers::NONE, Key::Num3);
    let faces: Vec<Element> = mesh_of(&harness, id)
        .faces()
        .take(4)
        .map(|(f, _)| Element::Polygon(f))
        .collect();
    select(&mut harness, id, faces);
    harness
        .state_mut()
        .workbench
        .start_uv_brush(UvBrushKind::Grab);
    harness.run();
    let at = screen(&harness, Vec2::splat(0.5));
    let s = steps(&harness);
    press_and_move(&mut harness, at, egui::vec2(20.0, 0.0), Modifiers::NONE);
    press(&mut harness, Modifiers::NONE, Key::Delete);
    let polygons = mesh_of(&harness, id).faces().count();
    for i in 1..=5 {
        harness.event(Event::PointerMoved(
            at + egui::vec2(20.0 + 3.0 * i as f32, 0.0),
        ));
        harness.step();
    }
    let_go(&mut harness, at + egui::vec2(35.0, 0.0));
    eprintln!(
        "delete mid-stroke: polygons {polygons}, steps {s} -> {}, note {}",
        steps(&harness),
        note(&harness)
    );
    // The gizmo's rule: during a drag, nothing else happens.
    assert_eq!(
        polygons, 16,
        "Delete went through in the middle of a stroke"
    );
}

#[test]
fn delete_during_a_uv_gizmo_drag_is_held_back() {
    let (mut harness, id, _) = top_piece_selected();
    over_editor(&mut harness);
    press(&mut harness, Modifiers::NONE, Key::W);
    let from = handle_spot(gizmo_centre(&harness), Tool::Move, UvHandle::MoveFree);
    press_and_move(&mut harness, from, egui::vec2(20.0, 0.0), Modifiers::NONE);
    press(&mut harness, Modifiers::NONE, Key::Delete);
    let_go(&mut harness, from + egui::vec2(20.0, 0.0));
    assert_eq!(mesh_of(&harness, id).faces().count(), 6);
}

#[test]
fn delete_with_the_lattice_up_then_dragging_a_handle() {
    let (mut harness, id) = plane_in_editor(2, 2);
    harness.state_mut().workbench.start_lattice();
    harness.run();
    assert!(harness.state().workbench.warping(), "{}", note(&harness));
    press(&mut harness, Modifiers::NONE, Key::Num3);
    let first = mesh_of(&harness, id).faces().next().unwrap().0;
    select(&mut harness, id, vec![Element::Polygon(first)]);
    press(&mut harness, Modifiers::NONE, Key::Delete);
    assert_eq!(mesh_of(&harness, id).faces().count(), 3);
    // The lattice still holds the deleted polygon's corners.
    drag_handle(&mut harness, 4, egui::vec2(20.0, 0.0));
    eprintln!(
        "lattice after delete: warping {}, faces {}, note {}",
        harness.state().workbench.warping(),
        mesh_of(&harness, id).faces().count(),
        note(&harness)
    );
    assert_eq!(mesh_of(&harness, id).faces().count(), 3);
    assert!(uvs_of(&harness, id).iter().all(|uv| uv.is_finite()));
    assert!(
        !note(&harness).contains("Couldn't"),
        "the lattice tried to change what's gone: {}",
        note(&harness)
    );
}

// ------------------------------------------------- the own selection ---

#[test]
#[ignore = "CONFUSING: with the editor keeping its own selection, X over the editor ignores what's picked there"]
fn x_over_the_editor_cuts_what_the_editor_has_selected() {
    let (mut harness, id) = plane_in_editor(2, 1);
    {
        let bench = &mut harness.state_mut().workbench;
        bench.uv_editor.set_own_selection(true, &bench.document);
    }
    harness.run();
    // Pick the middle edge in the editor: a click on its middle.
    let mesh = mesh_of(&harness, id);
    let edge = super::seams_tests::inner(&mesh)[0];
    let uvs = mesh::uvs::Uvs::of(&mesh).unwrap();
    let f = mesh.topology().edge_faces(edge).next().unwrap();
    let list = mesh.face(f).unwrap().to_vec();
    let (a, b) = edge.ends();
    let ka = list.iter().position(|v| *v == a).unwrap();
    let kb = list.iter().position(|v| *v == b).unwrap();
    let middle = (uvs.corner(&mesh, f, ka).unwrap() + uvs.corner(&mesh, f, kb).unwrap()) / 2.0;
    let at = screen(&harness, middle);
    click(&mut harness, at, Modifiers::NONE);
    let own = harness
        .state()
        .workbench
        .uv_editor
        .selected(harness.state().workbench.document(), id);
    assert!(own.contains(Element::Edge(edge)), "picked in the editor");
    assert!(
        harness
            .state()
            .workbench
            .document()
            .elements(id)
            .is_none_or(|e| e.is_empty()),
        "the viewport's stays empty"
    );
    press(&mut harness, Modifiers::NONE, Key::X);
    assert_eq!(islands_of(&harness, id), 2, "{}", note(&harness));
}

#[test]
fn the_own_selection_moves_with_the_gizmo_and_leaves_the_viewports() {
    let (mut harness, id, top) = top_piece_selected();
    {
        let bench = &mut harness.state_mut().workbench;
        bench.uv_editor.set_own_selection(true, &bench.document);
    }
    // Pick the bottom piece in the editor instead.
    let bottom = face_facing(&harness, id, -glam::Vec3::Z);
    let at = screen(&harness, face_middle(&harness, id, bottom));
    let s = steps(&harness);
    click(&mut harness, at, Modifiers::NONE);
    assert_eq!(steps(&harness), s, "no undo step");
    let viewport: Vec<Element> = harness
        .state()
        .workbench
        .document()
        .elements(id)
        .unwrap()
        .polygons()
        .map(Element::Polygon)
        .collect();
    assert_eq!(viewport, vec![Element::Polygon(top)]);
    over_editor(&mut harness);
    press(&mut harness, Modifiers::NONE, Key::W);
    let (tb, bb) = (
        face_middle(&harness, id, top),
        face_middle(&harness, id, bottom),
    );
    let from = screen(&harness, bb);
    drag(&mut harness, from, egui::vec2(30.0, 0.0), Modifiers::NONE);
    assert!(
        (face_middle(&harness, id, top) - tb).length() < 1e-5,
        "the top stayed"
    );
    assert!(
        face_middle(&harness, id, bottom).x > bb.x + 0.01,
        "the bottom moved"
    );
}

// ------------------------------------------------ selecting in the UVs ---

#[test]
fn grow_and_shrink_on_a_grid_and_at_their_limits() {
    let (mut harness, id) = plane_in_editor(3, 3);
    press(&mut harness, Modifiers::NONE, Key::Num3);
    let mesh = mesh_of(&harness, id);
    // The middle polygon: the one nearest the middle of the texture.
    let middle = mesh
        .faces()
        .map(|(f, _)| f)
        .min_by(|a, b| {
            (face_middle(&harness, id, *a) - Vec2::splat(0.5))
                .length()
                .total_cmp(&(face_middle(&harness, id, *b) - Vec2::splat(0.5)).length())
        })
        .unwrap();
    select(&mut harness, id, vec![Element::Polygon(middle)]);
    let count = |h: &H| {
        h.state()
            .workbench
            .document()
            .elements(id)
            .map_or(0, |e| e.polygons().count())
    };
    harness
        .state_mut()
        .workbench
        .run_uv_select(uv_select::UvSelect::Grow);
    harness.run();
    assert_eq!(count(&harness), 9);
    // Growing the whole piece again finds nothing more: no new undo step.
    let s = steps(&harness);
    harness
        .state_mut()
        .workbench
        .run_uv_select(uv_select::UvSelect::Grow);
    harness.run();
    eprintln!(
        "grow again: steps {s} -> {}, note {}",
        steps(&harness),
        note(&harness)
    );
    harness
        .state_mut()
        .workbench
        .run_uv_select(uv_select::UvSelect::Shrink);
    harness.run();
    assert_eq!(count(&harness), 1);
    // Shrinking one polygon: nothing left; it should say so, not empty
    // the selection silently.
    harness
        .state_mut()
        .workbench
        .run_uv_select(uv_select::UvSelect::Shrink);
    harness.run();
    eprintln!(
        "shrink one: count {}, note {}",
        count(&harness),
        note(&harness)
    );
    assert_eq!(count(&harness), 1, "shrinking a single polygon emptied it");
}

#[test]
fn growing_a_whole_piece_again_makes_no_undo_step() {
    let (mut harness, id) = plane_in_editor(2, 2);
    press(&mut harness, Modifiers::NONE, Key::Num3);
    let all: Vec<Element> = mesh_of(&harness, id)
        .faces()
        .map(|(f, _)| Element::Polygon(f))
        .collect();
    select(&mut harness, id, all);
    let s = steps(&harness);
    harness
        .state_mut()
        .workbench
        .run_uv_select(uv_select::UvSelect::Grow);
    harness.run();
    assert_eq!(steps(&harness), s, "{}", note(&harness));
}

#[test]
fn alt_a_over_the_editor_takes_whole_pieces() {
    let (mut harness, id) = plane_in_editor(2, 2);
    press(&mut harness, Modifiers::NONE, Key::Num3);
    let first = mesh_of(&harness, id).faces().next().unwrap().0;
    select(&mut harness, id, vec![Element::Polygon(first)]);
    over_editor(&mut harness);
    press(&mut harness, Modifiers::ALT, Key::A);
    let n = harness
        .state()
        .workbench
        .document()
        .elements(id)
        .map_or(0, |e| e.polygons().count());
    assert_eq!(n, 4, "{}", note(&harness));
}

// ------------------------------------------------------- texture lock ---

fn on_screen(harness: &H, world: glam::Vec3) -> Pos2 {
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = Vec2::new(rect.width(), rect.height());
    let at = tests::camera(harness)
        .project(world, size)
        .expect("in view");
    rect.min + egui::vec2(at.x, at.y)
}

fn x_arrow(harness: &H) -> Pos2 {
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = Vec2::new(rect.width(), rect.height());
    let gizmo = harness
        .state()
        .workbench
        .viewport()
        .gizmo()
        .expect("a gizmo");
    let length = gizmo.length(&tests::camera(harness), size);
    on_screen(harness, gizmo.origin + glam::Vec3::X * length * 0.8)
}

fn along_x(harness: &H, from: Pos2, pixels: f32) -> Pos2 {
    let a = on_screen(harness, glam::Vec3::ZERO);
    let b = on_screen(harness, glam::Vec3::X * 100.0);
    from + (b - a).normalized() * pixels
}

#[test]
fn texture_lock_from_the_palette_keeps_the_texture_when_a_point_moves() {
    for lock in [false, true] {
        let (mut harness, id) = plane_in_editor(2, 2);
        press(&mut harness, Modifiers::NONE, Key::Num1);
        let mesh = mesh_of(&harness, id);
        // The middle point.
        let (v, _) = mesh
            .vertices()
            .min_by(|a, b| a.1.length().total_cmp(&b.1.length()))
            .unwrap();
        select(&mut harness, id, vec![Element::Point(v)]);
        if lock {
            harness
                .get_by_label(crate::palette::TEXTURE_LOCK_LABEL)
                .click();
            harness.run();
            assert!(harness.state().workbench.viewport().options().texture_lock);
        }
        let before = uvs_of(&harness, id);
        let place = mesh_of(&harness, id).position(v).unwrap();
        // W, then the viewport gizmo's X arrow dragged.
        let viewport = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
        harness.event(Event::PointerMoved(viewport.center()));
        harness.run();
        press(&mut harness, Modifiers::NONE, Key::W);
        let start = x_arrow(&harness);
        let to = along_x(&harness, start, 40.0);
        let s = steps(&harness);
        drag(&mut harness, start, to - start, Modifiers::NONE);
        let moved = mesh_of(&harness, id).position(v).unwrap();
        assert!(
            moved.x > place.x + 1.0,
            "the point moved: {place} -> {moved}"
        );
        assert_eq!(steps(&harness), s + 1);
        let after = uvs_of(&harness, id);
        assert_eq!(same(&before, &after), !lock, "lock {lock}");
        undo_key(&mut harness);
        assert!(same(&uvs_of(&harness, id), &before));
    }
}

// ------------------------------------------------- exact and scaling ---

#[test]
#[ignore = "BUG: dragging a number in the Exact menu makes an undo step for every frame of the drag"]
fn dragging_the_exact_middle_is_one_undo_step() {
    let (mut harness, id, top) = top_piece_selected();
    let menu = harness
        .get_by_label(tr(crate::uv_editor::placing::EXACT_MENU))
        .rect();
    harness
        .get_by_label(tr(crate::uv_editor::placing::EXACT_MENU))
        .click();
    harness.run();
    // The popup's first number, just under the menu's button.
    let field = harness
        .get_all_by_role(egui::accesskit::Role::SpinButton)
        .map(|n| n.rect())
        .filter(|r| r.top() > menu.bottom() && r.top() < menu.bottom() + 120.0)
        .filter(|r| (r.left() - menu.left()).abs() < 300.0)
        .min_by(|a, b| {
            (a.top(), a.left())
                .partial_cmp(&(b.top(), b.left()))
                .unwrap()
        })
        .expect("the Middle U field")
        .center();
    eprintln!("exact menu {menu:?}, field {field:?}");
    let before = face_middle(&harness, id, top);
    let s = steps(&harness);
    harness.event(Event::PointerMoved(field));
    harness.run();
    button(&mut harness, field, true, Modifiers::NONE);
    for i in 1..=10 {
        harness.event(Event::PointerMoved(field + egui::vec2(3.0 * i as f32, 0.0)));
        harness.step();
    }
    let_go(&mut harness, field + egui::vec2(30.0, 0.0));
    let after = face_middle(&harness, id, top);
    eprintln!(
        "exact drag: {before} -> {after}, steps {s} -> {}",
        steps(&harness)
    );
    assert!(after.x > before.x, "moved across");
    assert_eq!(steps(&harness), s + 1, "one drag, one undo step");
}

#[test]
fn scaling_through_zero_with_the_gizmo_stays_finite_and_undoes() {
    let (mut harness, id, top) = top_piece_selected();
    over_editor(&mut harness);
    press(&mut harness, Modifiers::NONE, Key::S);
    let before = face_uvs(&harness, id, top);
    let centre = gizmo_centre(&harness);
    let from = handle_spot(centre, Tool::Scale, UvHandle::ScaleU);
    // Exactly onto the middle (scale 0), then past it.
    press_and_move(&mut harness, from, centre - from, Modifiers::NONE);
    let flat = face_uvs(&harness, id, top);
    assert!(flat.iter().all(|uv| uv.is_finite()));
    let width = flat.iter().map(|u| u.x).fold(f32::MIN, f32::max)
        - flat.iter().map(|u| u.x).fold(f32::MAX, f32::min);
    assert!(width < 1e-2, "squashed: {width}");
    for i in 1..=5 {
        harness.event(Event::PointerMoved(
            centre - (from - centre) * (i as f32 / 5.0),
        ));
        harness.step();
    }
    let_go(&mut harness, centre - (from - centre));
    assert!(face_uvs(&harness, id, top).iter().all(|uv| uv.is_finite()));
    undo_key(&mut harness);
    assert!(same(&face_uvs(&harness, id, top), &before));
}

#[test]
fn esc_during_a_brush_stroke_keeps_the_stroke_and_puts_the_brush_down() {
    let (mut harness, id) = plane_in_editor(4, 4);
    let before = uvs_of(&harness, id);
    harness
        .state_mut()
        .workbench
        .start_uv_brush(UvBrushKind::Grab);
    harness.run();
    let at = screen(&harness, Vec2::splat(0.5));
    let s = steps(&harness);
    press_and_move(&mut harness, at, egui::vec2(20.0, 0.0), Modifiers::NONE);
    press(&mut harness, Modifiers::NONE, Key::Escape);
    let_go(&mut harness, at + egui::vec2(20.0, 0.0));
    eprintln!(
        "esc mid-stroke: brush {:?}, kept {}, steps {s} -> {}",
        harness.state().workbench.uv_brush(),
        !same(&uvs_of(&harness, id), &before),
        steps(&harness)
    );
    assert!(harness.state().workbench.uv_brush().is_none());
    // Nothing selected by letting go.
    assert!(
        harness
            .state()
            .workbench
            .document()
            .elements(id)
            .is_none_or(|e| e.is_empty())
    );
}

// ------------------------------------------- navigating with tools up ---

/// A drag with `which` button, as navigation uses.
fn drag_with(harness: &mut H, which: PointerButton, from: Pos2, by: egui::Vec2) {
    harness.event(Event::PointerMoved(from));
    harness.run();
    harness.event(Event::PointerButton {
        pos: from,
        button: which,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    for i in 1..=8 {
        harness.event(Event::PointerMoved(from + by * (i as f32 / 8.0)));
        harness.step();
    }
    harness.event(Event::PointerButton {
        pos: from + by,
        button: which,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
}

#[test]
fn a_middle_drag_pans_the_editor_without_a_tool_up() {
    let (mut harness, _) = plane_in_editor(4, 4);
    let was = screen(&harness, Vec2::ZERO);
    let at = screen(&harness, Vec2::splat(0.5));
    drag_with(
        &mut harness,
        PointerButton::Middle,
        at,
        egui::vec2(40.0, 0.0),
    );
    assert!(
        (screen(&harness, Vec2::ZERO) - was).length() > 10.0,
        "panned"
    );
}

#[test]
#[ignore = "BUG: with a UV brush up, a middle-button (pan) drag brushes the UVs instead of panning"]
fn a_middle_drag_pans_rather_than_brushes() {
    let (mut harness, id) = plane_in_editor(4, 4);
    let before = uvs_of(&harness, id);
    harness
        .state_mut()
        .workbench
        .start_uv_brush(UvBrushKind::Grab);
    harness.run();
    let was = screen(&harness, Vec2::ZERO);
    let at = screen(&harness, Vec2::splat(0.5));
    drag_with(
        &mut harness,
        PointerButton::Middle,
        at,
        egui::vec2(40.0, 0.0),
    );
    let panned = (screen(&harness, Vec2::ZERO) - was).length() > 10.0;
    let brushed = !same(&uvs_of(&harness, id), &before);
    eprintln!("brush + middle drag: panned {panned}, brushed {brushed}");
    assert!(!brushed, "a pan drag brushed the UVs");
    assert!(panned, "a pan drag didn't pan");
}

#[test]
#[ignore = "BUG: with the lattice up, a middle-button (pan) drag near a handle drags the handle, and panning doesn't work"]
fn a_middle_drag_pans_rather_than_warps() {
    let (mut harness, id) = plane_in_editor(4, 4);
    let before = uvs_of(&harness, id);
    harness.state_mut().workbench.start_lattice();
    harness.run();
    let was = screen(&harness, Vec2::ZERO);
    let at = {
        let bench = &harness.state().workbench;
        bench
            .uv_editor
            .screen_of(bench.lattice.as_ref().unwrap().points[4])
            .unwrap()
    };
    drag_with(
        &mut harness,
        PointerButton::Middle,
        at,
        egui::vec2(40.0, 0.0),
    );
    let panned = (screen(&harness, Vec2::ZERO) - was).length() > 10.0;
    let warped = !same(&uvs_of(&harness, id), &before);
    eprintln!("lattice + middle drag: panned {panned}, warped {warped}");
    assert!(!warped, "a pan drag warped the UVs");
    assert!(panned, "a pan drag didn't pan");
}
