//! Tests for the labels on the palettes (decision 0126, milestone 7): the
//! mode palette's tooltips (its slide-out names are gone, decision 0167),
//! the top palette's icons and the select tool list.

use super::tests::fresh;
use crate::palette::SELECT_TOOL_MENU_LABEL;
use crate::selecting::SelectTool;
use egui::Event;
use egui_kittest::kittest::Queryable;

type H = egui_kittest::Harness<'static, super::tests::State>;

/// Lets `seconds` pass with the mouse where it is.
fn wait(harness: &mut H, seconds: f64) {
    let start = harness.ctx.input(|i| i.time);
    while harness.ctx.input(|i| i.time) - start < seconds {
        harness.step();
    }
    harness.run();
}

#[test]
fn resting_on_the_mode_palette_shows_only_tooltips() {
    // The project lead's choice (decision 0167): no strip of names slides
    // out any more; each button names itself in its tooltip.
    let mut harness = fresh();
    let at = harness.get_by_label("Points mode").rect().center();
    harness.event(Event::PointerMoved(at));
    wait(&mut harness, 1.5);
    // No row of names beside the palette: the Points mode's name is
    // there only once, in its tooltip.
    let points_named = harness
        .output()
        .shapes
        .iter()
        .filter(|c| matches!(&c.shape, egui::epaint::Shape::Text(t) if t.galley.text() == "Points"))
        .count();
    assert_eq!(points_named, 0, "no strip with the bare name");
    let tip_says_points = harness.output().shapes.iter().any(
        |c| matches!(&c.shape, egui::epaint::Shape::Text(t) if t.galley.text().contains("Points")),
    );
    assert!(tip_says_points, "the tooltip names the mode");
    // A click still picks the mode.
    harness.get_by_label("Polygons mode").click();
    harness.run();
    assert_eq!(
        harness.state().workbench.mode(),
        crate::palette::Mode::Polygons
    );
}

#[test]
fn the_tools_are_icons_only_with_transform_beside_them() {
    // The project lead's choice (decision 0140): no words beside the
    // icons, at any width; Transform (T) has its own button.
    let wide = fresh();
    for name in ["Move", "Scale", "Rotate", "Transform"] {
        let rect = wide.get_by_label(name).rect();
        assert!(rect.width() < 40.0, "{name} is an icon only: {rect:?}");
    }
    let snap = wide.get_by_label_contains("Snapping").rect();
    assert!(snap.width() < 40.0);
}

#[test]
fn transform_lights_only_its_own_button() {
    let mut harness = fresh();
    harness.get_by_label("Transform").click();
    harness.run();
    assert_eq!(
        harness.state().workbench.tool(),
        crate::transforming::Tool::All
    );
    harness.get_by_label("Move").click();
    harness.run();
    assert_eq!(
        harness.state().workbench.tool(),
        crate::transforming::Tool::Move
    );
}

#[test]
fn the_select_tool_list_picks_a_tool() {
    let mut harness = fresh();
    harness.get_by_label(SELECT_TOOL_MENU_LABEL).click();
    harness.run();
    harness.get_by_label_contains("Lasso").click();
    harness.run();
    let bench = &harness.state().workbench;
    assert_eq!(bench.viewport().select_tool(), SelectTool::Lasso);
    assert_eq!(bench.tool(), crate::transforming::Tool::Select);
}
