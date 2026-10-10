//! Tests for the workbench, run through the real interface without a window.

use super::*;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn info() -> AppInfo {
    AppInfo {
        name: "Test App".into(),
        version: "9.9.9".into(),
        family: "Test Family".into(),
    }
}

/// The workbench plus every action it has returned so far.
struct State {
    workbench: Workbench,
    actions: Vec<WorkbenchAction>,
}

fn harness_for(workbench: Workbench) -> Harness<'static, State> {
    Harness::builder()
        .with_size([1440.0, 900.0])
        .build_ui_state(
            |ui, state: &mut State| {
                let actions = state.workbench.show(ui);
                state.actions.extend(actions);
            },
            State {
                workbench,
                actions: Vec::new(),
            },
        )
}

fn fresh() -> Harness<'static, State> {
    let mut harness = harness_for(Workbench::new(info(), None));
    harness.run();
    harness
}

#[test]
fn all_panel_tabs_are_visible() {
    let harness = fresh();
    for panel in Panel::ALL {
        assert!(
            harness.query_by_label(panel.title()).is_some(),
            "missing tab: {}",
            panel.title()
        );
    }
}

#[test]
fn menus_follow_the_design_order() {
    let harness = fresh();
    let menus = [
        "File", "Edit", "Create", "Modes", "Select", "Tools", "Mesh", "UV", "View", "Window",
        "Help",
    ];
    let mut last_x = f32::MIN;
    for menu in menus {
        let node = harness.get_by_label(menu);
        let x = node.rect().min.x;
        assert!(x > last_x, "{menu} is out of order");
        last_x = x;
    }
}

#[test]
fn attributes_sit_above_objects_on_the_right() {
    let harness = fresh();
    let attributes = harness.get_by_label("Attributes").rect();
    let objects = harness.get_by_label("Objects").rect();
    let viewport = harness.get_by_label("Viewport").rect();
    assert!(
        attributes.min.y < objects.min.y,
        "Attributes must be on top"
    );
    assert!(
        attributes.min.x > viewport.min.x,
        "the managers are right of the viewport"
    );
    assert!((attributes.min.x - objects.min.x).abs() < 2.0, "one column");
}

#[test]
fn coordinates_sit_below_the_viewport() {
    let harness = fresh();
    let viewport = harness.get_by_label("Viewport").rect();
    let coordinates = harness.get_by_label("Coordinates").rect();
    assert!(coordinates.min.y > viewport.min.y);
}

#[test]
fn ctrl_q_asks_to_quit() {
    let mut harness = fresh();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Q);
    harness.run();
    assert_eq!(harness.state().actions, vec![WorkbenchAction::Quit]);
}

#[test]
fn file_quit_asks_to_quit() {
    let mut harness = fresh();
    harness.get_by_label("File").click();
    harness.run();
    // The button's label includes its shortcut, such as "Quit Ctrl+Q".
    harness.get_by_label_contains("Quit").click();
    harness.run();
    assert_eq!(harness.state().actions, vec![WorkbenchAction::Quit]);
}

#[test]
fn clicking_a_mode_selects_it() {
    let mut harness = fresh();
    assert_eq!(harness.state().workbench.mode(), Mode::Model);
    harness.get_by_label("Polygons mode").click();
    harness.run();
    assert_eq!(harness.state().workbench.mode(), Mode::Polygons);
    assert!(harness.query_by_label_contains("Polygons mode:").is_some());
}

#[test]
fn the_modes_menu_selects_a_mode_too() {
    let mut harness = fresh();
    harness.get_by_label("Modes").click();
    harness.run();
    harness.get_by_label("Edges").click();
    harness.run();
    assert_eq!(harness.state().workbench.mode(), Mode::Edges);
}

#[test]
fn commands_that_are_not_built_yet_cannot_be_clicked() {
    let harness = fresh();
    for label in ["Cube", "Move", "Undo", "Snapping", "Lock the X axis"] {
        assert!(
            harness.get_by_label(label).accesskit_node().is_disabled(),
            "{label} should be disabled until it is built"
        );
    }
}

#[test]
fn reset_layout_restores_the_standard_arrangement() {
    // A user's own arrangement: every panel as a tab in one group.
    let mut harness = harness_for(Workbench::new(
        info(),
        Some(DockState::new(vec![
            Panel::Objects,
            Panel::Viewport,
            Panel::Attributes,
            Panel::Coordinates,
        ])),
    ));
    harness.run();
    harness.get_by_label("View").click();
    harness.run();
    harness.get_by_label("Reset panel layout").click();
    harness.run();
    let workbench = &harness.state().workbench;
    assert_eq!(workbench.layout().iter_all_tabs().count(), 4);
    assert!(workbench.note().is_some_and(|n| n.contains("reset")));
    let attributes = harness.get_by_label("Attributes").rect();
    let objects = harness.get_by_label("Objects").rect();
    assert!(attributes.min.y < objects.min.y);
}

#[test]
fn a_broken_saved_layout_is_replaced_and_explained() {
    let broken = DockState::new(vec![Panel::Viewport]);
    let mut harness = harness_for(Workbench::new(info(), Some(broken)));
    harness.run();
    assert!(harness.query_by_label("Attributes").is_some());
    assert!(harness.query_by_label_contains("standard layout").is_some());
}

#[test]
fn help_opens_the_about_box() {
    let mut harness = fresh();
    harness.get_by_label("Help").click();
    harness.run();
    harness.get_by_label("About Test App").click();
    harness.run();
    assert!(harness.state().workbench.is_about_open());
    assert!(harness.query_by_label("Version 9.9.9").is_some());
    assert!(harness.query_by_label_contains("Test Family").is_some());
}

#[test]
fn window_title_shows_name_and_version() {
    let workbench = Workbench::new(info(), None);
    assert_eq!(workbench.window_title(), "Test App 9.9.9");
}

#[test]
fn panels_explain_what_is_coming() {
    let harness = fresh();
    assert!(harness.query_by_label("Nothing selected").is_some());
    assert!(harness.query_by_label("No objects yet").is_some());
}

#[test]
fn the_themed_interface_still_shows_everything() {
    let mut harness = harness_for(Workbench::new(info(), None));
    crate::theme::apply(&harness.ctx);
    harness.run();
    for panel in Panel::ALL {
        assert!(harness.query_by_label(panel.title()).is_some());
    }
    harness.get_by_label("Points mode").click();
    harness.run();
    assert_eq!(harness.state().workbench.mode(), Mode::Points);
}

#[test]
fn coordinates_fit_their_rows_with_no_empty_space_below() {
    // With the real theme, whose rows are taller than egui's defaults.
    let mut harness = harness_for(Workbench::new(info(), None));
    crate::theme::apply(&harness.ctx);
    harness.run();
    harness.run();
    let tab = harness.get_by_label("Coordinates").rect();
    let last_row = harness.get_by_label("Rotation B").rect();
    // The window is 900 high and the status bar 24: the Coordinates panel
    // ends at about 876, a few pixels below its last row.
    assert!(
        876.0 - last_row.max.y < 24.0,
        "{} px of empty space under the last row",
        876.0 - last_row.max.y
    );
    assert!(tab.min.y > 700.0, "Coordinates start at {}", tab.min.y);
}

#[test]
fn a_saved_layout_keeps_the_users_sizes() {
    let saved = crate::layout::default_layout();
    let workbench = Workbench::new(info(), Some(saved));
    assert!(!workbench.fit_pending);
}

#[test]
fn coordinates_show_every_field_as_in_the_mockup() {
    let harness = fresh();
    for group in ["Position", "Size"] {
        for axis in ["X", "Y", "Z"] {
            assert!(harness.query_by_label(&format!("{group} {axis}")).is_some());
        }
    }
    for axis in ["H", "P", "B"] {
        assert!(
            harness
                .query_by_label(&format!("Rotation {axis}"))
                .is_some()
        );
    }
    // Fields fill most of their group's width, as in the mockup.
    assert!(harness.get_by_label("Position X").rect().width() > 150.0);
    assert!(harness.query_by_label_contains("Object (Rel)").is_some());
    assert!(harness.query_by_label("Apply").is_some());
}
