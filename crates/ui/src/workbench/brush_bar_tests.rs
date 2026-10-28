//! The brush bar and tips (decision 0185): the bar in the Paint layout,
//! choosing a brush, keeping one, searching, and tips kept with strokes.

use super::brushing_tests::stroke;
use super::paint_layers_tests::{painting_the_cube, press};
use super::tests::camera;
use crate::brush_bar::BrushPreset;
use crate::panel::Panel;
use egui::{Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;
use glam::Vec3;
use paint::Tip;

type H = egui_kittest::Harness<'static, super::tests::State>;

fn on_screen(harness: &H, world: Vec3) -> Pos2 {
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = glam::Vec2::new(rect.width(), rect.height());
    let at = camera(harness).project(world, size).unwrap();
    rect.min + egui::vec2(at.x, at.y)
}

#[test]
fn the_bar_shows_brushes_and_choosing_one_sets_the_tip() {
    let mut harness = painting_the_cube();
    let bench = &harness.state().workbench;
    assert!(bench.dock.find_tab(&Panel::Brushes).is_some());
    assert!(
        harness
            .query_by_label(crate::brush_bar::BRUSH_ROW_LABEL)
            .is_some()
    );
    let size = harness.state().workbench.attributes.brush.brush.radius;
    press(&mut harness, "Brush Chalk");
    let brush = harness.state().workbench.attributes.brush.brush;
    assert_eq!(brush.tip, Tip::Chalk);
    assert_eq!(brush.radius, size, "the size is kept");
    // A stroke keeps its tip.
    let a = on_screen(&harness, Vec3::new(-20.0, -50.0, 50.0));
    let b = on_screen(&harness, Vec3::new(20.0, -50.0, 50.0));
    stroke(&mut harness, a, b, Modifiers::NONE, PointerButton::Primary);
    let doc = harness.state().workbench.document();
    let set = doc.scene().paint().set("Default").unwrap();
    assert_eq!(set.layers[0].strokes[0].tip, Tip::Chalk);
}

#[test]
fn a_brush_is_kept_under_a_name_and_found_by_searching() {
    let mut harness = painting_the_cube();
    press(&mut harness, "Brush Bristle");
    harness
        .state_mut()
        .workbench
        .attributes
        .brush
        .brush
        .hardness = 0.33;
    press(&mut harness, crate::brush_bar::KEEP_LABEL);
    let field = harness.get_by_label("Name of the brush to keep");
    field.click();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness
        .get_by_label("Name of the brush to keep")
        .type_text("Wood grain");
    harness.run();
    press(&mut harness, "Keep");
    let saved = harness.state().workbench.saved_brushes().to_vec();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].name, "Wood grain");
    assert_eq!(saved[0].tip(), Tip::Bristle);
    assert!((saved[0].hardness - 0.33).abs() < 1e-4);
    // Search: only matching brushes are shown.
    harness.get_by_label("Search brushes").click();
    harness.run();
    harness.get_by_label("Search brushes").type_text("wood");
    harness.run();
    harness.run();
    assert!(harness.query_by_label("Brush Wood grain").is_some());
    assert!(harness.query_by_label("Brush Chalk").is_none());
    // Choosing it brings its settings back.
    harness.state_mut().workbench.attributes.brush.brush.tip = Tip::Round;
    press(&mut harness, "Brush Wood grain");
    assert_eq!(
        harness.state().workbench.attributes.brush.brush.tip,
        Tip::Bristle
    );
}

#[test]
fn saved_brushes_out_of_range_are_left_out() {
    let good = BrushPreset {
        name: "Mine".into(),
        tip: Tip::Spatter.index(),
        hardness: 0.5,
        spacing: 0.2,
        flow: 0.7,
        size_pressure: Some(0.2),
        flow_pressure: None,
        kind: 0,
    };
    let bad = BrushPreset {
        tip: 99,
        ..good.clone()
    };
    let nan = BrushPreset {
        flow: f32::NAN,
        ..good.clone()
    };
    let bench = super::Workbench::new(super::tests::info(), None).with_saved_brushes(vec![
        good.clone(),
        bad,
        nan,
    ]);
    assert_eq!(bench.saved_brushes(), [good]);
}

#[test]
fn the_keys_window_lists_paint_modes_keys() {
    let bindings = super::shortcuts::bindings();
    let painting = |key: egui::Key| {
        bindings.iter().any(|b| {
            b.situation == super::shortcuts::Situation::Painting
                && b.key == crate::keyboard::KeyId::Key(key)
        })
    };
    for key in [
        egui::Key::Num1,
        egui::Key::Num2,
        egui::Key::Num4,
        egui::Key::P,
        egui::Key::D,
    ] {
        assert!(painting(key), "{key:?}");
    }
}
