//! The Paint tab (decision 0213): the brush's settings in a tab of their
//! own beside Attributes in the Paint layout, Attributes left to the
//! object.

use super::paint_layers_tests::painting_the_cube;
use crate::panel::Panel;
use egui_kittest::kittest::Queryable;

#[test]
fn the_paint_layout_shows_the_paint_tab_in_front_of_attributes() {
    let harness = painting_the_cube();
    let bench = &harness.state().workbench;
    let path = bench.dock.find_tab(&Panel::Paint).expect("the Paint tab");
    let leaf = bench.dock.leaf(path.node_path()).unwrap();
    assert!(leaf.tabs.contains(&Panel::Attributes), "beside Attributes");
    assert_eq!(leaf.tabs[leaf.active.0], Panel::Paint, "in front");
    assert!(bench.attributes.brush_in_paint_tab);
    // The brush's settings are drawn (by the Paint tab).
    assert!(harness.query_by_label("Brush flow").is_some());
}

type H = egui_kittest::Harness<'static, super::tests::State>;

/// Brings `panel`'s tab to the front, as a click on it does.
fn front(harness: &mut H, panel: Panel) {
    let dock = &mut harness.state_mut().workbench.dock;
    let path = dock.find_tab(&panel).unwrap();
    let _ = dock.set_active_tab(path);
    harness.run();
    harness.run();
}

#[test]
fn attributes_shows_only_the_object_while_painting() {
    let mut harness = painting_the_cube();
    front(&mut harness, Panel::Attributes);
    // The object's settings, and none of the brush's.
    assert!(harness.query_by_label("Brush flow").is_none());
    assert!(harness.query_by_label("Brush hardness").is_none());
    assert!(harness.state().workbench.attributes.painting);
    // Back to the Paint tab: the brush again.
    front(&mut harness, Panel::Paint);
    assert!(harness.query_by_label("Brush flow").is_some());
}

#[test]
fn without_the_paint_tab_the_brush_stays_in_attributes() {
    let mut harness = painting_the_cube();
    // A layout someone has arranged without it (it can't be closed, but
    // a damaged saved layout could lack it).
    let bench = &mut harness.state_mut().workbench;
    let path = bench.dock.find_tab(&Panel::Paint).unwrap();
    bench.dock.remove_tab(path);
    harness.run();
    harness.run();
    assert!(!harness.state().workbench.attributes.brush_in_paint_tab);
    assert!(harness.query_by_label("Brush flow").is_some());
}

#[test]
fn the_paint_tab_outside_paint_mode_says_how_to_get_there() {
    let mut harness = painting_the_cube();
    harness.state_mut().workbench.attributes.painting = false;
    let mut panel = std::mem::take(&mut harness.state_mut().workbench.attributes);
    let doc = harness.state().workbench.document().clone();
    let mut shown = egui_kittest::Harness::new_ui(move |ui| panel.paint_panel(ui, &doc));
    shown.run();
    assert!(shown.query_by_label("Not painting").is_some());
    assert!(shown.query_by_label("Brush flow").is_none());
}
