//! Masks and the polygon fill tool (decision 0182): a mask added from
//! the Paint layers panel and painted on, the fill tool filling a
//! polygon, a UV piece or a material on the model and in the UV Editor.

use super::brushing_tests::{painted_pixels, stroke};
use super::paint_layers_tests::{painting_the_cube, press};
use super::tests::camera;
use crate::attributes::PaintTool;
use egui::{Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;
use glam::Vec3;
use paint::{Composite, FillBy, TextureSet};

type H = egui_kittest::Harness<'static, super::tests::State>;

fn set(harness: &H) -> TextureSet {
    harness
        .state()
        .workbench
        .document()
        .scene()
        .paint()
        .set("Default")
        .map(|s| (**s).clone())
        .expect("the material has a texture set")
}

fn on_screen(harness: &H, world: Vec3) -> Pos2 {
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = glam::Vec2::new(rect.width(), rect.height());
    let at = camera(harness).project(world, size).unwrap();
    rect.min + egui::vec2(at.x, at.y)
}

fn click_front(harness: &mut H) {
    let at = on_screen(harness, Vec3::new(0.0, -50.0, 50.0));
    stroke(harness, at, at, Modifiers::NONE, PointerButton::Primary);
}

/// How many texels of the mixed texture show.
fn shown(harness: &H) -> usize {
    let image = Composite::of(&set(harness)).unwrap().image().clone();
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
fn the_fill_tool_fills_a_polygon_a_piece_or_the_material() {
    let mut harness = painting_the_cube();
    harness.key_press(egui::Key::Num4);
    harness.run();
    assert_eq!(
        harness.state().workbench.attributes.brush.tool,
        PaintTool::Fill
    );
    assert!(harness.query_by_label("Polygon fill").is_some());
    click_front(&mut harness);
    let size = harness.state().workbench.document().scene().texture().size as usize;
    let one = painted_pixels(&harness);
    // The cube's front: about a third of the texture each way.
    assert!(one > (size / 5).pow(2) && one < (size / 2).pow(2), "{one}");
    let fills = &set(&harness).layers[0].strokes[0].fills;
    assert_eq!(fills.len(), 1);
    assert_eq!(fills[0].by, FillBy::Polygon);
    // The whole material: every side.
    harness.state_mut().workbench.attributes.brush.fill_by = FillBy::Material;
    click_front(&mut harness);
    assert!(painted_pixels(&harness) >= one * 5);
    // One click, one undo step.
    harness.state_mut().workbench.document_mut().undo().unwrap();
    assert_eq!(painted_pixels(&harness), one);
    // 1 goes back to the brush.
    harness.key_press(egui::Key::Num1);
    harness.run();
    assert_eq!(
        harness.state().workbench.attributes.brush.tool,
        PaintTool::Brush
    );
}

#[test]
fn a_black_mask_hides_the_layer_until_filled_white() {
    let mut harness = painting_the_cube();
    press(&mut harness, "+ Fill");
    let everything = shown(&harness);
    assert!(everything > 0);
    press(&mut harness, "Add black mask");
    assert!(set(&harness).layers[0].mask.is_some());
    assert_eq!(shown(&harness), 0, "a black mask hides all of it");
    // Choose the mask and fill the front polygon white on it.
    harness.get_by_label("Mask of Fill").click();
    harness.run();
    assert!(harness.state().workbench.paint_layers.editing_mask());
    harness.state_mut().workbench.attributes.brush.brush.color = [255, 255, 255, 255];
    harness.key_press(egui::Key::Num4);
    harness.run();
    click_front(&mut harness);
    let front = shown(&harness);
    assert!(
        front > 0 && front < everything / 4,
        "{front} of {everything}"
    );
    // No new layer: the mask was painted.
    assert_eq!(set(&harness).layers.len(), 1);
    // Inverted, the rest shows instead.
    press(&mut harness, "Invert mask");
    assert!(shown(&harness) > everything - front - everything / 50);
    press(&mut harness, "Remove mask");
    assert_eq!(shown(&harness), everything);
}

#[test]
fn the_fill_tool_works_in_the_uv_editor() {
    let mut harness = painting_the_cube();
    harness.key_press(egui::Key::Num4);
    harness.run();
    harness.state_mut().workbench.attributes.brush.fill_by = FillBy::Piece;
    let doc = harness.state().workbench.document();
    let id = crate::uv_editor::shown_objects(doc)[0];
    let surface = paint::PaintSurface::of(&doc.scene().evaluated(id).unwrap().mesh);
    let t = surface.triangles()[0];
    let middle = (t.uvs[0] + t.uvs[1] + t.uvs[2]) / 3.0;
    let at = harness
        .state()
        .workbench
        .uv_editor
        .screen_of(middle)
        .unwrap();
    stroke(
        &mut harness,
        at,
        at,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    let size = harness.state().workbench.document().scene().texture().size as usize;
    let painted = painted_pixels(&harness);
    assert!(
        painted > (size / 5).pow(2) && painted < (size / 2).pow(2),
        "{painted}"
    );
}
