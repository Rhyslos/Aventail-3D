//! Tests for Extrude's settings in the Attributes panel around strips
//! grown from edges and points (the project lead's find): the shape and
//! bend rows stay after a strip instead of vanishing until Ctrl + Z, are
//! greyed out while the next drag grows a strip (which takes no shape),
//! and set the next extrude of polygons, which follows them. Inset's
//! Keep together follows the same way.

use super::modelling_tests::{
    H, cube, drag, drag_by, empty_spot, last_operation, note, on_screen, press, ready, screen_x,
    shape,
};
use super::*;
use egui::{Event, PointerButton};
use egui_kittest::kittest::{NodeT, Queryable};
use scene::{Element, ElementSelection, ExtrudeShape, Grouping, Operation};

/// The rows Extrude shows for its next drag, by their names.
const ROWS: [&str; 6] = [
    "Extrude direction",
    "End size",
    "Extrude steps",
    "Flat end",
    "Bend radius",
    "Bend segments",
];

/// A plane in place of the cube, selected, in the mode `mode_key` picks.
fn plane(mode_key: Key) -> H {
    let mut harness = super::tests::fresh();
    let doc = harness.state_mut().workbench.document_mut();
    let old = doc.scene().children(None)[0];
    doc.execute(scene::Command::Delete(vec![old])).unwrap();
    doc.execute(scene::Command::Create(scene::ObjectKind::Primitive(
        scene::Primitive::Plane(Default::default()),
    )))
    .unwrap();
    harness.run();
    press(&mut harness, Modifiers::NONE, mode_key);
    harness
}

/// The plane with its +X side selected, as an edge (in the Edges mode)
/// or its two points (in the Points mode), and Extrude in use.
fn plane_side(mode_key: Key) -> H {
    let mut harness = plane(mode_key);
    let id = cube(&harness);
    let mesh = shape(&harness);
    let right = |v: scene::VertexId| mesh.position(v).is_some_and(|p| p.x > 49.0);
    let side: Vec<Element> = if mode_key == Key::Num2 {
        mesh.edges()
            .iter()
            .filter(|e| right(e.ends().0) && right(e.ends().1))
            .map(|e| Element::Edge(*e))
            .collect()
    } else {
        mesh.vertices()
            .map(|(v, _)| v)
            .filter(|v| right(*v))
            .map(Element::Point)
            .collect()
    };
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements([(id, ElementSelection::of(side))], "Select", None);
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::D);
    harness
}

/// Grows the strip: a drag towards +X.
fn grow(harness: &mut H) {
    let from = empty_spot(harness);
    let way = screen_x(harness, 80.0);
    drag_by(harness, from, way);
}

/// Whether every one of Extrude's rows is in the panel, and whether each
/// is greyed out.
fn rows(harness: &H) -> Vec<(&'static str, Option<bool>)> {
    ROWS.iter()
        .map(|name| {
            // The row's name and its field share the label: the field
            // is the one that can be greyed out.
            let nodes: Vec<bool> = harness
                .query_all_by_label(name)
                .map(|n| n.accesskit_node().is_disabled())
                .collect();
            let greyed = (!nodes.is_empty()).then(|| nodes.iter().any(|g| *g));
            (*name, greyed)
        })
        .collect()
}

/// The one Keep together switch.
fn keep_together(harness: &H) -> egui_kittest::Node<'_> {
    let mut switches = harness
        .get_all_by_label("Keep together")
        .filter(|n| n.accesskit_node().role() == egui::accesskit::Role::CheckBox);
    let switch = switches.next().expect("a Keep together switch");
    assert!(switches.next().is_none(), "one Keep together switch");
    switch
}

fn all_greyed(harness: &H) -> bool {
    rows(harness).iter().all(|(_, g)| *g == Some(true))
}

fn none_greyed(harness: &H) -> bool {
    rows(harness).iter().all(|(_, g)| *g == Some(false))
}

#[test]
fn after_a_strip_from_an_edge_or_points_the_extrude_rows_stay() {
    // Regression: the rows vanished after the drag (the strip's Last edit
    // has only a Distance and Keep together) and came back only with
    // Ctrl + Z.
    for mode_key in [Key::Num2, Key::Num1] {
        let mut harness = plane_side(mode_key);
        grow(&mut harness);
        assert!(
            matches!(
                last_operation(&harness),
                Some(Operation::ExtrudeEdges { .. })
            ),
            "{mode_key:?}"
        );
        assert!(
            harness
                .query_by_label_contains("Last edit: Extrude 1 edge")
                .is_some(),
            "{mode_key:?}"
        );
        assert!(all_greyed(&harness), "{mode_key:?}: {:?}", rows(&harness));
        // The strip's own Distance is still there to change.
        assert!(harness.query_all_by_label("Distance").count() >= 1);
    }
}

#[test]
fn while_the_next_drag_grows_a_strip_the_shape_rows_are_greyed_out() {
    // A strip grows flat, so Direction, End size, Steps, Flat end and the
    // bend did nothing to it: they are greyed out, and say why.
    for mode_key in [Key::Num2, Key::Num1] {
        let harness = plane_side(mode_key);
        assert!(all_greyed(&harness), "{mode_key:?}: {:?}", rows(&harness));
        assert!(harness.state().workbench.attributes.extrude_grows_strips);
        // Keep together does apply to strips.
        assert!(!keep_together(&harness).accesskit_node().is_disabled());
    }
    // In the Polygons mode they shape the next drag.
    let harness = ready(Key::D);
    assert!(none_greyed(&harness), "{:?}", rows(&harness));
    assert!(!harness.state().workbench.attributes.extrude_grows_strips);
}

#[test]
fn edges_or_points_all_the_way_round_a_polygon_keep_the_shape_rows() {
    // Decision 0127: the cube's top rim pushes the top out, which the
    // rows shape.
    for (mode_key, kind) in [
        (Key::Num2, scene::ElementKind::Edges),
        (Key::Num1, scene::ElementKind::Points),
    ] {
        let mut harness = ready(Key::Q);
        let id = cube(&harness);
        press(&mut harness, Modifiers::NONE, mode_key);
        let mesh = shape(&harness);
        let top = mesh
            .faces()
            .map(|(f, _)| f)
            .max_by(|a, b| mesh.face_normal(*a).z.total_cmp(&mesh.face_normal(*b).z))
            .unwrap();
        let rim: Vec<Element> = if kind == scene::ElementKind::Edges {
            let topology = mesh.topology();
            topology
                .face_edges(top)
                .iter()
                .map(|e| Element::Edge(*e))
                .collect()
        } else {
            mesh.face(top)
                .unwrap()
                .iter()
                .map(|v| Element::Point(*v))
                .collect()
        };
        // Part of the rim only: a strip (with nothing open to grow from).
        harness
            .state_mut()
            .workbench
            .document_mut()
            .select_elements(
                [(id, ElementSelection::of(rim[..2].to_vec()))],
                "Select",
                None,
            );
        harness.run();
        press(&mut harness, Modifiers::NONE, Key::D);
        assert!(all_greyed(&harness), "{kind:?}: {:?}", rows(&harness));
        // All the way round: the top is pushed out, shaped by the rows.
        harness
            .state_mut()
            .workbench
            .document_mut()
            .select_elements([(id, ElementSelection::of(rim))], "Select", None);
        harness.run();
        assert!(none_greyed(&harness), "{kind:?}: {:?}", rows(&harness));
        harness.state_mut().workbench.attributes.extrude.steps = 3;
        drag(&mut harness, 60.0);
        match last_operation(&harness) {
            Some(Operation::ExtrudePolygons { shape, .. }) => assert_eq!(shape.steps, 3),
            other => panic!("{kind:?}: {other:?}"),
        }
        // Its Last edit shows its own rows, not greyed.
        assert!(none_greyed(&harness), "{kind:?}: {:?}", rows(&harness));
    }
}

#[test]
fn after_a_strip_the_rows_set_the_next_extrude_of_polygons() {
    // The project lead's case: extrude an edge, then change the settings
    // and drag a polygon. With the strip still the Last edit, the rows
    // under it are the tool's, and the polygon follows them.
    let mut harness = plane_side(Key::Num2);
    grow(&mut harness);
    let strips = shape(&harness).face_count();
    // 3 puts Extrude away (decision 0060); D brings it back.
    press(&mut harness, Modifiers::NONE, Key::Num3);
    press(&mut harness, Modifiers::NONE, Key::D);
    assert!(
        harness
            .query_by_label_contains("Last edit: Extrude 1 edge")
            .is_some()
    );
    assert!(none_greyed(&harness), "{:?}", rows(&harness));
    for _ in 0..2 {
        harness.get_by_label("Increase Extrude steps").click();
        harness.run();
    }
    assert_eq!(harness.state().workbench.attributes.extrude.steps, 3);
    // The strip itself is left as it was.
    assert!(matches!(
        last_operation(&harness),
        Some(Operation::ExtrudeEdges { .. })
    ));
    // Drag straight on the plane's middle: that polygon is extruded,
    // with three rows of walls.
    let middle = on_screen(&harness, glam::Vec3::ZERO);
    harness.event(Event::PointerMoved(middle));
    harness.run();
    harness.run();
    let up = on_screen(&harness, glam::Vec3::Z * 100.0) - middle;
    drag_by(&mut harness, middle, up.normalized() * 60.0);
    match last_operation(&harness) {
        Some(Operation::ExtrudePolygons { shape: made, .. }) => {
            assert_eq!(made.steps, 3);
        }
        other => panic!("{other:?}"),
    }
    // One polygon with three rows of four walls.
    assert_eq!(shape(&harness).face_count(), strips + 12);
}

#[test]
fn ctrl_and_the_wheel_say_a_strip_has_no_steps() {
    let mut harness = plane_side(Key::Num2);
    let steps = harness.state().workbench.attributes.extrude.steps;
    let at = on_screen(&harness, glam::Vec3::ZERO);
    super::modelling_tests::ctrl_wheel(&mut harness, at, true);
    assert_eq!(harness.state().workbench.attributes.extrude.steps, steps);
    assert!(note(&harness).contains("flat strip"), "{}", note(&harness));
}

#[test]
fn after_ctrl_z_changed_settings_shape_the_next_polygon_extrude() {
    // Regression for the other half of the find: the settings changed
    // after an undo are the ones the next drag uses.
    let mut harness = ready(Key::D);
    drag(&mut harness, 60.0);
    press(&mut harness, Modifiers::COMMAND, Key::Z);
    assert!(last_operation(&harness).is_none());
    assert!(none_greyed(&harness), "{:?}", rows(&harness));
    harness.get_by_label("Increase Extrude steps").click();
    harness.run();
    harness.get_by_label("Extrude direction").click();
    harness.run();
    harness.get_by_label("Along Z").click();
    harness.run();
    drag(&mut harness, 60.0);
    match last_operation(&harness) {
        Some(Operation::ExtrudePolygons { shape: made, .. }) => assert_eq!(
            made,
            ExtrudeShape {
                direction: scene::ExtrudeDirection::AlongZ,
                steps: 2,
                ..ExtrudeShape::default()
            }
        ),
        other => panic!("{other:?}"),
    }
    // And changed under Last edit, the next one follows too.
    harness.get_by_label("Increase Extrude steps").click();
    harness.run();
    drag(&mut harness, 60.0);
    match last_operation(&harness) {
        Some(Operation::ExtrudePolygons { shape: made, .. }) => assert_eq!(made.steps, 3),
        other => panic!("{other:?}"),
    }
}

#[test]
fn insets_keep_together_follows_after_an_inset_and_after_ctrl_z() {
    let mut harness = ready(Key::I);
    drag(&mut harness, 30.0);
    assert!(
        harness
            .query_by_label_contains("Last edit: Inset 1 polygon")
            .is_some()
    );
    // Its Keep together switch is there; switched off, the next inset is
    // made each on its own.
    let at = keep_together(&harness).rect().center();
    harness.event(Event::PointerMoved(at));
    harness.run();
    super::modelling_tests::button(
        &mut harness,
        at,
        true,
        PointerButton::Primary,
        Modifiers::NONE,
    );
    super::modelling_tests::button(
        &mut harness,
        at,
        false,
        PointerButton::Primary,
        Modifiers::NONE,
    );
    assert_eq!(harness.state().workbench.grouping(), Grouping::Separate);
    press(&mut harness, Modifiers::COMMAND, Key::Z);
    press(&mut harness, Modifiers::COMMAND, Key::Z);
    assert!(last_operation(&harness).is_none());
    let _ = keep_together(&harness);
    drag(&mut harness, 30.0);
    assert_eq!(
        last_operation(&harness).and_then(|op| op.grouping()),
        Some(Grouping::Separate)
    );
}
