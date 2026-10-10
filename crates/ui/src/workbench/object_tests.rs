//! Tests for the Object Manager, undo and redo, run through the real
//! interface without a window.

use super::tests::{State, TestRenderer, fresh, harness_with, info, main_menu, quick_harness};
use super::*;
use egui::{Event, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use scene::ObjectId;

/// The objects at the top of the tree, by name, top to bottom.
fn top_level_names(harness: &Harness<'static, State>) -> Vec<String> {
    let scene = harness.state().workbench.document().scene();
    scene
        .children(None)
        .iter()
        .map(|id| scene.object(*id).unwrap().name.clone())
        .collect()
}

fn id_of(harness: &Harness<'static, State>, name: &str) -> ObjectId {
    let scene = harness.state().workbench.document().scene();
    scene
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| scene.object(*id).unwrap().name == name)
        .unwrap_or_else(|| panic!("no object {name}"))
}

fn selected_names(harness: &Harness<'static, State>) -> Vec<String> {
    let doc = harness.state().workbench.document();
    doc.selection()
        .into_iter()
        .map(|id| doc.scene().object(id).unwrap().name.clone())
        .collect()
}

/// Where a row is on screen.
fn row(harness: &Harness<'static, State>, name: &str) -> egui::Rect {
    harness.get_by_label(name).rect()
}

/// Clicks with `button` at `at`, holding `modifiers`.
fn click_at(
    harness: &mut Harness<'static, State>,
    at: Pos2,
    button: PointerButton,
    modifiers: Modifiers,
) {
    harness.event(Event::ModifiersChanged(modifiers));
    harness.event(Event::PointerMoved(at));
    harness.step();
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: at,
            button,
            pressed,
            modifiers,
        });
    }
    harness.step();
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    harness.run();
}

fn click_row(harness: &mut Harness<'static, State>, name: &str, modifiers: Modifiers) {
    let at = row(harness, name).center();
    click_at(harness, at, PointerButton::Primary, modifiers);
}

fn double_click_row(harness: &mut Harness<'static, State>, name: &str) {
    let at = row(harness, name).center();
    harness.event(Event::PointerMoved(at));
    harness.step();
    for _ in 0..2 {
        for pressed in [true, false] {
            harness.event(Event::PointerButton {
                pos: at,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            });
            harness.step();
        }
    }
    harness.run();
}

/// Drags from one point to another with the left button.
fn drag(harness: &mut Harness<'static, State>, from: Pos2, to: Pos2) {
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

fn key(harness: &mut Harness<'static, State>, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    harness.run();
}

/// A fresh window with a group made from the Create menu, above the cube.
fn with_a_group() -> Harness<'static, State> {
    let mut harness = fresh();
    main_menu(&harness, "Create").click();
    harness.run();
    harness.get_by_label("Group").click();
    harness.run();
    assert_eq!(top_level_names(&harness), ["Group", "Test cube"]);
    harness
}

/// Whether the last frame drew the test cube.
fn cube_drawn(harness: &mut Harness<'static, State>) -> bool {
    harness.state_mut().renderer.drawn.clear();
    harness.step();
    harness.state().renderer.drawn.iter().all(|v| v.test_cube)
}

const SHIFT: Modifiers = Modifiers::SHIFT;

// ------------------------------------------------------------- the tree ---

#[test]
fn the_scene_starts_with_the_test_cube_listed() {
    let mut harness = fresh();
    assert!(harness.query_by_label("Test cube").is_some());
    assert!(
        harness.query_by_label("Hide Test cube").is_some(),
        "with its eye"
    );
    assert!(selected_names(&harness).is_empty());
    assert!(cube_drawn(&mut harness));
}

#[test]
fn clicking_a_row_selects_it_and_the_status_bar_counts() {
    let mut harness = fresh();
    click_row(&mut harness, "Test cube", Modifiers::NONE);
    assert_eq!(selected_names(&harness), ["Test cube"]);
    assert!(
        harness
            .query_by_label_contains("1 object selected")
            .is_some()
    );
}

#[test]
fn ctrl_click_adds_and_shift_click_selects_a_run() {
    let mut harness = with_a_group();
    // Three objects: Group 2, Group, Test cube.
    harness.get_by_label("Create group").click();
    harness.run();
    assert_eq!(top_level_names(&harness), ["Group 2", "Group", "Test cube"]);

    click_row(&mut harness, "Group 2", Modifiers::NONE);
    click_row(&mut harness, "Test cube", Modifiers::COMMAND);
    assert_eq!(selected_names(&harness), ["Group 2", "Test cube"]);
    click_row(&mut harness, "Test cube", Modifiers::COMMAND);
    assert_eq!(
        selected_names(&harness),
        ["Group 2"],
        "Ctrl+click again takes it out"
    );

    // Shift+click selects from the last row clicked.
    click_row(&mut harness, "Group 2", Modifiers::NONE);
    click_row(&mut harness, "Test cube", SHIFT);
    assert_eq!(selected_names(&harness), ["Group 2", "Group", "Test cube"]);
}

#[test]
fn clicking_empty_space_or_esc_selects_nothing() {
    let mut harness = fresh();
    click_row(&mut harness, "Test cube", Modifiers::NONE);
    let below = harness.get_by_label("Empty space below the objects").rect();
    click_at(
        &mut harness,
        below.center(),
        PointerButton::Primary,
        Modifiers::NONE,
    );
    assert!(selected_names(&harness).is_empty());
    click_row(&mut harness, "Test cube", Modifiers::NONE);
    key(&mut harness, Modifiers::NONE, Key::Escape);
    assert!(selected_names(&harness).is_empty());
}

#[test]
fn the_fold_arrow_hides_and_shows_whats_inside() {
    let mut harness = with_a_group();
    let cube = id_of(&harness, "Test cube");
    let group = id_of(&harness, "Group");
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Move {
            ids: vec![cube],
            parent: Some(group),
            index: 0,
        })
        .unwrap();
    harness.run();
    assert!(row(&harness, "Test cube").min.y > row(&harness, "Group").min.y);
    harness.get_by_label("Fold Group").click();
    harness.run();
    assert!(harness.query_by_label("Test cube").is_none(), "folded away");
    harness.get_by_label("Unfold Group").click();
    harness.run();
    assert!(harness.query_by_label("Test cube").is_some());
}

// ---------------------------------------------------- create and delete ---

#[test]
fn new_groups_go_on_top_and_are_selected() {
    let harness = with_a_group();
    assert_eq!(selected_names(&harness), ["Group"]);
}

#[test]
fn the_palette_and_object_menu_create_groups_too() {
    let mut harness = fresh();
    harness.get_by_label("Create group").click();
    harness.run();
    assert_eq!(top_level_names(&harness), ["Group", "Test cube"]);
    harness.get_by_label("Object").click();
    harness.run();
    harness.get_by_label("New group").click();
    harness.run();
    assert_eq!(top_level_names(&harness), ["Group 2", "Group", "Test cube"]);
}

#[test]
fn delete_and_backspace_delete_the_selection() {
    for delete in [Key::Delete, Key::Backspace] {
        let mut harness = fresh();
        click_row(&mut harness, "Test cube", Modifiers::NONE);
        key(&mut harness, Modifiers::NONE, delete);
        assert!(top_level_names(&harness).is_empty(), "{delete:?}");
        assert!(harness.query_by_label("No objects yet").is_some());
        assert!(!cube_drawn(&mut harness), "gone from the viewport");
    }
}

#[test]
fn delete_with_nothing_selected_does_nothing() {
    let mut harness = fresh();
    key(&mut harness, Modifiers::NONE, Key::Delete);
    assert_eq!(top_level_names(&harness), ["Test cube"]);
    assert_eq!(harness.state().workbench.document().undo_label(), None);
}

#[test]
fn the_right_click_menu_deletes_the_row_clicked() {
    let mut harness = with_a_group();
    let at = row(&harness, "Test cube").center();
    click_at(&mut harness, at, PointerButton::Secondary, Modifiers::NONE);
    assert_eq!(
        selected_names(&harness),
        ["Test cube"],
        "right-click selects it"
    );
    // The entry's name includes its key.
    harness.get_by_label_contains("Delete").click();
    harness.run();
    assert_eq!(top_level_names(&harness), ["Group"]);
}

// ------------------------------------------------------ undo and redo ---

#[test]
fn ctrl_z_undoes_and_ctrl_y_or_ctrl_shift_z_redo() {
    let mut harness = with_a_group();
    key(&mut harness, Modifiers::COMMAND, Key::Z);
    assert_eq!(top_level_names(&harness), ["Test cube"]);
    assert!(
        harness
            .query_by_label_contains("Undone: Create group")
            .is_some()
    );
    key(&mut harness, Modifiers::COMMAND, Key::Y);
    assert_eq!(top_level_names(&harness), ["Group", "Test cube"]);
    key(&mut harness, Modifiers::COMMAND, Key::Z);
    key(&mut harness, Modifiers::COMMAND | Modifiers::SHIFT, Key::Z);
    assert_eq!(top_level_names(&harness), ["Group", "Test cube"]);
}

#[test]
fn undo_brings_back_a_deleted_cube() {
    let mut harness = fresh();
    click_row(&mut harness, "Test cube", Modifiers::NONE);
    key(&mut harness, Modifiers::NONE, Key::Delete);
    key(&mut harness, Modifiers::COMMAND, Key::Z);
    assert_eq!(top_level_names(&harness), ["Test cube"]);
    assert!(cube_drawn(&mut harness));
}

#[test]
fn the_edit_menu_names_what_undo_would_do() {
    let mut harness = with_a_group();
    main_menu(&harness, "Edit").click();
    harness.run();
    let undo = harness.get_by_label_contains("Undo: Create group");
    assert!(!undo.accesskit_node().is_disabled());
    undo.click();
    harness.run();
    assert_eq!(top_level_names(&harness), ["Test cube"]);
}

#[test]
fn undo_and_redo_buttons_wait_for_something_to_do() {
    let mut harness = fresh();
    assert!(
        harness.get_by_label("Undo").accesskit_node().is_disabled(),
        "nothing to undo yet"
    );
    harness.get_by_label("Create group").click();
    harness.run();
    harness.get_by_label("Undo").click();
    harness.run();
    assert_eq!(top_level_names(&harness), ["Test cube"]);
    harness.get_by_label("Redo").click();
    harness.run();
    assert_eq!(top_level_names(&harness), ["Group", "Test cube"]);
}

// ----------------------------------------------------------- show/hide ---

#[test]
fn the_eye_hides_and_shows_in_the_viewport() {
    let mut harness = fresh();
    harness.get_by_label("Hide Test cube").click();
    harness.run();
    assert!(!cube_drawn(&mut harness));
    assert!(harness.query_by_label("Show Test cube").is_some());
    harness.get_by_label("Show Test cube").click();
    harness.run();
    assert!(cube_drawn(&mut harness));
    key(&mut harness, Modifiers::COMMAND, Key::Z);
    assert!(!cube_drawn(&mut harness), "hiding is undoable");
}

#[test]
fn hiding_a_group_hides_whats_inside() {
    let mut harness = fresh();
    click_row(&mut harness, "Test cube", Modifiers::NONE);
    key(&mut harness, Modifiers::ALT, Key::G);
    assert_eq!(top_level_names(&harness), ["Group"]);
    harness.get_by_label("Hide Group").click();
    harness.run();
    assert!(!cube_drawn(&mut harness));
    assert!(
        harness.query_by_label("Hide Test cube").is_some(),
        "the cube's own switch is still on"
    );
}

// --------------------------------------------------- group and ungroup ---

#[test]
fn alt_g_groups_and_shift_g_ungroups() {
    let mut harness = fresh();
    click_row(&mut harness, "Test cube", Modifiers::NONE);
    key(&mut harness, Modifiers::ALT, Key::G);
    assert_eq!(top_level_names(&harness), ["Group"]);
    assert_eq!(selected_names(&harness), ["Group"]);
    key(&mut harness, SHIFT, Key::G);
    assert_eq!(top_level_names(&harness), ["Test cube"]);
    assert_eq!(selected_names(&harness), ["Test cube"]);
}

// ------------------------------------------------------------- renaming ---

#[test]
fn double_clicking_a_name_renames_it() {
    let mut harness = quick_harness();
    double_click_row(&mut harness, "Test cube");
    assert!(harness.state().workbench.object_manager().is_renaming());
    // The old name is selected, so typing replaces it.
    harness.event(Event::Text("Crate".into()));
    harness.run();
    key(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(top_level_names(&harness), ["Crate"]);
    assert!(!harness.state().workbench.object_manager().is_renaming());
    key(&mut harness, Modifiers::COMMAND, Key::Z);
    assert_eq!(
        top_level_names(&harness),
        ["Test cube"],
        "renaming is undoable"
    );
}

#[test]
fn esc_while_renaming_keeps_the_old_name() {
    let mut harness = quick_harness();
    double_click_row(&mut harness, "Test cube");
    harness.event(Event::Text("Crate".into()));
    harness.run();
    key(&mut harness, Modifiers::NONE, Key::Escape);
    assert_eq!(top_level_names(&harness), ["Test cube"]);
    assert!(!harness.state().workbench.object_manager().is_renaming());
}

#[test]
fn keys_type_into_the_name_instead_of_deleting() {
    let mut harness = quick_harness();
    double_click_row(&mut harness, "Test cube");
    key(&mut harness, Modifiers::NONE, Key::Backspace);
    key(&mut harness, Modifiers::NONE, Key::Delete);
    assert_eq!(
        top_level_names(&harness),
        ["Test cube"],
        "the object is still there"
    );
}

#[test]
fn an_empty_name_is_refused_and_explained() {
    let mut harness = quick_harness();
    double_click_row(&mut harness, "Test cube");
    key(&mut harness, Modifiers::NONE, Key::Backspace);
    key(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(top_level_names(&harness), ["Test cube"]);
    assert!(
        harness
            .query_by_label_contains("a name can't be empty")
            .is_some()
    );
}

#[test]
fn the_edit_menu_starts_renaming_the_selected_object() {
    let mut harness = fresh();
    click_row(&mut harness, "Test cube", Modifiers::NONE);
    main_menu(&harness, "Edit").click();
    harness.run();
    harness.get_by_label("Rename").click();
    harness.run();
    assert!(harness.state().workbench.object_manager().is_renaming());
}

// ------------------------------------------------------------- dragging ---

#[test]
fn dragging_a_row_onto_another_puts_it_inside() {
    let mut harness = with_a_group();
    let from = row(&harness, "Test cube").center();
    let to = row(&harness, "Group").center();
    drag(&mut harness, from, to);
    assert_eq!(top_level_names(&harness), ["Group"]);
    let scene = harness.state().workbench.document().scene();
    assert_eq!(
        scene.parent(id_of(&harness, "Test cube")),
        Some(id_of(&harness, "Group"))
    );
    assert!(
        harness.query_by_label("Test cube").is_some(),
        "the group opens to show it"
    );
}

#[test]
fn dragging_to_the_top_edge_of_a_row_puts_it_above() {
    let mut harness = with_a_group();
    let from = row(&harness, "Test cube").center();
    let group = row(&harness, "Group");
    drag(
        &mut harness,
        from,
        group.center_top() + egui::vec2(0.0, 2.0),
    );
    assert_eq!(top_level_names(&harness), ["Test cube", "Group"]);
}

#[test]
fn dragging_into_empty_space_moves_it_out_to_the_top_level() {
    let mut harness = with_a_group();
    let from = row(&harness, "Test cube").center();
    let to = row(&harness, "Group").center();
    drag(&mut harness, from, to);
    let below = harness.get_by_label("Empty space below the objects").rect();
    let from = row(&harness, "Test cube").center();
    drag(&mut harness, from, below.center());
    assert_eq!(top_level_names(&harness), ["Group", "Test cube"]);
}

#[test]
fn a_group_cannot_be_dropped_inside_itself() {
    let mut harness = with_a_group();
    let from = row(&harness, "Test cube").center();
    let to = row(&harness, "Group").center();
    drag(&mut harness, from, to);
    let before = harness.state().workbench.document().scene().clone();
    let from = row(&harness, "Group").center();
    let to = row(&harness, "Test cube").center();
    drag(&mut harness, from, to);
    assert_eq!(*harness.state().workbench.document().scene(), before);
}

// -------------------------------------------------------------- framing ---

#[test]
fn f_frames_the_selected_objects() {
    let mut harness = fresh();
    // Far away, looking elsewhere: then select the cube and press F.
    harness
        .state_mut()
        .workbench
        .viewport_mut()
        .navigator_mut()
        .pan(glam::Vec2::new(4000.0, 0.0), 400.0);
    harness.run();
    click_row(&mut harness, "Test cube", Modifiers::NONE);
    harness.key_press(Key::F);
    harness.run();
    let pivot = harness.state().workbench.viewport().navigator().pivot();
    assert_eq!(pivot, scene::TEST_CUBE.centre());
}

#[test]
fn a_new_window_shows_the_cube_through_the_real_renderer_interface() {
    // Sanity check that the stand-in renderer sees the cube setting.
    let renderer = TestRenderer::default();
    let mut harness = harness_with(Workbench::new(info(), None), renderer);
    harness.run();
    assert!(cube_drawn(&mut harness));
}
