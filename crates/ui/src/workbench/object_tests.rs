//! Tests for the Object Manager, undo and redo, run through the real
//! interface without a window.

use super::tests::{State, empty, fresh, main_menu, quick_harness};
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
    assert_eq!(top_level_names(&harness), ["Group", "Cube"]);
    harness
}

/// Whether the last frame drew the cube (and so something).
fn cube_drawn(harness: &mut Harness<'static, State>) -> bool {
    harness.state_mut().renderer.drawn.clear();
    harness.step();
    let drawn = &harness.state().renderer.drawn;
    !drawn.is_empty() && drawn.iter().all(|v| !v.scene.items.is_empty())
}

const SHIFT: Modifiers = Modifiers::SHIFT;

// ------------------------------------------------------------- the tree ---

#[test]
fn a_cube_is_listed_with_its_eye() {
    let mut harness = fresh();
    assert!(harness.query_by_label("Cube").is_some());
    assert!(
        harness.query_by_label("Hide Cube").is_some(),
        "with its eye"
    );
    assert!(selected_names(&harness).is_empty());
    assert!(cube_drawn(&mut harness));
}

#[test]
fn clicking_a_row_selects_it_and_the_status_bar_counts() {
    let mut harness = fresh();
    click_row(&mut harness, "Cube", Modifiers::NONE);
    assert_eq!(selected_names(&harness), ["Cube"]);
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
    assert_eq!(top_level_names(&harness), ["Group 2", "Group", "Cube"]);

    click_row(&mut harness, "Group 2", Modifiers::NONE);
    click_row(&mut harness, "Cube", Modifiers::COMMAND);
    assert_eq!(selected_names(&harness), ["Group 2", "Cube"]);
    click_row(&mut harness, "Cube", Modifiers::COMMAND);
    assert_eq!(
        selected_names(&harness),
        ["Group 2"],
        "Ctrl+click again takes it out"
    );

    // Shift+click selects from the last row clicked.
    click_row(&mut harness, "Group 2", Modifiers::NONE);
    click_row(&mut harness, "Cube", SHIFT);
    assert_eq!(selected_names(&harness), ["Group 2", "Group", "Cube"]);
}

#[test]
fn clicking_empty_space_or_esc_selects_nothing() {
    let mut harness = fresh();
    click_row(&mut harness, "Cube", Modifiers::NONE);
    let below = harness.get_by_label("Empty space below the objects").rect();
    click_at(
        &mut harness,
        below.center(),
        PointerButton::Primary,
        Modifiers::NONE,
    );
    assert!(selected_names(&harness).is_empty());
    click_row(&mut harness, "Cube", Modifiers::NONE);
    key(&mut harness, Modifiers::NONE, Key::Escape);
    assert!(selected_names(&harness).is_empty());
}

#[test]
fn the_fold_arrow_hides_and_shows_whats_inside() {
    let mut harness = with_a_group();
    let cube = id_of(&harness, "Cube");
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
    assert!(row(&harness, "Cube").min.y > row(&harness, "Group").min.y);
    harness.get_by_label("Fold Group").click();
    harness.run();
    assert!(harness.query_by_label("Cube").is_none(), "folded away");
    harness.get_by_label("Unfold Group").click();
    harness.run();
    assert!(harness.query_by_label("Cube").is_some());
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
    assert_eq!(top_level_names(&harness), ["Group", "Cube"]);
    harness.get_by_label("Object").click();
    harness.run();
    harness.get_by_label("New group").click();
    harness.run();
    assert_eq!(top_level_names(&harness), ["Group 2", "Group", "Cube"]);
}

#[test]
fn delete_and_backspace_delete_the_selection() {
    for delete in [Key::Delete, Key::Backspace] {
        let mut harness = fresh();
        click_row(&mut harness, "Cube", Modifiers::NONE);
        key(&mut harness, Modifiers::NONE, delete);
        assert!(top_level_names(&harness).is_empty(), "{delete:?}");
        assert!(harness.query_by_label("No objects yet").is_some());
        assert!(!cube_drawn(&mut harness), "gone from the viewport");
    }
}

#[test]
fn delete_with_nothing_selected_does_nothing() {
    let mut harness = fresh();
    let steps = harness.state().workbench.document().undo_steps();
    key(&mut harness, Modifiers::NONE, Key::Delete);
    assert_eq!(top_level_names(&harness), ["Cube"]);
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
}

#[test]
fn the_right_click_menu_deletes_the_row_clicked() {
    let mut harness = with_a_group();
    let at = row(&harness, "Cube").center();
    click_at(&mut harness, at, PointerButton::Secondary, Modifiers::NONE);
    assert_eq!(selected_names(&harness), ["Cube"], "right-click selects it");
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
    assert_eq!(top_level_names(&harness), ["Cube"]);
    assert!(
        harness
            .query_by_label_contains("Undone: Create group")
            .is_some()
    );
    key(&mut harness, Modifiers::COMMAND, Key::Y);
    assert_eq!(top_level_names(&harness), ["Group", "Cube"]);
    key(&mut harness, Modifiers::COMMAND, Key::Z);
    key(&mut harness, Modifiers::COMMAND | Modifiers::SHIFT, Key::Z);
    assert_eq!(top_level_names(&harness), ["Group", "Cube"]);
}

#[test]
fn undo_brings_back_a_deleted_cube() {
    let mut harness = fresh();
    click_row(&mut harness, "Cube", Modifiers::NONE);
    key(&mut harness, Modifiers::NONE, Key::Delete);
    key(&mut harness, Modifiers::COMMAND, Key::Z);
    assert_eq!(top_level_names(&harness), ["Cube"]);
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
    assert_eq!(top_level_names(&harness), ["Cube"]);
}

#[test]
fn undo_and_redo_buttons_wait_for_something_to_do() {
    let mut harness = empty();
    assert!(
        harness.get_by_label("Undo").accesskit_node().is_disabled(),
        "nothing to undo yet"
    );
    harness.get_by_label("Create group").click();
    harness.run();
    harness.get_by_label("Undo").click();
    harness.run();
    assert!(top_level_names(&harness).is_empty());
    harness.get_by_label("Redo").click();
    harness.run();
    assert_eq!(top_level_names(&harness), ["Group"]);
}

// ----------------------------------------------------------- show/hide ---

#[test]
fn the_eye_hides_and_shows_in_the_viewport() {
    let mut harness = fresh();
    harness.get_by_label("Hide Cube").click();
    harness.run();
    assert!(!cube_drawn(&mut harness));
    assert!(harness.query_by_label("Show Cube").is_some());
    harness.get_by_label("Show Cube").click();
    harness.run();
    assert!(cube_drawn(&mut harness));
    key(&mut harness, Modifiers::COMMAND, Key::Z);
    assert!(!cube_drawn(&mut harness), "hiding is undoable");
}

#[test]
fn hiding_a_group_hides_whats_inside() {
    let mut harness = fresh();
    click_row(&mut harness, "Cube", Modifiers::NONE);
    key(&mut harness, Modifiers::ALT, Key::G);
    assert_eq!(top_level_names(&harness), ["Group"]);
    harness.get_by_label("Hide Group").click();
    harness.run();
    assert!(!cube_drawn(&mut harness));
    assert!(
        harness.query_by_label("Hide Cube").is_some(),
        "the cube's own switch is still on"
    );
}

// --------------------------------------------------- group and ungroup ---

#[test]
fn alt_g_groups_and_shift_g_ungroups() {
    let mut harness = fresh();
    click_row(&mut harness, "Cube", Modifiers::NONE);
    key(&mut harness, Modifiers::ALT, Key::G);
    assert_eq!(top_level_names(&harness), ["Group"]);
    assert_eq!(selected_names(&harness), ["Group"]);
    key(&mut harness, SHIFT, Key::G);
    assert_eq!(top_level_names(&harness), ["Cube"]);
    assert_eq!(selected_names(&harness), ["Cube"]);
}

#[test]
fn shift_g_leaves_objects_that_are_not_groups_alone() {
    // Regression: Shift+G with the cube selected deleted the cube.
    let mut harness = fresh();
    click_row(&mut harness, "Cube", Modifiers::NONE);
    key(&mut harness, SHIFT, Key::G);
    assert_eq!(top_level_names(&harness), ["Cube"]);
    assert!(cube_drawn(&mut harness));
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Select Cube"),
        "nothing new to undo but the click"
    );
    assert!(
        harness
            .query_by_label_contains("only groups can be ungrouped")
            .is_some()
    );
    // And the menu entry waits for a group.
    harness.get_by_label("Object").click();
    harness.run();
    assert!(
        harness
            .get_by_label_contains("Ungroup")
            .accesskit_node()
            .is_disabled()
    );
}

// ------------------------------------------------------------- renaming ---

#[test]
fn double_clicking_a_name_renames_it() {
    let mut harness = quick_harness();
    double_click_row(&mut harness, "Cube");
    assert!(harness.state().workbench.object_manager().is_renaming());
    // The old name is selected, so typing replaces it.
    harness.event(Event::Text("Crate".into()));
    harness.run();
    key(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(top_level_names(&harness), ["Crate"]);
    assert!(!harness.state().workbench.object_manager().is_renaming());
    key(&mut harness, Modifiers::COMMAND, Key::Z);
    assert_eq!(top_level_names(&harness), ["Cube"], "renaming is undoable");
}

#[test]
fn esc_while_renaming_keeps_the_old_name() {
    let mut harness = quick_harness();
    double_click_row(&mut harness, "Cube");
    harness.event(Event::Text("Crate".into()));
    harness.run();
    key(&mut harness, Modifiers::NONE, Key::Escape);
    assert_eq!(top_level_names(&harness), ["Cube"]);
    assert!(!harness.state().workbench.object_manager().is_renaming());
}

#[test]
fn keys_type_into_the_name_instead_of_deleting() {
    let mut harness = quick_harness();
    double_click_row(&mut harness, "Cube");
    key(&mut harness, Modifiers::NONE, Key::Backspace);
    key(&mut harness, Modifiers::NONE, Key::Delete);
    assert_eq!(
        top_level_names(&harness),
        ["Cube"],
        "the object is still there"
    );
}

#[test]
fn an_empty_name_is_refused_and_explained() {
    let mut harness = quick_harness();
    double_click_row(&mut harness, "Cube");
    key(&mut harness, Modifiers::NONE, Key::Backspace);
    key(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(top_level_names(&harness), ["Cube"]);
    assert!(
        harness
            .query_by_label_contains("a name can't be empty")
            .is_some()
    );
}

#[test]
fn the_edit_menu_starts_renaming_the_selected_object() {
    let mut harness = fresh();
    click_row(&mut harness, "Cube", Modifiers::NONE);
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
    let from = row(&harness, "Cube").center();
    let to = row(&harness, "Group").center();
    drag(&mut harness, from, to);
    assert_eq!(top_level_names(&harness), ["Group"]);
    let scene = harness.state().workbench.document().scene();
    assert_eq!(
        scene.parent(id_of(&harness, "Cube")),
        Some(id_of(&harness, "Group"))
    );
    assert!(
        harness.query_by_label("Cube").is_some(),
        "the group opens to show it"
    );
}

#[test]
fn dragging_to_the_top_edge_of_a_row_puts_it_above() {
    let mut harness = with_a_group();
    let from = row(&harness, "Cube").center();
    let group = row(&harness, "Group");
    drag(
        &mut harness,
        from,
        group.center_top() + egui::vec2(0.0, 2.0),
    );
    assert_eq!(top_level_names(&harness), ["Cube", "Group"]);
}

#[test]
fn dragging_into_empty_space_moves_it_out_to_the_top_level() {
    let mut harness = with_a_group();
    let from = row(&harness, "Cube").center();
    let to = row(&harness, "Group").center();
    drag(&mut harness, from, to);
    let below = harness.get_by_label("Empty space below the objects").rect();
    let from = row(&harness, "Cube").center();
    drag(&mut harness, from, below.center());
    assert_eq!(top_level_names(&harness), ["Group", "Cube"]);
}

#[test]
fn a_group_cannot_be_dropped_inside_itself() {
    let mut harness = with_a_group();
    let from = row(&harness, "Cube").center();
    let to = row(&harness, "Group").center();
    drag(&mut harness, from, to);
    let before = harness.state().workbench.document().scene().clone();
    let from = row(&harness, "Group").center();
    let to = row(&harness, "Cube").center();
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
    click_row(&mut harness, "Cube", Modifiers::NONE);
    harness.key_press(Key::F);
    harness.run();
    let pivot = harness.state().workbench.viewport().navigator().pivot();
    assert_eq!(pivot, crate::DEFAULT_FRAME.centre());
}

// ----------------------------------------------------------- primitives ---

/// The scene items the last frame drew, by name, and whether each was
/// selected.
fn drawn_items(harness: &mut Harness<'static, State>) -> Vec<(String, bool)> {
    harness.state_mut().renderer.drawn.clear();
    harness.step();
    let doc = harness.state().workbench.document();
    let last = harness.state().renderer.drawn.last().cloned().unwrap();
    last.scene
        .items
        .iter()
        .map(|i| (doc.scene().object(i.id).unwrap().name.clone(), i.selected))
        .collect()
}

#[test]
fn a_new_window_starts_with_an_empty_scene() {
    let mut harness = empty();
    assert!(top_level_names(&harness).is_empty());
    assert!(drawn_items(&mut harness).is_empty());
    assert!(harness.query_by_label("No objects yet").is_some());
}

#[test]
fn the_create_menu_makes_every_primitive() {
    let mut harness = empty();
    for kind in scene::PrimitiveKind::ALL {
        main_menu(&harness, "Create").click();
        harness.run();
        harness.get_by_label(kind.name()).click();
        harness.run();
        assert_eq!(top_level_names(&harness)[0], kind.name());
        assert_eq!(selected_names(&harness), [kind.name()], "new and selected");
    }
    let drawn = drawn_items(&mut harness);
    assert_eq!(drawn.len(), 7, "every one is drawn");
    assert_eq!(drawn[0], ("Cone".to_string(), true), "the newest, selected");
    assert!(drawn[1..].iter().all(|(_, selected)| !selected));
}

#[test]
fn the_palette_shape_buttons_create_primitives() {
    let mut harness = empty();
    for (button, name) in [
        ("Create cube", "Cube"),
        ("Create cylinder", "Cylinder"),
        ("Create plane", "Plane"),
        ("Create sphere", "Sphere"),
    ] {
        harness.get_by_label(button).click();
        harness.run();
        assert_eq!(top_level_names(&harness)[0], name);
    }
    key(&mut harness, Modifiers::COMMAND, Key::Z);
    assert_eq!(top_level_names(&harness), ["Plane", "Cylinder", "Cube"]);
}

#[test]
fn new_shapes_of_the_same_kind_get_numbers() {
    let mut harness = fresh();
    harness.get_by_label("Create cube").click();
    harness.run();
    assert_eq!(top_level_names(&harness), ["Cube 2", "Cube"]);
}

#[test]
fn selecting_a_row_marks_the_object_in_the_viewport_and_attributes() {
    let mut harness = fresh();
    assert_eq!(drawn_items(&mut harness), [("Cube".to_string(), false)]);
    click_row(&mut harness, "Cube", Modifiers::NONE);
    assert_eq!(drawn_items(&mut harness), [("Cube".to_string(), true)]);
    assert!(harness.query_by_label("Cube (cube)").is_some());
}

#[test]
fn selected_objects_are_outlined_in_the_accent_colour() {
    let mut harness = fresh();
    let light = |accent: crate::theme::Accent| {
        let c = accent.colors().light;
        [c.r(), c.g(), c.b()].map(|v| f32::from(v) / 255.0)
    };
    let drawn_color = |harness: &mut Harness<'static, State>| {
        harness.state_mut().renderer.drawn.clear();
        harness.step();
        harness
            .state()
            .renderer
            .drawn
            .last()
            .unwrap()
            .selection_color
    };
    assert_eq!(drawn_color(&mut harness), light(crate::theme::Accent::Red));
    harness
        .state_mut()
        .workbench
        .set_accent(crate::theme::Accent::Olive);
    harness.run();
    assert_eq!(
        drawn_color(&mut harness),
        light(crate::theme::Accent::Olive)
    );
}

#[test]
fn shapes_inside_a_group_move_with_it() {
    // The drawing places each object by its parents' transforms too.
    let mut harness = fresh();
    let cube = id_of(&harness, "Cube");
    let mut group = scene::Object::new(scene::ObjectKind::Group);
    group.transform.position = glam::Vec3::new(300.0, 0.0, 0.0);
    let document = harness.state_mut().workbench.document_mut();
    document
        .execute(scene::Command::Insert {
            object: group,
            place: scene::Place {
                parent: None,
                index: 0,
            },
        })
        .unwrap();
    let group = document.selection()[0];
    document
        .execute(scene::Command::Move {
            ids: vec![cube],
            parent: Some(group),
            index: 0,
        })
        .unwrap();
    harness.state_mut().renderer.drawn.clear();
    harness.step();
    let last = harness.state().renderer.drawn.last().cloned().unwrap();
    let moved = last.scene.items[0]
        .transform
        .transform_point3(glam::Vec3::ZERO);
    assert_eq!(moved, glam::Vec3::new(300.0, 0.0, 0.0));
}

// --------------------------------------------- copying shape settings ---

fn cube_size_of(harness: &Harness<'static, State>, name: &str) -> glam::Vec3 {
    let id = id_of(harness, name);
    match harness
        .state()
        .workbench
        .document()
        .scene()
        .object(id)
        .unwrap()
        .kind
    {
        scene::ObjectKind::Primitive(scene::Primitive::Cube(p)) => p.size,
        ref other => panic!("not a cube: {other:?}"),
    }
}

#[test]
fn the_right_click_menu_copies_and_pastes_shape_settings() {
    let mut harness = fresh();
    // Make the first cube taller, then add a second, standard one.
    let first = id_of(&harness, "Cube");
    let mut tall = scene::Primitive::Cube(Default::default());
    if let scene::Primitive::Cube(p) = &mut tall {
        p.size.z = 250.0;
    }
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::SetPrimitive {
            ids: vec![first],
            primitive: tall,
        })
        .unwrap();
    harness.get_by_label("Create cube").click();
    harness.run();
    let second = top_level_names(&harness)
        .into_iter()
        .find(|n| n != "Cube")
        .expect("a second cube");
    assert_eq!(cube_size_of(&harness, &second).z, 100.0);

    let at = row(&harness, "Cube").center();
    click_at(&mut harness, at, PointerButton::Secondary, Modifiers::NONE);
    harness.get_by_label("Copy settings").click();
    harness.run();

    let at = row(&harness, &second).center();
    click_at(&mut harness, at, PointerButton::Secondary, Modifiers::NONE);
    harness.get_by_label("Paste cube settings").click();
    harness.run();
    assert_eq!(cube_size_of(&harness, &second).z, 250.0);
    // One undo step puts it back.
    key(&mut harness, Modifiers::COMMAND, Key::Z);
    assert_eq!(cube_size_of(&harness, &second).z, 100.0);
}

#[test]
fn settings_paste_only_onto_the_same_kind() {
    let mut harness = fresh();
    let at = row(&harness, "Cube").center();
    click_at(&mut harness, at, PointerButton::Secondary, Modifiers::NONE);
    harness.get_by_label("Copy settings").click();
    harness.run();
    harness.get_by_label("Create sphere").click();
    harness.run();
    let at = row(&harness, "Sphere").center();
    click_at(&mut harness, at, PointerButton::Secondary, Modifiers::NONE);
    assert!(
        harness
            .get_by_label("Paste cube settings")
            .accesskit_node()
            .is_disabled()
    );
}

// ------------------------------------------------------- make editable ---

fn kind_of(harness: &Harness<'static, State>, name: &str) -> scene::ObjectKind {
    let id = id_of(harness, name);
    harness
        .state()
        .workbench
        .document()
        .scene()
        .object(id)
        .unwrap()
        .kind
        .clone()
}

fn is_mesh(harness: &Harness<'static, State>, name: &str) -> bool {
    matches!(kind_of(harness, name), scene::ObjectKind::Mesh(_))
}

#[test]
fn ctrl_e_makes_the_selected_shape_editable_and_undo_brings_its_settings_back() {
    let mut harness = fresh();
    click_row(&mut harness, "Cube", Modifiers::NONE);
    key(&mut harness, Modifiers::COMMAND, Key::E);
    assert!(is_mesh(&harness, "Cube"));
    assert!(
        harness.query_by_label("Cube (mesh)").is_some(),
        "the Attributes heading says so"
    );
    key(&mut harness, Modifiers::COMMAND, Key::Z);
    assert!(matches!(
        kind_of(&harness, "Cube"),
        scene::ObjectKind::Primitive(_)
    ));
}

#[test]
fn ctrl_e_on_a_group_explains_instead_of_doing_nothing_silently() {
    let mut harness = with_a_group();
    click_row(&mut harness, "Group", Modifiers::NONE);
    let steps = harness.state().workbench.document().undo_steps();
    key(&mut harness, Modifiers::COMMAND, Key::E);
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
    assert!(
        harness
            .query_by_label_contains("Nothing to make editable")
            .is_some()
    );
}

#[test]
fn the_object_menu_and_right_click_make_shapes_editable() {
    let mut harness = fresh();
    let at = row(&harness, "Cube").center();
    click_at(&mut harness, at, PointerButton::Secondary, Modifiers::NONE);
    harness.get_by_label_contains("Make editable").click();
    harness.run();
    assert!(is_mesh(&harness, "Cube"));
    // Now there is nothing left to bake: the entry is greyed out.
    harness.get_all_by_label("Object").last().unwrap().click();
    harness.run();
    assert!(
        harness
            .get_by_label_contains("Make editable")
            .accesskit_node()
            .is_disabled()
    );
}

#[test]
fn the_mesh_menu_makes_shapes_editable() {
    let mut harness = fresh();
    click_row(&mut harness, "Cube", Modifiers::NONE);
    main_menu(&harness, "Mesh").click();
    harness.run();
    harness.get_by_label_contains("Make editable").click();
    harness.run();
    assert!(is_mesh(&harness, "Cube"));
}

#[test]
fn ctrl_e_does_nothing_while_renaming() {
    let mut harness = quick_harness();
    double_click_row(&mut harness, "Cube");
    assert!(harness.state().workbench.object_manager().is_renaming());
    key(&mut harness, Modifiers::COMMAND, Key::E);
    assert!(!is_mesh(&harness, "Cube"));
}

#[test]
fn a_plain_c_no_longer_makes_shapes_editable() {
    let mut harness = fresh();
    click_row(&mut harness, "Cube", Modifiers::NONE);
    key(&mut harness, Modifiers::NONE, Key::C);
    assert!(!is_mesh(&harness, "Cube"));
}
