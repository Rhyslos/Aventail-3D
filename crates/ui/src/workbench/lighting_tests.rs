//! Light and shade (decision 0196): + Light and shade makes a layer of
//! warm light and cool shade on the shape in one undo step, its settings
//! make it again, and strokes go on a layer above it.

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

/// The cube, painting, on a small texture, with light and shade added.
fn with_light_and_shade() -> H {
    let mut harness = painting_the_cube();
    {
        let doc = harness.state_mut().workbench.document_mut();
        let mut texture = doc.scene().texture();
        texture.size = 256;
        doc.execute(scene::Command::SetTexture(texture)).unwrap();
    }
    harness.run();
    harness.get_by_label("+ Light and shade").click();
    harness.run_steps(4);
    harness
}

#[test]
fn light_and_shade_is_made_on_the_shape_in_one_undo_step() {
    let mut harness = with_light_and_shade();
    let set = set(&harness);
    let layer = set.layers.last().unwrap();
    assert_eq!(layer.name, "Light and shade");
    assert_eq!(layer.blend, paint::Blend::Overlay);
    let lighting = layer.lighting.unwrap();
    // The default sun is high: the top is lit, the back in shade.
    assert_eq!(top_at(&harness, Vec3::new(0.0, 0.0, 100.0)), lighting.light);
    let back = top_at(&harness, Vec3::new(0.0, 50.0, 50.0));
    assert!(back[2] > back[0], "cool at the back: {back:?}");
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Add a light and shade layer")
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
fn moving_the_sun_makes_it_again_and_strokes_go_above_it() {
    let mut harness = with_light_and_shade();
    let steps = harness.state().workbench.document().undo_steps();
    let back = Vec3::new(0.0, 50.0, 50.0);
    let before = top_at(&harness, back);
    // The sun moved round to the back, low (as the panel's sliders do).
    let mut changed = (*set(&harness)).clone();
    let layer = changed.layers.last_mut().unwrap();
    let mut l = layer.lighting.unwrap();
    l.around = 90.0;
    l.height = 10.0;
    layer.lighting = Some(l);
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
    harness.run_steps(4);
    let after = top_at(&harness, back);
    assert_ne!(before, after);
    assert_eq!(after, l.light, "the back is lit now");
    assert_eq!(
        harness.state().workbench.document().undo_steps(),
        steps + 1,
        "the change and its making are one step"
    );
    // A stroke with it chosen goes on a new layer above it.
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = glam::Vec2::new(rect.width(), rect.height());
    let at = camera(&harness)
        .project(Vec3::new(0.0, -50.0, 50.0), size)
        .unwrap();
    let at = rect.min + egui::vec2(at.x, at.y);
    let lit = set(&harness).layers.last().unwrap().clone();
    stroke(
        &mut harness,
        at,
        at,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    let after = set(&harness);
    assert_eq!(after.layers.len(), 2);
    assert_eq!(after.layers[0], lit, "light and shade is untouched");
}
