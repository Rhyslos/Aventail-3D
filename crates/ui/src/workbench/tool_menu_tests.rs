//! Tests for the tool menu (`E`) and the command list in the interface:
//! opening it at the mouse, stepping, using a tool by key, click and
//! flick, the tools of each mode, tools that can't be used now, the list
//! look, and the menus that read from the command list.

use super::tests::{State, fresh, harness_for, info, main_menu, with_a_cube};
use super::*;
use crate::TOOL_MENU_KEY;
use crate::commands::CommandId;
use crate::tool_menu::{Ring, ToolMenuStyle};
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
    assert!(
        harness
            .query_by_label_contains("Extrude. Key: D. Arrives in Phase 4")
            .is_some()
    );
    // A tool still to come says so instead.
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(note(&harness), "Extrude: arrives in phase 4.");
    assert!(!harness.state().workbench.is_tool_menu_open());
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
    let mut harness = harness_for(
        with_a_cube(Workbench::new(info(), None)).with_tool_menu_style(ToolMenuStyle::List),
    );
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
fn settings_switch_between_radial_and_list() {
    let mut harness = fresh();
    crate::theme::apply(&harness.ctx);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Comma);
    harness.run();
    assert!(harness.query_by_label("Tool menu (E)").is_some());
    harness.get_by_label("List").click();
    harness.run();
    assert_eq!(
        harness.state().workbench.tool_menu_style(),
        ToolMenuStyle::List
    );
    harness.get_by_label("Restore standard").click();
    harness.run();
    assert_eq!(
        harness.state().workbench.tool_menu_style(),
        ToolMenuStyle::Radial
    );
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
    assert!(extrude.accesskit_node().is_disabled(), "not built yet");
    let label = extrude.accesskit_node().label().unwrap_or_default();
    assert!(label.contains('D'), "shows its key: {label}");
    press(&mut harness, Modifiers::NONE, Key::Escape);
    main_menu(&harness, "Tools").click();
    harness.run();
    assert!(
        harness
            .query_by_label_contains("Move, rotate and scale")
            .is_some()
    );
    assert!(
        harness
            .query_by_label("Tip: press E for the tool menu")
            .is_some()
    );
}
