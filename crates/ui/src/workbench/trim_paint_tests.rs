//! Painting trim sheets live (decision 0202): a stroke stays in the trim
//! row it starts in unless that's switched off, and every object sharing
//! the material shows it.

use super::brushing_tests::stroke;
use super::paint_layers_tests::painting_the_cube;
use super::tests::camera;
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

/// The rows of the top layer that hold paint.
fn painted_rows(harness: &H) -> Vec<u32> {
    let doc = harness.state().workbench.document();
    let set = doc.scene().paint().set("Default").unwrap();
    let image = set.layers.last().and_then(|l| l.image()).unwrap();
    (0..image.size())
        .filter(|y| (0..image.size()).any(|x| image.pixel(x, *y)[3] > 0))
        .collect()
}

/// A big stroke across the cube's front, with the trim sheet in 8 rows.
fn big_stroke(trim_row: bool) -> (H, Vec<u32>) {
    let mut harness = painting_the_cube();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::SetTrim(scene::TrimSheet::even(8)))
        .unwrap();
    harness.state_mut().workbench.attributes.brush.trim_row = trim_row;
    harness.state_mut().workbench.attributes.brush.brush.radius = 80.0;
    harness.run();
    let a = on_screen(&harness, Vec3::new(-30.0, -50.0, 50.0));
    let b = on_screen(&harness, Vec3::new(30.0, -50.0, 50.0));
    stroke(&mut harness, a, b, Modifiers::NONE, PointerButton::Primary);
    let rows = painted_rows(&harness);
    (harness, rows)
}

#[test]
fn a_stroke_stays_in_the_trim_row_it_starts_in() {
    let (harness, kept) = big_stroke(true);
    let size = harness.state().workbench.document().scene().texture().size;
    let row = size / 8;
    assert!(!kept.is_empty());
    let first = kept[0] / row;
    assert!(
        kept.iter().all(|y| y / row == first),
        "one row of {row} texels: {:?}..{:?}",
        kept.first(),
        kept.last()
    );
    // Switched off, the same stroke spills over.
    let (_, free) = big_stroke(false);
    assert!(free.iter().any(|y| y / row != first));
    // The setting shows in Attributes while there are trim rows.
    assert!(harness.query_by_label("Stay in the row").is_some());
}
