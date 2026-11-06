//! Tests for version 88: version 87 with pieces' packing values.

use super::v87_tests::rich_document_v87;
use super::*;
use mesh::uvs::{PackParts, PackValues};
use scene::{Command, Operation};

/// Everything version 87 stores, with the first shape's polygons given a
/// packing group, a kept grain and twice the texture. The version 88
/// sample holds exactly this.
pub(super) fn rich_document_v88() -> Document {
    let mut doc = rich_document_v87();
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
    let faces: Vec<scene::FaceId> = doc
        .scene()
        .evaluated(id)
        .unwrap()
        .mesh
        .faces()
        .map(|(f, _)| f)
        .collect();
    doc.execute(Command::Record {
        id,
        operation: Operation::SetPackValues {
            faces,
            values: PackValues {
                group: 3,
                grain: 1,
                scale: 2.0,
            },
            which: PackParts {
                group: true,
                grain: true,
                scale: true,
            },
            anchors: None,
        },
    })
    .unwrap();
    doc
}

/// Every packing values edit, in the order recorded.
fn packs(doc: &Document) -> Vec<(PackValues, PackParts)> {
    let mut out = Vec::new();
    for (id, _) in doc.scene().walk() {
        let Some(object) = doc.scene().object(id) else {
            continue;
        };
        for layer in object.stack.layers() {
            for entry in &layer.entries {
                if let Operation::SetPackValues { values, which, .. } = &entry.operation {
                    out.push((*values, *which));
                }
            }
        }
    }
    out
}

#[test]
fn packing_values_survive_a_round_trip() {
    let doc = rich_document_v88();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(packs(&back), packs(&doc));
    assert_eq!(packs(&back).len(), 1);
}

#[test]
fn packing_values_out_of_range_are_refused() {
    let edit = super::v88::PackEditV88 {
        faces: vec![0].into_iter().collect(),
        group: -1,
        grain: 0,
        scale: 0.0,
        which: [true, false, false],
        anchors: None,
    };
    assert!(edit.clone().into_operation().is_err());
    let nan = super::v88::PackEditV88 {
        group: 0,
        scale: f32::NAN,
        ..edit
    };
    assert!(nan.into_operation().is_err());
}

const SAMPLE_V88: &[u8] = include_bytes!("../../tests/data/sample-v88.avtl");

#[test]
fn a_file_saved_by_format_version_88_opens_with_its_packing_values() {
    let doc = from_bytes(SAMPLE_V88).expect("the version 88 sample opens");
    assert_eq!(packs(&doc), packs(&rich_document_v88()));
}

#[test]
fn a_version_87_file_holds_no_packing_values() {
    const SAMPLE_V87: &[u8] = include_bytes!("../../tests/data/sample-v87.avtl");
    let doc = from_bytes(SAMPLE_V87).unwrap();
    assert!(packs(&doc).is_empty());
}
