//! Colour (decision 0183): the colour picker in Attributes, picking with
//! P and with Pick, and the project's palettes.

use super::brushing_tests::stroke;
use super::paint_layers_tests::{painting_the_cube, press};
use super::tests::camera;
use egui::{Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;
use glam::Vec3;
use std::path::Path;

type H = egui_kittest::Harness<'static, super::tests::State>;

fn colour(harness: &H) -> [u8; 3] {
    let c = harness.state().workbench.attributes.brush.brush.color;
    [c[0], c[1], c[2]]
}

fn set_colour(harness: &mut H, c: [u8; 3]) {
    harness.state_mut().workbench.attributes.brush.brush.color = [c[0], c[1], c[2], 255];
    harness.run();
}

fn on_screen(harness: &H, world: Vec3) -> Pos2 {
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = glam::Vec2::new(rect.width(), rect.height());
    let at = camera(harness).project(world, size).unwrap();
    rect.min + egui::vec2(at.x, at.y)
}

fn fill_front(harness: &mut H) {
    harness.key_press(egui::Key::Num4);
    harness.run();
    let at = on_screen(harness, Vec3::new(0.0, -50.0, 50.0));
    stroke(harness, at, at, Modifiers::NONE, PointerButton::Primary);
    harness.key_press(egui::Key::Num1);
    harness.run();
}

#[test]
fn the_picker_shows_and_typing_hex_sets_the_colour() {
    let mut harness = painting_the_cube();
    assert!(
        harness
            .query_by_label(crate::attributes::colour_picker::WHEEL_LABEL)
            .is_some()
    );
    let hex = harness.get_by_label("Colour as hex");
    hex.scroll_to_me();
    harness.run();
    harness.get_by_label("Colour as hex").click();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.get_by_label("Colour as hex").type_text("#3366CC");
    harness.run();
    harness.run();
    assert_eq!(colour(&harness), [0x33, 0x66, 0xCC]);
}

#[test]
fn p_picks_the_colour_shown_under_the_mouse() {
    let mut harness = painting_the_cube();
    set_colour(&mut harness, [200, 40, 30]);
    fill_front(&mut harness);
    set_colour(&mut harness, [0, 0, 0]);
    let at = on_screen(&harness, Vec3::new(5.0, -50.0, 45.0));
    harness.event(egui::Event::PointerMoved(at));
    harness.run();
    harness.key_press(egui::Key::P);
    harness.run();
    assert_eq!(colour(&harness), [200, 40, 30]);
    // An unpainted side gives the material's own colour.
    let side = on_screen(&harness, Vec3::new(50.0, 0.0, 50.0));
    harness.event(egui::Event::PointerMoved(side));
    harness.run();
    harness.key_press(egui::Key::P);
    harness.run();
    assert_ne!(colour(&harness), [200, 40, 30]);
    // P in Paint mode doesn't switch the view's projection.
    assert!(!camera(&harness).is_flat());
}

#[test]
fn pick_waits_for_a_click_which_paints_nothing() {
    let mut harness = painting_the_cube();
    set_colour(&mut harness, [10, 180, 60]);
    fill_front(&mut harness);
    set_colour(&mut harness, [255, 255, 255]);
    // The mouse comes over from the view first, as a hand would.
    harness.get_by_label("Pick (P)").scroll_to_me();
    harness.run();
    let button = harness.get_by_label("Pick (P)").rect().center();
    harness.event(egui::Event::PointerMoved(button));
    harness.run();
    press(&mut harness, "Pick (P)");
    assert!(harness.state().workbench.attributes.picking);
    let strokes = |h: &H| {
        h.state()
            .workbench
            .document()
            .scene()
            .paint()
            .set("Default")
            .unwrap()
            .layers[0]
            .strokes
            .len()
    };
    let before = strokes(&harness);
    let at = on_screen(&harness, Vec3::new(0.0, -50.0, 50.0));
    stroke(
        &mut harness,
        at,
        at + egui::vec2(20.0, 0.0),
        Modifiers::NONE,
        PointerButton::Primary,
    );
    assert_eq!(colour(&harness), [10, 180, 60]);
    assert!(!harness.state().workbench.attributes.picking);
    assert_eq!(strokes(&harness), before, "the click only picked");
}

#[test]
fn palettes_keep_colours_with_the_project() {
    let mut harness = painting_the_cube();
    set_colour(&mut harness, [1, 2, 3]);
    press(&mut harness, "+");
    let palettes = |h: &H| {
        h.state()
            .workbench
            .document()
            .scene()
            .paint()
            .palettes()
            .to_vec()
    };
    assert_eq!(palettes(&harness).len(), 1);
    assert_eq!(palettes(&harness)[0].colours, [[1, 2, 3]]);
    // A swatch click uses its colour.
    set_colour(&mut harness, [9, 9, 9]);
    press(&mut harness, "#010203");
    assert_eq!(colour(&harness), [1, 2, 3]);
    // One undo step each.
    harness.state_mut().workbench.document_mut().undo().unwrap();
    assert!(palettes(&harness).is_empty());
}

#[test]
fn palette_files_are_read_and_written() {
    let mut harness = painting_the_cube();
    let text = "GIMP Palette\nName: Sky\n10 20 30\n";
    harness
        .state_mut()
        .workbench
        .import_palette(Path::new("/tmp/sky.gpl"), text);
    harness.run();
    let bench = &harness.state().workbench;
    assert_eq!(bench.document().scene().paint().palettes()[0].name, "Sky");
    let (name, written) = bench.palette_file(0).unwrap();
    assert_eq!(name, "Sky");
    assert!(written.contains("10  20  30"));
    // Not a palette: refused with a note, nothing added.
    harness
        .state_mut()
        .workbench
        .import_palette(Path::new("/tmp/x.gpl"), "hello");
    assert_eq!(
        harness
            .state()
            .workbench
            .document()
            .scene()
            .paint()
            .palettes()
            .len(),
        1
    );
}
