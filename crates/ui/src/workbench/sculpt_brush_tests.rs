//! Tests for the brushes for forms in Sculpt mode (decision 0306): every
//! brush sculpts in one undo step, Grab and Snake hook follow the mouse in
//! a perspective and a flat view, the tool menu holds the brushes, and the
//! shelf picks them.

use super::brushing_tests::stroke;
use super::heavy_check::{H, on_screen};
use super::sculpt_tests::{front, sculpt_stroke, sculpting, shape};
use super::tests::camera;
use super::*;
use crate::commands::CommandId;
use crate::tool_menu::Ready;
use egui::{Modifiers, PointerButton};
use egui_kittest::kittest::Queryable;
use glam::Vec3;
use sculpt::BrushKind;

fn choose(harness: &mut H, kind: BrushKind) {
    harness.state_mut().workbench.attributes.sculpt.brush = kind;
    harness.run();
}

fn run(harness: &mut H, command: CommandId) {
    let ctx = harness.ctx.clone();
    let mut requests = Vec::new();
    harness
        .state_mut()
        .workbench
        .run_command(command, &ctx, &mut requests);
    harness.state_mut().workbench.carry_out(requests);
    harness.run();
}

#[test]
fn every_brush_sculpts_the_sphere_in_one_undo_step() {
    let mut harness = sculpting();
    // The Mask brush changes no shape (sculpt_mask_tests.rs).
    for kind in BrushKind::ALL.into_iter().filter(|k| *k != BrushKind::Mask) {
        choose(&mut harness, kind);
        let before = shape(&harness);
        let steps = harness.state().workbench.document().undo_steps();
        sculpt_stroke(&mut harness, Modifiers::NONE);
        assert_ne!(shape(&harness), before, "{kind:?} changed nothing");
        let doc = harness.state().workbench.document();
        assert_eq!(doc.undo_steps(), steps + 1, "{kind:?}");
        assert_eq!(doc.undo_label(), Some("Sculpt"), "{kind:?}");
        // Each brush starts from the round sphere again.
        harness.state_mut().workbench.document_mut().undo();
        harness.run();
        assert_eq!(shape(&harness), before, "{kind:?} undone exactly");
    }
}

/// The point of the sphere nearest `world`, before and after a stroke
/// with `kind` from `world`'s place on screen, `by` points along.
fn pull(harness: &mut H, kind: BrushKind, by: egui::Vec2) -> (Vec3, Vec3, f32) {
    choose(harness, kind);
    let (world, at) = front(harness);
    let before = shape(harness);
    let (v, from) = before
        .vertices()
        .min_by(|a, b| a.1.distance(world).total_cmp(&b.1.distance(world)))
        .unwrap();
    let view = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = glam::Vec2::new(view.width(), view.height());
    let per_pixel = camera(harness).world_per_pixel(from, size);
    stroke(
        harness,
        at,
        at + by,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    let now = shape(harness).position(v).unwrap();
    (from, now, per_pixel)
}

/// Grab follows the mouse: the lump under the press ends where the mouse
/// is, along the view's right and up.
fn grab_follows_the_mouse(harness: &mut H) {
    let cam = camera(harness);
    let (from, now, per_pixel) = pull(harness, BrushKind::Grab, egui::vec2(40.0, -30.0));
    let want = from + (cam.right() * 40.0 + cam.up() * 30.0) * per_pixel;
    let moved = want - from;
    assert!(
        now.distance(want) < moved.length() * 0.25,
        "{now} should be near {want} (from {from})"
    );
}

#[test]
fn grab_follows_the_mouse_in_a_perspective_view() {
    let mut harness = sculpting();
    assert!(!camera(&harness).is_flat());
    grab_follows_the_mouse(&mut harness);
}

#[test]
fn grab_follows_the_mouse_in_a_flat_view() {
    let mut harness = sculpting();
    let navigator = *harness.state().workbench.viewport().navigator();
    let mut flat = navigator.looking_from(glam::IVec3::new(0, -1, 0));
    if !flat.camera().is_flat() {
        flat = flat.toggled_projection();
    }
    *harness.state_mut().workbench.viewport_mut().navigator_mut() = flat;
    harness.run();
    assert!(camera(&harness).is_flat());
    grab_follows_the_mouse(&mut harness);
}

#[test]
fn snake_hook_pulls_the_surface_out_after_the_mouse() {
    let mut harness = sculpting();
    let cam = camera(&harness);
    let (from, now, per_pixel) = pull(&mut harness, BrushKind::SnakeHook, egui::vec2(0.0, -60.0));
    let pulled = (now - from).dot(cam.up());
    assert!(pulled > 60.0 * per_pixel * 0.6, "pulled {pulled}");
    // Ctrl doesn't turn a grabbing brush round: the same stroke with it
    // gives the same shape.
    let shapes: Vec<_> = [Modifiers::NONE, Modifiers::COMMAND]
        .into_iter()
        .map(|modifiers| {
            let mut harness = sculpting();
            choose(&mut harness, BrushKind::Grab);
            sculpt_stroke(&mut harness, modifiers);
            shape(&harness)
        })
        .collect();
    assert_eq!(shapes[0], shapes[1]);
}

#[test]
fn the_tool_menu_holds_the_brushes_in_sculpt_mode() {
    let mut harness = sculpting();
    let entries = harness.state().workbench.tool_menu_entries();
    assert_eq!(entries.len(), BrushKind::ALL.len());
    assert!(entries.iter().all(|e| e.ready == Ready::Yes));
    run(&mut harness, CommandId::SculptBrush(BrushKind::ElasticGrab));
    assert_eq!(
        harness.state().workbench.attributes.sculpt.brush,
        BrushKind::ElasticGrab
    );
    // Out of Sculpt mode they explain where they work.
    harness.state_mut().workbench.mode = Mode::Model;
    harness.run();
    let ready = harness
        .state()
        .workbench
        .readiness(CommandId::SculptBrush(BrushKind::Clay));
    assert!(matches!(ready, Ready::NotNow(_)));
}

#[test]
fn the_shelf_shows_every_brush_and_picks_one() {
    let mut harness = sculpting();
    for kind in BrushKind::ALL {
        assert!(
            harness.query_all_by_label(kind.name()).next().is_some(),
            "{kind:?} isn't on the shelf"
        );
    }
    // The shelf's card (the Sculpt tab's button picks it too).
    let card = harness
        .query_all_by_label("Snake hook")
        .find(|n| n.rect().height() > 40.0)
        .expect("a card on the shelf");
    card.scroll_to_me();
    harness.run();
    harness
        .query_all_by_label("Snake hook")
        .find(|n| n.rect().height() > 40.0)
        .unwrap()
        .click();
    harness.run();
    assert_eq!(
        harness.state().workbench.attributes.sculpt.brush,
        BrushKind::SnakeHook
    );
}

#[test]
fn a_grab_off_the_shape_holds_nothing_and_says_so() {
    let mut harness = sculpting();
    choose(&mut harness, BrushKind::Grab);
    let before = shape(&harness);
    let corner = on_screen(&harness, Vec3::new(400.0, 400.0, 0.0));
    stroke(
        &mut harness,
        corner,
        corner + egui::vec2(30.0, 0.0),
        Modifiers::NONE,
        PointerButton::Primary,
    );
    assert_eq!(shape(&harness), before);
    assert!(harness.state().workbench.note.is_some());
}
