//! "Why does this look wrong?" (decision 0203): opened from the Paint
//! layers panel, it finds flat lighting and too few pixels on a painted
//! cube, and their fixes work as one undo step.

use super::paint_layers_tests::{painting_the_cube, press};
use egui_kittest::kittest::Queryable;
use paint::advice::Finding;

type H = egui_kittest::Harness<'static, super::tests::State>;

/// The cube painted one even colour all over, with the window open.
fn evenly_painted(density: f32) -> H {
    let mut harness = painting_the_cube();
    {
        let doc = harness.state_mut().workbench.document_mut();
        let mut texture = doc.scene().texture();
        texture.size = 256;
        texture.density = density;
        doc.execute(scene::Command::SetTexture(texture)).unwrap();
    }
    harness.run();
    press(&mut harness, "+ Fill");
    press(&mut harness, "Why does this look wrong?");
    harness.run();
    harness
}

fn found(harness: &H) -> Vec<super::look_wrong::Found> {
    harness.state().workbench.look_wrong_found()
}

#[test]
fn flat_lighting_is_found_and_fixed_with_light_and_shade() {
    let mut harness = evenly_painted(50.0);
    assert!(harness.state().workbench.look_wrong.open, "the window opened");
    let found = found(&harness);
    let flat = found
        .iter()
        .find(|f| matches!(f.finding, Finding::FlatLight { .. }))
        .expect("flat lighting")
        .clone();
    assert!(!found.iter().any(|f| matches!(f.finding, Finding::Blurry { .. })));
    harness.state_mut().workbench.fix_found(&flat);
    harness.run_steps(4);
    let doc = harness.state().workbench.document();
    let set = doc.scene().paint().set("Default").unwrap();
    assert!(set.layers.iter().any(|l| l.lighting.is_some()));
    assert_eq!(doc.undo_label(), Some("Add a light and shade layer"));
}

#[test]
fn too_few_pixels_are_found_and_fixed_with_a_bigger_texture() {
    let mut harness = evenly_painted(2048.0);
    let blurry = found(&harness)
        .into_iter()
        .find(|f| matches!(f.finding, Finding::Blurry { .. }))
        .expect("too few pixels");
    assert!(blurry.text().contains("px/m"));
    harness.state_mut().workbench.fix_found(&blurry);
    harness.run_steps(4);
    let size = harness.state().workbench.document().scene().texture().size;
    assert!(size > 256, "{size}");
    // Showing it picks the polygons in the Polygons mode.
    harness.state_mut().workbench.show_found(&blurry);
    harness.run();
    assert_eq!(harness.state().workbench.mode, crate::palette::Mode::Polygons);
}
