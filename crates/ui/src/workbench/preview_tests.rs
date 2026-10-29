//! Paint previews (decision 0198): chosen in the Look list (decision 0224)
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
    harness
        .get_by_label(crate::viewport::paint_row::LOOK_LABEL)
        .click();
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
    harness
        .get_by_label(crate::viewport::paint_row::LOOK_LABEL)
        .click();
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
    harness
        .get_by_label(crate::viewport::paint_row::LOOK_LABEL)
        .click();
    harness.run();
    assert!(harness.query_by_label("Paint preview").is_none());
    harness.key_press(egui::Key::Escape);
    harness.run();
    // The view's menus are behind ☰ on the Paint page (decision 0224).
    harness
        .get_by_label(crate::viewport::paint_row::VIEW_MENUS_LABEL)
        .click();
    harness.run();
    harness.get_by_label("Display ⏵").hover();
    harness.run();
    harness.run();
    assert!(harness.query_by_label("Paint preview").is_none());
}

#[test]
fn the_player_view_draws_a_smaller_image_and_says_so() {
    let mut harness = painting();
    harness
        .get_by_label(crate::viewport::paint_row::LOOK_LABEL)
        .click();
    harness.run();
    harness
        .get_by_label(crate::viewport::look::PLAYER_ENTRY)
        .click();
    harness.run();
    harness.run();
    assert!(harness.state().workbench.view_options().player_view);
    // Its settings show while it's on.
    assert!(harness.query_by_label("Distance").is_some());
    harness.key_press(egui::Key::Escape);
    harness.run();
    let size = |h: &mut egui_kittest::Harness<'static, super::tests::State>, distance: f32| {
        let options = h.state_mut().workbench.viewport.options_mut();
        options.player.distance = distance;
        h.run();
        h.state().renderer.drawn.last().unwrap().size_px
    };
    // Far away: much smaller than the view shows it.
    let small = size(&mut harness, 50.0);
    // Nearer than the view: drawn as is.
    let near = size(&mut harness, 0.5);
    harness
        .state_mut()
        .workbench
        .viewport
        .options_mut()
        .player_view = false;
    harness.run();
    let full = harness.state().renderer.drawn.last().unwrap().size_px;
    assert_eq!(near, full);
    assert!(
        small[0] < full[0] / 2 && small[1] < full[1] / 2,
        "{small:?} of {full:?}"
    );
}
