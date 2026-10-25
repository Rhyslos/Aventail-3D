//! Tests for instances of objects whose modifiers read other objects
//! (decision 0142: an instance shows its original's final shape). The
//! sources are placed in the original's space, so the instance shows,
//! exports and bakes the same shape wherever it stands.

use crate::*;
use glam::{Mat4, Vec3};

fn cube(doc: &mut Document) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    doc.selection()[0]
}

/// A cube in a 2×2×2 cage with the top of its +X side raised 40 cm, and an instance of
/// it moved 500 cm along X: the cube and the instance.
fn bent_cube_and_moved_instance() -> (Document, ObjectId, ObjectId) {
    let mut doc = Document::new();
    let cube = cube(&mut doc);
    doc.execute(Command::AddLattice {
        ids: vec![cube],
        divisions: [2, 2, 2],
    })
    .unwrap();
    let cage = doc.selection()[0];
    let lattice = doc
        .scene()
        .object(cage)
        .unwrap()
        .kind
        .lattice()
        .unwrap()
        .clone();
    let top: Vec<u32> = (0..lattice.point_count())
        .filter(|i| lattice.rest(*i).z > 0.0 && lattice.rest(*i).x > 0.0)
        .map(|i| i as u32)
        .collect();
    doc.execute(Command::EditLattice {
        id: cage,
        edit: LatticeEdit::Transform {
            points: top,
            matrix: Mat4::from_translation(Vec3::Z * 40.0),
        },
    })
    .unwrap();
    doc.execute(Command::Duplicate {
        ids: vec![cube],
        instances: true,
    })
    .unwrap();
    let instance = doc.selection()[0];
    doc.execute(Command::SetTransforms(vec![(
        instance,
        Transform {
            position: Vec3::new(500.0, 0.0, 0.0),
            ..Transform::default()
        },
    )]))
    .unwrap();
    (doc, cube, instance)
}

fn height(mesh: &mesh::Mesh) -> f32 {
    mesh.bounds().unwrap().size().z
}

#[test]
fn a_moved_instance_of_a_lattice_bent_shape_shows_the_same_bend() {
    let (doc, cube, instance) = bent_cube_and_moved_instance();
    let scene = doc.scene();
    let original = scene.shown_shape(cube).unwrap();
    assert!((height(&original) - 140.0).abs() < 1.0, "the cube is bent");
    assert_eq!(*scene.shown_shape(instance).unwrap(), *original);
    assert_eq!(
        *scene.exported_shape(instance).unwrap(),
        *scene.exported_shape(cube).unwrap()
    );
}

#[test]
fn baking_a_moved_instance_of_a_lattice_bent_shape_keeps_the_bend() {
    let (mut doc, cube, instance) = bent_cube_and_moved_instance();
    let bent = doc.scene().exported_shape(cube).unwrap();
    doc.execute(Command::MakeEditable(vec![instance])).unwrap();
    assert_eq!(
        doc.scene().object(instance).unwrap().kind,
        ObjectKind::Mesh(bent)
    );
}

#[test]
fn a_moved_instance_of_an_array_round_a_centre_matches_its_original() {
    let mut doc = Document::new();
    let a = cube(&mut doc);
    let centre = cube(&mut doc);
    doc.execute(Command::SetTransforms(vec![(
        centre,
        Transform {
            position: Vec3::new(0.0, 300.0, 0.0),
            ..Transform::default()
        },
    )]))
    .unwrap();
    doc.execute(Command::AddModifier {
        ids: vec![a],
        kind: ModifierKind::Array(ArraySettings {
            options: mesh::array::ArrayOptions {
                layout: mesh::array::ArrayLayout::Radial,
                ..Default::default()
            },
            centre: Some(centre),
        }),
    })
    .unwrap();
    doc.execute(Command::Duplicate {
        ids: vec![a],
        instances: true,
    })
    .unwrap();
    let instance = doc.selection()[0];
    doc.execute(Command::SetTransforms(vec![(
        instance,
        Transform {
            position: Vec3::new(-700.0, 50.0, 20.0),
            ..Transform::default()
        },
    )]))
    .unwrap();
    let scene = doc.scene();
    assert_eq!(
        *scene.shown_shape(instance).unwrap(),
        *scene.shown_shape(a).unwrap()
    );
}
