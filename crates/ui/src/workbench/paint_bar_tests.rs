//! The floating brush bar (decision 0213): over the viewport while
//! painting, changing the brush, and the colour picker as its pop-up.

use super::paint_layers_tests::{painting_the_cube, press};
use crate::palette::Mode;
use egui_kittest::kittest::Queryable;

type H = egui_kittest::Harness<'static, super::tests::State>;

fn strokes(harness: &H) -> usize {
    harness
        .state()
        .workbench
        .document()
        .scene()
        .paint()
        .set("Default")
        .map(|s| s.layers.iter().map(|l| l.strokes.len()).sum())
        .unwrap_or(0)
}

#[test]
fn the_bar_shows_while_painting_and_its_numbers_live_only_there() {
    let mut harness = painting_the_cube();
    assert!(
        harness
            .query_by_label(crate::paint_bar::BAR_LABEL)
            .is_some()
    );
    assert!(harness.state().workbench.attributes.brush_bar_shown);
    // Each number once: on the bar, not again in the Paint tab.
    for label in [
        "Brush flow",
        "Brush hardness",
        "Brush spacing",
        "Brush size on screen",
    ] {
        assert_eq!(harness.query_all_by_label(label).count(), 1, "{label}");
    }
    // The bar sits over the viewport.
    let view = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let bar = harness.get_by_label(crate::paint_bar::BAR_LABEL).rect();
    assert!(view.contains(bar.min), "{bar:?} in {view:?}");
    // Not in the Model mode.
    harness.state_mut().workbench.mode = Mode::Model;
    harness.run();
    harness.run();
    assert!(
        harness
            .query_by_label(crate::paint_bar::BAR_LABEL)
            .is_none()
    );
}

#[test]
fn typing_in_the_bar_changes_the_brush() {
    let mut harness = painting_the_cube();
    harness.get_by_label("Brush flow").click();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.get_by_label("Brush flow").type_text("40");
    harness.key_press(egui::Key::Enter);
    harness.run();
    harness.run();
    let flow = harness.state().workbench.attributes.brush.brush.flow;
    assert!((flow - 0.4).abs() < 1e-4, "{flow}");
}

#[test]
fn the_switches_turn_mirror_pen_and_smoothing_on_and_off() {
    let mut harness = painting_the_cube();
    let before = harness.state().workbench.attributes.brush;
    press(&mut harness, crate::paint_bar::MIRROR_SWITCH);
    press(&mut harness, crate::paint_bar::PEN_SWITCH);
    press(&mut harness, crate::paint_bar::SMOOTHING_SWITCH);
    let after = harness.state().workbench.attributes.brush;
    assert_ne!(after.mirror_on, before.mirror_on);
    assert_ne!(after.size_pressure.on, before.size_pressure.on);
    assert_ne!(after.smoothing_on, before.smoothing_on);
    press(&mut harness, crate::paint_bar::MIRROR_SWITCH);
    assert_eq!(
        harness.state().workbench.attributes.brush.mirror_on,
        before.mirror_on
    );
}

#[test]
fn a_click_on_the_bar_paints_nothing() {
    let mut harness = painting_the_cube();
    let before = strokes(&harness);
    press(&mut harness, crate::paint_bar::PEN_SWITCH);
    assert_eq!(strokes(&harness), before);
}

#[test]
fn the_colour_square_opens_the_picker_and_the_fill_tool_keeps_only_it() {
    let mut harness = painting_the_cube();
    let wheel = crate::attributes::colour_picker::WHEEL_LABEL;
    assert!(harness.query_by_label(wheel).is_none(), "shut to start");
    harness.get_by_label(crate::paint_bar::COLOUR_LABEL).click();
    harness.run();
    harness.run();
    assert!(harness.query_by_label(wheel).is_some());
    // The polygon fill tool: its colour, and none of the brush's numbers.
    harness.key_press(egui::Key::Num4);
    harness.run();
    harness.run();
    assert!(
        harness
            .query_by_label(crate::paint_bar::COLOUR_LABEL)
            .is_some()
    );
    assert!(harness.query_by_label("Brush flow").is_none());
}

#[test]
fn the_eraser_has_no_colour_square() {
    let mut harness = painting_the_cube();
    harness.key_press(egui::Key::Num2);
    harness.run();
    harness.run();
    assert!(
        harness
            .query_by_label(crate::paint_bar::COLOUR_LABEL)
            .is_none()
    );
    assert!(harness.query_by_label("Brush flow").is_some());
}

#[test]
fn the_second_colour_swaps_with_the_first() {
    // Decision 0226: two colour squares, as in painting programs.
    let mut harness = painting_the_cube();
    let before = harness.state().workbench.attributes.brush;
    harness
        .get_by_label(crate::paint_bar::SECOND_COLOUR_LABEL)
        .click();
    harness.run();
    let after = harness.state().workbench.attributes.brush;
    assert_eq!(after.brush.color, before.second_color);
    assert_eq!(after.second_color, before.brush.color);
}

#[test]
fn the_brush_list_chooses_a_ready_made_brush() {
    let mut harness = painting_the_cube();
    harness
        .get_by_label(crate::paint_bar::BRUSH_LIST_LABEL)
        .click();
    harness.run();
    let hard = crate::brush_bar::built_in()
        .into_iter()
        .find(|p| p.name == "Hard round")
        .expect("a hard round brush");
    harness
        .get_by_label(&format!("      {}", hard.name))
        .click();
    harness.run();
    assert!(hard.matches(&harness.state().workbench.attributes.brush));
}

#[test]
fn dragging_a_bar_sideways_changes_its_number() {
    let mut harness = painting_the_cube();
    let before = harness.state().workbench.attributes.brush.brush.flow;
    let field = harness.get_by_label("Brush flow").rect();
    let from = field.center();
    harness.event(egui::Event::PointerMoved(from));
    harness.step();
    harness.event(egui::Event::PointerButton {
        pos: from,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    harness.step();
    for i in 1..=5 {
        harness.event(egui::Event::PointerMoved(
            from - egui::vec2(6.0 * i as f32, 0.0),
        ));
        harness.step();
    }
    harness.event(egui::Event::PointerButton {
        pos: from - egui::vec2(30.0, 0.0),
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    harness.run();
    let after = harness.state().workbench.attributes.brush.brush.flow;
    assert!(after < before - 0.1, "dragged left: {before} to {after}");
    assert_eq!(strokes(&harness), 0, "nothing painted");
}
