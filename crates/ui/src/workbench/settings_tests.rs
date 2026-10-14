//! Tests for the Settings window: opening it, and each setting changing
//! the same value the rest of the window uses.

use super::tests::{State, fresh, main_menu};
use super::*;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn open(harness: &mut Harness<'static, State>) {
    // The window's title uses the app's heading font, as in the app.
    crate::theme::apply(&harness.ctx);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Comma);
    harness.run();
    assert!(harness.state().workbench.is_settings_open());
}

fn section(harness: &mut Harness<'static, State>, name: &str) {
    harness
        .get_by_label(&format!("Settings section: {name}"))
        .click();
    harness.run();
}

#[test]
fn ctrl_comma_and_the_edit_menu_open_settings() {
    let mut harness = fresh();
    assert!(!harness.state().workbench.is_settings_open());
    open(&mut harness);
    assert!(harness.query_by_label("Settings").is_some(), "the title");
    // Ctrl + , again closes it.
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Comma);
    harness.run();
    assert!(!harness.state().workbench.is_settings_open());
    main_menu(&harness, "Edit").click();
    harness.run();
    harness.get_by_label_contains("Settings…").click();
    harness.run();
    assert!(harness.state().workbench.is_settings_open());
}

#[test]
fn settings_have_interface_viewport_and_undo_sections() {
    let mut harness = fresh();
    open(&mut harness);
    for name in ["Interface", "Viewport", "Undo"] {
        assert!(
            harness
                .query_by_label(&format!("Settings section: {name}"))
                .is_some(),
            "{name}"
        );
    }
    // Interface is shown first.
    assert!(harness.query_by_label("Petrol blue").is_some());
}

#[test]
fn red_is_the_standard_accent_and_settings_change_it() {
    let mut harness = fresh();
    assert_eq!(
        harness.state().workbench.accent(),
        crate::theme::Accent::Red
    );
    open(&mut harness);
    harness.get_by_label("Petrol blue").click();
    harness.run();
    harness.run();
    assert_eq!(
        harness.state().workbench.accent(),
        crate::theme::Accent::PetrolBlue
    );
    assert_eq!(
        crate::theme::accent(&harness.ctx),
        crate::theme::Accent::PetrolBlue
    );
}

#[test]
fn the_accent_is_no_longer_in_the_window_menu() {
    let mut harness = fresh();
    main_menu(&harness, "Window").click();
    harness.run();
    assert!(harness.query_by_label_contains("Accent colour").is_none());
}

#[test]
fn units_change_here_and_beside_the_shape_title_alike() {
    let mut harness = fresh();
    open(&mut harness);
    harness.get_by_label("Centimetres (cm)").click();
    harness.run();
    assert_eq!(harness.state().workbench.units(), crate::Units::Centimetres);
}

#[test]
fn navigation_and_grid_settings_change_the_viewport() {
    let mut harness = fresh();
    open(&mut harness);
    section(&mut harness, "Viewport");
    harness.get_by_label("Middle orbits").click();
    harness.run();
    harness.get_by_label("Invert orbit direction").click();
    harness.run();
    harness
        .get_by_label(crate::GridDetail::HalfMetre.label())
        .click();
    harness.run();
    let options = harness.state().workbench.view_options();
    assert_eq!(options.navigation.scheme, tools::Scheme::MiddleOrbits);
    assert!(options.navigation.invert_orbit);
    assert_eq!(options.grid_detail, crate::GridDetail::HalfMetre);
}

#[test]
fn the_undo_section_shows_whats_in_use_and_changes_the_limit() {
    let mut harness = fresh();
    open(&mut harness);
    section(&mut harness, "Undo");
    // The fresh window's cube and its unselecting are two steps.
    assert!(
        harness
            .query_by_label_contains("In use: 1 KB of 2 GB (2 steps)")
            .is_some()
    );
    harness.get_by_label("512 MB").click();
    harness.run();
    let workbench = &harness.state().workbench;
    assert_eq!(workbench.undo_memory(), crate::UndoMemory::Mb512);
    assert_eq!(
        workbench.document().history_limit(),
        crate::UndoMemory::Mb512.limit()
    );
}

#[test]
fn undo_memory_is_no_longer_in_the_edit_menu() {
    let mut harness = fresh();
    main_menu(&harness, "Edit").click();
    harness.run();
    assert!(harness.query_by_label_contains("Undo memory").is_none());
}

#[test]
fn restore_standard_puts_everything_back() {
    let workbench = Workbench::new(super::tests::info(), None)
        .with_accent(crate::theme::Accent::Olive)
        .with_units(crate::Units::Centimetres)
        .with_undo_memory(crate::UndoMemory::Unlimited);
    let mut harness = super::tests::harness_for(workbench);
    harness.run();
    open(&mut harness);
    harness.get_by_label("Restore standard").click();
    harness.run();
    let workbench = &harness.state().workbench;
    assert_eq!(workbench.accent(), crate::theme::Accent::Red);
    assert_eq!(workbench.units(), crate::Units::Metres);
    assert_eq!(workbench.undo_memory(), crate::UndoMemory::Gb2);
    assert_eq!(
        workbench.document().history_limit(),
        scene::HistoryLimit::STANDARD
    );
    assert_eq!(
        workbench.view_options().navigation,
        tools::NavigationSettings::default()
    );
}

#[test]
fn the_close_button_closes_settings() {
    let mut harness = fresh();
    open(&mut harness);
    harness.get_by_label("Close window").click();
    harness.run();
    assert!(!harness.state().workbench.is_settings_open());
}

#[test]
fn the_dissolve_warning_can_be_switched_back_on() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .viewport_mut()
        .options_mut()
        .dissolve_without_warning = true;
    open(&mut harness);
    section(&mut harness, "Interface");
    harness
        .get_by_label("Ask before dissolving across a sharp corner")
        .click();
    harness.run();
    assert!(
        !harness
            .state()
            .workbench
            .view_options()
            .dissolve_without_warning
    );
}
