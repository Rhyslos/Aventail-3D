//! Tests for version 92: version 91 with UV intents.

use super::v91_tests::rich_document_v91;
use super::*;
use mesh::uv_intents::UvIntent;
use scene::{Command, Operation};

/// Everything version 91 stores, with the first shape's polygons told to
/// stay on a trim row. The version 92 sample holds exactly this.
pub(super) fn rich_document_v92() -> Document {
    let mut doc = rich_document_v91();
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
        operation: Operation::SetUvIntent {
            faces,
            intent: UvIntent::TrimRow {
                bottom: 0.25,
                top: 0.5,
            },
            anchors: None,
        },
    })
    .unwrap();
    doc
}

fn intents(doc: &Document) -> Vec<Operation> {
    let mut out = Vec::new();
    for (id, _) in doc.scene().walk() {
        let Some(object) = doc.scene().object(id) else {
            continue;
        };
        for layer in object.stack.layers() {
            for entry in &layer.entries {
                if matches!(entry.operation, Operation::SetUvIntent { .. }) {
                    out.push(entry.operation.clone());
                }
            }
        }
    }
    out
}

#[test]
fn intents_survive_a_round_trip() {
    let doc = rich_document_v92();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(intents(&back), intents(&doc));
    assert_eq!(intents(&back).len(), 1);
}

#[test]
fn damaged_intents_are_refused() {
    let unknown = super::v92::IntentEditV92 {
        faces: crate::limits::Bounded::new(),
        kind: 9,
        a: 0.0,
        b: 0.0,
        anchors: None,
    };
    assert!(unknown.into_operation().is_err());
    let bad_row = super::v92::IntentEditV92 {
        faces: crate::limits::Bounded::new(),
        kind: 3,
        a: 0.6,
        b: 0.2,
        anchors: None,
    };
    assert!(bad_row.into_operation().is_err());
    let nan = super::v92::IntentEditV92 {
        faces: crate::limits::Bounded::new(),
        kind: 2,
        a: f32::NAN,
        b: 0.0,
        anchors: None,
    };
    assert!(nan.into_operation().is_err());
}

const SAMPLE_V92: &[u8] = include_bytes!("../../tests/data/sample-v92.avtl");

#[test]
fn a_file_saved_by_format_version_92_opens_with_its_intents() {
    let doc = from_bytes(SAMPLE_V92).expect("the version 92 sample opens");
    assert_eq!(intents(&doc), intents(&rich_document_v92()));
}

#[test]
fn a_version_91_file_holds_no_intents() {
    const SAMPLE_V91: &[u8] = include_bytes!("../../tests/data/sample-v91.avtl");
    let doc = from_bytes(SAMPLE_V91).unwrap();
    assert!(intents(&doc).is_empty());
}
