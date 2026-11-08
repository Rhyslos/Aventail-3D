//! The layout buttons on the menu bar and the setting that brings back
//! the small tabs (decision 0265), with Help's temporary quick switch.

use super::bars::{TRY_BUTTONS_ENTRY, TRY_SMALL_TABS_ENTRY};
use super::layouts::LayoutChoice;
use super::tests::{State, fresh, main_menu};
use crate::palette::Mode;
use crate::viewport::{LayoutSwitch, ViewOptions};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

/// Every layout's name and layout, left to right.
const LAYOUTS: [(&str, LayoutChoice); 4] = [
    ("Standard", LayoutChoice::Standard),
    ("UV Edit", LayoutChoice::UvEdit),
    ("Paint", LayoutChoice::Paint),
    ("Material", LayoutChoice::Material),
];

/// The layout button or tab called `name`: the highest widget with that
/// name, as the menu bar is at the top.
fn on_the_menu_bar<'h>(
    harness: &'h Harness<'static, State>,
    name: &'h str,
) -> egui_kittest::Node<'h> {
    main_menu(harness, name)
}

fn style(harness: &Harness<'static, State>) -> LayoutSwitch {
    harness.state().workbench.viewport.options().layout_switch
}

fn set_style(harness: &mut Harness<'static, State>, style: LayoutSwitch) {
    harness
        .state_mut()
        .workbench
        .viewport
        .options_mut()
        .layout_switch = style;
    harness.run();
    harness.run();
}

/// Clicks each layout in turn, then Standard again, checking the layout
/// and mode each time.
fn click_every_layout(harness: &mut Harness<'static, State>) {
    for (name, choice) in LAYOUTS.into_iter().skip(1).chain([LAYOUTS[0]]) {
        on_the_menu_bar(harness, name).click();
        harness.run();
        harness.run();
        let workbench = &harness.state().workbench;
        assert_eq!(workbench.layout_choice(), choice, "{name}");
        let mode = match choice {
            LayoutChoice::Standard => Mode::Model,
            LayoutChoice::UvEdit => Mode::Uv,
            LayoutChoice::Paint => Mode::Paint,
            LayoutChoice::Material => Mode::Material,
        };
        assert_eq!(workbench.mode, mode, "{name}");
    }
}

#[test]
fn the_buttons_are_the_standard_and_sit_right_of_the_menus() {
    let harness = fresh();
    assert_eq!(style(&harness), LayoutSwitch::Buttons);
    let help = main_menu(&harness, "Help").rect();
    let mut last = help.max.x;
    for (name, _) in LAYOUTS {
        let rect = on_the_menu_bar(&harness, name).rect();
        assert!(rect.min.x > last, "{name} is in order, left to right");
        assert!(
            (rect.center().y - help.center().y).abs() < 1.0,
            "{name} is centred on the menu bar"
        );
        last = rect.max.x;
    }
}

#[test]
fn clicking_each_button_switches_the_layout() {
    let mut harness = fresh();
    click_every_layout(&mut harness);
}

#[test]
fn clicking_the_layout_in_use_changes_nothing() {
    let mut harness = fresh();
    on_the_menu_bar(&harness, "Standard").click();
    harness.run();
    assert_eq!(
        harness.state().workbench.layout_choice(),
        LayoutChoice::Standard
    );
    assert_eq!(harness.state().workbench.mode, Mode::Model);
}

#[test]
fn the_small_tabs_switch_the_layouts_too() {
    let mut harness = fresh();
    set_style(&mut harness, LayoutSwitch::SmallTabs);
    click_every_layout(&mut harness);
}

#[test]
fn the_menu_bar_is_taller_with_the_buttons() {
    let mut harness = fresh();
    let with_buttons = main_menu(&harness, "File").rect().height();
    set_style(&mut harness, LayoutSwitch::SmallTabs);
    let with_tabs = main_menu(&harness, "File").rect().height();
    assert!(
        with_buttons > with_tabs + 3.0,
        "{with_buttons} against {with_tabs}"
    );
    // Back to the buttons: the bar grows again.
    set_style(&mut harness, LayoutSwitch::Buttons);
    assert_eq!(main_menu(&harness, "File").rect().height(), with_buttons);
}

#[test]
fn help_switches_to_the_small_tabs_and_back() {
    let mut harness = fresh();
    main_menu(&harness, "Help").click();
    harness.run();
    assert!(harness.query_by_label(TRY_BUTTONS_ENTRY).is_none());
    harness.get_by_label(TRY_SMALL_TABS_ENTRY).click();
    harness.run();
    harness.run();
    assert_eq!(style(&harness), LayoutSwitch::SmallTabs);
    // The menu closed; the tabs work.
    assert!(harness.query_by_label(TRY_SMALL_TABS_ENTRY).is_none());
    on_the_menu_bar(&harness, "UV Edit").click();
    harness.run();
    assert_eq!(
        harness.state().workbench.layout_choice(),
        LayoutChoice::UvEdit
    );
    // And back, keeping the layout in use.
    main_menu(&harness, "Help").click();
    harness.run();
    harness.get_by_label(TRY_BUTTONS_ENTRY).click();
    harness.run();
    assert_eq!(style(&harness), LayoutSwitch::Buttons);
    assert_eq!(
        harness.state().workbench.layout_choice(),
        LayoutChoice::UvEdit
    );
}

#[test]
fn settings_choose_and_restore_the_buttons() {
    let mut harness = fresh();
    harness.state_mut().workbench.settings.open = true;
    harness.run();
    assert!(
        harness
            .query_by_label(crate::settings::LAYOUTS_HEADING)
            .is_some()
    );
    harness.get_by_label("Small tabs").click();
    harness.run();
    assert_eq!(style(&harness), LayoutSwitch::SmallTabs);
    harness.get_by_label("Buttons with pictures").click();
    harness.run();
    assert_eq!(style(&harness), LayoutSwitch::Buttons);
    harness.get_by_label("Small tabs").click();
    harness.run();
    harness.get_by_label("Restore standard").click();
    harness.run();
    assert_eq!(style(&harness), LayoutSwitch::Buttons);
}

#[test]
fn the_choice_is_remembered_with_the_view_options() {
    for choice in LayoutSwitch::ALL {
        let options = ViewOptions {
            layout_switch: choice,
            ..Default::default()
        };
        let text = ron::to_string(&options).unwrap();
        let back: ViewOptions = ron::from_str(&text).unwrap();
        assert_eq!(back.layout_switch, choice);
    }
    // Options saved before the setting: the buttons, the standard.
    let old: ViewOptions = ron::from_str("(paint_tools_left: true)").unwrap();
    assert_eq!(old.layout_switch, LayoutSwitch::Buttons);
    assert!(old.paint_tools_left);
    // A value no version wrote is refused, never guessed at: the app
    // then starts with the standard view options.
    assert!(ron::from_str::<ViewOptions>("(layout_switch: Huge)").is_err());
}

#[test]
fn the_other_choice_flips_both_ways() {
    for choice in LayoutSwitch::ALL {
        assert_ne!(choice.other(), choice);
        assert_eq!(choice.other().other(), choice);
    }
}

#[test]
fn random_saved_options_never_panic() {
    // A fuzz test: whatever the settings file holds, reading the view
    // options gives options or an error, never a crash, and any options
    // read have one of the two layout switches.
    let pieces = [
        "(",
        ")",
        ",",
        ":",
        "layout_switch",
        "Buttons",
        "SmallTabs",
        "paint_tools_left",
        "true",
        "\"",
        "Some(",
        "[",
        "]",
        "-1",
        "1e99",
        "Small",
        " ",
    ];
    let mut seed: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for _ in 0..3000 {
        let mut text = String::new();
        for _ in 0..(next() % 16) {
            text.push_str(pieces[(next() % pieces.len() as u64) as usize]);
        }
        if let Ok(options) = ron::from_str::<ViewOptions>(&text) {
            assert!(LayoutSwitch::ALL.contains(&options.layout_switch), "{text}");
        }
    }
}
