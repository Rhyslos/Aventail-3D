//! Tests for the *Bend, twist or taper* modifier in the window (decision
//! 0158): the palette button, the Mesh menu and command search add it,
//! its settings show in the Attributes panel by kind, the Kind list
//! changes it in one undo step, and its helper is drawn over the view
//! while the shape is selected.

use super::modelling_tests::H;
use super::tests::{empty, mesh_group};
use egui_kittest::kittest::{NodeT, Queryable};
use scene::{AxisDeformKind, AxisDeformOptions, Command, ModifierKind, ObjectId, ObjectKind};

/// A window with a pole cut into slices, selected, in the Model mode.
fn pole() -> (H, ObjectId) {
    let mut harness = empty();
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(Command::Create(ObjectKind::Primitive(
        scene::Primitive::Cylinder(mesh::primitives::CylinderParams {
            height: 200.0,
            height_segments: 20,
            ..Default::default()
        }),
    )))
    .unwrap();
    let id = doc.selection()[0];
    harness.run();
    (harness, id)
}

fn options(harness: &H, id: ObjectId) -> AxisDeformOptions {
    match &harness.state().workbench.document().scene().modifiers(id)[0].kind {
        ModifierKind::BendTwistTaper(options) => *options,
        other => panic!("{other:?}"),
    }
}

fn shows(harness: &H, name: &str) -> bool {
    harness.query_all_by_label(name).count() >= 1
}

/// How many helper lines the overlay holds.
fn helper_lines(harness: &H) -> usize {
    harness
        .state()
        .workbench
        .components
        .overlay()
        .lines
        .iter()
        .filter(|(_, s)| *s == crate::OverlayStyle::Helper)
        .count()
}

#[test]
fn the_palette_button_bends_the_pole_and_draws_its_helper() {
    let (mut harness, id) = pole();
    assert_eq!(helper_lines(&harness), 0);
    harness.get_by_label("Bend, twist or taper").click();
    harness.run();
    assert_eq!(options(&harness, id), AxisDeformOptions::default());
    let doc = harness.state().workbench.document();
    assert_eq!(doc.undo_label(), Some("Add Bend"));
    let input = doc.scene().unmodified_shape(id).unwrap();
    let shown = doc.scene().shown_shape(id).unwrap();
    assert!(shown.same_structure(&input));
    assert_ne!(*shown, *input);
    for name in [
        "Kind",
        "Axis",
        "Angle",
        "Direction",
        "Limit from",
        "Limit to",
        "Centre X",
        "Centre Z",
    ] {
        assert!(shows(&harness, name), "{name}");
    }
    assert!(!shows(&harness, "Amount"), "a bend has no taper amount");
    // The axis (32 lines), two limit crosses (4) and the centre (3).
    assert_eq!(helper_lines(&harness), 39);
    // Not selected, no helper.
    harness
        .state_mut()
        .workbench
        .document_mut()
        .clear_selection();
    harness.run();
    assert_eq!(helper_lines(&harness), 0);
}

#[test]
fn the_kind_list_makes_it_a_taper() {
    let (mut harness, id) = pole();
    harness.get_by_label("Bend, twist or taper").click();
    harness.run();
    let list = || {
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::ComboBox
    };
    harness
        .get_all_by_label("Kind")
        .find(list())
        .expect("the Kind list")
        .scroll_to_me();
    harness.run_steps(4);
    harness
        .get_all_by_label("Kind")
        .find(list())
        .expect("the Kind list")
        .click();
    harness.run_steps(4);
    harness.get_by_label("Taper").click();
    harness.run();
    assert_eq!(options(&harness, id).kind, AxisDeformKind::Taper);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Change Taper")
    );
    assert!(shows(&harness, "Amount") && shows(&harness, "Curve"));
    assert!(!shows(&harness, "Direction"));
}

#[test]
fn the_mesh_menu_and_command_search_offer_it_under_deform() {
    let (mut harness, id) = pole();
    let entries = harness.state().workbench.search_entries(&harness.ctx);
    let entry = entries
        .iter()
        .find(|e| e.name == "Bend, twist or taper")
        .unwrap();
    assert_eq!(entry.place, "Mesh");
    assert!(entry.not_now.is_none());
    mesh_group(&mut harness, super::mesh_menu::MESH_MODIFIERS_MENU);
    harness
        .get_all_by_label("Bend, twist or taper")
        .find(|n| !n.accesskit_node().is_disabled())
        .expect("Bend, twist or taper can be chosen")
        .click();
    harness.run();
    let doc = harness.state().workbench.document();
    assert_eq!(doc.scene().modifiers(id).len(), 1);
    assert_eq!(doc.undo_label(), Some("Add Bend"));
}
