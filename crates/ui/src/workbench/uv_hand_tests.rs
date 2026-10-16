//! Tests for editing UVs by hand in the window (decision 0097): the flat
//! gizmo in the UV Editor, Esc and a right click cancelling it, the turn
//! and flip commands, and the UV ring of the tool menu over the editor.

use super::modelling_tests::{H, cube, last_operation, press};
use super::tests::quick_harness;
use super::*;
use crate::commands::CommandId;
use crate::uv_editor::gizmo::{UvHandle, handle_spot};
use egui::{Event, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;
use glam::Vec2;

/// A cube, selected, in the UV Edit layout (the UV mode), its top island
/// selected.
fn top_selected() -> H {
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
    let (_, uv) = top(&harness);
    let at = screen(&harness, uv);
    harness.event(Event::PointerMoved(at));
    harness.step();
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        harness.step();
    }
    harness.run();
    harness
}

/// The top polygon and the middle of its UVs.
fn top(harness: &H) -> (scene::FaceId, Vec2) {
    let mesh = super::modelling_tests::shape(harness);
    let uvs = mesh::uvs::Uvs::of(&mesh).unwrap();
    let (face, corners) = mesh
        .faces()
        .find(|(f, _)| mesh.face_normal(*f).dot(glam::Vec3::Z) > 0.99)
        .unwrap();
    let sum: Vec2 = (0..corners.len())
        .map(|k| uvs.corner(&mesh, face, k).unwrap())
        .sum();
    (face, sum / corners.len() as f32)
}

/// The top polygon's first corner's UV.
fn top_corner(harness: &H) -> Vec2 {
    let mesh = super::modelling_tests::shape(harness);
    let uvs = mesh::uvs::Uvs::of(&mesh).unwrap();
    let (face, _) = top(harness);
    uvs.corner(&mesh, face, 0).unwrap()
}

fn screen(harness: &H, uv: Vec2) -> Pos2 {
    harness.state().workbench.uv_editor.screen_of(uv).unwrap()
}

/// Screen points per texture width.
fn zoom(harness: &H) -> f32 {
    screen(harness, Vec2::X).x - screen(harness, Vec2::ZERO).x
}

fn gizmo_centre(harness: &H) -> Pos2 {
    let bench = &harness.state().workbench;
    bench
        .uv_editor
        .gizmo_centre(bench.document(), bench.mode)
        .expect("the gizmo shows")
}

/// Presses at `from` and moves by `by` in steps, without letting go.
fn drag_without_letting_go(harness: &mut H, from: Pos2, by: egui::Vec2) {
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
        harness.event(Event::PointerMoved(from + by * (i as f32 / 10.0)));
        harness.step();
    }
}

fn let_go(harness: &mut H, at: Pos2) {
    harness.event(Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    harness.run();
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

#[test]
fn w_shows_the_gizmo_and_a_drag_moves_the_island_as_one_undo_step() {
    let mut harness = top_selected();
    over_editor(&mut harness);
    press(&mut harness, Modifiers::NONE, Key::W);
    assert_eq!(harness.state().workbench.tool(), Tool::Move);
    let (_, before) = top(&harness);
    let steps = harness.state().workbench.document().undo_steps();
    let centre = gizmo_centre(&harness);
    let from = handle_spot(centre, Tool::Move, UvHandle::MoveU);
    let by = egui::vec2(60.0, 25.0);
    drag_without_letting_go(&mut harness, from, by);
    // Shown live, before letting go; the arrow across only moves across.
    let (_, during) = top(&harness);
    let expected = 60.0 / zoom(&harness);
    assert!((during.x - before.x - expected).abs() < 0.02, "{during}");
    assert!((during.y - before.y).abs() < 1e-4);
    let_go(&mut harness, from + by);
    let (_, after) = top(&harness);
    assert!((after - during).length() < 1e-4);
    let doc = harness.state().workbench.document();
    assert_eq!(doc.undo_steps(), steps + 1, "one step for the whole drag");
    assert!(matches!(
        last_operation(&harness),
        Some(scene::Operation::TransformUvs { .. })
    ));
    // The selection didn't change: the press was on the gizmo.
    assert_eq!(
        harness
            .state()
            .workbench
            .document()
            .elements(cube(&harness))
            .unwrap()
            .count(scene::ElementKind::Polygons),
        1
    );
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    let (_, undone) = top(&harness);
    assert!((undone - before).length() < 1e-5);
}

#[test]
fn esc_and_a_right_click_cancel_a_gizmo_drag() {
    for right_click in [false, true] {
        let mut harness = top_selected();
        over_editor(&mut harness);
        press(&mut harness, Modifiers::NONE, Key::T);
        let (_, before) = top(&harness);
        let steps = harness.state().workbench.document().undo_steps();
        let centre = gizmo_centre(&harness);
        let from = handle_spot(centre, Tool::All, UvHandle::MoveFree);
        let by = egui::vec2(40.0, -30.0);
        drag_without_letting_go(&mut harness, from, by);
        assert!((top(&harness).1 - before).length() > 0.01);
        if right_click {
            harness.event(Event::PointerButton {
                pos: from + by,
                button: PointerButton::Secondary,
                pressed: true,
                modifiers: Modifiers::NONE,
            });
            harness.step();
        } else {
            harness.key_press(Key::Escape);
            harness.step();
        }
        let_go(&mut harness, from + by);
        assert!((top(&harness).1 - before).length() < 1e-5);
        let doc = harness.state().workbench.document();
        assert_eq!(doc.undo_steps(), steps);
        assert!(!harness.state().workbench.uv_editor.dragging_gizmo());
    }
}

#[test]
fn the_ring_turns_the_island_about_its_middle() {
    let mut harness = top_selected();
    press(&mut harness, Modifiers::NONE, Key::R);
    let (_, middle) = top(&harness);
    let corner = top_corner(&harness);
    let centre = gizmo_centre(&harness);
    // A quarter turn anticlockwise on screen, from the ring's right to
    // its top, with snapping off (the standard).
    let from = handle_spot(centre, Tool::Rotate, UvHandle::Turn);
    let radius = from.distance(centre);
    harness.event(Event::PointerMoved(from));
    harness.step();
    harness.event(Event::PointerButton {
        pos: from,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    let start = std::f32::consts::FRAC_PI_4;
    let mut at = from;
    for i in 1..=12 {
        // The screen's angle grows clockwise: going down is anticlockwise.
        let a = start - std::f32::consts::FRAC_PI_2 * (i as f32 / 12.0);
        at = centre + egui::vec2(a.cos(), a.sin()) * radius;
        harness.event(Event::PointerMoved(at));
        harness.step();
    }
    let_go(&mut harness, at);
    let (_, still) = top(&harness);
    assert!((still - middle).length() < 1e-3, "turned about its middle");
    let was = corner - middle;
    let now = top_corner(&harness) - middle;
    let turned = Vec2::new(-was.y, was.x);
    assert!((now - turned).length() < 0.02, "{now} vs {turned}");
}

#[test]
fn turn_and_flip_commands_change_the_selected_uvs() {
    let mut harness = top_selected();
    let (_, middle) = top(&harness);
    let was = top_corner(&harness) - middle;
    let run = |harness: &mut H, command| {
        let mut requests = Vec::new();
        let ctx = harness.ctx.clone();
        harness
            .state_mut()
            .workbench
            .run_command(command, &ctx, &mut requests);
        harness.run();
    };
    run(&mut harness, CommandId::UvTurnLeft);
    let now = top_corner(&harness) - middle;
    assert!((now - Vec2::new(-was.y, was.x)).length() < 1e-4);
    run(&mut harness, CommandId::UvTurnRight);
    assert!((top_corner(&harness) - middle - was).length() < 1e-4);
    run(&mut harness, CommandId::UvFlipU);
    assert!((top_corner(&harness) - middle - Vec2::new(-was.x, was.y)).length() < 1e-4);
    run(&mut harness, CommandId::UvFlipV);
    assert!((top_corner(&harness) - middle - Vec2::new(-was.x, -was.y)).length() < 1e-4);
    assert!(
        harness
            .state()
            .workbench
            .note()
            .unwrap_or_default()
            .contains("Ctrl + Z")
    );
    // In the Model mode there is nothing to turn.
    harness.state_mut().workbench.mode = Mode::Model;
    run(&mut harness, CommandId::UvTurnLeft);
    assert!(
        harness
            .state()
            .workbench
            .note()
            .unwrap_or_default()
            .contains("selected UVs")
    );
}

#[test]
fn e_over_the_editor_brings_the_uv_ring_in_every_editing_mode() {
    let mut harness = top_selected();
    harness.state_mut().workbench.mode = Mode::Polygons;
    over_editor(&mut harness);
    press(&mut harness, Modifiers::NONE, crate::TOOL_MENU_KEY);
    let bench = &harness.state().workbench;
    assert!(bench.is_tool_menu_open());
    let commands: Vec<CommandId> = bench
        .tool_menu_entries()
        .iter()
        .map(|e| e.slot.command)
        .collect();
    assert_eq!(commands, crate::commands::UV_RING.to_vec());
    press(&mut harness, Modifiers::NONE, Key::Escape);
    // Over the viewport, the modelling ring.
    let viewport = harness.get_by_label(crate::VIEWPORT_LABEL).rect().center();
    harness.event(Event::PointerMoved(viewport));
    harness.run();
    press(&mut harness, Modifiers::NONE, crate::TOOL_MENU_KEY);
    let bench = &harness.state().workbench;
    assert!(bench.is_tool_menu_open());
    assert!(
        bench
            .tool_menu_entries()
            .iter()
            .any(|e| e.slot.command == CommandId::Extrude)
    );
}

#[test]
fn a_closed_editor_no_longer_takes_keys_where_it_was() {
    let mut harness = top_selected();
    over_editor(&mut harness);
    assert!(harness.state().workbench.uv_editor.hovered(&harness.ctx));
    harness
        .state_mut()
        .workbench
        .switch_layout(LayoutChoice::Standard);
    harness.run();
    harness.run();
    assert!(!harness.state().workbench.uv_editor.hovered(&harness.ctx));
}
