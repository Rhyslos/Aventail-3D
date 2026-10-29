//! The brush shelf's Colours tab (decision 0213): colours used lately,
//! kept by themselves, and the palettes, a click to paint with a colour
//! and a double-click to open the picker.

use super::brushing_tests::stroke;
use super::paint_layers_tests::{painting_the_cube, press};
use super::tests::camera;
use crate::colour_shelf::{BRUSHES_TAB, COLOURS_TAB};
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

fn set_colour(harness: &mut H, c: [u8; 3]) {
    harness.state_mut().workbench.attributes.brush.brush.color = [c[0], c[1], c[2], 255];
    harness.run();
}

fn colour(harness: &H) -> [u8; 3] {
    let c = harness.state().workbench.attributes.brush.brush.color;
    [c[0], c[1], c[2]]
}

fn paint_a_stroke(harness: &mut H) {
    let a = on_screen(harness, Vec3::new(-20.0, -50.0, 50.0));
    let b = on_screen(harness, Vec3::new(20.0, -50.0, 50.0));
    stroke(harness, a, b, Modifiers::NONE, PointerButton::Primary);
}

#[test]
fn colours_painted_with_are_kept_newest_first() {
    let mut harness = painting_the_cube();
    assert!(harness.state().workbench.used_colours().is_empty());
    set_colour(&mut harness, [200, 10, 10]);
    paint_a_stroke(&mut harness);
    set_colour(&mut harness, [10, 200, 10]);
    paint_a_stroke(&mut harness);
    assert_eq!(
        harness.state().workbench.used_colours(),
        [[10, 200, 10], [200, 10, 10]]
    );
    // The eraser uses no colour.
    harness.key_press(egui::Key::Num2);
    harness.run();
    set_colour(&mut harness, [1, 1, 1]);
    paint_a_stroke(&mut harness);
    assert_eq!(harness.state().workbench.used_colours().len(), 2);
}

#[test]
fn a_click_paints_with_a_used_colour_and_a_double_click_opens_the_picker() {
    let mut harness = painting_the_cube();
    set_colour(&mut harness, [200, 10, 10]);
    paint_a_stroke(&mut harness);
    set_colour(&mut harness, [0, 0, 0]);
    press(&mut harness, COLOURS_TAB);
    let used = "Used #C80A0A";
    press(&mut harness, used);
    assert_eq!(colour(&harness), [200, 10, 10]);
    let wheel = crate::attributes::colour_picker::WHEEL_LABEL;
    assert!(harness.query_by_label(wheel).is_none());
    // A double-click in the shelf asks the brush bar to open its picker
    // (the shelf's own test checks the double-click itself).
    harness.state_mut().workbench.attributes.open_colour_picker = true;
    harness.run();
    harness.run();
    assert!(harness.query_by_label(wheel).is_some(), "the picker opened");
}

#[test]
fn the_shelf_switches_between_brushes_and_colours() {
    let mut harness = painting_the_cube();
    assert!(
        harness
            .query_by_label(crate::brush_bar::BRUSH_ROW_LABEL)
            .is_some()
    );
    press(&mut harness, COLOURS_TAB);
    assert!(
        harness
            .query_by_label(crate::brush_bar::BRUSH_ROW_LABEL)
            .is_none()
    );
    assert!(
        harness
            .query_by_label(crate::colour_shelf::ADD_LABEL)
            .is_some()
    );
    press(&mut harness, BRUSHES_TAB);
    assert!(
        harness
            .query_by_label(crate::brush_bar::BRUSH_ROW_LABEL)
            .is_some()
    );
}

#[test]
fn used_colours_are_kept_between_sessions() {
    let bench =
        super::Workbench::new(super::tests::info(), None).with_used_colours(vec![[1, 2, 3]; 40]);
    assert_eq!(bench.used_colours().len(), crate::colour_shelf::USED_KEPT);
}
