//! Tests for Bend along curve and Shrinkwrap onto in the window (decision
//! 0147): the shape selected first, the curve or target Ctrl + clicked
//! after; the palette buttons, the Mesh menu and command search add them;
//! the Attributes panel shows their settings, and Use selected limits a
//! shrinkwrap to chosen points.

use super::modelling_tests::{H, cube, press};
use super::tests::{fresh, mesh_group};
use egui::{Key, Modifiers};
use egui_kittest::kittest::{NodeT, Queryable};
use scene::{
    Command, CurveObject, CurveShape, Element, ElementSelection, Modifier, ModifierKind, ObjectId,
    ObjectKind, PrimitiveKind, ReadyCurve,
};

fn modifiers(harness: &H, id: ObjectId) -> Vec<Modifier> {
    harness
        .state()
        .workbench
        .document()
        .scene()
        .modifiers(id)
        .to_vec()
}

/// The fresh window's cube, with a circle curve and a plane made beside
/// it, nothing selected.
fn cube_curve_and_plane() -> (H, ObjectId, ObjectId, ObjectId) {
    let mut harness = fresh();
    let shape = cube(&harness);
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(Command::Create(ObjectKind::Curve(CurveObject::line(
        CurveShape::Ready(ReadyCurve::Circle { radius: 200.0 }),
    ))))
    .unwrap();
    let curve = doc.selection()[0];
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Plane.standard(),
    )))
    .unwrap();
    let plane = doc.selection()[0];
    doc.clear_selection();
    harness.run();
    (harness, shape, curve, plane)
}

/// Selects `first`, then adds `then` (as Ctrl + click does).
fn pick_in_order(harness: &mut H, first: ObjectId, then: ObjectId) {
    let doc = harness.state_mut().workbench.document_mut();
    doc.set_selection([first]);
    doc.set_selection([first, then]);
    harness.run();
}

fn enabled(harness: &H, name: &str) -> bool {
    harness
        .get_all_by_label(name)
        .any(|n| !n.accesskit_node().is_disabled())
}

fn click(harness: &mut H, name: &str) {
    harness
        .get_all_by_label(name)
        .find(|n| !n.accesskit_node().is_disabled())
        .expect("the button")
        .click();
    harness.run();
}

#[test]
fn the_palette_buttons_need_the_shape_then_the_curve_or_target() {
    let (mut harness, shape, curve, plane) = cube_curve_and_plane();
    assert!(!enabled(&harness, "Bend along curve"), "nothing selected");
    assert!(!enabled(&harness, "Shrinkwrap onto"), "nothing selected");
    pick_in_order(&mut harness, shape, plane);
    assert!(!enabled(&harness, "Bend along curve"), "not a curve");
    assert!(enabled(&harness, "Shrinkwrap onto"));
    pick_in_order(&mut harness, shape, curve);
    click(&mut harness, "Bend along curve");
    let list = modifiers(&harness, shape);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].kind.sources(), vec![curve]);
    assert!(modifiers(&harness, curve).is_empty());
    // Its settings show with the shape selected.
    let doc = harness.state_mut().workbench.document_mut();
    doc.set_selection([shape]);
    harness.run();
    for name in ["Curve", "Axis", "Stretch to fit", "Offset", "Twist"] {
        assert!(harness.query_all_by_label(name).count() >= 1, "{name}");
    }
    // A switch is one undo step.
    for scroll in [true, false] {
        let switch = harness
            .get_all_by_label("Stretch to fit")
            .find(|n| n.accesskit_node().role() == egui::accesskit::Role::CheckBox)
            .expect("the switch");
        if scroll {
            switch.scroll_to_me();
        } else {
            switch.click();
        }
        harness.run();
    }
    let stretched = |h: &H| match &modifiers(h, shape)[0].kind {
        ModifierKind::BendAlongCurve(b) => b.options.stretch,
        other => panic!("{other:?}"),
    };
    assert!(stretched(&harness));
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    assert!(!stretched(&harness));
    // Select curve selects it.
    harness.get_by_label("Select curve").click();
    harness.run();
    assert_eq!(
        harness.state().workbench.document().selection(),
        vec![curve]
    );
}

#[test]
fn use_selected_limits_the_wrap_to_chosen_points() {
    let (mut harness, shape, _, plane) = cube_curve_and_plane();
    pick_in_order(&mut harness, shape, plane);
    click(&mut harness, "Shrinkwrap onto");
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([shape]);
    harness.run();
    for name in ["Target", "Mode", "Offset", "Points"] {
        assert!(harness.query_all_by_label(name).count() >= 1, "{name}");
    }
    let disabled = |h: &H| {
        h.get_all_by_label("Use selected")
            .next()
            .expect("the button")
            .accesskit_node()
            .is_disabled()
    };
    assert!(disabled(&harness), "not in the Points mode");
    press(&mut harness, Modifiers::NONE, Key::Num1);
    let top: Vec<Element> = {
        let doc = harness.state().workbench.document();
        let mesh = doc.scene().unmodified_shape(shape).unwrap();
        let high = mesh.bounds().unwrap().max.z;
        mesh.vertices()
            .filter(|(_, p)| (p.z - high).abs() < 1e-3)
            .map(|(v, _)| Element::Point(v))
            .collect()
    };
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements([(shape, ElementSelection::of(top))], "Select", None);
    harness.run();
    harness
        .get_all_by_label("Use selected")
        .next()
        .unwrap()
        .scroll_to_me();
    harness.run();
    assert!(!disabled(&harness));
    harness.get_by_label("Use selected").click();
    harness.run();
    let ModifierKind::Shrinkwrap(settings) = &modifiers(&harness, shape)[0].kind else {
        panic!("a shrinkwrap")
    };
    assert_eq!(settings.points.len(), 4);
    assert!(harness.query_all_by_label("4 chosen").count() >= 1);
}

#[test]
fn the_mesh_menu_and_command_search_offer_them_under_deform() {
    let (mut harness, shape, curve, plane) = cube_curve_and_plane();
    let entries = harness.state().workbench.search_entries(&harness.ctx);
    for name in ["Bend along curve", "Shrinkwrap onto"] {
        let entry = entries.iter().find(|e| e.name == name).unwrap();
        assert_eq!(entry.place, "Mesh");
        assert!(entry.not_now.is_some(), "nothing selected");
    }
    pick_in_order(&mut harness, shape, curve);
    mesh_group(&mut harness, super::mesh_menu::MESH_MODIFIERS_MENU);
    click(&mut harness, "Bend along curve");
    assert_eq!(modifiers(&harness, shape).len(), 1);
    pick_in_order(&mut harness, shape, plane);
    mesh_group(&mut harness, super::mesh_menu::MESH_MODIFIERS_MENU);
    click(&mut harness, "Shrinkwrap onto");
    assert_eq!(modifiers(&harness, shape).len(), 2);
}
