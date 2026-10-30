//! Paint shared by accident (decision 0214): the warning in the Paint
//! layers panel while painting a shape whose texture others share over
//! the same UVs, and its two fixes.

use super::paint_layers_tests::{painting_the_cube, press};
use crate::paint_layers::shared_note::{OWN_TEXTURE, PACK_TOGETHER};
use egui_kittest::kittest::Queryable;
use scene::{Command, ObjectId, ObjectKind, PrimitiveKind};

type H = egui_kittest::Harness<'static, super::tests::State>;

/// Painting the cube, with a sphere beside it on the same material: its
/// UVs lie over the cube's, as every new shape's do.
fn cube_and_sphere() -> (H, ObjectId, ObjectId) {
    let mut harness = painting_the_cube();
    let cube = harness.state().workbench.document().selection()[0];
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Sphere.standard(),
    )))
    .unwrap();
    let sphere = doc.selection()[0];
    // Both on Wood, given to them on purpose: the standard material
    // never warns, as painting gives a shape its own (decision 0232).
    let mut wood = scene::Material::standard();
    wood.name = "Wood".into();
    for id in [cube, sphere] {
        doc.execute(Command::SetMaterials {
            id,
            materials: vec![wood.clone()],
        })
        .unwrap();
    }
    doc.set_selection([cube]);
    harness.run();
    harness.run();
    (harness, cube, sphere)
}

fn material_of(harness: &H, id: ObjectId) -> String {
    let object = harness
        .state()
        .workbench
        .document()
        .scene()
        .object(id)
        .unwrap();
    object
        .materials
        .first()
        .map_or_else(|| scene::Material::standard().name, |m| m.name.clone())
}

#[test]
fn the_panel_warns_and_names_the_other_shape() {
    let (harness, cube, sphere) = cube_and_sphere();
    let shared = harness
        .state()
        .workbench
        .paint_layers
        .shared
        .clone()
        .expect("found");
    assert_eq!(shared.owner, cube);
    assert_eq!(
        shared.others.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        [sphere]
    );
    assert!(harness.query_by_label(OWN_TEXTURE).is_some());
    assert!(harness.query_by_label(PACK_TOGETHER).is_some());
}

#[test]
fn no_warning_for_a_shape_alone() {
    let harness = painting_the_cube();
    assert!(harness.state().workbench.paint_layers.shared.is_none());
    assert!(harness.query_by_label(OWN_TEXTURE).is_none());
}

#[test]
fn own_texture_gives_a_new_material_with_the_painting_in_one_step() {
    let (mut harness, cube, sphere) = cube_and_sphere();
    // Some paint on the shared texture first.
    let size = harness.state().workbench.document().scene().texture().size;
    let mut set = paint::TextureSet::new(size).unwrap();
    set.layers.push(paint::Layer::paint(
        "Paint",
        paint::Image::new(size).unwrap(),
    ));
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(Command::SetTextureSet {
            material: "Wood".into(),
            set: Some(set),
        })
        .unwrap();
    harness.run();
    press(&mut harness, OWN_TEXTURE);
    let own = material_of(&harness, cube);
    assert_ne!(own, "Wood");
    assert_eq!(material_of(&harness, sphere), "Wood", "the other keeps it");
    let paint = |h: &H| {
        h.state()
            .workbench
            .document()
            .scene()
            .paint()
            .set(&own)
            .is_some()
    };
    assert!(paint(&harness), "the painting is copied");
    harness.run();
    assert!(
        harness.state().workbench.paint_layers.shared.is_none(),
        "fixed"
    );
    // One undo step takes it all back.
    harness.state_mut().workbench.document_mut().undo().unwrap();
    assert_eq!(material_of(&harness, cube), "Wood");
    assert!(!paint(&harness));
}

#[test]
fn packing_lays_them_side_by_side_in_one_step() {
    let (mut harness, cube, sphere) = cube_and_sphere();
    press(&mut harness, PACK_TOGETHER);
    harness.run();
    harness.run();
    assert!(
        harness.state().workbench.paint_layers.shared.is_none(),
        "fixed"
    );
    assert_eq!(material_of(&harness, cube), "Wood");
    assert_eq!(material_of(&harness, sphere), "Wood");
    harness.state_mut().workbench.document_mut().undo().unwrap();
    harness.run();
    harness.run();
    assert!(
        harness.state().workbench.paint_layers.shared.is_some(),
        "back"
    );
}

/// Painting the cube with a sphere beside it, both on the standard
/// material, as new shapes are.
fn cube_and_plain_sphere() -> (H, ObjectId, ObjectId) {
    let mut harness = painting_the_cube();
    let cube = harness.state().workbench.document().selection()[0];
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Sphere.standard(),
    )))
    .unwrap();
    let sphere = doc.selection()[0];
    // Off to the side, out of the brush's way.
    doc.execute(Command::SetTransforms(vec![(
        sphere,
        scene::Transform {
            position: glam::Vec3::new(400.0, 0.0, 0.0),
            ..scene::Transform::default()
        },
    )]))
    .unwrap();
    doc.set_selection([cube]);
    harness.run();
    harness.run();
    (harness, cube, sphere)
}

#[test]
fn a_stroke_gives_the_shape_its_own_texture_first() {
    // Decision 0232: each model its own texture.
    let (mut harness, cube, sphere) = cube_and_plain_sphere();
    super::paint_layers_tests::paint_front(&mut harness);
    let own = material_of(&harness, cube);
    let cube_name = harness
        .state()
        .workbench
        .document()
        .scene()
        .object(cube)
        .unwrap()
        .name
        .clone();
    assert_eq!(own, cube_name, "named after the shape");
    assert_eq!(material_of(&harness, sphere), "Default");
    let doc = harness.state().workbench.document();
    assert!(
        doc.scene().paint().set(&own).is_some(),
        "painted on its own"
    );
    assert!(
        doc.scene().paint().set("Default").is_none(),
        "not on the shared one"
    );
    // Two steps back: the stroke, then the own texture.
    let doc = harness.state_mut().workbench.document_mut();
    doc.undo().unwrap();
    assert!(doc.undo().unwrap().contains("its own texture"));
    assert_eq!(material_of(&harness, cube), "Default");
}

#[test]
fn a_layer_added_in_the_panel_goes_to_the_shapes_own_texture() {
    let (mut harness, cube, sphere) = cube_and_plain_sphere();
    press(&mut harness, "+ Fill");
    let own = material_of(&harness, cube);
    assert_ne!(own, "Default");
    assert_eq!(material_of(&harness, sphere), "Default");
    let doc = harness.state().workbench.document();
    assert_eq!(doc.scene().paint().set(&own).unwrap().layers.len(), 1);
    assert!(doc.scene().paint().set("Default").is_none());
    // The panel shows the new material.
    assert_eq!(
        harness
            .state()
            .workbench
            .paint_layers
            .material(doc)
            .as_deref(),
        Some(own.as_str())
    );
}

#[test]
fn the_sphere_painted_after_keeps_the_standard_material() {
    let (mut harness, cube, sphere) = cube_and_plain_sphere();
    press(&mut harness, "+ Fill");
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([sphere]);
    harness.run();
    harness.run();
    press(&mut harness, "+ Fill");
    assert_eq!(material_of(&harness, sphere), "Default", "alone on it now");
    assert_ne!(material_of(&harness, cube), "Default");
}
