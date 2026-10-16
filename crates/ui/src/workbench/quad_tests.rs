//! Tests for the quad menu (decision 0094): off as standard, opened by a
//! right-click in the viewport when on, its four lists, and closing.

use super::modelling_tests::{H, cube};
use super::tests::{fresh, viewport_spot};
use super::*;
use egui::{Event, PointerButton};
use egui_kittest::kittest::Queryable;

fn right_click(harness: &mut H, at: egui::Pos2) {
    harness.event(Event::PointerMoved(at));
    harness.step();
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Secondary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        harness.step();
    }
    harness.run();
}

/// The node labelled `label` nearest to `at` (menus elsewhere share some
/// names).
fn near<'h>(harness: &'h H, label: &'h str, at: egui::Pos2) -> egui_kittest::Node<'h> {
    harness
        .get_all_by_label_contains(label)
        .min_by(|a, b| {
            let da = a.rect().center().distance(at);
            let db = b.rect().center().distance(at);
            da.total_cmp(&db)
        })
        .unwrap_or_else(|| panic!("no {label}"))
}

/// The window with the quad menu switched on and the cube selected.
fn switched_on() -> H {
    let mut harness = fresh();
    harness.state_mut().workbench.tool_menu.settings.quad_menu = true;
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    harness
}

#[test]
fn it_is_off_as_standard() {
    let mut harness = fresh();
    let at = viewport_spot(&harness);
    right_click(&mut harness, at);
    assert!(!harness.state().workbench.is_quad_menu_open());
}

#[test]
fn a_right_click_opens_four_lists_around_the_mouse() {
    let mut harness = switched_on();
    let at = viewport_spot(&harness);
    right_click(&mut harness, at);
    assert!(harness.state().workbench.is_quad_menu_open());
    for title in ["Show", "Modes", "Edit", "Tools"] {
        assert!(
            near(&harness, title, at).rect().center().distance(at) < 300.0,
            "{title}"
        );
    }
    // Show is up and left of the mouse, Tools down and right.
    let show = near(&harness, "Show", at).rect();
    let tools = near(&harness, "Tools", at).rect();
    assert!(show.max.x < at.x && show.max.y < at.y);
    assert!(tools.min.x > at.x && tools.min.y > at.y);
    // The Model mode's tools, from the command list.
    assert!(
        harness
            .query_by_label_contains("Apply rotation and scale")
            .is_some()
    );
}

#[test]
fn picking_a_mode_switches_to_it_and_closes() {
    let mut harness = switched_on();
    let at = viewport_spot(&harness);
    right_click(&mut harness, at);
    near(&harness, "Edges", at).click();
    harness.run();
    assert_eq!(harness.state().workbench.mode, Mode::Edges);
    assert!(!harness.state().workbench.is_quad_menu_open());
}

#[test]
fn esc_or_a_click_elsewhere_closes_it() {
    let mut harness = switched_on();
    let at = viewport_spot(&harness);
    right_click(&mut harness, at);
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!harness.state().workbench.is_quad_menu_open());
    assert!(
        !harness.state().workbench.document().selection().is_empty(),
        "Esc only closed the menu"
    );
    right_click(&mut harness, at);
    let far = at + egui::vec2(-400.0, 250.0);
    harness.event(Event::PointerMoved(far));
    harness.step();
    harness.event(Event::PointerButton {
        pos: far,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    harness.run();
    assert!(!harness.state().workbench.is_quad_menu_open());
}

#[test]
fn a_right_click_that_belongs_to_something_else_doesnt_open_it() {
    let mut harness = switched_on();
    harness
        .state_mut()
        .workbench
        .start_target_pick(super::aligning::TargetUse::SnapTo);
    let at = viewport_spot(&harness);
    right_click(&mut harness, at);
    assert!(!harness.state().workbench.is_quad_menu_open());
}

#[test]
fn hide_selected_and_show_all_work_from_it() {
    let mut harness = switched_on();
    let id = cube(&harness);
    let at = viewport_spot(&harness);
    right_click(&mut harness, at);
    harness.get_by_label_contains("Hide selected").click();
    harness.run();
    let shown = |h: &H| {
        h.state()
            .workbench
            .document()
            .scene()
            .object(id)
            .unwrap()
            .visible
    };
    assert!(!shown(&harness));
    right_click(&mut harness, at);
    harness.get_by_label_contains("Show all").click();
    harness.run();
    assert!(shown(&harness));
}

#[test]
fn the_setting_is_in_the_settings_window() {
    let mut harness = fresh();
    harness.state_mut().workbench.settings.open = true;
    harness.run();
    let switch = harness.get_by_label(crate::settings::QUAD_MENU_SWITCH);
    switch.scroll_to_me();
    harness.run();
    harness
        .get_by_label(crate::settings::QUAD_MENU_SWITCH)
        .click();
    harness.run();
    assert!(harness.state().workbench.tool_menu_settings().quad_menu);
}
