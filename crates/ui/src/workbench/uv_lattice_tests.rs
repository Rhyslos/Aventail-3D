//! Tests for the UV lattice through the window: dragging its handles with
//! the mouse, undo and redo with it up, UV edits made while it is up, and
//! moving the view over it. (The first three were found by the UV test
//! pass.)

use super::modelling_tests::{H, cube, note, press};
use super::tests::quick_harness;
use super::*;
use egui::{Event, PointerButton, Pos2};
use glam::Vec2;
use scene::{Element, ElementSelection};

// ------------------------------------------------------------ helpers ---

/// The cube, selected, in the UV Edit layout.
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

/// A plane of `n` by `m` polygons, selected, in the UV Edit layout.
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
    (harness, id)
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

fn handles(harness: &H) -> Vec<Vec2> {
    harness
        .state()
        .workbench
        .lattice
        .as_ref()
        .unwrap()
        .points
        .clone()
}

fn screen(harness: &H, uv: Vec2) -> Pos2 {
    harness.state().workbench.uv_editor.screen_of(uv).unwrap()
}

/// A drag with `which` button from `from` by `by`, in steps, as a hand
/// does it (hovering first).
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

/// Drags lattice handle `k` by `by` screen points with the left button.
fn drag_handle(harness: &mut H, k: usize, by: egui::Vec2) {
    let at = screen(harness, handles(harness)[k]);
    drag_with(harness, PointerButton::Primary, at, by);
}

fn undo_key(harness: &mut H) {
    press(harness, Modifiers::COMMAND, Key::Z);
}

fn redo_key(harness: &mut H) {
    press(harness, Modifiers::COMMAND | Modifiers::SHIFT, Key::Z);
}

// -------------------------------------------------------------- tests ---

#[test]
fn dragging_a_handle_with_the_mouse_bends_the_uvs_one_step() {
    let mut harness = uv_edit_cube();
    let id = cube(&harness);
    let original = uvs_of(&harness, id);
    harness.state_mut().workbench.start_lattice();
    harness.run();
    let steps = harness.state().workbench.document().undo_steps();
    let was = handles(&harness)[4];
    drag_handle(&mut harness, 4, egui::vec2(40.0, 0.0));
    assert!(!same(&uvs_of(&harness, id), &original), "it bent");
    assert!(handles(&harness)[4].x > was.x, "the handle moved");
    assert_eq!(
        harness.state().workbench.document().undo_steps(),
        steps + 1,
        "one drag, one step"
    );
}

#[test]
fn undoing_a_lattice_drag_stays_undone_after_the_next_drag() {
    // What dragging only the corner handle gives.
    let expected = {
        let mut harness = uv_edit_cube();
        harness.state_mut().workbench.start_lattice();
        harness.run();
        drag_handle(&mut harness, 0, egui::vec2(-20.0, 20.0));
        uvs_of(&harness, cube(&harness))
    };
    let mut harness = uv_edit_cube();
    let id = cube(&harness);
    let original = uvs_of(&harness, id);
    harness.state_mut().workbench.start_lattice();
    harness.run();
    assert!(harness.state().workbench.warping(), "{}", note(&harness));
    let grid = handles(&harness);
    drag_handle(&mut harness, 4, egui::vec2(40.0, 0.0));
    assert!(
        !same(&uvs_of(&harness, id), &original),
        "the middle handle bent it"
    );
    undo_key(&mut harness);
    assert!(same(&uvs_of(&harness, id), &original), "undone");
    assert!(same(&handles(&harness), &grid), "the handles went back too");
    drag_handle(&mut harness, 0, egui::vec2(-20.0, 20.0));
    assert!(
        same(&uvs_of(&harness, id), &expected),
        "only the second drag's warp should be there"
    );
}

#[test]
fn redo_puts_the_drag_and_its_handle_back() {
    let mut harness = uv_edit_cube();
    let id = cube(&harness);
    harness.state_mut().workbench.start_lattice();
    harness.run();
    drag_handle(&mut harness, 4, egui::vec2(40.0, 0.0));
    let (bent, moved) = (uvs_of(&harness, id), handles(&harness));
    undo_key(&mut harness);
    redo_key(&mut harness);
    assert!(same(&uvs_of(&harness, id), &bent), "redone");
    assert!(same(&handles(&harness), &moved), "the handle came back");
    // And a further drag carries on from there.
    drag_handle(&mut harness, 0, egui::vec2(-20.0, 0.0));
    undo_key(&mut harness);
    assert!(same(&uvs_of(&harness, id), &bent));
    assert!(same(&handles(&harness), &moved));
}

#[test]
fn a_lattice_drag_keeps_uv_edits_made_while_it_is_up() {
    let mut harness = uv_edit_cube();
    let id = cube(&harness);
    harness.state_mut().workbench.start_lattice();
    harness.run();
    // Turn the top piece a quarter while the lattice is up (UV menu).
    harness.state_mut().workbench.mode = Mode::Polygons;
    let top = face_facing(&harness, id, glam::Vec3::Z);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements(
            [(id, ElementSelection::of([Element::Polygon(top)]))],
            "Select",
            None,
        );
    harness.run();
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
    // Undoing the drag leaves the turn.
    undo_key(&mut harness);
    assert!(same(&face_uvs(&harness, id, top), &turned));
}

#[test]
fn a_middle_drag_pans_rather_than_warps() {
    let (mut harness, id) = plane_in_editor(4, 4);
    let before = uvs_of(&harness, id);
    harness.state_mut().workbench.start_lattice();
    harness.run();
    assert!(harness.state().workbench.warping(), "{}", note(&harness));
    let grid = handles(&harness);
    let was = screen(&harness, Vec2::ZERO);
    let at = screen(&harness, grid[4]);
    drag_with(
        &mut harness,
        PointerButton::Middle,
        at,
        egui::vec2(40.0, 0.0),
    );
    assert!(
        same(&uvs_of(&harness, id), &before),
        "a pan drag warped the UVs"
    );
    assert!(same(&handles(&harness), &grid), "a pan drag moved a handle");
    assert!(
        (screen(&harness, Vec2::ZERO) - was).length() > 10.0,
        "a pan drag didn't pan"
    );
    // The left button still drags the handle afterwards.
    let at = screen(&harness, grid[4]);
    drag_with(
        &mut harness,
        PointerButton::Primary,
        at,
        egui::vec2(20.0, 0.0),
    );
    assert!(
        !same(&uvs_of(&harness, id), &before),
        "the handle still drags"
    );
}

#[test]
fn the_wheel_zooms_once_over_the_lattice() {
    let zoomed = |lattice: bool| {
        let (mut harness, _) = plane_in_editor(2, 2);
        if lattice {
            harness.state_mut().workbench.start_lattice();
            harness.run();
            assert!(harness.state().workbench.warping());
        }
        let at = screen(&harness, Vec2::splat(0.5));
        harness.event(Event::PointerMoved(at));
        harness.run();
        let width = screen(&harness, Vec2::X).x - screen(&harness, Vec2::ZERO).x;
        harness.event(Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, 50.0),
            modifiers: Modifiers::NONE,
            phase: egui::TouchPhase::Move,
        });
        // The wheel is smoothed over frames: the same number each time.
        for _ in 0..30 {
            harness.step();
        }
        (screen(&harness, Vec2::X).x - screen(&harness, Vec2::ZERO).x) / width
    };
    let (plain, over) = (zoomed(false), zoomed(true));
    assert!(plain > 1.01, "the wheel zooms: {plain}");
    assert!(
        (plain - over).abs() < 1e-3,
        "{plain} without, {over} with the lattice"
    );
}

#[test]
fn a_drag_after_deleting_a_polygon_leaves_what_is_gone_alone() {
    let (mut harness, id) = plane_in_editor(2, 2);
    harness.state_mut().workbench.start_lattice();
    harness.run();
    assert!(harness.state().workbench.warping(), "{}", note(&harness));
    press(&mut harness, Modifiers::NONE, Key::Num3);
    let first = mesh_of(&harness, id).faces().next().unwrap().0;
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements(
            [(id, ElementSelection::of([Element::Polygon(first)]))],
            "Select",
            None,
        );
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::Delete);
    assert_eq!(mesh_of(&harness, id).faces().count(), 3);
    let before = uvs_of(&harness, id);
    drag_handle(&mut harness, 4, egui::vec2(20.0, 0.0));
    assert_eq!(mesh_of(&harness, id).faces().count(), 3);
    assert!(uvs_of(&harness, id).iter().all(|uv| uv.is_finite()));
    assert!(!same(&uvs_of(&harness, id), &before), "the rest bent");
    assert!(!note(&harness).contains("Couldn't"), "{}", note(&harness));
}
