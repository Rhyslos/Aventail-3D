//! The UV brush and UV Editor controls behaving like the rest of the app:
//! regression tests for the problems a test pass of the UV suite found
//! (brush and Isolate, keys mid-stroke, Esc and Tab mid-drag, the middle
//! button panning, gizmo keys with a brush up, X with the editor's own
//! selection, and one undo step per drag in the Exact menu).

use super::modelling_tests::{H, cube, note, press};
use super::seams_tests::{add, inner};
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

/// A plane `n` × `m` polygons, selected alone, in the UV Edit layout and
/// the Edges mode.
fn plane_in_editor(n: u32, m: u32) -> (H, scene::ObjectId) {
    let mut harness = quick_harness();
    let id = add(
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

/// The cube in the UV mode with its top piece selected.
fn top_piece_selected() -> (H, scene::ObjectId, scene::FaceId) {
    let mut harness = uv_edit_cube();
    let id = cube(&harness);
    let top = face_facing(&harness, id, glam::Vec3::Z);
    select(&mut harness, id, vec![Element::Polygon(top)]);
    assert_eq!(harness.state().workbench.mode, Mode::Uv);
    (harness, id, top)
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

fn same(a: &[Vec2], b: &[Vec2]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (*x - *y).length() < 1e-4)
}

fn face_uvs(harness: &H, id: scene::ObjectId, face: scene::FaceId) -> Vec<Vec2> {
    let mesh = mesh_of(harness, id);
    let uvs = mesh::uvs::Uvs::of(&mesh).unwrap();
    let n = mesh.face(face).unwrap().len();
    (0..n)
        .map(|k| uvs.corner(&mesh, face, k).unwrap())
        .collect()
}

fn face_middle(harness: &H, id: scene::ObjectId, face: scene::FaceId) -> Vec2 {
    let uvs = face_uvs(harness, id, face);
    uvs.iter().sum::<Vec2>() / uvs.len() as f32
}

/// A spot just inside face `face`'s first corner (a brush moves corners).
fn near_corner(harness: &H, id: scene::ObjectId, face: scene::FaceId) -> Vec2 {
    let corner = face_uvs(harness, id, face)[0];
    corner + (face_middle(harness, id, face) - corner) * 0.1
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

fn button(harness: &mut H, which: PointerButton, at: Pos2, pressed: bool) {
    harness.event(Event::PointerButton {
        pos: at,
        button: which,
        pressed,
        modifiers: Modifiers::NONE,
    });
    harness.step();
}

/// Presses `which` at `from` and moves by `by` in steps, without letting
/// go (hovering first, as a hand does).
fn press_and_move(harness: &mut H, which: PointerButton, from: Pos2, by: egui::Vec2) {
    harness.event(Event::PointerMoved(from));
    harness.run();
    harness.step();
    button(harness, which, from, true);
    for i in 1..=10 {
        harness.event(Event::PointerMoved(from + by * (i as f32 / 10.0)));
        harness.step();
    }
}

fn let_go(harness: &mut H, which: PointerButton, at: Pos2) {
    button(harness, which, at, false);
    harness.run();
}

fn drag(harness: &mut H, from: Pos2, by: egui::Vec2) {
    press_and_move(harness, PointerButton::Primary, from, by);
    let_go(harness, PointerButton::Primary, from + by);
}

/// Moves on by `by` in a few steps, the button still held.
fn move_on(harness: &mut H, from: Pos2, by: egui::Vec2) {
    for i in 1..=5 {
        harness.event(Event::PointerMoved(from + by * (i as f32 / 5.0)));
        harness.step();
    }
}

fn steps(harness: &H) -> usize {
    harness.state().workbench.document().undo_steps()
}

fn grab(harness: &mut H) {
    harness
        .state_mut()
        .workbench
        .start_uv_brush(UvBrushKind::Grab);
    harness.run();
}

// ------------------------------------------------------------- brush ---

#[test]
fn a_brush_stroke_through_the_window_is_one_undo_step() {
    let mut harness = uv_edit_cube();
    let id = cube(&harness);
    let top = face_facing(&harness, id, glam::Vec3::Z);
    let before = uvs_of(&harness, id);
    let s = steps(&harness);
    grab(&mut harness);
    let at = screen(&harness, near_corner(&harness, id, top));
    drag(&mut harness, at, egui::vec2(25.0, 10.0));
    assert!(
        !same(&uvs_of(&harness, id), &before),
        "the stroke moved UVs"
    );
    assert_eq!(steps(&harness), s + 1);
    press(&mut harness, Modifiers::COMMAND, Key::Z);
    assert!(same(&uvs_of(&harness, id), &before));
}

#[test]
fn a_brush_leaves_pieces_hidden_by_isolate_alone() {
    let mut harness = uv_edit_cube();
    let id = cube(&harness);
    harness.state_mut().workbench.mode = Mode::Polygons;
    let top = face_facing(&harness, id, glam::Vec3::Z);
    select(&mut harness, id, vec![Element::Polygon(top)]);
    {
        let bench = &mut harness.state_mut().workbench;
        bench.uv_editor.set_isolating(true, &bench.document);
    }
    harness.run();
    harness.run();
    let bottom = face_facing(&harness, id, -glam::Vec3::Z);
    let before = face_uvs(&harness, id, bottom);
    grab(&mut harness);
    let at = screen(&harness, near_corner(&harness, id, bottom));
    drag(&mut harness, at, egui::vec2(25.0, 0.0));
    assert!(
        same(&face_uvs(&harness, id, bottom), &before),
        "the hidden bottom piece moved"
    );
    // The shown piece still brushes.
    let top_before = face_uvs(&harness, id, top);
    let at = screen(&harness, near_corner(&harness, id, top));
    drag(&mut harness, at, egui::vec2(25.0, 0.0));
    assert!(!same(&face_uvs(&harness, id, top), &top_before));
}

#[test]
fn delete_and_undo_wait_for_a_brush_stroke() {
    let (mut harness, id) = plane_in_editor(4, 4);
    press(&mut harness, Modifiers::NONE, Key::Num3);
    let faces: Vec<Element> = mesh_of(&harness, id)
        .faces()
        .take(4)
        .map(|(f, _)| Element::Polygon(f))
        .collect();
    select(&mut harness, id, faces);
    grab(&mut harness);
    let at = screen(&harness, Vec2::splat(0.5));
    let s = steps(&harness);
    press_and_move(
        &mut harness,
        PointerButton::Primary,
        at,
        egui::vec2(20.0, 0.0),
    );
    press(&mut harness, Modifiers::NONE, Key::Delete);
    assert_eq!(
        mesh_of(&harness, id).faces().count(),
        16,
        "Delete went through in the middle of a stroke"
    );
    press(&mut harness, Modifiers::COMMAND, Key::Z);
    move_on(
        &mut harness,
        at + egui::vec2(20.0, 0.0),
        egui::vec2(15.0, 0.0),
    );
    let_go(
        &mut harness,
        PointerButton::Primary,
        at + egui::vec2(35.0, 0.0),
    );
    assert_eq!(mesh_of(&harness, id).faces().count(), 16);
    assert_eq!(steps(&harness), s + 1, "the stroke is still one step");
    // After the stroke, the keys work again.
    press(&mut harness, Modifiers::NONE, Key::Delete);
    assert_eq!(
        mesh_of(&harness, id).faces().count(),
        12,
        "{}",
        note(&harness)
    );
}

#[test]
fn esc_during_a_brush_stroke_takes_it_back_and_keeps_the_brush() {
    let (mut harness, id) = plane_in_editor(4, 4);
    let before = uvs_of(&harness, id);
    grab(&mut harness);
    let at = screen(&harness, Vec2::splat(0.5));
    let s = steps(&harness);
    press_and_move(
        &mut harness,
        PointerButton::Primary,
        at,
        egui::vec2(20.0, 0.0),
    );
    assert!(!same(&uvs_of(&harness, id), &before));
    press(&mut harness, Modifiers::NONE, Key::Escape);
    assert!(same(&uvs_of(&harness, id), &before), "the stroke went back");
    // The rest of the drag does nothing.
    move_on(
        &mut harness,
        at + egui::vec2(20.0, 0.0),
        egui::vec2(15.0, 0.0),
    );
    let_go(
        &mut harness,
        PointerButton::Primary,
        at + egui::vec2(35.0, 0.0),
    );
    assert!(same(&uvs_of(&harness, id), &before));
    assert_eq!(steps(&harness), s, "no step left behind");
    assert_eq!(
        harness.state().workbench.uv_brush(),
        Some(UvBrushKind::Grab)
    );
    // Nothing selected by letting go.
    assert!(
        harness
            .state()
            .workbench
            .document()
            .elements(id)
            .is_none_or(|e| e.is_empty())
    );
    // Esc again, between strokes, puts the brush down as before.
    press(&mut harness, Modifiers::NONE, Key::Escape);
    assert!(harness.state().workbench.uv_brush().is_none());
}

#[test]
fn tab_during_a_brush_stroke_waits() {
    let (mut harness, _) = plane_in_editor(4, 4);
    grab(&mut harness);
    let at = screen(&harness, Vec2::splat(0.5));
    press_and_move(
        &mut harness,
        PointerButton::Primary,
        at,
        egui::vec2(20.0, 0.0),
    );
    press(&mut harness, Modifiers::NONE, Key::Tab);
    eprintln!("note after Tab: {}", note(&harness));
    assert_eq!(harness.state().workbench.mode, Mode::Edges);
    let_go(
        &mut harness,
        PointerButton::Primary,
        at + egui::vec2(20.0, 0.0),
    );
    press(&mut harness, Modifiers::NONE, Key::Tab);
    assert_eq!(harness.state().workbench.mode, Mode::Model);
}

#[test]
fn a_middle_drag_pans_rather_than_brushes() {
    let (mut harness, id) = plane_in_editor(4, 4);
    let before = uvs_of(&harness, id);
    grab(&mut harness);
    let was = screen(&harness, Vec2::ZERO);
    let at = screen(&harness, Vec2::splat(0.5));
    press_and_move(
        &mut harness,
        PointerButton::Middle,
        at,
        egui::vec2(40.0, 0.0),
    );
    let_go(
        &mut harness,
        PointerButton::Middle,
        at + egui::vec2(40.0, 0.0),
    );
    assert!(
        same(&uvs_of(&harness, id), &before),
        "a pan drag brushed the UVs"
    );
    assert!(
        (screen(&harness, Vec2::ZERO) - was).length() > 10.0,
        "a pan drag didn't pan"
    );
    // The left button still brushes afterwards.
    let at = screen(&harness, Vec2::splat(0.5));
    drag(&mut harness, at, egui::vec2(20.0, 0.0));
    assert!(!same(&uvs_of(&harness, id), &before));
}

#[test]
fn a_gizmo_key_with_a_uv_brush_up_lets_the_gizmo_be_dragged() {
    let mut harness = uv_edit_cube();
    let id = cube(&harness);
    harness.state_mut().workbench.mode = Mode::Polygons;
    let top = face_facing(&harness, id, glam::Vec3::Z);
    select(&mut harness, id, vec![Element::Polygon(top)]);
    grab(&mut harness);
    over_editor(&mut harness);
    press(&mut harness, Modifiers::NONE, Key::W);
    assert!(
        harness.state().workbench.uv_brush().is_none(),
        "W puts the brush down"
    );
    let before = face_middle(&harness, id, top);
    let from = handle_spot(gizmo_centre(&harness), Tool::Move, UvHandle::MoveU);
    drag(&mut harness, from, egui::vec2(50.0, 0.0));
    let after = face_middle(&harness, id, top);
    assert!(
        (after.y - before.y).abs() < 1e-4 && after.x > before.x + 0.01,
        "the arrow across should move the piece across: {before} -> {after}"
    );
}

// ------------------------------------------------------------- gizmo ---

#[test]
fn tab_during_a_uv_gizmo_drag_waits_like_the_viewport() {
    let (mut harness, id, top) = top_piece_selected();
    over_editor(&mut harness);
    press(&mut harness, Modifiers::NONE, Key::W);
    let before = face_middle(&harness, id, top);
    let s = steps(&harness);
    let from = handle_spot(gizmo_centre(&harness), Tool::Move, UvHandle::MoveFree);
    press_and_move(
        &mut harness,
        PointerButton::Primary,
        from,
        egui::vec2(40.0, 0.0),
    );
    assert!(harness.state().workbench.uv_editor.dragging_gizmo());
    press(&mut harness, Modifiers::NONE, Key::Tab);
    assert_eq!(
        harness.state().workbench.mode,
        Mode::Uv,
        "Tab left the UV mode"
    );
    assert!(note(&harness).contains("Let go"), "{}", note(&harness));
    let_go(
        &mut harness,
        PointerButton::Primary,
        from + egui::vec2(40.0, 0.0),
    );
    assert!(
        face_middle(&harness, id, top).x > before.x + 0.01,
        "the move stayed"
    );
    assert_eq!(steps(&harness), s + 1);
    // Once let go, Tab steps back as usual.
    press(&mut harness, Modifiers::NONE, Key::Tab);
    assert_eq!(harness.state().workbench.mode, Mode::Model);
}

#[test]
fn grid_snapping_puts_the_pivot_on_the_grid() {
    // The manual's words: the gizmo's pivot lands on the grid (here a
    // corner of the selection's box), not always the selection's middle.
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
    let low = |h: &H| {
        face_uvs(h, id, top)
            .into_iter()
            .fold(Vec2::splat(f32::INFINITY), Vec2::min)
    };
    let was = low(&harness);
    let from = screen(&harness, was);
    drag(&mut harness, from, egui::vec2(37.0, 0.0));
    let now = low(&harness);
    assert!((now - was).length() > 1e-3, "it moved");
    let on_grid = |x: f32| ((x * 10.0) - (x * 10.0).round()).abs() < 1e-3;
    assert!(
        on_grid(now.x),
        "the corner {now} isn't on the grid of tenths"
    );
}

// ------------------------------------------------- the own selection ---

#[test]
fn x_over_the_editor_cuts_what_the_editor_has_selected() {
    let (mut harness, id) = plane_in_editor(2, 1);
    {
        let bench = &mut harness.state_mut().workbench;
        bench.uv_editor.set_own_selection(true, &bench.document);
    }
    harness.run();
    // Pick the middle edge in the editor: a click on its middle.
    let mesh = mesh_of(&harness, id);
    let edge = inner(&mesh)[0];
    let uvs = mesh::uvs::Uvs::of(&mesh).unwrap();
    let f = mesh.topology().edge_faces(edge).next().unwrap();
    let list = mesh.face(f).unwrap().to_vec();
    let (a, b) = edge.ends();
    let ka = list.iter().position(|v| *v == a).unwrap();
    let kb = list.iter().position(|v| *v == b).unwrap();
    let middle = (uvs.corner(&mesh, f, ka).unwrap() + uvs.corner(&mesh, f, kb).unwrap()) / 2.0;
    let at = screen(&harness, middle);
    harness.event(Event::PointerMoved(at));
    harness.run();
    button(&mut harness, PointerButton::Primary, at, true);
    button(&mut harness, PointerButton::Primary, at, false);
    harness.run();
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
    assert_eq!(
        mesh::uvs::islands(&mesh_of(&harness, id)).len(),
        2,
        "{}",
        note(&harness)
    );
}

// ------------------------------------------------------------- exact ---

#[test]
fn dragging_the_exact_middle_is_one_undo_step() {
    let (mut harness, id, top) = top_piece_selected();
    let label = tr(crate::uv_editor::placing::EXACT_MENU);
    let menu = harness.get_by_label(label).rect();
    harness.get_by_label(label).click();
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
    let before = face_middle(&harness, id, top);
    let s = steps(&harness);
    drag(&mut harness, field, egui::vec2(30.0, 0.0));
    let after = face_middle(&harness, id, top);
    assert!(after.x > before.x, "moved across");
    assert_eq!(steps(&harness), s + 1, "one drag, one undo step");
    // A second drag is a step of its own.
    drag(&mut harness, field, egui::vec2(30.0, 0.0));
    assert_eq!(steps(&harness), s + 2);
    press(&mut harness, Modifiers::COMMAND, Key::Z);
    assert!((face_middle(&harness, id, top) - after).length() < 1e-4);
}
