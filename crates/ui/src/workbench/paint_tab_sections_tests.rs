//! The Paint tab in folding sections (decision 0228): Tip, Lays down,
//! Pen and smoothing, Symmetry and trim, Paint through and effects, and
//! Keys, with what's set inside named on the right; channel buttons with
//! coloured dots.

use super::paint_layers_tests::{painting_the_cube, press};
use egui_kittest::kittest::Queryable;

type H = egui_kittest::Harness<'static, super::tests::State>;

/// Whether `inside` shows: its section is open.
fn shows(harness: &H, inside: &str) -> bool {
    harness.query_by_label(inside).is_some()
}

#[test]
fn the_brush_settings_are_in_sections_with_tip_open_to_start_with() {
    let harness = painting_the_cube();
    for title in [
        "Tip",
        "Pen and smoothing",
        "Symmetry and trim",
        "Paint through and effects",
        "Keys for painting",
    ] {
        assert!(harness.query_by_label(title).is_some(), "{title}");
    }
    assert!(shows(&harness, "Size space"), "Tip is open");
    // Pen and smoothing and the keys start closed.
    for inside in ["Pressure → size", "Pick a colour"] {
        assert!(!shows(&harness, inside), "{inside} is in a closed section");
    }
    // Lays down needs the realism channels.
    assert!(harness.query_by_label("Lays down").is_none());
}

#[test]
fn a_click_on_a_section_opens_and_closes_it_and_it_stays_so() {
    let mut harness = painting_the_cube();
    assert!(!shows(&harness, "Paint through"));
    press(&mut harness, "Paint through and effects");
    assert!(shows(&harness, "Paint through"));
    harness.run();
    harness.run();
    assert!(shows(&harness, "Paint through"), "kept open");
    press(&mut harness, "Paint through and effects");
    assert!(!shows(&harness, "Paint through"));
    press(&mut harness, "Tip");
    assert!(harness.query_by_label("Size space").is_none(), "Tip closed");
}

#[test]
fn the_keys_are_listed_one_per_line() {
    let mut harness = painting_the_cube();
    press(&mut harness, "Keys for painting");
    for what in [
        "Blur",
        "Fill the layer",
        "Pick a colour",
        "Mirror on or off",
    ] {
        assert!(harness.query_by_label(what).is_some(), "{what}");
    }
}

#[test]
fn with_the_realism_channels_the_brush_says_what_it_lays_down() {
    let mut harness = painting_the_cube();
    press(&mut harness, crate::paint_layers::REALISM_LABEL);
    assert!(harness.query_by_label("Lays down").is_some());
    assert!(harness.query_by_label("Roughness").is_some());
    // The eraser lays nothing down.
    harness.key_press(egui::Key::Num2);
    harness.run();
    harness.run();
    assert!(harness.query_by_label("Lays down").is_none());
}

#[test]
fn the_fill_tool_has_its_own_sections() {
    let mut harness = painting_the_cube();
    harness.key_press(egui::Key::Num4);
    harness.run();
    harness.run();
    assert!(harness.query_by_label("Fills").is_some());
    assert!(harness.query_by_label("UV piece").is_some(), "open");
    assert!(harness.query_by_label("Symmetry and trim").is_some());
    assert!(harness.query_by_label("Tip").is_none());
}

#[test]
fn every_channel_but_all_has_a_dot() {
    use crate::attributes::channel_view::ChannelView;
    let brush = egui::Color32::RED;
    assert_eq!(ChannelView::All.dot(brush), None);
    assert_eq!(ChannelView::Colour.dot(brush), Some(brush));
    for view in ChannelView::ALL.into_iter().skip(1) {
        assert!(view.dot(brush).is_some());
    }
}
