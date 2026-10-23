//! Tests for the labels on the palettes (decision 0126, milestone 7): the
//! mode palette's names sliding out on hover, words beside the top
//! palette's icons where there's room, and the select tool list.

use super::tests::{fresh, fresh_of_width};
use crate::palette::{NAMES_AFTER, NAMES_PREFIX, SELECT_TOOL_MENU_LABEL};
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

fn rest_on(harness: &mut H, label: &str) {
    let at = harness.get_by_label(label).rect().center();
    harness.event(Event::PointerMoved(at));
    harness.run();
}

fn names_shown(harness: &H) -> bool {
    harness
        .query_by_label(&format!("{NAMES_PREFIX}Points"))
        .is_some()
}

#[test]
fn resting_on_the_mode_palette_slides_out_its_names() {
    let mut harness = fresh();
    let at = harness.get_by_label("Model mode").rect().center();
    harness.event(Event::PointerMoved(at));
    harness.step();
    harness.step();
    assert!(!names_shown(&harness), "not at once");
    wait(&mut harness, NAMES_AFTER + 0.3);
    assert!(names_shown(&harness), "after a moment");
    // Each name sits level with its button, to the right of it.
    let button = harness.get_by_label("Points mode").rect();
    let name = harness
        .get_by_label(&format!("{NAMES_PREFIX}Points"))
        .rect();
    assert!((name.center().y - button.center().y).abs() < 1.0);
    assert!(name.min.x >= button.max.x);
    assert!(
        harness
            .query_by_label(&format!("{NAMES_PREFIX}Edit pivot"))
            .is_some()
    );
    // Leaving puts it away.
    harness.event(Event::PointerMoved(egui::pos2(700.0, 450.0)));
    wait(&mut harness, 0.5);
    assert!(!names_shown(&harness));
}

#[test]
fn clicking_a_name_does_what_its_button_does_and_puts_the_names_away() {
    let mut harness = fresh();
    rest_on(&mut harness, "Model mode");
    wait(&mut harness, NAMES_AFTER + 0.3);
    harness
        .get_by_label(&format!("{NAMES_PREFIX}Polygons"))
        .click();
    harness.run();
    assert_eq!(
        harness.state().workbench.mode(),
        crate::palette::Mode::Polygons
    );
    wait(&mut harness, 0.5);
    assert!(!names_shown(&harness), "away until the mouse leaves");
}

#[test]
fn a_click_on_a_button_puts_the_names_away_too() {
    let mut harness = fresh();
    rest_on(&mut harness, "Model mode");
    wait(&mut harness, NAMES_AFTER + 0.3);
    harness.get_by_label("Edges mode").click();
    wait(&mut harness, 0.5);
    assert!(!names_shown(&harness));
    assert_eq!(
        harness.state().workbench.mode(),
        crate::palette::Mode::Edges
    );
}

#[test]
fn words_beside_the_tools_when_there_is_room() {
    let wide = fresh();
    let moving = wide.get_by_label("Move").rect();
    assert!(moving.width() > 40.0, "Move has its word: {moving:?}");
    let snap = wide.get_by_label_contains("Snapping").rect();
    assert!(snap.width() > 40.0);
    // Narrow: icons only.
    let narrow = fresh_of_width(1000.0);
    let moving = narrow.get_by_label("Move").rect();
    assert!(moving.width() < 40.0, "{moving:?}");
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
