//! Tests for version 87: version 86 with UVs set corner by corner.

use super::v86_tests::rich_document_v86;
use super::*;
use scene::{Command, Operation};

/// Everything version 86 stores, with the first shape's first polygon's
/// UVs set by hand. The version 87 sample holds exactly this.
pub(super) fn rich_document_v87() -> Document {
    let mut doc = rich_document_v86();
    let id = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| {
            doc.scene()
                .evaluated(*id)
                .is_some_and(|s| s.mesh.faces().next().is_some())
        })
        .unwrap();
    let mesh = doc.scene().evaluated(id).unwrap().mesh;
    let (f, list) = mesh.faces().next().unwrap();
    let corners: Vec<(scene::FaceId, scene::VertexId, glam::Vec2)> = list
        .iter()
        .enumerate()
        .map(|(k, v)| (f, *v, glam::Vec2::new(0.1 * k as f32, 0.25)))
        .collect();
    doc.execute(Command::Record {
        id,
        operation: Operation::SetUvs { corners },
    })
    .unwrap();
    doc
}

/// Every set of corners set, in the order recorded.
fn set_uvs(doc: &Document) -> Vec<Vec<(scene::FaceId, scene::VertexId, glam::Vec2)>> {
    let mut out = Vec::new();
    for (id, _) in doc.scene().walk() {
        let Some(object) = doc.scene().object(id) else {
            continue;
        };
        for layer in object.stack.layers() {
            for entry in &layer.entries {
                if let Operation::SetUvs { corners } = &entry.operation {
                    out.push(corners.clone());
                }
            }
        }
    }
    out
}

#[test]
fn uvs_set_by_hand_survive_a_round_trip() {
    let doc = rich_document_v87();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(set_uvs(&back), set_uvs(&doc));
    assert_eq!(set_uvs(&back).len(), 1);
}

#[test]
fn a_uv_that_isnt_a_number_is_refused() {
    let edit = super::v87::UvsEditV87 {
        corners: vec![(0, 0, [f32::NAN, 0.0])].into_iter().collect(),
    };
    assert!(edit.into_operation().is_err());
}

const SAMPLE_V87: &[u8] = include_bytes!("../../tests/data/sample-v87.avtl");

#[test]
fn a_file_saved_by_format_version_87_opens_with_its_uvs() {
    let doc = from_bytes(SAMPLE_V87).expect("the version 87 sample opens");
    assert_eq!(set_uvs(&doc), set_uvs(&rich_document_v87()));
}

#[test]
fn a_version_86_file_holds_no_uvs_set_by_hand() {
    const SAMPLE_V86: &[u8] = include_bytes!("../../tests/data/sample-v86.avtl");
    let doc = from_bytes(SAMPLE_V86).unwrap();
    assert!(set_uvs(&doc).is_empty());
}
