//! Tests for the Displace modifier in the window (decision 0157): the
//! palette button, the Mesh menu and command search add it, its settings
//! show in the Attributes panel, the Noise list changes it in one undo
//! step, and Use selected limits it to chosen points.

use super::modelling_tests::{H, press};
use super::tests::{empty, mesh_group};
use egui::{Key, Modifiers};
use egui_kittest::kittest::{NodeT, Queryable};
use scene::{
    Command, DisplaceSettings, Element, ElementSelection, ModifierKind, NoiseKind, ObjectId,
    ObjectKind,
};

/// A window with a sphere, selected, in the Model mode.
fn sphere() -> (H, ObjectId) {
    let mut harness = empty();
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(Command::Create(ObjectKind::Primitive(
        scene::PrimitiveKind::Sphere.standard(),
    )))
    .unwrap();
    let id = doc.selection()[0];
    harness.run();
    (harness, id)
}

fn settings(harness: &H, id: ObjectId) -> DisplaceSettings {
    match &harness.state().workbench.document().scene().modifiers(id)[0].kind {
        ModifierKind::Displace(settings) => settings.clone(),
        other => panic!("{other:?}"),
    }
}

fn shows(harness: &H, name: &str) -> bool {
    harness.query_all_by_label(name).count() >= 1
}

#[test]
fn the_palette_button_roughens_the_sphere() {
    let (mut harness, id) = sphere();
    harness.get_by_label("Displace").click();
    harness.run();
    assert_eq!(settings(&harness, id), DisplaceSettings::default());
    let doc = harness.state().workbench.document();
    let input = doc.scene().unmodified_shape(id).unwrap();
    let shown = doc.scene().shown_shape(id).unwrap();
    assert!(shown.same_structure(&input));
    assert_ne!(*shown, *input);
    for name in [
        "Noise",
        "Strength",
        "Scale",
        "Detail",
        "Seed",
        "Direction",
        "Points",
        "Falloff",
    ] {
        assert!(shows(&harness, name), "{name}");
    }
}

#[test]
fn the_noise_list_changes_it() {
    let (mut harness, id) = sphere();
    harness.get_by_label("Displace").click();
    harness.run();
    let list = || {
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::ComboBox
    };
    harness
        .get_all_by_label("Noise")
        .find(list())
        .expect("the Noise list")
        .scroll_to_me();
    harness.run_steps(4);
    harness
        .get_all_by_label("Noise")
        .find(list())
        .expect("the Noise list")
        .click();
    harness.run_steps(4);
    harness.get_by_label("Cells").click();
    harness.run();
    assert_eq!(settings(&harness, id).options.noise, NoiseKind::Cells);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Change Displace")
    );
}

#[test]
fn use_selected_limits_it_to_chosen_points() {
    let (mut harness, id) = sphere();
    harness.get_by_label("Displace").click();
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::Num1);
    let top: Vec<Element> = {
        let doc = harness.state().workbench.document();
        let mesh = doc.scene().unmodified_shape(id).unwrap();
        let high = mesh.bounds().unwrap().max.z;
        mesh.vertices()
            .filter(|(_, p)| p.z > high - 20.0)
            .map(|(v, _)| Element::Point(v))
            .collect()
    };
    let count = top.len();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements([(id, ElementSelection::of(top))], "Select", None);
    harness.run();
    harness
        .get_all_by_label("Use selected")
        .next()
        .unwrap()
        .scroll_to_me();
    harness.run();
    harness.get_by_label("Use selected").click();
    harness.run();
    assert_eq!(settings(&harness, id).points.len(), count);
    assert!(shows(&harness, &format!("{count} chosen")));
    // The bottom of the sphere, far from the chosen points, stays.
    let doc = harness.state().workbench.document();
    let input = doc.scene().unmodified_shape(id).unwrap();
    let shown = doc.scene().shown_shape(id).unwrap();
    let low = input.bounds().unwrap().min.z;
    for (v, p) in input.vertices().filter(|(_, p)| p.z < low + 20.0) {
        assert_eq!(shown.position(v), Some(p));
    }
}

#[test]
fn the_mesh_menu_and_command_search_offer_it_under_deform() {
    let (mut harness, id) = sphere();
    let entries = harness.state().workbench.search_entries(&harness.ctx);
    let entry = entries.iter().find(|e| e.name == "Displace").unwrap();
    assert_eq!(entry.place, "Mesh");
    assert!(entry.not_now.is_none());
    mesh_group(&mut harness, super::mesh_menu::MESH_MODIFIERS_MENU);
    harness
        .get_all_by_label("Displace")
        .find(|n| !n.accesskit_node().is_disabled())
        .expect("Displace can be chosen")
        .click();
    harness.run();
    let doc = harness.state().workbench.document();
    assert_eq!(doc.scene().modifiers(id).len(), 1);
    assert_eq!(doc.undo_label(), Some("Add Displace"));
}
