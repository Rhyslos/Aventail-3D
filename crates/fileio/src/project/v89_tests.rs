//! Tests for version 89: version 88 with changes to UV sets.

use super::v88_tests::rich_document_v88;
use super::*;
use mesh::uv_sets::SetChange;
use scene::{Command, Operation};

/// Everything version 88 stores, with a second UV set added to the first
/// shape and made active. The version 89 sample holds exactly this.
pub(super) fn rich_document_v89() -> Document {
    let mut doc = rich_document_v88();
    let id = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| {
            doc.scene()
                .evaluated(*id)
                .is_some_and(|s| mesh::uvs::has_uvs(&s.mesh))
        })
        .unwrap();
    for change in [
        SetChange::Add("detail".into()),
        SetChange::MakeActive("detail".into()),
    ] {
        doc.execute(Command::Record {
            id,
            operation: Operation::UvSet { change },
        })
        .unwrap();
    }
    doc
}

/// Every UV set change, in the order recorded.
fn changes(doc: &Document) -> Vec<SetChange> {
    let mut out = Vec::new();
    for (id, _) in doc.scene().walk() {
        let Some(object) = doc.scene().object(id) else {
            continue;
        };
        for layer in object.stack.layers() {
            for entry in &layer.entries {
                if let Operation::UvSet { change } = &entry.operation {
                    out.push(change.clone());
                }
            }
        }
    }
    out
}

#[test]
fn uv_set_changes_survive_a_round_trip() {
    let doc = rich_document_v89();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(changes(&back), changes(&doc));
    assert_eq!(changes(&back).len(), 2);
    for change in [
        SetChange::Copy {
            from: "a".into(),
            to: "b".into(),
        },
        SetChange::Rename {
            from: "a".into(),
            to: "b".into(),
        },
        SetChange::Delete("a".into()),
    ] {
        let edit = super::v89::UvSetEditV89::of(&Operation::UvSet {
            change: change.clone(),
        })
        .unwrap();
        assert_eq!(
            edit.into_operation().unwrap(),
            Operation::UvSet { change }
        );
    }
}

#[test]
fn a_broken_uv_set_change_is_refused() {
    let bad = super::v89::UvSetEditV89 {
        kind: 9,
        name: "a".into(),
        other: String::new(),
    };
    assert!(bad.into_operation().is_err());
    let nameless = super::v89::UvSetEditV89 {
        kind: 0,
        name: String::new(),
        other: String::new(),
    };
    assert!(nameless.into_operation().is_err());
}

const SAMPLE_V89: &[u8] = include_bytes!("../../tests/data/sample-v89.avtl");

#[test]
fn a_file_saved_by_format_version_89_opens_with_its_uv_sets() {
    let doc = from_bytes(SAMPLE_V89).expect("the version 89 sample opens");
    assert_eq!(changes(&doc), changes(&rich_document_v89()));
}

#[test]
fn a_version_88_file_holds_no_uv_set_changes() {
    const SAMPLE_V88: &[u8] = include_bytes!("../../tests/data/sample-v88.avtl");
    let doc = from_bytes(SAMPLE_V88).unwrap();
    assert!(changes(&doc).is_empty());
}
