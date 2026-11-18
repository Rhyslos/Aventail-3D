//! Tests for symmetry and masks in Sculpt mode (decision 0310): the keys
//! (L, 9 again, Ctrl + I, Alt + M, Ctrl + wheel), a mask painted and
//! shown darker holding the shape still, a mirrored stroke, the box mask,
//! the gizmo moving the unmasked part, Make symmetrical, Make a shape from
//! the mask, the Sculpt menu's commands and the brush's circles on the
//! surface.

use super::heavy_check::H;
use super::sculpt_stroke_tests::{drag, sculpting_with, under};
use super::sculpt_tests::{front, shape, sphere};
use crate::commands::CommandId;
use crate::sculpt_masks::{MaskTool, SculptAction};
use crate::sculpt_tab::StrokeKind;
use crate::viewport::GizmoEvent;
use egui::{Event, Modifiers, Pos2};
use glam::Vec3;
use sculpt::BrushKind;

/// A window sculpting a dense sphere, with the Mask brush at full
/// strength.
fn masking() -> H {
    let mut harness = sculpting_with(StrokeKind::Freehand);
    harness.key_press(egui::Key::Num9);
    harness.run();
    let settings = &mut harness.state_mut().workbench.attributes.sculpt;
    assert_eq!(settings.brush, BrushKind::Mask);
    settings.of_mut(BrushKind::Mask).radius = 15.0;
    harness
}

/// The live sculpt's mask value of point `v`.
fn masked(harness: &H, v: scene::VertexId) -> u8 {
    let live = harness.state().workbench.sculpting.live.as_ref().unwrap();
    live.live.mask().get(v.get() as usize)
}

/// The document's sculpt of the sphere, if it has one.
fn doc_sculpt(harness: &H) -> Option<sculpt::Sculpt> {
    let wb = &harness.state().workbench;
    wb.document()
        .scene()
        .sculpt_target(sphere(harness))
        .and_then(|t| t.sculpt)
}

fn run(harness: &mut H, command: CommandId) {
    let mut requests = Vec::new();
    let ctx = harness.ctx.clone();
    harness
        .state_mut()
        .workbench
        .run_command(command, &ctx, &mut requests);
    harness.run();
}

#[test]
fn a_painted_mask_shows_darker_and_holds_the_shape_still() {
    let mut harness = masking();
    let (_, c) = front(&harness);
    let before = shape(&harness);
    let v = under(&harness, &before, c);
    let steps = harness.state().workbench.document().undo_steps();
    drag(
        &mut harness,
        &[c - egui::vec2(8.0, 0.0), c + egui::vec2(8.0, 0.0)],
    );
    assert!(masked(&harness, v) > 200);
    // One undo step, the shape unchanged, the mask drawn.
    assert_eq!(harness.state().workbench.document().undo_steps(), steps + 1);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Mask")
    );
    assert_eq!(shape(&harness), before);
    let item = &harness.state().workbench.drawing.summary().drawing.items[0];
    assert!(
        item.mask
            .as_ref()
            .is_some_and(|m| m[v.get() as usize] > 200)
    );
    // Draw over it: the masked point stays.
    harness.key_press(egui::Key::Num1);
    harness.run();
    drag(
        &mut harness,
        &[c - egui::vec2(4.0, 0.0), c + egui::vec2(4.0, 0.0)],
    );
    assert_eq!(
        shape(&harness).position(v),
        before.position(v),
        "a masked point moved"
    );
    // Undo the draw and the mask: the mask goes.
    harness.key_press_modifiers(Modifiers::COMMAND, egui::Key::Z);
    harness.key_press_modifiers(Modifiers::COMMAND, egui::Key::Z);
    harness.run();
    assert_eq!(masked(&harness, v), 0);
    let item = &harness.state().workbench.drawing.summary().drawing.items[0];
    assert!(item.mask.is_none());
}

#[test]
fn ctrl_i_inverts_alt_m_clears_and_ctrl_wheel_grows_and_shrinks() {
    let mut harness = masking();
    let (_, c) = front(&harness);
    let before = shape(&harness);
    let v = under(&harness, &before, c);
    let far = under(&harness, &before, c + egui::vec2(60.0, 0.0));
    // Nothing masked: the keys say so and change nothing.
    let start = harness.state().workbench.document().undo_steps();
    harness.key_press_modifiers(Modifiers::COMMAND, egui::Key::I);
    harness.run();
    assert_eq!(harness.state().workbench.document().undo_steps(), start);
    drag(&mut harness, &[c, c + egui::vec2(2.0, 0.0)]);
    let steps = harness.state().workbench.document().undo_steps();
    harness.key_press_modifiers(Modifiers::COMMAND, egui::Key::I);
    harness.run();
    assert!(masked(&harness, v) < 50);
    assert_eq!(masked(&harness, far), 255);
    assert_eq!(harness.state().workbench.document().undo_steps(), steps + 1);
    harness.key_press_modifiers(Modifiers::COMMAND, egui::Key::I);
    harness.run();
    let painted: Vec<u8> = harness
        .state()
        .workbench
        .sculpting
        .live
        .as_ref()
        .unwrap()
        .live
        .mask()
        .values();
    let count = |m: &[u8]| m.iter().filter(|x| **x > 0).count();
    // Ctrl + wheel up grows it a ring, down shrinks it back.
    let wheel = |harness: &mut H, y: f32| {
        harness.event(Event::PointerMoved(c));
        harness.event(Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, y),
            modifiers: Modifiers::COMMAND,
            phase: egui::TouchPhase::Move,
        });
        harness.run();
    };
    harness.event(Event::ModifiersChanged(Modifiers::COMMAND));
    wheel(&mut harness, 1.0);
    let grown = harness
        .state()
        .workbench
        .sculpting
        .live
        .as_ref()
        .unwrap()
        .live
        .mask()
        .values();
    assert!(count(&grown) > count(&painted), "grown");
    wheel(&mut harness, -1.0);
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    let back = harness
        .state()
        .workbench
        .sculpting
        .live
        .as_ref()
        .unwrap()
        .live
        .mask()
        .values();
    assert!(count(&back) < count(&grown), "shrunk");
    harness.key_press_modifiers(Modifiers::ALT, egui::Key::M);
    harness.run();
    assert!(
        !harness
            .state()
            .workbench
            .sculpting
            .live
            .as_ref()
            .unwrap()
            .live
            .has_mask()
    );
    assert!(doc_sculpt(&harness).unwrap().mask().is_zero());
}

#[test]
fn nine_goes_round_the_ways_of_masking_and_a_box_masks_through_the_shape() {
    let mut harness = masking();
    for want in [
        MaskTool::Box,
        MaskTool::Lasso,
        MaskTool::Brush,
        MaskTool::Box,
    ] {
        harness.key_press(egui::Key::Num9);
        harness.run();
        assert_eq!(harness.state().workbench.attributes.sculpt.mask_tool, want);
    }
    let (_, c) = front(&harness);
    let before = shape(&harness);
    let inside = under(&harness, &before, c);
    let outside = under(&harness, &before, c + egui::vec2(60.0, 0.0));
    let steps = harness.state().workbench.document().undo_steps();
    drag(
        &mut harness,
        &[c - egui::vec2(10.0, 10.0), c + egui::vec2(10.0, 10.0)],
    );
    assert_eq!(masked(&harness, inside), 255);
    assert_eq!(masked(&harness, outside), 0);
    // Points on the far side, behind it, are caught too.
    let wb = &harness.state().workbench;
    let live = wb.sculpting.live.as_ref().unwrap();
    let bounds = before.bounds().unwrap();
    let middle = (bounds.min.y + bounds.max.y) / 2.0;
    let behind = before
        .vertices()
        .filter(|(v, p)| p.y > middle + 10.0 && live.live.mask().get(v.get() as usize) == 255)
        .count();
    assert!(behind > 0, "nothing behind was masked");
    assert_eq!(wb.document().undo_steps(), steps + 1);
    assert_eq!(
        wb.document().undo_label(),
        Some(crate::text::tr("Box mask"))
    );
}

#[test]
fn l_mirrors_a_stroke_onto_the_other_side() {
    let mut harness = sculpting_with(StrokeKind::Freehand);
    harness.key_press(egui::Key::L);
    harness.run();
    assert!(harness.state().workbench.attributes.sculpt.symmetry.on);
    let (_, c) = front(&harness);
    let before = shape(&harness);
    let right = c + egui::vec2(30.0, 0.0);
    let left = c - egui::vec2(30.0, 0.0);
    let v = under(&harness, &before, right);
    let p = before.position(v).unwrap();
    // The point at its mirrored place.
    let w = before
        .vertices()
        .min_by(|a, b| {
            a.1.distance(p * Vec3::new(-1.0, 1.0, 1.0))
                .total_cmp(&b.1.distance(p * Vec3::new(-1.0, 1.0, 1.0)))
        })
        .unwrap()
        .0;
    let _ = left;
    let steps = harness.state().workbench.document().undo_steps();
    drag(&mut harness, &[right, right + egui::vec2(0.0, 6.0)]);
    let now = shape(&harness);
    let (a, b) = (
        now.position(v).unwrap() - p,
        now.position(w).unwrap() - before.position(w).unwrap(),
    );
    assert!(a.length() > 0.1);
    assert!(
        (a * Vec3::new(-1.0, 1.0, 1.0) - b).length() < 1e-3,
        "{a} {b}"
    );
    assert_eq!(harness.state().workbench.document().undo_steps(), steps + 1);
    // The brush shows on the surface, with its reflection dashed.
    harness.event(Event::PointerMoved(right));
    harness.run();
    let circles = harness.state().workbench.viewport.sculpt_circles();
    assert_eq!(circles.len(), 2);
    assert!(!circles[0].dashed && circles[1].dashed);
    assert!((circles[0].centre.x + circles[1].centre.x).abs() < 1e-2);
    harness.key_press(egui::Key::L);
    harness.run();
    let circles = harness.state().workbench.viewport.sculpt_circles();
    assert_eq!(circles.len(), 1);
    // Off the shape, the brush is drawn flat on the view.
    harness.event(Event::PointerMoved(Pos2::new(5.0, 300.0)));
    harness.run();
    assert!(
        harness
            .state()
            .workbench
            .viewport
            .sculpt_circles()
            .is_empty()
    );
}

#[test]
fn the_gizmo_moves_the_unmasked_part_only() {
    let mut harness = masking();
    let (_, c) = front(&harness);
    let before = shape(&harness);
    let v = under(&harness, &before, c);
    drag(
        &mut harness,
        &[c - egui::vec2(8.0, 0.0), c + egui::vec2(8.0, 0.0)],
    );
    harness.key_press(egui::Key::W);
    harness.run();
    let gizmo = harness
        .state_mut()
        .workbench
        .sculpt_gizmo()
        .expect("Sculpt mode's gizmo")
        .expect("one with a mask");
    // At the unmasked part's middle: about the sphere's.
    let middle = before.bounds().unwrap();
    assert!(gizmo.origin.distance((middle.min + middle.max) / 2.0) < 10.0);
    let steps = harness.state().workbench.document().undo_steps();
    let wb = &mut harness.state_mut().workbench;
    assert!(wb.sculpt_gizmo_events(&[GizmoEvent::Begin {
        handle: tools::Handle::Axis(2),
        slide: false,
    }]));
    let pointer = wb
        .viewport
        .pointer()
        .unwrap_or(crate::components::ViewPointer {
            camera: tools::Camera::default(),
            size: glam::Vec2::ONE,
            at: glam::Vec2::ZERO,
        });
    wb.sculpt_gizmo_events(&[
        GizmoEvent::Update {
            change: tools::Change::Translate(Vec3::Z * 10.0),
            handle: tools::Handle::Axis(2),
            pointer,
        },
        GizmoEvent::End,
    ]);
    harness.run();
    let now = shape(&harness);
    let top = before
        .vertices()
        .max_by(|a, b| a.1.z.total_cmp(&b.1.z))
        .unwrap()
        .0;
    assert!((now.position(top).unwrap().z - before.position(top).unwrap().z - 10.0).abs() < 1e-3);
    assert_eq!(
        now.position(v),
        before.position(v),
        "the masked point stays"
    );
    assert_eq!(harness.state().workbench.document().undo_steps(), steps + 1);
}

#[test]
fn the_sculpt_menu_makes_symmetrical_and_makes_a_shape_from_the_mask() {
    let mut harness = masking();
    let (_, c) = front(&harness);
    let right = c + egui::vec2(30.0, 0.0);
    // A lump on the right only.
    harness.key_press(egui::Key::Num1);
    harness.run();
    drag(&mut harness, &[right, right + egui::vec2(0.0, 6.0)]);
    let drawn = harness.state().workbench.document().undo_steps();
    run(
        &mut harness,
        CommandId::Sculpt(SculptAction::MakeSymmetricalMinus),
    );
    let steps = harness.state().workbench.document().undo_steps();
    assert_eq!(steps, drawn + 1, "{:?}", harness.state().workbench.note);
    // Already symmetrical: nothing more to do.
    run(
        &mut harness,
        CommandId::Sculpt(SculptAction::MakeSymmetricalMinus),
    );
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
    // Nothing masked yet: no shape, a note.
    run(&mut harness, CommandId::Sculpt(SculptAction::ShapeFromMask));
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
    run(&mut harness, CommandId::Sculpt(SculptAction::MaskFacingUp));
    run(&mut harness, CommandId::Sculpt(SculptAction::ShapeFromMask));
    let wb = &harness.state().workbench;
    let objects = wb.document().scene().walk();
    assert_eq!(objects.len(), 2, "{:?}", wb.note);
    // The commands are in command search, for Sculpt mode only.
    assert!(crate::commands::ALL.contains(&CommandId::Sculpt(SculptAction::InvertMask)));
}
