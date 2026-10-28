//! Brush feel (decision 0184): pen pressure on size and flow, the
//! eraser end, a pen through the system's touch, smoothing and Shift +
//! click lines.

use super::brushing_tests::{painted_pixels, stroke};
use super::paint_layers_tests::painting_the_cube;
use super::tests::camera;
use crate::PenState;
use egui::{Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;
use glam::Vec3;

type H = egui_kittest::Harness<'static, super::tests::State>;

fn on_screen(harness: &H, world: Vec3) -> Pos2 {
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = glam::Vec2::new(rect.width(), rect.height());
    let at = camera(harness).project(world, size).unwrap();
    rect.min + egui::vec2(at.x, at.y)
}

fn pen(pressure: f32) -> Option<PenState> {
    Some(PenState {
        pressure,
        tilt: None,
        eraser: false,
    })
}

/// A dab at the front's middle with the tablet pen at `pressure` (or a
/// mouse), and how many texels it painted.
fn dab_with(pressure: Option<f32>) -> usize {
    let mut harness = painting_the_cube();
    harness
        .state_mut()
        .workbench
        .set_tablet_pen(pressure.and_then(pen));
    let at = on_screen(&harness, Vec3::new(10.0, -50.0, 60.0));
    stroke(
        &mut harness,
        at,
        at,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    painted_pixels(&harness)
}

#[test]
fn pressing_harder_paints_a_bigger_dab_and_a_mouse_paints_fully() {
    let light = dab_with(Some(0.2));
    let hard = dab_with(Some(1.0));
    let mouse = dab_with(None);
    assert!(light * 3 < hard, "{light} against {hard}");
    assert!((mouse as f32 - hard as f32).abs() < hard as f32 * 0.05);
}

#[test]
fn pressure_on_flow_and_strengths_are_kept() {
    let mut harness = painting_the_cube();
    {
        let brush = &mut harness.state_mut().workbench.attributes.brush;
        brush.size_pressure.on = false;
        brush.flow_pressure.on = true;
        brush.flow_pressure.least = 0.0;
    }
    harness.state_mut().workbench.set_tablet_pen(pen(0.5));
    let at = on_screen(&harness, Vec3::new(10.0, -50.0, 60.0));
    stroke(
        &mut harness,
        at,
        at,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    let doc = harness.state().workbench.document();
    let set = doc.scene().paint().set("Default").unwrap();
    let dab = set.layers[0].strokes[0].dabs[0];
    assert!((dab.strength - 0.5).abs() < 1e-4, "{}", dab.strength);
    let image = set.layers[0].image().unwrap();
    let most = (0..image.size())
        .flat_map(|y| (0..image.size()).map(move |x| (x, y)))
        .map(|(x, y)| image.pixel(x, y)[3])
        .max()
        .unwrap();
    assert!((100..=160).contains(&most), "half strength: {most}");
}

#[test]
fn the_pens_eraser_end_erases() {
    let mut harness = painting_the_cube();
    let at = on_screen(&harness, Vec3::new(10.0, -50.0, 60.0));
    stroke(
        &mut harness,
        at,
        at,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    let painted = painted_pixels(&harness);
    harness.state_mut().workbench.set_tablet_pen(Some(PenState {
        pressure: 1.0,
        tilt: None,
        eraser: true,
    }));
    stroke(
        &mut harness,
        at,
        at,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    assert!(painted_pixels(&harness) < painted / 2);
    // The brush itself is still the paint brush.
    assert_eq!(
        harness.state().workbench.attributes.brush.brush.kind,
        paint::BrushKind::Paint
    );
}

#[test]
fn a_pen_through_touch_gives_its_pressure() {
    let mut harness = painting_the_cube();
    let at = on_screen(&harness, Vec3::new(10.0, -50.0, 60.0));
    harness.event(egui::Event::Touch {
        device_id: egui::TouchDeviceId(1),
        id: egui::TouchId(1),
        phase: egui::TouchPhase::Move,
        pos: at,
        force: Some(0.3),
    });
    harness.run();
    assert_eq!(
        harness.state().workbench.pen().map(|p| p.pressure),
        Some(0.3)
    );
    harness.event(egui::Event::Touch {
        device_id: egui::TouchDeviceId(1),
        id: egui::TouchId(1),
        phase: egui::TouchPhase::End,
        pos: at,
        force: None,
    });
    harness.run();
    assert!(harness.state().workbench.pen().is_none());
}

#[test]
fn d_turns_smoothing_on_and_the_brush_lags_behind() {
    let mut harness = painting_the_cube();
    harness.key_press(egui::Key::D);
    harness.run();
    assert!(harness.state().workbench.attributes.brush.smoothing_on);
    harness.state_mut().workbench.attributes.brush.smoothing = 1.0;
    let from = on_screen(&harness, Vec3::new(-30.0, -50.0, 50.0));
    let to = on_screen(&harness, Vec3::new(30.0, -50.0, 50.0));
    stroke(
        &mut harness,
        from,
        to,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    let doc = harness.state().workbench.document();
    let dabs = &doc.scene().paint().set("Default").unwrap().layers[0].strokes[0].dabs;
    let furthest = dabs.iter().map(|d| d.at.x).fold(f32::MIN, f32::max);
    // The string held the brush back from the stroke's end.
    assert!(furthest < 25.0, "{furthest}");
    harness.key_press(egui::Key::D);
    harness.run();
    assert!(!harness.state().workbench.attributes.brush.smoothing_on);
}

#[test]
fn shift_click_paints_a_line_from_the_last_stroke() {
    let mut harness = painting_the_cube();
    let a = on_screen(&harness, Vec3::new(-30.0, -50.0, 50.0));
    let b = on_screen(&harness, Vec3::new(30.0, -50.0, 50.0));
    stroke(&mut harness, a, a, Modifiers::NONE, PointerButton::Primary);
    stroke(&mut harness, b, b, Modifiers::SHIFT, PointerButton::Primary);
    let doc = harness.state().workbench.document();
    let set = doc.scene().paint().set("Default").unwrap();
    let line = &set.layers[0].strokes[1];
    assert!(line.dabs.len() > 5, "dabs all along: {}", line.dabs.len());
    let xs: Vec<f32> = line.dabs.iter().map(|d| d.at.x).collect();
    assert!(xs.iter().any(|x| *x < -20.0) && xs.iter().any(|x| *x > 20.0));
}
