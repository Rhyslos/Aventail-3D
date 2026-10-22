//! Tests for materials in the window: the Attributes panel's Materials
//! section, Assign, and Select → Select by material (decision 0074).

use super::modelling_tests::{H, cube, last_operation, note, press, shape};
use super::selection::SelectRequest;
use super::tests::fresh;
use super::*;
use egui_kittest::kittest::{NodeT, Queryable};
use scene::{Material, Operation};

fn materials(harness: &H) -> Vec<Material> {
    let doc = harness.state().workbench.document();
    doc.scene().object(cube(harness)).unwrap().materials.clone()
}

fn disabled(harness: &H, label: &str) -> bool {
    harness.get_by_label(label).accesskit_node().is_disabled()
}

/// The cube, made a plain mesh (so the panel is short enough to show
/// every row), in the Polygons mode, its top selected, with a second
/// material added from the panel.
fn with_two_materials() -> H {
    let mut harness = fresh();
    let id = cube(&harness);
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(scene::Command::MakeEditable(vec![id])).unwrap();
    doc.set_selection([id]);
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::Num3);
    let mesh = shape(&harness);
    let top = mesh
        .faces()
        .map(|(f, _)| f)
        .max_by(|a, b| mesh.face_normal(*a).z.total_cmp(&mesh.face_normal(*b).z))
        .unwrap();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements(
            [(
                id,
                scene::ElementSelection::of([scene::Element::Polygon(top)]),
            )],
            "Select",
            None,
        );
    harness.run();
    harness.get_by_label("Add material").scroll_to_me();
    harness.run();
    harness.get_by_label("Add material").click();
    harness.run();
    harness
}

#[test]
fn the_panel_lists_the_standard_material_until_one_is_added() {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    assert!(harness.query_by_label("Default colour").is_some());
    assert!(materials(&harness).is_empty());
    // In the Model mode there are no polygons to assign to.
    assert!(disabled(&harness, "Assign Default"));
    harness.get_by_label("Add material").scroll_to_me();
    harness.run();
    harness.get_by_label("Add material").click();
    harness.run();
    let list = materials(&harness);
    assert_eq!(list.len(), 2);
    assert_eq!(list[0], Material::standard());
    assert_eq!(list[1].name, "Material 2");
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Change materials")
    );
    assert!(harness.query_by_label("Material 2 colour").is_some());
}

#[test]
fn assign_gives_the_selected_polygons_the_material() {
    let mut harness = with_two_materials();
    assert!(!disabled(&harness, "Assign Material 2"));
    harness.get_by_label("Assign Material 2").click();
    harness.run();
    assert!(matches!(
        last_operation(&harness),
        Some(Operation::SetMaterial { ref faces, slot: 1, .. }) if faces.len() == 1
    ));
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Give 1 polygon Material 2")
    );
    assert!(note(&harness).contains("Ctrl + Z"));
    assert_eq!(
        mesh::material::faces_with_slot(&shape(&harness), 1).len(),
        1
    );
    // The panel counts them.
    assert!(harness.query_by_label("1 polygon").is_some());
    assert!(harness.query_by_label("5 polygons").is_some());
}

#[test]
fn a_used_material_isnt_removed() {
    let mut harness = with_two_materials();
    harness.get_by_label("Assign Material 2").click();
    harness.run();
    let steps = harness.state().workbench.document().undo_steps();
    harness.get_by_label("Remove last").click();
    harness.run();
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
    assert_eq!(materials(&harness).len(), 2);
    let note = harness.state().workbench.attributes.material_note();
    assert!(
        note.is_some_and(|n| n.contains("is used by 1 polygon")),
        "{note:?}"
    );
    // Once no polygon uses it, it goes.
    press(&mut harness, Modifiers::COMMAND, Key::Z);
    harness.get_by_label("Remove last").click();
    harness.run();
    assert_eq!(materials(&harness).len(), 1);
}

#[test]
fn select_by_material_selects_the_polygons_using_it() {
    let mut harness = with_two_materials();
    harness.get_by_label("Assign Material 2").click();
    harness.run();
    let id = cube(&harness);
    for (name, expected) in [("Default", 5), ("Material 2", 1)] {
        harness
            .state_mut()
            .workbench
            .select_requests
            .push(SelectRequest::Material(name.into()));
        harness.run();
        let selected = harness
            .state()
            .workbench
            .document()
            .elements(id)
            .unwrap()
            .polygons()
            .count();
        assert_eq!(selected, expected, "{name}");
    }
    assert_eq!(note(&harness), "1 polygon using Material 2 selected.");
}

#[test]
fn materials_are_named_and_coloured_in_the_panel_as_one_step_each() {
    let mut harness = with_two_materials();
    let id = cube(&harness);
    // Typing a name gives the material that name.
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::SetMaterials {
            id,
            materials: vec![
                Material::standard(),
                Material {
                    name: "Trim".into(),
                    color: [0.1, 0.2, 0.3],
                },
            ],
        })
        .unwrap();
    harness.run();
    assert!(harness.query_by_label("Assign Trim").is_some());
    assert!(harness.query_by_label("Trim colour").is_some());
}
