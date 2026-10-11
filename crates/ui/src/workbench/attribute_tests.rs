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
        .clone()
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

/// Clicks into a field, replaces its text with `text` and presses Enter.
fn type_into(harness: &mut Harness<'static, State>, label: &str, text: &str) {
    click_field(harness, label);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.event(Event::Text(text.into()));
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
}

/// Clicks a node by its accessible name, holding `modifiers`.
fn click_with(harness: &mut Harness<'static, State>, label: &str, modifiers: Modifiers) {
    let at = harness.get_by_label(label).rect().center();
    harness.event(Event::ModifiersChanged(modifiers));
    harness.event(Event::PointerMoved(at));
    harness.step();
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers,
        });
        harness.step();
    }
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
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
    // Half a centimetre per point dragged, whichever units are shown.
    assert!((width - 140.0).abs() < 2.0, "wider: {width}");
    assert_eq!(cube_size(&harness, id).y, 100.0, "only the width");
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
    assert_eq!(cube_size(&harness, id).x, 100.0, "the whole drag undone");
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
    // Lengths are typed in metres as standard.
    type_into(&mut harness, "Depth", "3.5");
    assert_eq!(cube_size(&harness, id).y, 350.0);
    // While typing, Delete and the other document keys are left alone.
    assert_eq!(
        harness.state().workbench.document().scene().len(),
        1,
        "nothing deleted"
    );
}

// --------------------------------------------------------- step arrows ---

#[test]
fn the_step_arrows_nudge_a_length_by_a_centimetre() {
    let mut harness = selected_cube();
    let id = only_object(&harness);
    let steps = harness.state().workbench.document().undo_steps();
    click_with(&mut harness, "Increase Width", Modifiers::NONE);
    assert_eq!(cube_size(&harness, id).x, 101.0);
    click_with(&mut harness, "Increase Width", Modifiers::NONE);
    assert_eq!(cube_size(&harness, id).x, 102.0);
    assert_eq!(
        harness.state().workbench.document().undo_steps(),
        steps + 2,
        "each click is its own undo step"
    );
    click_with(&mut harness, "Decrease Width", Modifiers::SHIFT);
    assert_eq!(cube_size(&harness, id).x, 92.0, "Shift: ten steps");
    assert_eq!(cube_size(&harness, id).y, 100.0, "only the width");
}

#[test]
fn the_step_arrows_count_segments_and_stop_at_the_limits() {
    let mut harness = selected_cube();
    let id = only_object(&harness);
    let segments = |h: &Harness<'static, State>| match primitive(h, id) {
        Primitive::Cube(p) => p.segments[0],
        other => panic!("{other:?}"),
    };
    assert_eq!(segments(&harness), 1);
    let steps = harness.state().workbench.document().undo_steps();
    click_with(&mut harness, "Decrease Segments X", Modifiers::NONE);
    assert_eq!(segments(&harness), 1, "never below one");
    assert_eq!(
        harness.state().workbench.document().undo_steps(),
        steps,
        "nothing changed, nothing to undo"
    );
    click_with(&mut harness, "Increase Segments X", Modifiers::SHIFT);
    assert_eq!(segments(&harness), 11);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert_eq!(segments(&harness), 1);
}

#[test]
fn the_step_arrows_stop_at_the_smallest_length() {
    let mut harness = selected_cube();
    let id = only_object(&harness);
    type_into(&mut harness, "Width", "0");
    let smallest = cube_size(&harness, id).x;
    assert!(smallest > 0.0, "never zero: {smallest}");
    click_with(&mut harness, "Decrease Width", Modifiers::NONE);
    assert_eq!(cube_size(&harness, id).x, smallest);
}

// --------------------------------------------------------------- units ---

#[test]
fn lengths_show_in_metres_and_switch_to_centimetres() {
    let mut harness = selected_cube();
    let id = only_object(&harness);
    assert_eq!(harness.state().workbench.units(), Units::Metres);
    // No switch beside the Shape title any more: the Units menu does it.
    assert!(harness.query_by_label("Lengths in centimetres").is_none());
    let choose = |h: &mut Harness<'static, State>, units: &str| {
        h.get_by_label("Units").click();
        h.run();
        h.get_by_label(units).click();
        h.run();
    };
    choose(&mut harness, "Centimetres");
    assert_eq!(harness.state().workbench.units(), Units::Centimetres);
    type_into(&mut harness, "Height", "150");
    assert_eq!(cube_size(&harness, id).z, 150.0, "typed in centimetres");
    choose(&mut harness, "Metres");
    assert_eq!(harness.state().workbench.units(), Units::Metres);
    type_into(&mut harness, "Height", "2");
    assert_eq!(cube_size(&harness, id).z, 200.0, "typed in metres");
}

#[test]
fn the_units_menu_switches_units_too() {
    let mut harness = selected_cube();
    harness.get_by_label("Units").click();
    harness.run();
    harness.get_by_label("Centimetres").click();
    harness.run();
    assert_eq!(harness.state().workbench.units(), Units::Centimetres);
}

#[test]
fn remembered_units_are_restored() {
    let workbench = Workbench::new(super::tests::info(), None).with_units(Units::Centimetres);
    assert_eq!(workbench.units(), Units::Centimetres);
}

// ------------------------------------------------------- round ends ---

#[test]
fn cylinder_ends_take_rings_and_a_centre_point() {
    let mut harness = empty();
    harness.get_by_label("Create cylinder").click();
    harness.run();
    let id = only_object(&harness);
    let ends = |h: &Harness<'static, State>| match primitive(h, id) {
        Primitive::Cylinder(p) => (p.cap_segments, p.cap_centre),
        other => panic!("{other:?}"),
    };
    assert_eq!(ends(&harness), (1, false), "one n-gon each end as standard");
    click_with(&mut harness, "Increase Cap segments", Modifiers::NONE);
    assert_eq!(ends(&harness), (2, false));
    click_field(&mut harness, "Centre point");
    assert_eq!(ends(&harness), (2, true));
}

#[test]
fn cones_and_discs_have_the_end_settings_too() {
    for (name, labels) in [
        ("Cone", ["Cap segments", "Centre point"]),
        ("Disc", ["Ring segments", "Centre point"]),
    ] {
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

// ------------------------------------------- meshes and recorded edits ---

#[test]
fn a_mesh_shows_its_size_instead_of_settings() {
    let mut harness = selected_cube();
    let id = only_object(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::MakeEditable(vec![id]))
        .unwrap();
    harness.run();
    assert!(harness.query_by_label("Cube (mesh)").is_some());
    assert!(harness.query_by_label("Points: 8").is_some());
    assert!(harness.query_by_label("Polygons: 6").is_some());
    assert!(
        harness.query_all_by_label("Width").next().is_none(),
        "no shape settings"
    );
    assert!(harness.query_by_label("Lengths in metres").is_none());
}

fn record_lift(harness: &mut Harness<'static, State>, id: ObjectId, points: &[u32]) {
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Record {
            id,
            operation: scene::Operation::MovePoints {
                points: points
                    .iter()
                    .map(|n| scene::VertexId::from_raw(*n))
                    .collect(),
                offset: glam::Vec3::Z * 10.0,
            },
        })
        .unwrap();
    harness.run();
}

#[test]
fn recorded_edits_are_counted_and_settings_stay_editable() {
    let mut harness = selected_cube();
    let id = only_object(&harness);
    assert!(harness.query_by_label_contains("recorded on top").is_none());
    record_lift(&mut harness, id, &[0]);
    record_lift(&mut harness, id, &[1]);
    assert!(
        harness
            .query_by_label("2 edits recorded on top of the shape")
            .is_some()
    );
    // The settings are still there, and changing them keeps the edits.
    click_with(&mut harness, "Increase Width", Modifiers::NONE);
    assert_eq!(cube_size(&harness, id).x, 101.0);
    let object = harness
        .state()
        .workbench
        .document()
        .scene()
        .object(id)
        .unwrap()
        .clone();
    assert_eq!(object.stack.operation_count(), 2);
    assert!(harness.query_by_label_contains("no longer").is_none());
}

#[test]
fn edits_that_no_longer_fit_are_flagged() {
    let mut harness = selected_cube();
    let id = only_object(&harness);
    record_lift(&mut harness, id, &[0, 900]);
    assert!(
        harness
            .query_by_label("1 edit no longer fully fits the shape")
            .is_some()
    );
}

// ----------------------------------------------------- the mesh section ---

#[test]
fn every_shape_shows_what_its_mesh_is_made_of() {
    let harness = selected_cube();
    for label in ["Points: 8", "Edges: 12", "Polygons: 6", "Triangles: 12"] {
        assert!(harness.query_by_label(label).is_some(), "{label}");
    }
    assert!(harness.query_by_label("Closed").is_some());
    assert!(harness.query_by_label("No problems found").is_some());
    assert!(
        harness.query_by_label_contains("Pieces").is_none(),
        "one piece"
    );
}

#[test]
fn the_counts_follow_the_settings() {
    let mut harness = selected_cube();
    click_with(&mut harness, "Increase Segments X", Modifiers::NONE);
    // Two segments along X: 12 points, 20 edges, 10 faces.
    assert!(harness.query_by_label("Points: 12").is_some());
    assert!(harness.query_by_label("Edges: 20").is_some());
    assert!(harness.query_by_label("Polygons: 10").is_some());
    assert!(harness.query_by_label("Triangles: 20").is_some());
}

#[test]
fn an_open_shape_says_so_without_calling_it_a_problem() {
    let mut harness = empty();
    harness.get_by_label("Create plane").click();
    harness.run();
    assert!(harness.query_by_label("Open (4 border edges)").is_some());
    assert!(harness.query_by_label("No problems found").is_some());
}

#[test]
fn problems_from_recorded_edits_are_flagged() {
    let mut harness = selected_cube();
    let id = only_object(&harness);
    // Move point 0 onto one of its neighbours: an edge with no length.
    let shape = harness
        .state()
        .workbench
        .document()
        .scene()
        .evaluated(id)
        .unwrap();
    let topology = mesh::Topology::new(&shape.mesh);
    let p0 = scene::VertexId::from_raw(0);
    let neighbour = topology.vertex_neighbours(p0)[0];
    let offset = shape.mesh.position(neighbour).unwrap() - shape.mesh.position(p0).unwrap();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Record {
            id,
            operation: scene::Operation::MovePoints {
                points: vec![p0],
                offset,
            },
        })
        .unwrap();
    harness.run();
    assert!(harness.query_by_label("1 edge with no length").is_some());
    assert!(harness.query_by_label("No problems found").is_none());
    // Undo: clean again.
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert!(harness.query_by_label("No problems found").is_some());
}

#[test]
fn a_plain_mesh_also_explains_itself() {
    let mut harness = selected_cube();
    let id = only_object(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::MakeEditable(vec![id]))
        .unwrap();
    harness.run();
    assert!(harness.query_by_label("Triangles: 12").is_some());
    assert!(harness.query_by_label_contains("A plain mesh").is_some());
}

#[test]
fn a_group_has_no_mesh_section() {
    let mut harness = empty();
    harness.get_by_label("Create group").click();
    harness.run();
    assert!(harness.query_by_label_contains("Triangles").is_none());
}
