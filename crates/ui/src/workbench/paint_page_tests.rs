//! The Paint page made plain (decision 0223): no modelling toolbar, no
//! mode column, one view with no labels, the grid only when asked for.

use super::paint_layers_tests::painting_the_cube;
use egui::{Event, Modifiers, PointerButton};
use egui_kittest::kittest::Queryable;

type H = egui_kittest::Harness<'static, super::tests::State>;

fn last_view(harness: &H) -> &crate::ViewportView {
    harness.state().renderer.drawn.last().expect("drawn")
}

#[test]
fn painting_has_no_modelling_toolbar_or_mode_column() {
    let mut harness = painting_the_cube();
    assert!(harness.query_by_label("Undo").is_none(), "no toolbar");
    assert!(harness.query_by_label("Points mode").is_none(), "no modes");
    // Back in the Model mode, both are there again.
    harness.state_mut().workbench.mode = crate::palette::Mode::Model;
    harness.run();
    harness.run();
    assert!(harness.query_by_label("Undo").is_some());
    assert!(harness.query_by_label("Points mode").is_some());
}

#[test]
fn the_left_bar_stays_for_the_paint_tools_when_set() {
    let mut harness = painting_the_cube();
    harness
        .state_mut()
        .workbench
        .viewport
        .options_mut()
        .paint_tools_left = true;
    harness.run();
    harness.run();
    assert!(
        harness
            .query_by_label(crate::paint_tools::COLUMN_LABEL)
            .is_some()
    );
    assert!(
        harness.query_by_label("Points mode").is_none(),
        "tools only"
    );
}

#[test]
fn painting_shows_one_view_whatever_is_pressed() {
    let mut harness = painting_the_cube();
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    // A middle click, which elsewhere splits the view into four.
    harness.event(Event::PointerMoved(rect.center()));
    harness.step();
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: rect.center(),
            button: PointerButton::Middle,
            pressed,
            modifiers: Modifiers::NONE,
        });
        harness.step();
    }
    harness.key_press(crate::viewport::FOUR_VIEWS_KEY);
    harness.run();
    assert!(!harness.state().workbench.viewport.four_views());
    // Four views chosen before painting: still one while painting.
    harness.state_mut().workbench.viewport.set_four_views(true);
    harness.run();
    assert_eq!(harness.query_all_by_label(crate::VIEWPORT_LABEL).count(), 1);
}

#[test]
fn the_grid_is_off_while_painting_until_ticked() {
    let mut harness = painting_the_cube();
    assert!(!last_view(&harness).show_grid);
    harness
        .get_by_label(crate::viewport::paint_row::SHOW_LABEL)
        .click();
    harness.run();
    harness
        .get_by_label(crate::viewport::look::PAINT_GRID_LABEL)
        .click();
    harness.run();
    assert!(last_view(&harness).show_grid);
    assert!(harness.state().workbench.view_options().paint_grid);
    // The Model mode always shows it.
    harness
        .state_mut()
        .workbench
        .viewport
        .options_mut()
        .paint_grid = false;
    harness.state_mut().workbench.mode = crate::palette::Mode::Model;
    harness.run();
    harness.run();
    assert!(last_view(&harness).show_grid);
}

#[test]
fn the_grid_setting_is_remembered() {
    let options = crate::viewport::ViewOptions {
        paint_grid: true,
        ..Default::default()
    };
    let back: crate::viewport::ViewOptions =
        ron::from_str(&ron::to_string(&options).unwrap()).unwrap();
    assert!(back.paint_grid);
    let old: crate::viewport::ViewOptions = ron::from_str("()").unwrap();
    assert!(!old.paint_grid, "off as standard");
}
