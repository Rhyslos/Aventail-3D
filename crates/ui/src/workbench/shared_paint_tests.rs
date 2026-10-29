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
    doc.set_selection([cube]);
    harness.run();
    harness.run();
    (harness, cube, sphere)
}

fn material_of(harness: &H, id: ObjectId) -> String {
    harness.state().workbench.document().scene().object(id).unwrap().materials[0]
        .name
        .clone()
}

#[test]
fn the_panel_warns_and_names_the_other_shape() {
    let (harness, cube, sphere) = cube_and_sphere();
    let shared = harness.state().workbench.paint_layers.shared.clone().expect("found");
    assert_eq!(shared.owner, cube);
    assert_eq!(shared.others.iter().map(|(id, _)| *id).collect::<Vec<_>>(), [sphere]);
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
    set.layers
        .push(paint::Layer::paint("Paint", paint::Image::new(size).unwrap()));
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(Command::SetTextureSet {
            material: "Default".into(),
            set: Some(set),
        })
        .unwrap();
    harness.run();
    press(&mut harness, OWN_TEXTURE);
    let own = material_of(&harness, cube);
    assert_ne!(own, "Default");
    assert_eq!(material_of(&harness, sphere), "Default", "the other keeps it");
    let paint = |h: &H| h.state().workbench.document().scene().paint().set(&own).is_some();
    assert!(paint(&harness), "the painting is copied");
    harness.run();
    assert!(harness.state().workbench.paint_layers.shared.is_none(), "fixed");
    // One undo step takes it all back.
    harness.state_mut().workbench.document_mut().undo().unwrap();
    assert_eq!(material_of(&harness, cube), "Default");
    assert!(!paint(&harness));
}

#[test]
fn packing_lays_them_side_by_side_in_one_step() {
    let (mut harness, cube, sphere) = cube_and_sphere();
    press(&mut harness, PACK_TOGETHER);
    harness.run();
    harness.run();
    assert!(harness.state().workbench.paint_layers.shared.is_none(), "fixed");
    assert_eq!(material_of(&harness, cube), "Default");
    assert_eq!(material_of(&harness, sphere), "Default");
    harness.state_mut().workbench.document_mut().undo().unwrap();
    harness.run();
    harness.run();
    assert!(harness.state().workbench.paint_layers.shared.is_some(), "back");
}
