//! Tests for version 93: version 92 with the baker's cage painted.

use super::v92_tests::rich_document_v92;
use super::*;
use scene::{Command, Operation};

/// Everything version 92 stores, with the first shape's first point's
/// cage pushed out. The version 93 sample holds exactly this.
pub(super) fn rich_document_v93() -> Document {
    let mut doc = rich_document_v92();
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
    let v = doc
        .scene()
        .evaluated(id)
        .unwrap()
        .mesh
        .vertices()
        .next()
        .unwrap()
        .0;
    doc.execute(Command::Record {
        id,
        operation: Operation::SetCagePush {
            points: vec![(v, 3.5)],
        },
    })
    .unwrap();
    doc
}

fn cages(doc: &Document) -> Vec<Operation> {
    let mut out = Vec::new();
    for (id, _) in doc.scene().walk() {
        let Some(object) = doc.scene().object(id) else {
            continue;
        };
        for layer in object.stack.layers() {
            for entry in &layer.entries {
                if matches!(entry.operation, Operation::SetCagePush { .. }) {
                    out.push(entry.operation.clone());
                }
            }
        }
    }
    out
}

#[test]
fn cage_paint_survives_a_round_trip() {
    let doc = rich_document_v93();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(cages(&back), cages(&doc));
    assert_eq!(cages(&back).len(), 1);
}

#[test]
fn a_push_that_isnt_a_number_is_refused() {
    let bad = super::v93::CageEditV93 {
        points: vec![(0, f32::INFINITY)].into_iter().collect(),
    };
    assert!(bad.into_operation().is_err());
}

const SAMPLE_V93: &[u8] = include_bytes!("../../tests/data/sample-v93.avtl");

#[test]
fn a_file_saved_by_format_version_93_opens_with_its_cage() {
    let doc = from_bytes(SAMPLE_V93).expect("the version 93 sample opens");
    assert_eq!(cages(&doc), cages(&rich_document_v93()));
}

#[test]
fn a_version_92_file_holds_no_cage() {
    const SAMPLE_V92: &[u8] = include_bytes!("../../tests/data/sample-v92.avtl");
    let doc = from_bytes(SAMPLE_V92).unwrap();
    assert!(cages(&doc).is_empty());
}
