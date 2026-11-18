//! Tests for Remesh for sculpting in the window (decision 0312): Ctrl + R
//! as one undo step, selected shapes merged, the question before a heavy
//! remesh and the refusal of one too fine, Cancel, strokes waiting, the
//! voxel grid, the estimate, the brush bar's button and the Sculpt menu's
//! command.

use super::heavy_check::H;
use super::sculpt_tests::{front, sculpt_stroke, sculpting, shape, sphere};
use crate::commands::CommandId;
use crate::sculpt_masks::SculptAction;
use crate::tool_menu::Ready;
use egui::Modifiers;
use egui_kittest::kittest::Queryable;
use scene::{Command, ObjectId, ObjectKind, PrimitiveKind};

/// Sets the voxel size, showing its grid for a frame (`step`), then
/// takes the grid away, so the window settles.
fn voxel(harness: &mut H, size: f32) {
    harness
        .state_mut()
        .workbench
        .attributes
        .sculpt
        .remesh
        .voxel_size = size;
    harness.step();
    harness
        .state_mut()
        .workbench
        .sculpting
        .remeshing
        .hide_grid();
    harness.run();
}

/// Presses Ctrl + R (a frame, so a remesh starts), without waiting.
fn press_ctrl_r(harness: &mut H) {
    harness.event(egui::Event::Key {
        key: egui::Key::R,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::COMMAND,
    });
    harness.step();
}

/// Presses Ctrl + R, waits for the remesh and lets the window take it.
fn remesh(harness: &mut H) {
    press_ctrl_r(harness);
    finish(harness);
}

/// Waits for a remesh under way and lets the window take it.
fn finish(harness: &mut H) {
    harness.state().workbench.sculpting.remeshing.wait();
    harness.step();
    harness.run();
}

fn note(harness: &H) -> String {
    harness
        .state()
        .workbench
        .note()
        .unwrap_or_default()
        .to_string()
}

fn steps(harness: &H) -> usize {
    harness.state().workbench.document().undo_steps()
}

/// A cube overlapping the sphere's side, selected after it.
fn with_a_cube(harness: &mut H) -> ObjectId {
    let ball = sphere(harness);
    let wb = &mut harness.state_mut().workbench;
    wb.document_mut()
        .execute(Command::Create(ObjectKind::Primitive(
            PrimitiveKind::Cube.standard(),
        )))
        .unwrap();
    let cube = wb.document().selection()[0];
    let mut moved = wb.document().scene().object(cube).unwrap().transform;
    moved.position.x += 45.0;
    wb.document_mut()
        .execute(Command::SetTransforms(vec![(cube, moved)]))
        .unwrap();
    wb.document_mut().set_selection([ball, cube]);
    harness.run();
    cube
}

#[test]
fn ctrl_r_remeshes_the_sculpt_in_one_undo_step() {
    let mut harness = sculpting();
    let id = sphere(&harness);
    sculpt_stroke(&mut harness, Modifiers::NONE);
    voxel(&mut harness, 4.0);
    let estimate = harness
        .state()
        .workbench
        .attributes
        .sculpt
        .remesh
        .polygons
        .expect("an estimate");
    let before = harness
        .state()
        .workbench
        .document()
        .scene()
        .object(id)
        .unwrap()
        .clone();
    let start = steps(&harness);
    remesh(&mut harness);
    assert_eq!(steps(&harness), start + 1);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Remesh for sculpting")
    );
    assert!(
        note(&harness).contains("Remeshed into"),
        "{}",
        note(&harness)
    );
    let wb = &harness.state().workbench;
    let object = wb.document().scene().object(id).unwrap();
    assert!(matches!(object.kind, ObjectKind::Mesh(_)));
    let layers = object.stack.layers();
    assert_eq!(layers.len(), 1);
    assert!(layers[0].is_sculpt());
    let made = shape(&harness);
    let report = mesh::MeshReport::new(&made);
    assert_eq!(report.border_edges, 0);
    let polygons = made.face_count() as f64;
    assert!(
        (polygons - estimate as f64).abs() < polygons * 0.2,
        "{polygons} vs {estimate}"
    );
    // The live sculpt follows the new shape, and sculpting goes on.
    harness.run();
    let live = harness.state().workbench.sculpting.live.as_ref().unwrap();
    assert_eq!(live.live.mesh().face_count(), made.face_count());
    sculpt_stroke(&mut harness, Modifiers::NONE);
    assert_eq!(steps(&harness), start + 2);
    // Undo twice: the sphere and its sculpt as they were.
    harness.key_press_modifiers(Modifiers::COMMAND, egui::Key::Z);
    harness.key_press_modifiers(Modifiers::COMMAND, egui::Key::Z);
    harness.run();
    assert_eq!(
        *harness
            .state()
            .workbench
            .document()
            .scene()
            .object(id)
            .unwrap(),
        before
    );
}

#[test]
fn selected_shapes_merge_into_the_sculpt() {
    let mut harness = sculpting();
    let ball = sphere(&harness);
    let cube = with_a_cube(&mut harness);
    voxel(&mut harness, 4.0);
    // The shape sculpted is the first selected; the other merges into it.
    let target = harness.state().workbench.sculpting.sculpted().unwrap();
    let other = if target == ball { cube } else { ball };
    let reach = |h: &H| {
        let b = h
            .state()
            .workbench
            .document()
            .scene()
            .evaluated(target)
            .unwrap()
            .mesh;
        b.bounds().unwrap().size().x
    };
    let wide = reach(&harness);
    remesh(&mut harness);
    let wb = &harness.state().workbench;
    assert!(!wb.document().scene().contains(other), "{}", note(&harness));
    assert_eq!(wb.document().selection(), vec![target]);
    assert!(
        note(&harness).contains("Merged 2 shapes"),
        "{}",
        note(&harness)
    );
    assert!(reach(&harness) > wide + 20.0);
    let made = wb.document().scene().evaluated(target).unwrap().mesh;
    assert_eq!(mesh::MeshReport::new(&made).pieces, 1);
    harness.key_press_modifiers(Modifiers::COMMAND, egui::Key::Z);
    harness.run();
    assert!(harness.state().workbench.document().scene().contains(other));
}

#[test]
fn a_heavy_remesh_asks_first_and_one_too_fine_is_refused() {
    let mut harness = sculpting();
    let start = steps(&harness);
    // About 4.7 million polygons: asked first; Esc says no.
    voxel(&mut harness, 0.1);
    press_ctrl_r(&mut harness);
    harness.run();
    assert!(harness.state().workbench.sculpting.remeshing.asking());
    harness.get_by_label("Remesh anyway");
    harness.key_press(egui::Key::Escape);
    harness.run();
    let wb = &harness.state().workbench;
    assert!(!wb.sculpting.remeshing.asking());
    assert!(!wb.sculpting.remeshing.busy());
    assert_eq!(
        wb.mode,
        crate::palette::Mode::Sculpt,
        "Esc answered, didn't leave"
    );
    // About 19 million: refused at once, saying what fits.
    voxel(&mut harness, 0.05);
    press_ctrl_r(&mut harness);
    harness.run();
    assert!(
        note(&harness).contains("million polygons"),
        "{}",
        note(&harness)
    );
    assert!(
        note(&harness).contains("Make Voxel size"),
        "{}",
        note(&harness)
    );
    assert!(!harness.state().workbench.sculpting.remeshing.busy());
    assert_eq!(steps(&harness), start);
}

#[test]
fn cancel_stops_it_and_strokes_wait_while_it_runs() {
    let mut harness = sculpting();
    let before = shape(&harness);
    let start = steps(&harness);
    // About 2 million polygons: long enough to stop.
    voxel(&mut harness, 0.15);
    press_ctrl_r(&mut harness);
    harness.step();
    assert!(harness.state().workbench.sculpting.remeshing.busy());
    assert!(
        harness
            .query_all_by_label("Remeshing for sculpting")
            .next()
            .is_some()
    );
    // A press on the shape sculpts nothing while it runs.
    let (_, at) = front(&harness);
    harness.event(egui::Event::PointerMoved(at));
    harness.step();
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        harness.step();
    }
    assert!(note(&harness).contains("Remeshing"), "{}", note(&harness));
    assert!(!harness.state().workbench.sculpting.stroking());
    // So do the mask's actions.
    harness.state_mut().workbench.note = None;
    harness
        .state_mut()
        .workbench
        .run_sculpt_action(SculptAction::InvertMask);
    assert!(note(&harness).contains("Remeshing"), "{}", note(&harness));
    harness.event(egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    finish(&mut harness);
    assert!(note(&harness).contains("stopped"), "{}", note(&harness));
    assert_eq!(steps(&harness), start);
    assert_eq!(shape(&harness), before);
    assert_eq!(harness.state().workbench.mode, crate::palette::Mode::Sculpt);
}

#[test]
fn changing_the_voxel_size_shows_its_grid_on_the_shape() {
    let mut harness = sculpting();
    assert!(harness.state().workbench.viewport().voxel_grid().is_none());
    harness
        .state_mut()
        .workbench
        .attributes
        .sculpt
        .remesh
        .voxel_size = 3.0;
    harness.step();
    let grid = *harness
        .state()
        .workbench
        .viewport()
        .voxel_grid()
        .expect("the grid shows");
    assert_eq!(grid.step, 3.0);
    // On the sphere's surface, facing the view.
    let b = shape(&harness).bounds().unwrap();
    let r = (b.max.x - b.min.x) / 2.0;
    assert!(
        (grid.centre.distance(b.centre()) - r).abs() < 2.0,
        "{grid:?}"
    );
    let towards = -harness.state().workbench.viewport().camera().forward();
    assert!(grid.normal.dot(towards) > 0.0);
    // It goes after a moment.
    std::thread::sleep(super::sculpt_remeshing::grid_shown_for());
    harness.step();
    assert!(harness.state().workbench.viewport().voxel_grid().is_none());
    // The estimate follows the size: half the size, about four times the
    // polygons.
    let at = |h: &H| {
        h.state()
            .workbench
            .attributes
            .sculpt
            .remesh
            .polygons
            .unwrap() as f64
    };
    let coarse = at(&harness);
    voxel(&mut harness, 1.5);
    assert!((at(&harness) / coarse - 4.0).abs() < 0.1);
    // Leaving Sculpt mode takes the grid away.
    harness
        .state_mut()
        .workbench
        .attributes
        .sculpt
        .remesh
        .voxel_size = 2.0;
    harness.step();
    assert!(harness.state().workbench.viewport().voxel_grid().is_some());
    harness.state_mut().workbench.mode = crate::palette::Mode::Model;
    harness.run();
    assert!(harness.state().workbench.viewport().voxel_grid().is_none());
}

#[test]
fn the_brush_bar_and_the_sculpt_menu_remesh_too() {
    let mut harness = sculpting();
    voxel(&mut harness, 5.0);
    let start = steps(&harness);
    harness.get_by_label("Remesh voxel size");
    harness.get_by_label("Remesh the sculpt").click();
    harness.step();
    harness.step();
    finish(&mut harness);
    assert_eq!(steps(&harness), start + 1);
    let mut requests = Vec::new();
    let ctx = harness.ctx.clone();
    let command = CommandId::Sculpt(SculptAction::Remesh);
    assert_eq!(harness.state().workbench.readiness(command), Ready::Yes);
    harness
        .state_mut()
        .workbench
        .run_command(command, &ctx, &mut requests);
    harness.step();
    finish(&mut harness);
    assert_eq!(steps(&harness), start + 2);
    // Outside Sculpt mode it says where it works.
    harness.state_mut().workbench.mode = crate::palette::Mode::Model;
    harness.run();
    assert!(matches!(
        harness.state().workbench.readiness(command),
        Ready::NotNow(_)
    ));
}
