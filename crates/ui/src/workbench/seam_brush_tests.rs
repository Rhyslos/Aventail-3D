//! The seam blend brush (decision 0294): on the shelf with no key, the
//! seams shown while it is in use, a stroke along a seam evening out its
//! two sides on the layer being painted in one undo step, and nothing
//! changed (and a word in the status bar) away from seams.

use super::brushing::NO_SEAM_UNDER_BRUSH;
use super::brushing_tests::stroke;
use super::modelling_tests::on_screen;
use super::paint_layers_tests::{press, set};
use super::seam_blend_tests::{blending_the_cube, seams_in_view};
use super::tests::State;
use crate::attributes::PaintTool;
use egui::{Modifiers, PointerButton};
use glam::{Vec2, Vec3};
use paint::BrushKind;
use paint::seams::Seam;

type H = egui_kittest::Harness<'static, State>;

/// The cube painted a colour a side, with the seam blend brush chosen
/// from the shelf.
fn brushing_the_cube() -> H {
    let mut harness = blending_the_cube();
    harness.key_press(egui::Key::Num1);
    harness.run();
    press(&mut harness, "Brush Seam blend");
    harness.state_mut().workbench.attributes.brush.brush.radius = 40.0;
    harness
}

/// How different the two sides of `seam` are on the bottom layer: the
/// colour gap between texels just inside each side, summed along it.
fn difference(harness: &H, seam: &Seam) -> u32 {
    let set = set(harness);
    let image = set.layers[0].image().unwrap();
    let size = image.size() as f32;
    let texel = |uv: Vec2| Vec2::new(uv.x * size, (1.0 - uv.y) * size);
    let inside = |side: &paint::seams::SeamSide, t: f32| {
        let (a, b) = (texel(side.uvs[0]), texel(side.uvs[1]));
        let on = a + (b - a) * t;
        let mut inward = (b - a).perp().normalize_or_zero();
        if (texel(side.inward) - a).dot(inward) < 0.0 {
            inward = -inward;
        }
        let p = on + inward * 1.5;
        image.pixel(p.x as u32, p.y as u32)
    };
    (1..10)
        .map(|i| {
            let t = i as f32 / 10.0;
            let (a, b) = (inside(&seam.sides[0], t), inside(&seam.sides[1], t));
            (0..3).map(|c| u32::from(a[c].abs_diff(b[c]))).sum::<u32>()
        })
        .sum()
}

#[test]
fn the_shelf_chooses_it_with_no_key_and_shows_the_seams() {
    let harness = brushing_the_cube();
    let workbench = &harness.state().workbench;
    assert_eq!(workbench.attributes.brush.tool, PaintTool::Brush);
    assert_eq!(workbench.attributes.brush.brush.kind, BrushKind::SeamBlend);
    let lines = workbench.viewport.seam_lines().expect("seams shown");
    assert!(!lines.all.is_empty() && lines.hovered.is_empty());
    // It is one of the ready-made effect brushes.
    let preset = crate::brush_bar::built_in()
        .into_iter()
        .find(|p| p.kind() == BrushKind::SeamBlend)
        .expect("on the shelf");
    assert_eq!(preset.name, "Seam blend");
}

#[test]
fn a_stroke_along_a_seam_evens_out_its_sides_in_one_undo_step() {
    let mut harness = brushing_the_cube();
    let seam = seams_in_view(&harness)[0];
    let before = difference(&harness, &seam);
    assert!(before > 100, "the sides differ to begin with: {before}");
    let steps = harness.state().workbench.document().undo_steps();
    let [a, b] = seam.ends;
    let from = on_screen(&harness, a + (b - a) * 0.05);
    let to = on_screen(&harness, a + (b - a) * 0.95);
    stroke(
        &mut harness,
        from,
        to,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    let after = difference(&harness, &seam);
    assert!(after < before / 2, "{before} → {after}");
    // On the paint layer, as a seam blend stroke kept with it; no new
    // layer.
    let now = set(&harness);
    assert_eq!(now.layers.len(), 1);
    assert_eq!(
        now.layers[0].strokes.last().map(|s| s.kind),
        Some(BrushKind::SeamBlend)
    );
    assert_ne!(
        harness.state().workbench.note.as_deref(),
        Some(crate::text::tr(NO_SEAM_UNDER_BRUSH))
    );
    // One undo step puts it back.
    let doc = harness.state_mut().workbench.document_mut();
    assert_eq!(doc.undo_steps(), steps + 1);
    assert_eq!(doc.undo_label(), Some("Seam blend stroke"));
    doc.undo().unwrap();
    harness.run();
    assert_eq!(difference(&harness, &seam), before);
}

#[test]
fn away_from_seams_it_changes_nothing_and_says_so() {
    let mut harness = brushing_the_cube();
    harness.state_mut().workbench.attributes.brush.brush.radius = 8.0;
    let before = set(&harness).layers[0].image().unwrap().clone();
    // Round the middle of the cube's front, well inside its side.
    let from = on_screen(&harness, Vec3::new(-8.0, -50.0, 50.0));
    let to = on_screen(&harness, Vec3::new(8.0, -50.0, 50.0));
    stroke(
        &mut harness,
        from,
        to,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    let after = set(&harness).layers[0].image().unwrap().clone();
    for y in 0..before.size() {
        for x in 0..before.size() {
            assert_eq!(after.pixel(x, y), before.pixel(x, y), "{x} {y}");
        }
    }
    assert_eq!(
        harness.state().workbench.note.as_deref(),
        Some(crate::text::tr(NO_SEAM_UNDER_BRUSH))
    );
}
