//! Mirror painting (decision 0187): L turns it on, each dab is painted on
//! the other side too (even the side the camera can't see), smudge and
//! clone stay on their side, and in the UV Editor dabs flip across the
//! texture's middle.

use super::brushing_tests::{painted_pixels, stroke};
use super::paint_layers_tests::painting_the_cube;
use super::tests::camera;
use super::uv_brushing_tests::{cube_surface, on_editor, painting_in_the_editor};
use crate::attributes::MirrorAxis;
use egui::{Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;
use glam::{Vec2, Vec3};

type H = egui_kittest::Harness<'static, super::tests::State>;

fn on_screen(harness: &H, world: Vec3) -> Pos2 {
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = glam::Vec2::new(rect.width(), rect.height());
    let at = camera(harness).project(world, size).unwrap();
    rect.min + egui::vec2(at.x, at.y)
}

/// Where the last stroke's dabs are, in the cube's own space.
fn dabs(harness: &H) -> Vec<Vec3> {
    let doc = harness.state().workbench.document();
    let set = doc.scene().paint().set("Default").unwrap();
    let stroke = set.layers[0].strokes.last().unwrap();
    stroke.dabs.iter().map(|d| d.at).collect()
}

/// A short stroke on the left of the cube's front.
fn paint_left(harness: &mut H) {
    harness.state_mut().workbench.attributes.brush.brush.radius = 12.0;
    let a = on_screen(harness, Vec3::new(-30.0, -50.0, 25.0));
    let b = on_screen(harness, Vec3::new(-20.0, -50.0, 25.0));
    stroke(harness, a, b, Modifiers::NONE, PointerButton::Primary);
}

#[test]
fn l_turns_mirror_painting_on_and_off() {
    let mut harness = painting_the_cube();
    assert!(!harness.state().workbench.attributes.brush.mirror_on);
    harness.key_press(egui::Key::L);
    harness.run();
    assert!(harness.state().workbench.attributes.brush.mirror_on);
    assert!(harness.state().workbench.note.is_some());
    harness.key_press(egui::Key::L);
    harness.run();
    assert!(!harness.state().workbench.attributes.brush.mirror_on);
}

#[test]
fn a_mirrored_stroke_paints_both_sides_of_the_x_axis() {
    // Without mirror: one side.
    let mut alone = painting_the_cube();
    paint_left(&mut alone);
    let one_side = painted_pixels(&alone);
    assert!(dabs(&alone).iter().all(|d| d.x < 0.0));

    let mut harness = painting_the_cube();
    harness.key_press(egui::Key::L);
    harness.run();
    paint_left(&mut harness);
    let at = dabs(&harness);
    let left = at.iter().filter(|d| d.x < -10.0).count();
    let right: Vec<_> = at.iter().filter(|d| d.x > 10.0).collect();
    assert!(left > 0 && left == right.len(), "{at:?}");
    // The reflection of each dab, on the same face.
    for d in right {
        assert!((d.y + 50.0).abs() < 0.5, "{d}");
        assert!(
            at.iter()
                .any(|l| (l.x + d.x).abs() < 0.5 && (l.z - d.z).abs() < 0.5)
        );
    }
    let both = painted_pixels(&harness);
    assert!(
        both as f32 > one_side as f32 * 1.7,
        "{one_side} on one side, {both} with mirror"
    );
    // One stroke, one undo step.
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
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
fn the_mirror_paints_the_side_the_camera_cannot_see() {
    let mut harness = painting_the_cube();
    harness.state_mut().workbench.attributes.brush.mirror_axis = MirrorAxis::Y;
    harness.key_press(egui::Key::L);
    harness.run();
    paint_left(&mut harness);
    // The front's reflection across Y is the back, facing away.
    let at = dabs(&harness);
    assert!(at.iter().any(|d| (d.y - 50.0).abs() < 0.5), "{at:?}");
    assert!(at.iter().any(|d| (d.y + 50.0).abs() < 0.5), "{at:?}");
}

#[test]
fn smudge_and_clone_stay_on_their_side() {
    let mut harness = painting_the_cube();
    paint_left(&mut harness);
    harness.key_press(egui::Key::L);
    harness.key_press(egui::Key::Num5);
    harness.run();
    paint_left(&mut harness);
    assert!(dabs(&harness).iter().all(|d| d.x < 0.0));
}

#[test]
fn the_polygon_fill_fills_the_mirrored_polygon_too() {
    let mut harness = painting_the_cube();
    harness.state_mut().workbench.attributes.brush.mirror_axis = MirrorAxis::Y;
    harness.key_press(egui::Key::L);
    harness.key_press(egui::Key::Num4);
    harness.run();
    let at = on_screen(&harness, Vec3::new(-25.0, -50.0, 25.0));
    stroke(
        &mut harness,
        at,
        at,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    let doc = harness.state().workbench.document();
    let set = doc.scene().paint().set("Default").unwrap();
    let fills = &set.layers[0].strokes.last().unwrap().fills;
    // The front and the back: different faces.
    assert_eq!(fills.len(), 2, "{fills:?}");
    assert_ne!(fills[0].face, fills[1].face);
}

#[test]
fn in_the_uv_editor_dabs_flip_across_the_middle() {
    let mut harness = painting_in_the_editor();
    harness.key_press(egui::Key::L);
    harness.run();
    let surface = cube_surface(&harness);
    let t = surface.triangles()[0];
    let inside = (t.uvs[0] + t.uvs[1] + t.uvs[2]) / 3.0;
    let at = on_editor(&harness, inside);
    stroke(
        &mut harness,
        at,
        at,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    let doc = harness.state().workbench.document();
    let set = doc.scene().paint().set("Default").unwrap();
    let uvs: Vec<Vec2> = set.layers[0].strokes[0]
        .dabs
        .iter()
        .filter_map(|d| d.flat_uv())
        .collect();
    let flipped = Vec2::new(1.0 - inside.x, inside.y);
    assert!(uvs.iter().any(|uv| uv.distance(inside) < 0.02), "{uvs:?}");
    assert!(uvs.iter().any(|uv| uv.distance(flipped) < 0.02), "{uvs:?}");
}

#[test]
fn the_mirror_switch_shows_on_the_brush_bar() {
    let harness = painting_the_cube();
    assert!(
        harness
            .query_by_label(crate::paint_bar::MIRROR_SWITCH)
            .is_some()
    );
}
