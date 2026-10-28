//! Tests for the modifier lists (decision 0168): an object's modifiers as
//! rows under it in the Objects panel and after its layers in Edit
//! layers; a click shows only that modifier's settings; the tick, a drag,
//! Delete and the right-click menu.

use super::tests::{State, fresh};
use crate::attributes::modifier_rows::ON_LABEL;
use crate::modifier_list::{BAKE_UP_TO, DUPLICATE, ROW_PREFIX, TICK_PREFIX};
use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use scene::{Command, Modifier, ModifierKind, ObjectId, RemeshOptions, SubdivideOptions};

type H = Harness<'static, State>;

fn cube(harness: &H) -> ObjectId {
    let scene = harness.state().workbench.document().scene();
    scene.walk()[0].0
}

fn modifiers(harness: &H, id: ObjectId) -> Vec<Modifier> {
    harness
        .state()
        .workbench
        .document()
        .scene()
        .modifiers(id)
        .to_vec()
}

fn run(harness: &mut H, command: Command) {
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(command)
        .unwrap();
    harness.run();
}

fn subdivide() -> ModifierKind {
    ModifierKind::Subdivide(SubdivideOptions::default())
}

fn remesh() -> ModifierKind {
    ModifierKind::Remesh(RemeshOptions {
        voxel_size: 20.0,
        smooth: 0,
    })
}

/// A cube with a Subdivide then a Remesh, nothing selected.
fn cube_with_two() -> (H, ObjectId) {
    let mut harness = fresh();
    let id = cube(&harness);
    for kind in [subdivide(), remesh()] {
        run(
            &mut harness,
            Command::AddModifier {
                ids: vec![id],
                kind,
            },
        );
    }
    harness
        .state_mut()
        .workbench
        .document_mut()
        .clear_selection();
    harness.run();
    (harness, id)
}

fn row_label(name: &str) -> String {
    format!("{}{name}", crate::text::tr(ROW_PREFIX))
}

fn row(harness: &H, name: &str) -> egui::Rect {
    harness.get_by_label(&row_label(name)).rect()
}

fn click_at(harness: &mut H, at: Pos2, button: PointerButton) {
    harness.event(Event::PointerMoved(at));
    harness.step();
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: at,
            button,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    harness.step();
    harness.run();
}

fn drag(harness: &mut H, from: Pos2, to: Pos2) {
    harness.event(Event::PointerMoved(from));
    harness.step();
    harness.event(Event::PointerButton {
        pos: from,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    for i in 1..=8 {
        harness.event(Event::PointerMoved(from + (to - from) * (i as f32 / 8.0)));
        harness.step();
    }
    harness.event(Event::PointerButton {
        pos: to,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
}

fn picked(harness: &H) -> crate::modifier_list::Picked {
    let workbench = &harness.state().workbench;
    crate::modifier_list::picked_place(workbench.document(), workbench.attributes.picked_modifier)
        .map(|(id, index)| (id, modifiers(harness, id)[index].id))
}

#[test]
fn the_objects_panel_lists_them_under_their_object_in_order() {
    let (harness, _) = cube_with_two();
    let object = harness.get_by_label("Cube").rect();
    let first = row(&harness, "Subdivide");
    let second = row(&harness, "Remesh");
    assert!(object.max.y <= first.min.y + 0.5, "under the object");
    assert!(first.max.y <= second.min.y + 0.5, "first to last");
    // Their icons sit a step in from the object's.
    assert!(first.min.x >= object.min.x - 0.5);
    // Each has its tick, ticked.
    let tick_label = format!("{}Remesh", crate::text::tr(TICK_PREFIX));
    let tick = harness.get_by_label(&tick_label);
    assert_eq!(
        tick.accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::True)
    );
}

#[test]
fn clicking_a_modifier_selects_its_object_and_shows_only_its_settings() {
    let (mut harness, id) = cube_with_two();
    let at = row(&harness, "Remesh").center();
    click_at(&mut harness, at, PointerButton::Primary);
    assert_eq!(harness.state().workbench.document().selection(), vec![id]);
    let remesh_id = modifiers(&harness, id)[1].id;
    assert_eq!(picked(&harness), Some((id, remesh_id)));
    assert_eq!(harness.get_all_by_label(ON_LABEL).count(), 1);
    assert!(harness.query_all_by_label("Voxel size").next().is_some());
    assert!(
        harness
            .query_all_by_label("Levels for export")
            .next()
            .is_none(),
        "not the other one's"
    );
    assert!(
        harness.query_all_by_label("Name").next().is_none(),
        "not the object's"
    );
    // Clicking the object's own row goes back to its settings.
    let at = harness.get_by_label("Cube").rect().center();
    click_at(&mut harness, at, PointerButton::Primary);
    assert_eq!(picked(&harness), None);
    assert!(harness.query_all_by_label("Voxel size").next().is_none());
    assert!(harness.query_by_label(ON_LABEL).is_none());
}

#[test]
fn a_click_in_the_viewport_goes_back_to_the_object() {
    let (mut harness, _) = cube_with_two();
    let at = row(&harness, "Subdivide").center();
    click_at(&mut harness, at, PointerButton::Primary);
    assert!(picked(&harness).is_some());
    let spot = super::tests::viewport_spot(&harness);
    click_at(&mut harness, spot, PointerButton::Primary);
    assert_eq!(picked(&harness), None);
}

#[test]
fn the_tick_switches_a_modifier_on_and_off() {
    let (mut harness, id) = cube_with_two();
    harness
        .get_by_label(&format!("{}Subdivide", crate::text::tr(TICK_PREFIX)))
        .click();
    harness.run();
    assert!(!modifiers(&harness, id)[0].on);
    assert!(modifiers(&harness, id)[1].on);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Switch Subdivide off")
    );
    harness
        .get_by_label(&format!("{}Subdivide", crate::text::tr(TICK_PREFIX)))
        .click();
    harness.run();
    assert!(modifiers(&harness, id)[0].on);
}

#[test]
fn dragging_a_row_changes_the_order() {
    let (mut harness, id) = cube_with_two();
    let from = row(&harness, "Subdivide").center();
    let below = row(&harness, "Remesh");
    drag(&mut harness, from, Pos2::new(from.x, below.max.y - 2.0));
    let kinds: Vec<&str> = modifiers(&harness, id)
        .iter()
        .map(|m| m.kind.name())
        .collect();
    assert_eq!(kinds, ["Remesh", "Subdivide"]);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Move Subdivide down")
    );
    // A drag that ends where it started changes nothing.
    let steps = harness.state().workbench.document().undo_steps();
    let at = row(&harness, "Remesh").center();
    drag(&mut harness, at, at + egui::vec2(30.0, 2.0));
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
}

#[test]
fn delete_removes_the_modifier_picked_not_its_object() {
    let (mut harness, id) = cube_with_two();
    let at = row(&harness, "Subdivide").center();
    click_at(&mut harness, at, PointerButton::Primary);
    harness.key_press(Key::Delete);
    harness.run();
    assert!(harness.state().workbench.document().scene().contains(id));
    let kinds: Vec<&str> = modifiers(&harness, id)
        .iter()
        .map(|m| m.kind.name())
        .collect();
    assert_eq!(kinds, ["Remesh"]);
    // Nothing picked any more (it's gone): Backspace now deletes the cube,
    // as before.
    assert_eq!(picked(&harness), None);
    harness.key_press(Key::Backspace);
    harness.run();
    assert!(!harness.state().workbench.document().scene().contains(id));
}

#[test]
fn the_right_click_menu_duplicates_moves_bakes_and_removes() {
    let (mut harness, id) = cube_with_two();
    let open_menu = |harness: &mut H, name: &str| {
        let at = harness
            .get_all_by_label(&row_label(name))
            .next()
            .unwrap()
            .rect()
            .center();
        click_at(harness, at, PointerButton::Secondary);
    };
    open_menu(&mut harness, "Subdivide");
    harness.get_by_label(crate::text::tr(DUPLICATE)).click();
    harness.run();
    let kinds =
        |h: &H| -> Vec<&'static str> { modifiers(h, id).iter().map(|m| m.kind.name()).collect() };
    assert_eq!(kinds(&harness), ["Subdivide", "Subdivide", "Remesh"]);
    open_menu(&mut harness, "Remesh");
    harness.get_by_label("Move up").click();
    harness.run();
    assert_eq!(kinds(&harness), ["Subdivide", "Remesh", "Subdivide"]);
    // Bake the first: it becomes part of the shape; the rest stay.
    open_menu(&mut harness, "Subdivide");
    harness.get_by_label(crate::text::tr(BAKE_UP_TO)).click();
    harness.run();
    assert_eq!(kinds(&harness), ["Remesh", "Subdivide"]);
    assert!(matches!(
        harness
            .state()
            .workbench
            .document()
            .scene()
            .object(id)
            .unwrap()
            .kind,
        scene::ObjectKind::Mesh(_)
    ));
    open_menu(&mut harness, "Remesh");
    // Its label ends with the key: "Remove Delete".
    harness.get_by_label_contains("Remove").click();
    harness.run();
    assert_eq!(kinds(&harness), ["Subdivide"]);
}

#[test]
fn a_modifier_added_from_the_palette_shows_its_settings_at_once() {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    harness.get_by_label("Subdivide").click();
    harness.run();
    let added = modifiers(&harness, id)[0].id;
    assert_eq!(picked(&harness), Some((id, added)));
    assert!(
        harness
            .query_all_by_label("Levels for export")
            .next()
            .is_some()
    );
}

#[test]
fn folding_the_object_hides_its_modifiers() {
    let (mut harness, _) = cube_with_two();
    assert!(harness.query_by_label(&row_label("Remesh")).is_some());
    harness.get_by_label("Fold Cube").click();
    harness.run();
    assert!(harness.query_by_label(&row_label("Remesh")).is_none());
    harness.get_by_label("Unfold Cube").click();
    harness.run();
    assert!(harness.query_by_label(&row_label("Remesh")).is_some());
}

#[test]
fn the_edit_layers_panel_leaves_them_to_the_objects_panel() {
    // One home for modifiers (decision 0174): Edit layers lists layers and
    // edits only.
    let (mut harness, id) = cube_with_two();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    crate::layout::show_tab(
        &mut harness.state_mut().workbench.dock,
        crate::panel::Panel::EditLayers,
    );
    harness.run();
    // The Objects panel is behind it now: no modifier rows anywhere.
    assert_eq!(harness.query_all_by_label(&row_label("Remesh")).count(), 0);
}

#[test]
fn a_tools_settings_stay_above_the_modifier_picked() {
    let (mut harness, id) = cube_with_two();
    super::tests::open_modifier(&mut harness, id, 1);
    harness.key_press(Key::Num2);
    harness.run();
    harness
        .state_mut()
        .workbench
        .set_tool(crate::transforming::Tool::LoopCut);
    harness.run();
    let cuts = harness.get_all_by_label("Cuts").next().unwrap().rect();
    let switch = harness.get_by_label(ON_LABEL).rect();
    assert!(cuts.max.y < switch.min.y, "the tool's settings first");
}
