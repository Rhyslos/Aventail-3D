//! Tests for version 90: version 89 with lightmap UVs made.

use super::v89_tests::rich_document_v89;
use super::*;
use scene::{Command, Operation};

/// Everything version 89 stores, with lightmap UVs made on the first
/// shape with UVs. The version 90 sample holds exactly this.
pub(super) fn rich_document_v90() -> Document {
    let mut doc = rich_document_v89();
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
    doc.execute(Command::Record {
        id,
        operation: Operation::MakeLightmap {
            size: 256,
            padding: 4,
            hardness: mesh::Hardness::ByAngle(30.0),
        },
    })
    .unwrap();
    doc
}

fn lightmaps(doc: &Document) -> Vec<Operation> {
    let mut out = Vec::new();
    for (id, _) in doc.scene().walk() {
        let Some(object) = doc.scene().object(id) else {
            continue;
        };
        for layer in object.stack.layers() {
            for entry in &layer.entries {
                if matches!(entry.operation, Operation::MakeLightmap { .. }) {
                    out.push(entry.operation.clone());
                }
            }
        }
    }
    out
}

#[test]
fn lightmaps_survive_a_round_trip() {
    let doc = rich_document_v90();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(lightmaps(&back), lightmaps(&doc));
    assert_eq!(lightmaps(&back).len(), 1);
}

#[test]
fn lightmap_settings_out_of_range_are_refused() {
    let bad = super::v90::LightmapEditV90 {
        size: 0,
        padding: 4,
        angle: None,
    };
    assert!(bad.into_operation().is_err());
    let nan = super::v90::LightmapEditV90 {
        size: 512,
        padding: 4,
        angle: Some(f32::NAN),
    };
    assert!(nan.into_operation().is_err());
}

const SAMPLE_V90: &[u8] = include_bytes!("../../tests/data/sample-v90.avtl");

#[test]
fn a_file_saved_by_format_version_90_opens_with_its_lightmap() {
    let doc = from_bytes(SAMPLE_V90).expect("the version 90 sample opens");
    assert_eq!(lightmaps(&doc), lightmaps(&rich_document_v90()));
}

#[test]
fn a_version_89_file_holds_no_lightmap() {
    const SAMPLE_V89: &[u8] = include_bytes!("../../tests/data/sample-v89.avtl");
    let doc = from_bytes(SAMPLE_V89).unwrap();
    assert!(lightmaps(&doc).is_empty());
}
