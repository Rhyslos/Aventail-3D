//! Tests for version 85: version 84 with moves made with texture lock on.

use super::v84_tests::rich_document_v84;
use super::*;
use scene::{Command, Operation};

/// Everything version 84 stores, with a move and a turn of the first
/// object's points made with texture lock on. The version 85 sample holds
/// exactly this.
pub(super) fn rich_document_v85() -> Document {
    let mut doc = rich_document_v84();
    let id = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| {
            doc.scene()
                .evaluated(*id)
                .is_some_and(|s| s.mesh.vertices().next().is_some())
        })
        .unwrap();
    let point = doc
        .scene()
        .evaluated(id)
        .unwrap()
        .mesh
        .vertices()
        .next()
        .unwrap()
        .0;
    for operation in [
        Operation::MovePoints {
            points: vec![point],
            offset: glam::Vec3::new(1.0, 0.0, 0.0),
            anchors: None,
            keep_texture: true,
        },
        Operation::TransformPoints {
            points: vec![point],
            matrix: glam::Mat4::from_scale(glam::Vec3::splat(1.1)),
            anchors: None,
            keep_texture: true,
        },
    ] {
        doc.execute(Command::Record { id, operation }).unwrap();
    }
    doc
}

/// Every move's texture lock, in the order recorded.
fn locks(doc: &Document) -> Vec<bool> {
    let mut out = Vec::new();
    for (id, _) in doc.scene().walk() {
        let Some(object) = doc.scene().object(id) else {
            continue;
        };
        for layer in object.stack.layers() {
            for entry in &layer.entries {
                if let Operation::MovePoints { keep_texture, .. }
                | Operation::TransformPoints { keep_texture, .. } = &entry.operation
                {
                    out.push(*keep_texture);
                }
            }
        }
    }
    out
}

#[test]
fn texture_lock_survives_a_round_trip() {
    let doc = rich_document_v85();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(locks(&back), locks(&doc));
    assert_eq!(locks(&back).iter().filter(|k| **k).count(), 2);
}

const SAMPLE_V85: &[u8] = include_bytes!("../../tests/data/sample-v85.avtl");

#[test]
fn a_file_saved_by_format_version_85_opens_with_its_texture_lock() {
    let doc = from_bytes(SAMPLE_V85).expect("the version 85 sample opens");
    assert_eq!(locks(&doc), locks(&rich_document_v85()));
}

#[test]
fn moves_in_a_version_84_file_stretch_the_texture() {
    const SAMPLE_V84: &[u8] = include_bytes!("../../tests/data/sample-v84.avtl");
    let doc = from_bytes(SAMPLE_V84).unwrap();
    assert!(locks(&doc).iter().all(|k| !k));
}
