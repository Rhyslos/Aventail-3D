//! Tests for the Scatter modifier in the window (decision 0146): the
//! surface selected first, the shapes Ctrl + clicked after; the palette
//! button, the Mesh menu and command search add it; the Attributes panel
//! shows its settings, and Use selected limits it to chosen polygons.

use super::modelling_tests::{H, cube, press};
use super::tests::{fresh, mesh_group};
use egui::{Key, Modifiers};
use egui_kittest::kittest::{NodeT, Queryable};
use scene::{
    Command, Element, ElementSelection, Modifier, ModifierKind, ObjectId, ObjectKind, PrimitiveKind,
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

/// The fresh window's cube with a sphere made beside it, nothing
/// selected: the cube is the surface, the sphere the shape.
fn cube_and_sphere() -> (H, ObjectId, ObjectId) {
    let mut harness = fresh();
    let surface = cube(&harness);
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Sphere.standard(),
    )))
    .unwrap();
    let shape = doc.selection()[0];
    doc.clear_selection();
    harness.run();
    (harness, surface, shape)
}

/// Selects `first`, then adds `then` (as Ctrl + click does).
fn pick_in_order(harness: &mut H, first: ObjectId, then: ObjectId) {
    let doc = harness.state_mut().workbench.document_mut();
    doc.set_selection([first]);
    doc.set_selection([first, then]);
    harness.run();
}

fn button_disabled(harness: &H) -> bool {
    harness
        .get_all_by_label("Scatter onto surface")
        .all(|n| n.accesskit_node().is_disabled())
}

#[test]
fn the_palette_button_scatters_the_others_over_the_first_selected() {
    let (mut harness, surface, shape) = cube_and_sphere();
    assert!(button_disabled(&harness), "nothing selected");
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([surface]);
    harness.run();
    assert!(button_disabled(&harness), "nothing to scatter");
    pick_in_order(&mut harness, surface, shape);
    harness
        .get_all_by_label("Scatter onto surface")
        .find(|n| !n.accesskit_node().is_disabled())
        .expect("the button")
        .click();
    harness.run();
    let list = modifiers(&harness, surface);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].kind.sources(), vec![shape]);
    assert!(modifiers(&harness, shape).is_empty());
    // Its settings show with the surface selected.
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([surface]);
    harness.run();
    for name in [
        "Shapes",
        "Place on",
        "Count",
        "Seed",
        "Spacing",
        "Align to surface",
        "Random turn",
        "Smallest size",
        "Largest size",
        "Offset",
        "On polygons",
    ] {
        assert!(harness.query_all_by_label(name).count() >= 1, "{name}");
    }
    // A switch is one undo step.
    harness
        .get_all_by_label("Align to surface")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::CheckBox)
        .expect("the switch")
        .scroll_to_me();
    harness.run();
    harness
        .get_all_by_label("Align to surface")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::CheckBox)
        .expect("the switch")
        .click();
    harness.run();
    let ModifierKind::Scatter(settings) = &modifiers(&harness, surface)[0].kind else {
        panic!("scatter")
    };
    assert!(!settings.options.align);
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    let ModifierKind::Scatter(settings) = &modifiers(&harness, surface)[0].kind else {
        panic!("scatter")
    };
    assert!(settings.options.align);
    // Select shapes selects the sphere.
    harness.get_by_label("Select shapes").click();
    harness.run();
    assert_eq!(
        harness.state().workbench.document().selection(),
        vec![shape]
    );
}

#[test]
fn use_selected_limits_the_copies_to_chosen_polygons() {
    let (mut harness, surface, shape) = cube_and_sphere();
    pick_in_order(&mut harness, surface, shape);
    let add = crate::modifiers::command(harness.state().workbench.document(), {
        crate::commands::CommandId::Scatter
    })
    .unwrap();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(add)
        .unwrap();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([surface]);
    harness.run();
    super::tests::open_modifier(&mut harness, surface, 0);
    let use_selected = |h: &H| {
        h.get_all_by_label("Use selected")
            .next()
            .expect("the button")
            .accesskit_node()
            .is_disabled()
    };
    assert!(use_selected(&harness), "not in the Polygons mode");
    press(&mut harness, Modifiers::NONE, Key::Num3);
    let top: Vec<Element> = {
        let doc = harness.state().workbench.document();
        let mesh = doc.scene().unmodified_shape(surface).unwrap();
        mesh.faces()
            .map(|(f, _)| f)
            .filter(|f| mesh.face_normal(*f).z > 0.9)
            .map(Element::Polygon)
            .collect()
    };
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements([(surface, ElementSelection::of(top))], "Select", None);
    harness.run();
    harness
        .get_all_by_label("Use selected")
        .next()
        .unwrap()
        .scroll_to_me();
    harness.run();
    assert!(!use_selected(&harness));
    harness.get_by_label("Use selected").click();
    harness.run();
    let ModifierKind::Scatter(settings) = &modifiers(&harness, surface)[0].kind else {
        panic!("scatter")
    };
    assert_eq!(settings.polygons.len(), 1);
    assert!(harness.query_all_by_label("1 chosen").count() >= 1);
}

#[test]
fn the_mesh_menu_and_command_search_offer_it_under_generate() {
    let (mut harness, surface, shape) = cube_and_sphere();
    let entries = harness.state().workbench.search_entries(&harness.ctx);
    let entry = entries
        .iter()
        .find(|e| e.name == "Scatter onto surface")
        .unwrap();
    assert_eq!(entry.place, "Mesh");
    assert!(entry.not_now.is_some(), "nothing selected");
    pick_in_order(&mut harness, surface, shape);
    mesh_group(&mut harness, super::mesh_menu::MESH_MODIFIERS_MENU);
    harness
        .get_all_by_label("Scatter onto surface")
        .find(|n| !n.accesskit_node().is_disabled())
        .expect("Scatter can be chosen")
        .click();
    harness.run();
    assert_eq!(modifiers(&harness, surface).len(), 1);
}
