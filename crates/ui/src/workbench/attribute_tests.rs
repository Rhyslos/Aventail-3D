//! Tests for the Attribute Manager: showing and editing a selected
//! object's settings, through the real interface without a window.

use super::tests::{State, empty, fresh, main_menu};
use super::*;
use egui::{Event, PointerButton};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use scene::{ObjectId, Primitive};

fn only_object(harness: &Harness<'static, State>) -> ObjectId {
    harness.state().workbench.document().scene().children(None)[0]
}

fn primitive(harness: &Harness<'static, State>, id: ObjectId) -> Primitive {
    match harness
        .state()
        .workbench
        .document()
        .scene()
        .object(id)
        .unwrap()
        .kind
    {
        scene::ObjectKind::Primitive(p) => p,
        other => panic!("not a primitive: {other:?}"),
    }
}

fn cube_size(harness: &Harness<'static, State>, id: ObjectId) -> glam::Vec3 {
    match primitive(harness, id) {
        Primitive::Cube(p) => p.size,
        other => panic!("not a cube: {other:?}"),
    }
}

/// A window with a cube, selected.
fn selected_cube() -> Harness<'static, State> {
    let mut harness = fresh();
    let id = only_object(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    harness
}

/// Where a setting's field is: right of its label (both carry the name).
fn field_rect(harness: &Harness<'static, State>, label: &str) -> egui::Rect {
    harness
        .get_all_by_label(label)
        .max_by(|a, b| a.rect().min.x.total_cmp(&b.rect().min.x))
        .unwrap_or_else(|| panic!("no field {label}"))
        .rect()
}

/// Clicks a setting's field.
fn click_field(harness: &mut Harness<'static, State>, label: &str) {
    let at = field_rect(harness, label).center();
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

/// Drags a value field sideways by `dx` points.
fn drag_field(harness: &mut Harness<'static, State>, label: &str, dx: f32) {
    let from = field_rect(harness, label).center();
    harness.event(Event::PointerMoved(from));
    harness.step();
    harness.event(Event::PointerButton {
        pos: from,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    for i in 1..=10 {
        harness.event(Event::PointerMoved(
            from + egui::vec2(dx * i as f32 / 10.0, 0.0),
        ));
        harness.step();
    }
    harness.event(Event::PointerButton {
        pos: from + egui::vec2(dx, 0.0),
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
}

#[test]
fn nothing_selected_says_so() {
    let harness = fresh();
    assert!(harness.query_by_label("Nothing selected").is_some());
}

#[test]
fn a_selected_cube_shows_its_name_and_shape_settings() {
    let harness = selected_cube();
    assert!(harness.query_by_label("Cube (cube)").is_some());
    for label in [
        "Object name",
        "Width",
        "Depth",
        "Height",
        "Segments X",
        "Segments Y",
        "Segments Z",
    ] {
        assert!(
            harness.query_all_by_label(label).next().is_some(),
            "{label}"
        );
    }
}

#[test]
fn every_kind_shows_its_own_settings() {
    let expected: [(&str, &[&str]); 7] = [
        ("Cube", &["Width", "Segments Z"]),
        ("Plane", &["Width", "Segments Y"]),
        ("Cylinder", &["Radius", "Sides", "Closed ends"]),
        ("Sphere", &["Radius", "Rings"]),
        ("Disc", &["Hole radius"]),
        ("Tube", &["Outer radius", "Inner radius"]),
        ("Cone", &["Bottom radius", "Top radius"]),
    ];
    for (name, labels) in expected {
        let mut harness = empty();
        main_menu(&harness, "Create").click();
        harness.run();
        harness.get_by_label(name).click();
        harness.run();
        for label in labels {
            assert!(
                harness.query_all_by_label(label).next().is_some(),
                "{name}: {label}"
            );
        }
    }
}

#[test]
fn a_group_has_only_basic_settings() {
    let mut harness = empty();
    harness.get_by_label("Create group").click();
    harness.run();
    assert!(harness.query_by_label("Object name").is_some());
    assert!(harness.query_all_by_label("Width").next().is_none());
}

#[test]
fn several_selected_objects_are_counted() {
    let mut harness = fresh();
    harness.get_by_label("Create cube").click();
    harness.run();
    let ids = harness
        .state()
        .workbench
        .document()
        .scene()
        .children(None)
        .to_vec();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection(ids);
    harness.run();
    assert!(harness.query_by_label("2 objects selected").is_some());
}

#[test]
fn dragging_a_value_changes_the_shape_in_one_undo_step() {
    let mut harness = selected_cube();
    let id = only_object(&harness);
    let steps = harness.state().workbench.document().undo_steps();
    drag_field(&mut harness, "Width", 80.0);
    let width = cube_size(&harness, id).x;
    assert!(width > 200.0, "wider: {width}");
    assert_eq!(cube_size(&harness, id).y, 200.0, "only the width");
    assert_eq!(
        harness.state().workbench.document().undo_steps(),
        steps + 1,
        "one step"
    );
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Change cube settings")
    );
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert_eq!(cube_size(&harness, id).x, 200.0, "the whole drag undone");
    // A second drag is a second step.
    drag_field(&mut harness, "Width", 40.0);
    drag_field(&mut harness, "Height", -40.0);
    assert_eq!(harness.state().workbench.document().undo_steps(), steps + 2);
}

#[test]
fn values_stay_within_limits() {
    let mut harness = selected_cube();
    let id = only_object(&harness);
    drag_field(&mut harness, "Segments X", -500.0);
    let Primitive::Cube(p) = primitive(&harness, id) else {
        panic!()
    };
    assert_eq!(p.segments[0], 1, "at least one segment");
}

#[test]
fn the_shape_redraws_when_its_settings_change() {
    let mut harness = selected_cube();
    let mesh = |h: &mut Harness<'static, State>| {
        h.state_mut().renderer.drawn.clear();
        h.step();
        h.state().renderer.drawn.last().unwrap().scene.items[0]
            .mesh
            .clone()
    };
    let before = mesh(&mut harness);
    drag_field(&mut harness, "Segments X", 100.0);
    let after = mesh(&mut harness);
    assert!(
        after.face_count() > before.face_count(),
        "more segments, more faces"
    );
}

#[test]
fn closed_ends_switch_off_and_back() {
    let mut harness = empty();
    harness.get_by_label("Create cylinder").click();
    harness.run();
    let id = only_object(&harness);
    click_field(&mut harness, "Closed ends");
    harness.run();
    let Primitive::Cylinder(p) = primitive(&harness, id) else {
        panic!()
    };
    assert!(!p.caps);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    let Primitive::Cylinder(p) = primitive(&harness, id) else {
        panic!()
    };
    assert!(p.caps);
}

#[test]
fn the_name_field_renames_and_follows_renames_elsewhere() {
    let mut harness = selected_cube();
    let id = only_object(&harness);
    harness.get_by_label("Object name").click();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.event(Event::Text("Crate".into()));
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    let name = |h: &Harness<'static, State>| {
        h.state()
            .workbench
            .document()
            .scene()
            .object(id)
            .unwrap()
            .name
            .clone()
    };
    assert_eq!(name(&harness), "Crate");
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert_eq!(name(&harness), "Cube");
    assert!(
        harness.query_by_label("Cube (cube)").is_some(),
        "the heading follows"
    );
}

#[test]
fn the_visible_switch_hides_the_object() {
    let mut harness = selected_cube();
    let id = only_object(&harness);
    click_field(&mut harness, "Visible");
    harness.run();
    assert!(!harness.state().workbench.document().scene().is_shown(id));
}

#[test]
fn settings_can_be_copied_to_other_shapes_of_the_same_kind() {
    let mut harness = selected_cube();
    let first = only_object(&harness);
    drag_field(&mut harness, "Height", 60.0);
    let copied = cube_size(&harness, first);
    let edit_menu = |h: &mut Harness<'static, State>| {
        // The Attributes panel's Edit menu: the one nearest the top right.
        let node = h
            .get_all_by_label("Edit")
            .max_by(|a, b| {
                (a.rect().min.x - a.rect().min.y).total_cmp(&(b.rect().min.x - b.rect().min.y))
            })
            .unwrap();
        node.click();
        h.run();
    };
    edit_menu(&mut harness);
    harness.get_by_label("Copy settings").click();
    harness.run();
    harness.get_by_label("Create cube").click();
    harness.run();
    let second = harness.state().workbench.document().selection()[0];
    assert_ne!(cube_size(&harness, second), copied);
    edit_menu(&mut harness);
    harness.get_by_label("Paste cube settings").click();
    harness.run();
    assert_eq!(cube_size(&harness, second), copied);
}

#[test]
fn typing_a_value_sets_it_exactly() {
    let mut harness = selected_cube();
    let id = only_object(&harness);
    click_field(&mut harness, "Depth");
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.event(Event::Text("350".into()));
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(cube_size(&harness, id).y, 350.0);
    // While typing, Delete and the other document keys are left alone.
    assert_eq!(
        harness.state().workbench.document().scene().len(),
        1,
        "nothing deleted"
    );
}
