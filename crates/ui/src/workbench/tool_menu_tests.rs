//! Tests for the tool menu (`E`) and the command list in the interface:
//! opening it at the mouse, stepping, using a tool by key, click and
//! flick, the tools of each mode, tools that can't be used now, the list
//! look, and the menus that read from the command list.

use super::tests::{State, fresh, harness_for, info, main_menu, with_a_cube};
use super::*;
use crate::TOOL_MENU_KEY;
use crate::commands::CommandId;
use crate::tool_menu::{Ring, ToolMenuSettings, ToolMenuStyle};
use egui::{Event, PointerButton, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

/// Where the tests point the mouse: the middle of the window, so the
/// ring opens right there.
const MIDDLE: egui::Pos2 = pos2(720.0, 450.0);

fn press(harness: &mut Harness<'static, State>, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    harness.run();
}

fn point(harness: &mut Harness<'static, State>, at: egui::Pos2) {
    harness.hover_at(at);
    harness.run();
}

/// Opens the menu with a tap of E, the mouse in the middle.
fn open(harness: &mut Harness<'static, State>) {
    point(harness, MIDDLE);
    press(harness, Modifiers::NONE, TOOL_MENU_KEY);
    assert!(harness.state().workbench.is_tool_menu_open());
}

fn highlighted(harness: &Harness<'static, State>) -> Option<CommandId> {
    let bench = &harness.state().workbench;
    bench.tool_menu.highlighted(&bench.tool_menu_entries())
}

fn note(harness: &Harness<'static, State>) -> String {
    harness
        .state()
        .workbench
        .note()
        .unwrap_or_default()
        .to_string()
}

/// A harness with the cube selected.
fn cube_selected() -> Harness<'static, State> {
    let mut harness = fresh();
    let id = harness.state().workbench.document().scene().children(None)[0];
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    harness
}

fn click_at(harness: &mut Harness<'static, State>, at: egui::Pos2) {
    harness.event(Event::PointerMoved(at));
    harness.step();
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        harness.step();
    }
    harness.run();
}

#[test]
fn e_opens_the_tools_for_objects_with_their_keys() {
    let mut harness = fresh();
    open(&mut harness);
    assert!(harness.query_by_label("Tool menu: Move").is_some());
    assert!(harness.query_by_label("Tool menu: Make editable").is_some());
    assert!(harness.query_by_label("Tool menu: Extrude").is_none());
    assert_eq!(highlighted(&harness), Some(CommandId::Move));
    // The middle says what the highlighted tool is.
    assert!(
        harness
            .query_by_label_contains("Move. Key: W. Move the selection")
            .is_some()
    );
}

#[test]
fn tapping_e_steps_through_and_enter_uses_the_tool() {
    let mut harness = fresh();
    open(&mut harness);
    press(&mut harness, Modifiers::NONE, TOOL_MENU_KEY);
    press(&mut harness, Modifiers::NONE, TOOL_MENU_KEY);
    assert_eq!(highlighted(&harness), Some(CommandId::Scale));
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert!(!harness.state().workbench.is_tool_menu_open());
    assert_eq!(harness.state().workbench.tool(), Tool::Scale);
}

#[test]
fn shift_e_and_the_arrow_keys_step_both_ways_and_space_uses() {
    let mut harness = fresh();
    point(&mut harness, MIDDLE);
    press(&mut harness, Modifiers::SHIFT, TOOL_MENU_KEY);
    assert_eq!(
        highlighted(&harness),
        Some(CommandId::ShapeList),
        "opens at the last"
    );
    press(&mut harness, Modifiers::NONE, Key::ArrowRight);
    assert_eq!(highlighted(&harness), Some(CommandId::Move), "round");
    press(&mut harness, Modifiers::NONE, Key::ArrowDown);
    assert_eq!(highlighted(&harness), Some(CommandId::Rotate));
    press(&mut harness, Modifiers::NONE, Key::ArrowUp);
    press(&mut harness, Modifiers::SHIFT, TOOL_MENU_KEY);
    assert_eq!(highlighted(&harness), Some(CommandId::ShapeList));
    // The shape list opens where the menu was.
    press(&mut harness, Modifiers::NONE, Key::Space);
    assert!(harness.state().workbench.shape_picker().is_open());
    assert!(!harness.state().workbench.is_tool_menu_open());
}

#[test]
fn clicking_a_tool_uses_it() {
    let mut harness = fresh();
    open(&mut harness);
    harness.get_by_label("Tool menu: Rotate").click();
    harness.run();
    assert_eq!(harness.state().workbench.tool(), Tool::Rotate);
    assert!(!harness.state().workbench.is_tool_menu_open());
}

#[test]
fn a_click_anywhere_in_a_tools_slice_uses_it() {
    let mut harness = fresh();
    open(&mut harness);
    // Straight up, past the Move button: still Move.
    let ring = Ring::new(MIDDLE, crate::commands::OBJECT_RING.len());
    let beyond = MIDDLE - vec2(0.0, ring.radius + 30.0);
    click_at(&mut harness, beyond);
    assert_eq!(harness.state().workbench.tool(), Tool::Move);
}

#[test]
fn a_click_in_the_middle_or_far_away_closes_it() {
    for at in [MIDDLE, pos2(80.0, 850.0)] {
        let mut harness = fresh();
        open(&mut harness);
        click_at(&mut harness, at);
        assert!(!harness.state().workbench.is_tool_menu_open(), "{at:?}");
        assert_eq!(harness.state().workbench.tool(), Tool::Select);
    }
}

#[test]
fn holding_e_pointing_and_letting_go_uses_the_tool() {
    let mut harness = fresh();
    point(&mut harness, MIDDLE);
    harness.key_down(TOOL_MENU_KEY);
    harness.run();
    assert!(harness.state().workbench.is_tool_menu_open());
    // Towards Scale (third place on the ring).
    let ring = Ring::new(MIDDLE, crate::commands::OBJECT_RING.len());
    let towards = (ring.place(2) - MIDDLE).normalized();
    point(&mut harness, MIDDLE + towards * 80.0);
    assert_eq!(highlighted(&harness), Some(CommandId::Scale));
    harness.key_up(TOOL_MENU_KEY);
    harness.run();
    assert_eq!(harness.state().workbench.tool(), Tool::Scale);
    assert!(!harness.state().workbench.is_tool_menu_open());
}

#[test]
fn letting_go_without_pointing_keeps_it_open() {
    let mut harness = fresh();
    point(&mut harness, MIDDLE);
    harness.key_down(TOOL_MENU_KEY);
    harness.run();
    harness.key_up(TOOL_MENU_KEY);
    harness.run();
    assert!(harness.state().workbench.is_tool_menu_open());
    assert_eq!(harness.state().workbench.tool(), Tool::Select);
}

#[test]
fn esc_closes_it_and_keeps_the_selection() {
    let mut harness = cube_selected();
    open(&mut harness);
    press(&mut harness, Modifiers::NONE, Key::Escape);
    assert!(!harness.state().workbench.is_tool_menu_open());
    assert_eq!(harness.state().workbench.document().selection().len(), 1);
}

#[test]
fn editing_shows_the_modelling_tools_in_their_places() {
    let mut harness = cube_selected();
    press(&mut harness, Modifiers::NONE, Key::Num2);
    open(&mut harness);
    assert!(harness.query_by_label("Tool menu: Extrude").is_some());
    assert!(harness.query_by_label("Tool menu: Hard edges").is_some());
    // No moving or scaling here, and no polygon-only tools.
    assert!(harness.query_by_label("Tool menu: Move").is_none());
    assert!(harness.query_by_label("Tool menu: Inset").is_none());
    assert!(harness.query_by_label_contains("Extrude. Key: D").is_some());
    assert!(
        harness
            .query_by_label_contains("Extrude. Key: D. Arrives")
            .is_none(),
        "Extrude is built"
    );
    // Extrude is highlighted first: Enter makes it the tool in use.
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(harness.state().workbench.tool(), Tool::Extrude);
    assert!(!harness.state().workbench.is_tool_menu_open());
    // A tool still to come says so instead.
    open(&mut harness);
    harness.get_by_label("Tool menu: Hard edges").click();
    harness.run();
    assert_eq!(note(&harness), "Hard edges: arrives in phase 4.");
}

#[test]
fn a_tool_that_cant_be_used_now_says_why() {
    let mut harness = fresh();
    open(&mut harness);
    harness.get_by_label("Tool menu: Ungroup").click();
    harness.run();
    assert_eq!(note(&harness), "Nothing is selected.");
    let mut harness = cube_selected();
    open(&mut harness);
    harness.get_by_label("Tool menu: Ungroup").click();
    harness.run();
    assert_eq!(
        note(&harness),
        "Nothing to ungroup: only groups can be ungrouped."
    );
}

#[test]
fn tools_from_the_menu_change_the_scene_like_their_keys() {
    let mut harness = cube_selected();
    open(&mut harness);
    harness.get_by_label("Tool menu: Make editable").click();
    harness.run();
    let doc = harness.state().workbench.document();
    let id = doc.selection()[0];
    assert!(matches!(
        doc.scene().object(id).unwrap().kind,
        ObjectKind::Mesh(_)
    ));
    open(&mut harness);
    harness.get_by_label("Tool menu: Group objects").click();
    harness.run();
    let doc = harness.state().workbench.document();
    assert_eq!(doc.scene().children(None).len(), 1);
    let top = doc.scene().children(None)[0];
    assert_eq!(doc.scene().object(top).unwrap().kind, ObjectKind::Group);
    open(&mut harness);
    harness.get_by_label("Tool menu: New group").click();
    harness.run();
    assert_eq!(
        harness
            .state()
            .workbench
            .document()
            .scene()
            .children(None)
            .len(),
        2
    );
}

#[test]
fn the_uv_mode_has_no_tools_yet() {
    let mut harness = fresh();
    harness.state_mut().workbench.mode = Mode::Uv;
    point(&mut harness, MIDDLE);
    press(&mut harness, Modifiers::NONE, TOOL_MENU_KEY);
    assert!(!harness.state().workbench.is_tool_menu_open());
    assert!(note(&harness).contains("Phase 8"));
}

#[test]
fn the_list_look_holds_the_same_tools() {
    let mut harness = harness_for(with_a_cube(Workbench::new(info(), None)).with_tool_menu(
        ToolMenuSettings {
            style: ToolMenuStyle::List,
            ..ToolMenuSettings::default()
        },
    ));
    harness.run();
    open(&mut harness);
    assert!(harness.query_by_label("Tools for objects").is_some());
    assert!(harness.query_by_label("Tool menu: Move").is_some());
    press(&mut harness, Modifiers::NONE, TOOL_MENU_KEY);
    assert_eq!(highlighted(&harness), Some(CommandId::Rotate));
    harness.get_by_label("Tool menu: Scale").click();
    harness.run();
    assert_eq!(harness.state().workbench.tool(), Tool::Scale);
}

#[test]
fn e_and_b_never_show_at_once() {
    let mut harness = fresh();
    open(&mut harness);
    press(&mut harness, Modifiers::NONE, crate::SHAPE_LIST_KEY);
    assert!(harness.state().workbench.shape_picker().is_open());
    assert!(!harness.state().workbench.is_tool_menu_open());
    press(&mut harness, Modifiers::NONE, TOOL_MENU_KEY);
    assert!(harness.state().workbench.is_tool_menu_open());
    assert!(!harness.state().workbench.shape_picker().is_open());
}

#[test]
fn ctrl_e_still_makes_editable_and_e_types_into_names() {
    let mut harness = cube_selected();
    point(&mut harness, MIDDLE);
    press(&mut harness, Modifiers::COMMAND, Key::E);
    assert!(!harness.state().workbench.is_tool_menu_open());
    let doc = harness.state().workbench.document();
    assert!(matches!(
        doc.scene().object(doc.selection()[0]).unwrap().kind,
        ObjectKind::Mesh(_)
    ));
    harness.get_by_label("Object name").click();
    harness.run();
    press(&mut harness, Modifiers::NONE, TOOL_MENU_KEY);
    assert!(!harness.state().workbench.is_tool_menu_open());
}

#[test]
fn settings_have_a_radial_menu_section_with_every_look_and_switch() {
    let mut harness = fresh();
    crate::theme::apply(&harness.ctx);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Comma);
    harness.run();
    assert!(harness.query_by_label("Radial menu").is_some());
    for style in ToolMenuStyle::ALL {
        assert!(harness.query_by_label(style.name()).is_some(), "{style:?}");
    }
    harness.get_by_label("List").click();
    harness.run();
    assert_eq!(
        harness.state().workbench.tool_menu_settings().style,
        ToolMenuStyle::List
    );
    harness.get_by_label("Inner edge").click();
    harness.get_by_label("Show keys around the ring").click();
    harness.get_by_label("Tap E to open").click();
    harness.run();
    assert_eq!(
        harness.state().workbench.tool_menu_settings(),
        ToolMenuSettings {
            style: ToolMenuStyle::InnerEdge,
            show_keys: false,
            tap_to_open: false,
        }
    );
    harness.get_by_label("Restore standard").click();
    harness.run();
    assert_eq!(
        harness.state().workbench.tool_menu_settings(),
        ToolMenuSettings::default()
    );
}

/// A harness whose tool menu opens only while E is held.
fn hold_only() -> Harness<'static, State> {
    let mut harness = harness_for(with_a_cube(Workbench::new(info(), None)).with_tool_menu(
        ToolMenuSettings {
            tap_to_open: false,
            ..ToolMenuSettings::default()
        },
    ));
    harness.run();
    harness
}

#[test]
fn with_tapping_off_a_tap_does_nothing() {
    let mut harness = hold_only();
    point(&mut harness, MIDDLE);
    press(&mut harness, Modifiers::NONE, TOOL_MENU_KEY);
    assert!(!harness.state().workbench.is_tool_menu_open());
    assert_eq!(harness.state().workbench.tool(), Tool::Select);
}

#[test]
fn with_tapping_off_the_menu_shows_while_held() {
    let mut harness = hold_only();
    point(&mut harness, MIDDLE);
    harness.key_down(TOOL_MENU_KEY);
    harness.run();
    assert!(harness.state().workbench.is_tool_menu_open());
    // Letting go without pointing at a tool closes it, doing nothing.
    harness.key_up(TOOL_MENU_KEY);
    harness.run();
    assert!(!harness.state().workbench.is_tool_menu_open());
    assert_eq!(harness.state().workbench.tool(), Tool::Select);
    // Letting go after pointing uses the tool.
    harness.key_down(TOOL_MENU_KEY);
    harness.run();
    let ring = Ring::new(MIDDLE, crate::commands::OBJECT_RING.len());
    let towards = (ring.place(1) - MIDDLE).normalized();
    point(&mut harness, MIDDLE + towards * 90.0);
    harness.key_up(TOOL_MENU_KEY);
    harness.run();
    assert_eq!(harness.state().workbench.tool(), Tool::Rotate);
}

#[test]
fn every_ring_look_works_the_same() {
    for style in [
        ToolMenuStyle::ContinuousRing,
        ToolMenuStyle::InnerEdge,
        ToolMenuStyle::Outlined,
        ToolMenuStyle::SolidTiles,
    ] {
        let mut harness = harness_for(with_a_cube(Workbench::new(info(), None)).with_tool_menu(
            ToolMenuSettings {
                style,
                ..ToolMenuSettings::default()
            },
        ));
        harness.run();
        open(&mut harness);
        assert!(
            harness.query_by_label("Tool menu: Move").is_some(),
            "{style:?}"
        );
        assert!(harness.query_by_label_contains("Move. Key: W").is_some());
        harness.get_by_label("Tool menu: Scale").click();
        harness.run();
        assert_eq!(harness.state().workbench.tool(), Tool::Scale, "{style:?}");
        // A click in the middle (the cancel sign) closes it.
        open(&mut harness);
        click_at(&mut harness, MIDDLE + vec2(25.0, 0.0));
        assert!(!harness.state().workbench.is_tool_menu_open(), "{style:?}");
    }
}

#[test]
fn every_mode_uses_the_same_ring() {
    // The same places, at the same size: Delete sits in the same spot in
    // the Model ring and the modelling ring.
    let mut harness = cube_selected();
    open(&mut harness);
    let model = harness.get_by_label("Tool menu: Delete").rect();
    press(&mut harness, Modifiers::NONE, Key::Escape);
    press(&mut harness, Modifiers::NONE, Key::Num1);
    open(&mut harness);
    let points = harness.get_by_label("Tool menu: Delete").rect();
    assert!((model.center() - points.center()).length() < 0.5);
    assert!((model.size() - points.size()).length() < 0.5);
}

#[test]
fn the_mesh_and_tools_menus_read_from_the_command_list() {
    let mut harness = fresh();
    main_menu(&harness, "Mesh").click();
    harness.run();
    for command in crate::commands::MODELLING_RING {
        let name = command.name();
        assert!(harness.query_by_label_contains(name).is_some(), "{name}");
    }
    let extrude = harness.get_by_label_contains("Extrude");
    assert!(!extrude.accesskit_node().is_disabled(), "built");
    let label = extrude.accesskit_node().label().unwrap_or_default();
    assert!(label.contains('D'), "shows its key: {label}");
    let bevel = harness.get_by_label_contains("Bevel");
    assert!(bevel.accesskit_node().is_disabled(), "not built yet");
    press(&mut harness, Modifiers::NONE, Key::Escape);
    main_menu(&harness, "Tools").click();
    harness.run();
    assert!(harness.query_by_label_contains("Transform").is_some());
    assert!(
        harness
            .query_by_label("Tip: press E for the tool menu")
            .is_some()
    );
}

#[test]
fn pointing_at_the_middle_cancels_instead_of_using_the_last_tool() {
    // Regression: moving the mouse back into the middle kept the last
    // tool lit, and Enter (or letting go of E) still used it.
    let mut harness = fresh();
    open(&mut harness);
    let ring = Ring::new(MIDDLE, crate::commands::RING_PLACES);
    let towards = (ring.place(2) - MIDDLE).normalized();
    point(&mut harness, MIDDLE + towards * 90.0);
    assert_eq!(highlighted(&harness), Some(CommandId::Scale));
    point(&mut harness, MIDDLE + vec2(6.0, 4.0));
    assert_eq!(
        highlighted(&harness),
        None,
        "nothing lit on the cancel sign"
    );
    assert!(
        harness
            .query_by_label(crate::tool_menu::CANCEL_TEXT)
            .is_some()
    );
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert!(!harness.state().workbench.is_tool_menu_open());
    assert_eq!(harness.state().workbench.tool(), Tool::Select);
}

#[test]
fn letting_go_of_e_on_the_middle_does_nothing() {
    let mut harness = fresh();
    point(&mut harness, MIDDLE);
    harness.key_down(TOOL_MENU_KEY);
    harness.run();
    let ring = Ring::new(MIDDLE, crate::commands::RING_PLACES);
    let towards = (ring.place(1) - MIDDLE).normalized();
    point(&mut harness, MIDDLE + towards * 90.0);
    assert_eq!(highlighted(&harness), Some(CommandId::Rotate));
    point(&mut harness, MIDDLE - vec2(5.0, 5.0));
    harness.key_up(TOOL_MENU_KEY);
    harness.run();
    assert!(!harness.state().workbench.is_tool_menu_open());
    assert_eq!(harness.state().workbench.tool(), Tool::Select);
    // Stepping with E after pointing at the middle picks a tool again.
    open(&mut harness);
    point(&mut harness, MIDDLE + vec2(3.0, 0.0));
    press(&mut harness, Modifiers::NONE, TOOL_MENU_KEY);
    assert!(highlighted(&harness).is_some());
}
