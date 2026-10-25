//! Tests that an export building only the placing its format writes
//! ([`Frames`]) gives exactly the file it gave building both.

use super::*;
use scene::{Collision, Command, Lods, ObjectKind, PrimitiveKind, Transform};

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

/// A moved, turned sphere with levels of detail and a box collision, a
/// cube mirrored by a negative scale, and an instance of the sphere.
fn scene() -> Document {
    let mut doc = Document::new();
    let sphere = create(&mut doc, PrimitiveKind::Sphere);
    doc.execute(Command::SetLods {
        ids: vec![sphere],
        lods: Lods {
            on: true,
            ..Lods::default()
        },
    })
    .unwrap();
    doc.execute(Command::SetCollision {
        ids: vec![sphere],
        collision: Collision::Box,
    })
    .unwrap();
    doc.execute(Command::SetTransforms(vec![(
        sphere,
        Transform {
            position: Vec3::new(120.0, -40.0, 30.0),
            rotation: Vec3::new(0.0, 0.0, 40.0),
            ..Transform::default()
        },
    )]))
    .unwrap();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    doc.execute(Command::SetTransforms(vec![(
        cube,
        Transform {
            scale: Vec3::new(-1.0, 2.0, 1.0),
            ..Transform::default()
        },
    )]))
    .unwrap();
    doc.execute(Command::Duplicate {
        ids: vec![sphere],
        instances: true,
    })
    .unwrap();
    doc
}

fn options() -> ExportOptions {
    ExportOptions {
        lod_levels: true,
        godot_names: true,
        ..ExportOptions::default()
    }
}

#[test]
fn each_format_writes_the_same_file_from_its_own_placing_alone() {
    let doc = scene();
    for format in ExportFormat::ALL {
        let options = format.options(&options());
        let both = write_files(&prepare(&doc, &options), format, &options, "test", "x.f").unwrap();
        let only = export_bytes(&doc, format, &options, "test", "x.f").unwrap();
        assert_eq!(only.main, both.main, "{format:?}");
        assert_eq!(only.beside, both.beside, "{format:?}");
        assert_eq!(only.summary, both.summary, "{format:?}");
    }
}

#[test]
fn only_the_placing_asked_for_is_built() {
    let doc = scene();
    let options = options();
    for object in prepare_for(&doc, &options, Frames::Local) {
        assert!(object.world.polygons.is_empty());
        assert!(!object.local.polygons.is_empty());
        assert!(std::ptr::eq(object.built(), &object.local));
        for level in &object.levels {
            assert!(level.world.polygons.is_empty() && !level.local.polygons.is_empty());
        }
    }
    for object in prepare_for(&doc, &options, Frames::World) {
        assert!(object.local.polygons.is_empty());
        assert!(std::ptr::eq(object.built(), &object.world));
    }
    for object in prepare(&doc, &options) {
        assert_eq!(object.world.polygons.len(), object.local.polygons.len());
    }
}

#[test]
fn a_kit_piece_writes_the_same_file_from_its_own_placing_alone() {
    let doc = scene();
    let root = doc.scene().walk()[0].0;
    for format in ExportFormat::ALL {
        let options = format.options(&options());
        let both = prepare_piece(&doc, root, &options, Frames::Both);
        let only = prepare_piece(&doc, root, &options, format.frames());
        let a = write_files(&both, format, &options, "test", "x.f").unwrap();
        let b = write_files(&only, format, &options, "test", "x.f").unwrap();
        assert_eq!(a.main, b.main, "{format:?}");
    }
}
