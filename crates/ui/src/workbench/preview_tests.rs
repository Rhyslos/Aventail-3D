//! Paint previews (decision 0198): chosen in the shading button's list
//! and the Display menu while the surface is Painted, passed to the
//! renderer, and Cavity left off for all but Lit.

use super::brushing_tests::painting;
use crate::viewport::{PaintPreview, Surface};
use egui_kittest::kittest::Queryable;

#[test]
fn a_paint_preview_is_chosen_from_the_shading_list_while_painted() {
    let mut harness = painting();
    assert_eq!(
        harness.state().workbench.view_options().surface,
        Surface::Painted
    );
    harness.get_by_label(crate::viewport::SHADING_LABEL).click();
    harness.run();
    assert!(harness.query_by_label("Paint preview").is_some());
    harness.get_by_label(PaintPreview::Values.label()).click();
    harness.run();
    harness.run();
    assert_eq!(
        harness.state().workbench.view_options().paint_preview,
        PaintPreview::Values
    );
    let view = harness.state().renderer.drawn.last().unwrap().clone();
    assert_eq!(view.paint_preview, PaintPreview::Values);
    assert!(!view.cavity, "no cavity over a value check");
    // Back to Lit: cavity again.
    harness.get_by_label(crate::viewport::SHADING_LABEL).click();
    harness.run();
    harness.get_by_label(PaintPreview::Lit.label()).click();
    harness.run();
    harness.run();
    assert!(harness.state().renderer.drawn.last().unwrap().cavity);
}

#[test]
fn the_previews_are_offered_only_for_the_painted_surface() {
    let mut harness = painting();
    harness.state_mut().workbench.viewport.options_mut().surface = Surface::Plain;
    harness.run();
    harness.get_by_label(crate::viewport::SHADING_LABEL).click();
    harness.run();
    assert!(harness.query_by_label("Paint preview").is_none());
    harness.key_press(egui::Key::Escape);
    harness.run();
    harness.get_by_label("Display").click();
    harness.run();
    assert!(harness.query_by_label("Paint preview").is_none());
}
