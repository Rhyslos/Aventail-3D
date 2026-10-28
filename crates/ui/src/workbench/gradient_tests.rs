//! Gradient fill layers (decision 0194): + Gradient makes one on the
//! shape in one undo step, its settings make it again, strokes go on a
//! layer above it, and it is made again when the UVs move.

use super::brushing_tests::stroke;
use super::paint_layers_tests::painting_the_cube;
use super::tests::camera;
use egui::{Modifiers, PointerButton};
use egui_kittest::kittest::Queryable;
use glam::Vec3;
use std::sync::Arc;

type H = egui_kittest::Harness<'static, super::tests::State>;

fn set(harness: &H) -> Arc<paint::TextureSet> {
    Arc::clone(
        harness
            .state()
            .workbench
            .document()
            .scene()
            .paint()
            .set("Default")
            .expect("a texture set"),
    )
}

/// The colour of the top layer where the cube's surface nearest `at` is.
fn top_at(harness: &H, at: Vec3) -> [u8; 4] {
    let doc = harness.state().workbench.document();
    let id = doc.scene().walk()[0].0;
    let surface = paint::PaintSurface::of(&doc.scene().evaluated(id).unwrap().mesh);
    let (i, w) = surface.closest(at, &[0]).unwrap();
    let t = &surface.triangles()[i];
    let uv = t.uvs[0] * w[0] + t.uvs[1] * w[1] + t.uvs[2] * w[2];
    let set = set(harness);
    let size = set.size as f32;
    let image = set.layers.last().unwrap().image().unwrap().clone();
    image.pixel((uv.x * size) as u32, ((1.0 - uv.y) * size) as u32)
}

/// The cube, painting, on a small texture, with a gradient added.
fn with_a_gradient() -> H {
    let mut harness = painting_the_cube();
    {
        let doc = harness.state_mut().workbench.document_mut();
        let mut texture = doc.scene().texture();
        texture.size = 256;
        doc.execute(scene::Command::SetTexture(texture)).unwrap();
    }
    harness.run();
    harness.get_by_label("+ Gradient").click();
    harness.run();
    harness.run();
    harness
}

#[test]
fn a_gradient_is_made_on_the_shape_in_one_undo_step() {
    let mut harness = with_a_gradient();
    let set = set(&harness);
    let layer = set.layers.last().unwrap();
    assert_eq!(layer.name, "Gradient");
    assert!(layer.gradient.is_some());
    // Its settings include what it runs across (decision 0199).
    assert!(harness.query_by_label("Across").is_some());
    let low = top_at(&harness, Vec3::new(0.0, -50.0, 3.0));
    let high = top_at(&harness, Vec3::new(0.0, -50.0, 97.0));
    assert!(
        low[0] + 100 < high[0],
        "dark low, light high: {low:?} {high:?}"
    );
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Add a gradient layer")
    );
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
fn changing_a_gradient_makes_it_again_and_strokes_go_above_it() {
    let mut harness = with_a_gradient();
    let steps = harness.state().workbench.document().undo_steps();
    // Turned upside down by its settings (as the panel's sliders do).
    let mut changed = (*set(&harness)).clone();
    let layer = changed.layers.last_mut().unwrap();
    let mut g = layer.gradient.unwrap();
    std::mem::swap(&mut g.from, &mut g.to);
    layer.gradient = Some(g);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::SetTextureSet {
            material: "Default".into(),
            set: Some(changed),
        })
        .unwrap();
    harness
        .state_mut()
        .workbench
        .paint_layers
        .gradients_to_make
        .push("Default".into());
    harness.run();
    let low = top_at(&harness, Vec3::new(0.0, -50.0, 3.0));
    let high = top_at(&harness, Vec3::new(0.0, -50.0, 97.0));
    assert!(low[0] > high[0] + 100, "light low now: {low:?} {high:?}");
    assert_eq!(
        harness.state().workbench.document().undo_steps(),
        steps + 1,
        "the change and its making are one step"
    );
    // A stroke with the gradient chosen goes on a new layer above it.
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = glam::Vec2::new(rect.width(), rect.height());
    let at = camera(&harness)
        .project(Vec3::new(0.0, -50.0, 50.0), size)
        .unwrap();
    let at = rect.min + egui::vec2(at.x, at.y);
    let before = set(&harness).layers.last().unwrap().clone();
    stroke(
        &mut harness,
        at,
        at,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    let after = set(&harness);
    assert_eq!(after.layers.len(), 2);
    assert_eq!(after.layers[0], before, "the gradient is untouched");
}
