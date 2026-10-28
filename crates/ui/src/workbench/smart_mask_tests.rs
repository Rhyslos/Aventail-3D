//! Smart masks (decision 0195): Add smart mask makes a mask from the
//! shape in one undo step, its settings make it again, and strokes never
//! paint into it.

use super::brushing_tests::stroke;
use super::paint_layers_tests::{painting_the_cube, press};
use super::tests::camera;
use egui::{Modifiers, PointerButton};
use egui_kittest::kittest::Queryable;
use glam::Vec3;
use paint::{Composite, SmartKind, TextureSet};

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

/// How many texels of the mixed texture show.
fn shown(harness: &H) -> usize {
    let image = Composite::of(&set(harness)).unwrap().image().clone();
    (0..image.size())
        .flat_map(|y| (0..image.size()).map(move |x| (x, y)))
        .filter(|&(x, y)| image.pixel(x, y)[3] > 0)
        .count()
}

/// The cube, painting on a small texture, with a fill layer that has a
/// smart mask.
fn with_a_smart_mask() -> (H, usize) {
    let mut harness = painting_the_cube();
    {
        let doc = harness.state_mut().workbench.document_mut();
        let mut texture = doc.scene().texture();
        texture.size = 256;
        doc.execute(scene::Command::SetTexture(texture)).unwrap();
    }
    harness.run();
    press(&mut harness, "+ Fill");
    let everything = shown(&harness);
    harness.get_by_label("Add smart mask").scroll_to_me();
    harness.run();
    harness.get_by_label("Add smart mask").click();
    harness.run_steps(4);
    (harness, everything)
}

#[test]
fn a_smart_mask_shows_the_edges_in_one_undo_step() {
    let (mut harness, everything) = with_a_smart_mask();
    let mask = set(&harness).layers[0].mask.clone().unwrap();
    assert_eq!(mask.smart.unwrap().kind, SmartKind::Edges);
    let edges = shown(&harness);
    assert!(
        edges > 500 && edges < everything / 4,
        "only near the edges: {edges} of {everything}"
    );
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Add a smart mask")
    );
    harness.state_mut().workbench.document_mut().undo().unwrap();
    harness.run();
    assert!(set(&harness).layers[0].mask.is_none());
    assert_eq!(shown(&harness), everything);
}

#[test]
fn changing_a_smart_mask_makes_it_again() {
    let (mut harness, everything) = with_a_smart_mask();
    let edges = shown(&harness);
    let steps = harness.state().workbench.document().undo_steps();
    // Facing up, as the panel's kind list does.
    let mut changed = set(&harness);
    let mask = changed.layers[0].mask.as_mut().unwrap();
    mask.smart = Some(paint::SmartMask::of(SmartKind::FacingUp));
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
    let up = shown(&harness);
    assert_ne!(up, edges);
    // The top of the cube: a sixth of its polygons' texels.
    assert!(
        up > everything / 20 && up < everything / 4,
        "{up} of {everything}"
    );
    assert_eq!(
        harness.state().workbench.document().undo_steps(),
        steps + 1,
        "the change and its making are one step"
    );
}

#[test]
fn strokes_never_paint_into_a_smart_mask() {
    let (mut harness, _) = with_a_smart_mask();
    harness.get_by_label("Mask of Fill").click();
    harness.run();
    let before = set(&harness).layers[0].mask.clone().unwrap();
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = glam::Vec2::new(rect.width(), rect.height());
    let at = camera(&harness)
        .project(Vec3::new(0.0, -50.0, 50.0), size)
        .unwrap();
    let at = rect.min + egui::vec2(at.x, at.y);
    stroke(
        &mut harness,
        at,
        at,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    assert_eq!(set(&harness).layers[0].mask.as_ref(), Some(&before));
}

#[test]
fn an_edge_highlight_lightens_only_the_outer_edges() {
    let mut harness = painting_the_cube();
    {
        let doc = harness.state_mut().workbench.document_mut();
        let mut texture = doc.scene().texture();
        texture.size = 256;
        doc.execute(scene::Command::SetTexture(texture)).unwrap();
    }
    harness.run();
    press(&mut harness, "+ Fill");
    let everything = shown(&harness);
    harness.get_by_label("+ Edge highlight").scroll_to_me();
    harness.run();
    harness.get_by_label("+ Edge highlight").click();
    harness.run_steps(4);
    let set = set(&harness);
    let top = set.layers.last().unwrap();
    assert_eq!(top.name, "Edge highlight");
    assert_eq!(top.blend, paint::Blend::Screen);
    let smart = top.mask.as_ref().unwrap().smart.unwrap();
    assert_eq!(smart.kind, SmartKind::Edges);
    // Made on the shape: some of the mask shows, far from all of it.
    let mask = &top.mask.as_ref().unwrap().image;
    let lit = (0..mask.size())
        .flat_map(|y| (0..mask.size()).map(move |x| (x, y)))
        .filter(|&(x, y)| mask.pixel(x, y)[0] > 128)
        .count();
    assert!(lit > 200 && lit < everything / 5, "{lit} of {everything}");
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Add an edge highlight")
    );
}
