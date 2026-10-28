//! The effect brushes, projection and clone (decision 0186): their keys,
//! clone's V + click, projection's picture, and smudge and blur strokes
//! changing what's painted.

use super::brushing_tests::stroke;
use super::paint_layers_tests::{painting_the_cube, press};
use super::tests::camera;
use egui::{Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;
use glam::Vec3;
use paint::BrushKind;

type H = egui_kittest::Harness<'static, super::tests::State>;

fn on_screen(harness: &H, world: Vec3) -> Pos2 {
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = glam::Vec2::new(rect.width(), rect.height());
    let at = camera(harness).project(world, size).unwrap();
    rect.min + egui::vec2(at.x, at.y)
}

fn kind(harness: &H) -> BrushKind {
    harness.state().workbench.attributes.brush.brush.kind
}

fn layer_pixels(harness: &H) -> Vec<[u8; 4]> {
    let doc = harness.state().workbench.document();
    let set = doc.scene().paint().set("Default").unwrap();
    let image = set.layers[0].image().unwrap();
    (0..image.size())
        .flat_map(|y| (0..image.size()).map(move |x| (x, y)))
        .map(|(x, y)| image.pixel(x, y))
        .collect()
}

/// Paints a red half and a blue half across the cube's front.
fn red_and_blue(harness: &mut H) {
    harness.state_mut().workbench.attributes.brush.brush.radius = 12.0;
    harness.state_mut().workbench.attributes.brush.brush.color = [255, 0, 0, 255];
    let a = on_screen(harness, Vec3::new(-30.0, -50.0, 50.0));
    let b = on_screen(harness, Vec3::new(-2.0, -50.0, 50.0));
    stroke(harness, a, b, Modifiers::NONE, PointerButton::Primary);
    harness.state_mut().workbench.attributes.brush.brush.color = [0, 0, 255, 255];
    let a = on_screen(harness, Vec3::new(2.0, -50.0, 50.0));
    let b = on_screen(harness, Vec3::new(30.0, -50.0, 50.0));
    stroke(harness, a, b, Modifiers::NONE, PointerButton::Primary);
}

#[test]
fn keys_three_five_and_six_pick_projection_smudge_and_clone() {
    let mut harness = painting_the_cube();
    for (key, want) in [
        (egui::Key::Num3, BrushKind::Projection),
        (egui::Key::Num5, BrushKind::Smudge),
        (egui::Key::Num6, BrushKind::Clone),
        (egui::Key::Num1, BrushKind::Paint),
    ] {
        harness.key_press(key);
        harness.run();
        assert_eq!(kind(&harness), want);
    }
    // The effect brushes are in the brush bar too.
    press(&mut harness, "Brush Blur");
    assert_eq!(kind(&harness), BrushKind::Blur);
}

#[test]
fn blur_mixes_the_colours_where_they_meet() {
    let mut harness = painting_the_cube();
    red_and_blue(&mut harness);
    let purple = |pixels: &[[u8; 4]]| {
        pixels
            .iter()
            .filter(|p| p[3] > 200 && p[0] > 60 && p[2] > 60)
            .count()
    };
    let before = purple(&layer_pixels(&harness));
    press(&mut harness, "Brush Blur");
    let a = on_screen(&harness, Vec3::new(-20.0, -50.0, 50.0));
    let b = on_screen(&harness, Vec3::new(20.0, -50.0, 50.0));
    for _ in 0..3 {
        stroke(&mut harness, a, b, Modifiers::NONE, PointerButton::Primary);
    }
    let after = purple(&layer_pixels(&harness));
    assert!(after > before + 50, "{before} → {after}");
}

#[test]
fn smudge_drags_red_into_the_blue() {
    let mut harness = painting_the_cube();
    red_and_blue(&mut harness);
    let red = |pixels: &[[u8; 4]]| {
        pixels
            .iter()
            .filter(|p| p[3] > 200 && p[0] > 128 && p[2] < 128)
            .count()
    };
    let before = red(&layer_pixels(&harness));
    harness.key_press(egui::Key::Num5);
    harness.run();
    let a = on_screen(&harness, Vec3::new(-25.0, -50.0, 50.0));
    let b = on_screen(&harness, Vec3::new(30.0, -50.0, 50.0));
    stroke(&mut harness, a, b, Modifiers::NONE, PointerButton::Primary);
    let after = red(&layer_pixels(&harness));
    assert!(after > before + 100, "{before} → {after}");
}

#[test]
fn clone_needs_a_source_set_with_v_and_click() {
    let mut harness = painting_the_cube();
    red_and_blue(&mut harness);
    harness.key_press(egui::Key::Num6);
    harness.run();
    let target = on_screen(&harness, Vec3::new(25.0, -50.0, 25.0));
    stroke(
        &mut harness,
        target,
        target,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    assert!(
        harness.state().workbench.note.is_some(),
        "asks for a source"
    );
    // Hold V and click the red half: the source.
    let red = on_screen(&harness, Vec3::new(-25.0, -50.0, 50.0));
    harness.event(egui::Event::Key {
        key: egui::Key::V,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    stroke(
        &mut harness,
        red,
        red,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    harness.event(egui::Event::Key {
        key: egui::Key::V,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    assert!(harness.state().workbench.attributes.clone_source_set);
    // Painting copies red to where blue was (25 cm right of the source).
    let blue = on_screen(&harness, Vec3::new(25.0, -50.0, 50.0));
    stroke(
        &mut harness,
        blue,
        blue,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    let doc = harness.state().workbench.document();
    let set = doc.scene().paint().set("Default").unwrap();
    assert_eq!(set.layers[0].strokes.last().unwrap().kind, BrushKind::Clone);
}

#[test]
fn projection_paints_the_chosen_picture() {
    let mut harness = painting_the_cube();
    harness.key_press(egui::Key::Num3);
    harness.run();
    let at = on_screen(&harness, Vec3::new(0.0, -50.0, 50.0));
    stroke(
        &mut harness,
        at,
        at,
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
            .is_empty(),
        "no picture yet: nothing painted"
    );
    // A green picture big enough to cover the view.
    let image = fileio::image_file::ReadImage {
        width: 4,
        height: 4,
        rgba: [0, 200, 0, 255].repeat(16),
    };
    harness
        .state_mut()
        .workbench
        .set_stencil("green.png", &image);
    harness.state_mut().workbench.attributes.projection.scale = 400.0;
    harness.run();
    stroke(
        &mut harness,
        at,
        at,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    let green = layer_pixels(&harness)
        .iter()
        .filter(|p| p[3] > 200 && p[1] > 150 && p[0] < 50)
        .count();
    assert!(green > 100, "{green}");
}
