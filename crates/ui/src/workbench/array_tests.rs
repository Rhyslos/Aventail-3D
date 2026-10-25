//! Tests for the Array modifier in the window (decision 0153): the
//! palette button, the Mesh menu and command search add it, its settings
//! show in the Attributes panel for each layout, and a ring's Centre list
//! offers the other objects.

use super::modelling_tests::H;
use super::tests::{empty, mesh_group};
use egui_kittest::kittest::{NodeT, Queryable};
use scene::{ArrayLayout, ArraySettings, Command, ModifierKind, ObjectId, ObjectKind};

/// A window with a cube, selected, in the Model mode, and a sphere.
fn cube_and_sphere() -> (H, ObjectId, ObjectId) {
    let mut harness = empty();
    let doc = harness.state_mut().workbench.document_mut();
    let mut made = Vec::new();
    for kind in [scene::PrimitiveKind::Cube, scene::PrimitiveKind::Sphere] {
        doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
            .unwrap();
        made.push(doc.selection()[0]);
    }
    let (cube, sphere) = (made[0], made[1]);
    doc.set_selection([cube]);
    harness.run();
    (harness, cube, sphere)
}

fn settings(harness: &H, id: ObjectId) -> ArraySettings {
    match &harness.state().workbench.document().scene().modifiers(id)[0].kind {
        ModifierKind::Array(settings) => *settings,
        other => panic!("{other:?}"),
    }
}

fn set_layout(harness: &mut H, id: ObjectId, layout: ArrayLayout) {
    let doc = harness.state_mut().workbench.document_mut();
    let mut modifier = doc.scene().modifiers(id)[0].clone();
    if let ModifierKind::Array(settings) = &mut modifier.kind {
        settings.options.layout = layout;
    }
    doc.execute(Command::ChangeModifier {
        id,
        index: 0,
        modifier,
    })
    .unwrap();
    harness.run();
}

fn shows(harness: &H, name: &str) -> bool {
    harness.query_all_by_label(name).count() >= 1
}

#[test]
fn the_palette_button_adds_three_copies_in_a_line() {
    let (mut harness, cube_id, _) = cube_and_sphere();
    harness.get_by_label("Array").click();
    harness.run();
    assert_eq!(settings(&harness, cube_id), ArraySettings::default());
    let shown = harness
        .state()
        .workbench
        .document()
        .scene()
        .shown_shape(cube_id)
        .unwrap();
    assert_eq!(shown.face_count(), 18);
    for name in [
        "Layout",
        "Count",
        "Relative to size",
        "Offset X",
        "Offset Y",
        "Offset Z",
        "Merge touching copies",
        "UVs side by side",
    ] {
        assert!(shows(&harness, name), "{name}");
    }
    assert!(!shows(&harness, "Merge distance"), "only when merging");
    set_layout(&mut harness, cube_id, ArrayLayout::Grid);
    for name in ["Count X", "Count Y", "Count Z", "Spacing X", "Spacing Z"] {
        assert!(shows(&harness, name), "{name}");
    }
    set_layout(&mut harness, cube_id, ArrayLayout::Radial);
    for name in ["Angle", "Axis", "Centre", "Radius"] {
        assert!(shows(&harness, name), "{name}");
    }
}

#[test]
fn a_ring_s_centre_can_be_another_object() {
    let (mut harness, cube_id, sphere) = cube_and_sphere();
    harness.get_by_label("Array").click();
    harness.run();
    set_layout(&mut harness, cube_id, ArrayLayout::Radial);
    harness
        .get_all_by_label("Centre")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::ComboBox)
        .expect("the Centre list")
        .scroll_to_me();
    harness.run_steps(4);
    harness
        .get_all_by_label("Centre")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::ComboBox)
        .expect("the Centre list")
        .click();
    harness.run_steps(4);
    let name = harness
        .state()
        .workbench
        .document()
        .scene()
        .object(sphere)
        .unwrap()
        .name
        .clone();
    assert!(shows(&harness, "Its pivot"));
    // The list's entry is the narrow one (the Objects panel's row is as
    // wide as the panel).
    harness
        .get_all_by_label(&name)
        .min_by(|a, b| {
            let width = |n: &egui_kittest::Node<'_>| {
                n.accesskit_node()
                    .bounding_box()
                    .map_or(f64::MAX, |r| r.width())
            };
            width(a).total_cmp(&width(b))
        })
        .expect("the sphere in the list")
        .click();
    harness.run();
    assert_eq!(settings(&harness, cube_id).centre, Some(sphere));
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Change Array")
    );
}

#[test]
fn the_mesh_menu_and_command_search_offer_it_under_generate() {
    let (mut harness, cube_id, _) = cube_and_sphere();
    let entries = harness.state().workbench.search_entries(&harness.ctx);
    let entry = entries.iter().find(|e| e.name == "Array").unwrap();
    assert_eq!(entry.place, "Mesh");
    assert!(entry.not_now.is_none());
    mesh_group(&mut harness, super::mesh_menu::MESH_MODIFIERS_MENU);
    harness
        .get_all_by_label("Array")
        .find(|n| !n.accesskit_node().is_disabled())
        .expect("Array can be chosen")
        .click();
    harness.run();
    let doc = harness.state().workbench.document();
    assert_eq!(doc.scene().modifiers(cube_id).len(), 1);
    assert_eq!(doc.undo_label(), Some("Add Array"));
}
