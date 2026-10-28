//! Painting with the brush in Paint mode (decision 0179): strokes on the
//! model, one undo step each, the brush's keys and its Ctrl + drags.

use super::tests::{State, camera, fresh};
use crate::palette::Mode;
use crate::viewport::Surface;
use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;
use glam::Vec3;

type H = egui_kittest::Harness<'static, State>;

/// Where `world` shows in the viewport, in window points.
fn on_screen(harness: &H, world: Vec3) -> Pos2 {
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = glam::Vec2::new(rect.width(), rect.height());
    let at = camera(harness).project(world, size).unwrap();
    rect.min + egui::vec2(at.x, at.y)
}

pub(super) fn painting() -> H {
    let mut harness = fresh();
    harness.state_mut().workbench.mode = Mode::Paint;
    harness.run();
    harness
}

/// Presses the left button at `from`, moves to `to` in steps, lets go.
pub(super) fn stroke(
    harness: &mut H,
    from: Pos2,
    to: Pos2,
    modifiers: Modifiers,
    button: PointerButton,
) {
    harness.event(Event::ModifiersChanged(modifiers));
    harness.event(Event::PointerMoved(from));
    harness.step();
    harness.event(Event::PointerButton {
        pos: from,
        button,
        pressed: true,
        modifiers,
    });
    harness.step();
    for i in 1..=6 {
        harness.event(Event::PointerMoved(from + (to - from) * (i as f32 / 6.0)));
        harness.step();
    }
    harness.event(Event::PointerButton {
        pos: to,
        button,
        pressed: false,
        modifiers,
    });
    harness.step();
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    harness.run();
}

pub(super) fn painted_pixels(harness: &H) -> usize {
    let doc = harness.state().workbench.document();
    let Some(set) = doc.scene().paint().set("Default") else {
        return 0;
    };
    let image = set.layers.last().and_then(|l| l.image()).unwrap();
    let mut count = 0;
    for y in 0..image.size() {
        for x in 0..image.size() {
            if image.pixel(x, y)[3] > 0 {
                count += 1;
            }
        }
    }
    count
}

#[test]
fn paint_mode_shows_the_painted_surface_and_the_brush() {
    let mut harness = painting();
    assert_eq!(
        harness.state().workbench.viewport.options().surface,
        Surface::Painted
    );
    assert!(harness.query_by_label("Brush flow").is_some());
    // Back to the Model mode: the surface it had.
    harness.state_mut().workbench.mode = Mode::Model;
    harness.run();
    assert_eq!(
        harness.state().workbench.viewport.options().surface,
        Surface::Plain
    );
    assert!(harness.query_by_label("Brush flow").is_none());
}

#[test]
fn a_stroke_on_the_cube_paints_its_texture_in_one_undo_step() {
    let mut harness = painting();
    let front = on_screen(&harness, Vec3::new(-20.0, -50.0, 50.0));
    let across = on_screen(&harness, Vec3::new(20.0, -50.0, 60.0));
    stroke(
        &mut harness,
        front,
        across,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    let doc = harness.state().workbench.document();
    let set = doc
        .scene()
        .paint()
        .set("Default")
        .expect("a texture set made");
    assert_eq!(set.layers.len(), 1);
    assert_eq!(set.layers[0].name, "Paint");
    let painted = painted_pixels(&harness);
    assert!(painted > 50, "{painted}");
    // The stroke is kept with the layer, its dabs on the cube's front.
    let strokes = &set.layers[0].strokes;
    assert_eq!(strokes.len(), 1);
    assert!(strokes[0].dabs.len() >= 2);
    assert!(strokes[0].dabs.iter().all(|d| (d.at.y + 50.0).abs() < 0.5));
    // One stroke, one step.
    harness.state_mut().workbench.document_mut().undo().unwrap();
    assert!(
        harness
            .state()
            .workbench
            .document()
            .scene()
            .paint()
            .is_empty()
    );
}

#[test]
fn the_eraser_takes_the_paint_away() {
    let mut harness = painting();
    let front = on_screen(&harness, Vec3::new(-20.0, -50.0, 50.0));
    let across = on_screen(&harness, Vec3::new(20.0, -50.0, 50.0));
    stroke(
        &mut harness,
        front,
        across,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    let before = painted_pixels(&harness);
    harness.key_press(egui::Key::Num2);
    harness.run();
    assert_eq!(
        harness.state().workbench.attributes.brush.brush.kind,
        paint::BrushKind::Erase
    );
    stroke(
        &mut harness,
        front,
        across,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    assert!(painted_pixels(&harness) < before / 2);
    // 1 paints again; the mode keys don't switch modes in Paint mode.
    harness.key_press(egui::Key::Num1);
    harness.run();
    assert_eq!(harness.state().workbench.mode, Mode::Paint);
}

#[test]
fn clicking_beside_the_shapes_paints_nothing() {
    let mut harness = painting();
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let corner = rect.min + egui::vec2(20.0, 40.0);
    stroke(
        &mut harness,
        corner,
        corner + egui::vec2(10.0, 0.0),
        Modifiers::NONE,
        PointerButton::Primary,
    );
    assert!(
        harness
            .state()
            .workbench
            .document()
            .scene()
            .paint()
            .is_empty()
    );
}

#[test]
fn brackets_and_ctrl_drags_change_the_brush() {
    let mut harness = painting();
    let radius = |h: &H| h.state().workbench.attributes.brush.brush.radius;
    let before = radius(&harness);
    harness.key_press(egui::Key::CloseBracket);
    harness.run();
    assert!(radius(&harness) > before);
    harness.key_press(egui::Key::OpenBracket);
    harness.key_press(egui::Key::OpenBracket);
    harness.run();
    assert!(radius(&harness) < before);
    // Ctrl + right drag to the right: bigger, and nothing painted or
    // orbited.
    let camera_before = camera(&harness);
    let at = on_screen(&harness, Vec3::new(0.0, -50.0, 50.0));
    let small = radius(&harness);
    stroke(
        &mut harness,
        at,
        at + egui::vec2(80.0, 0.0),
        Modifiers::COMMAND,
        PointerButton::Secondary,
    );
    assert!(radius(&harness) > small);
    assert_eq!(camera(&harness), camera_before, "no orbit");
    assert!(
        harness
            .state()
            .workbench
            .document()
            .scene()
            .paint()
            .is_empty()
    );
    // Ctrl + left drag to the left: less flow.
    let flow = harness.state().workbench.attributes.brush.brush.flow;
    stroke(
        &mut harness,
        at,
        at - egui::vec2(60.0, 0.0),
        Modifiers::COMMAND,
        PointerButton::Primary,
    );
    assert!(harness.state().workbench.attributes.brush.brush.flow < flow);
    assert!(
        harness
            .state()
            .workbench
            .document()
            .scene()
            .paint()
            .is_empty()
    );
    // Esc leaves Paint mode.
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert_eq!(harness.state().workbench.mode, Mode::Model);
}

#[test]
fn a_brush_measured_on_the_surface_paints_its_size_in_centimetres() {
    let mut harness = painting();
    harness.state_mut().workbench.attributes.brush.on_surface = true;
    harness
        .state_mut()
        .workbench
        .attributes
        .brush
        .surface_radius = 10.0;
    let at = on_screen(&harness, Vec3::new(0.0, -50.0, 50.0));
    stroke(
        &mut harness,
        at,
        at,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    let painted = painted_pixels(&harness) as f32;
    // A disc 10 cm across in radius on a 100 cm side of a 2048 texture
    // laid out in thirds: a circle of about 68 pixels' radius.
    let size = harness.state().workbench.document().scene().texture().size as f32;
    let side = size / 3.0;
    let expected = std::f32::consts::PI * (side * 10.0 / 100.0).powi(2);
    assert!(
        painted > expected * 0.6 && painted < expected * 1.6,
        "{painted} against {expected}"
    );
}
