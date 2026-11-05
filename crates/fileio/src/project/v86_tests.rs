//! Tests for version 86: version 85 with projected and world-aligned UVs.

use super::v85_tests::rich_document_v85;
use super::*;
use scene::{Command, Operation, ProjectOptions, Projection};

/// Everything version 85 stores, with a box projection and a
/// world-aligned one on the first shape. The version 86 sample holds
/// exactly this.
pub(super) fn rich_document_v86() -> Document {
    let mut doc = rich_document_v85();
    let id = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| {
            doc.scene()
                .shape(*id)
                .is_some_and(|s| s.mesh.faces().next().is_some())
        })
        .unwrap();
    let faces: Vec<scene::FaceId> = doc
        .scene()
        .shape(id)
        .unwrap()
        .mesh
        .faces()
        .map(|(f, _)| f)
        .collect();
    for options in [
        ProjectOptions {
            kind: Projection::Box,
            placement: glam::Mat4::from_scale(glam::Vec3::splat(0.5)),
            world: None,
        },
        ProjectOptions {
            kind: Projection::Box,
            placement: glam::Mat4::from_translation(glam::Vec3::new(2.0, 0.0, 0.0)),
            world: Some(0.25),
        },
    ] {
        doc.execute(Command::Record {
            id,
            operation: Operation::ProjectUvs {
                faces: faces.clone(),
                options,
                anchors: None,
            },
        })
        .unwrap();
    }
    doc
}

/// Every projection's options, in the order recorded.
fn projections(doc: &Document) -> Vec<ProjectOptions> {
    let mut out = Vec::new();
    for (id, _) in doc.scene().walk() {
        let Some(object) = doc.scene().object(id) else {
            continue;
        };
        for layer in object.stack.layers() {
            for entry in &layer.entries {
                if let Operation::ProjectUvs { options, .. } = &entry.operation {
                    out.push(*options);
                }
            }
        }
    }
    out
}

#[test]
fn projections_survive_a_round_trip() {
    let doc = rich_document_v86();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(projections(&back), projections(&doc));
    assert_eq!(projections(&back).len(), 2);
    for kind in Projection::ALL {
        let edit = super::v86::ProjectEditV86::of(&Operation::ProjectUvs {
            faces: vec![scene::FaceId::from_raw(0)],
            options: ProjectOptions {
                kind,
                ..Default::default()
            },
            anchors: None,
        })
        .unwrap();
        match edit.into_operation().unwrap() {
            Operation::ProjectUvs { options, .. } => assert_eq!(options.kind, kind),
            _ => panic!("not a projection"),
        }
    }
}

#[test]
fn a_damaged_projection_is_refused() {
    let good = super::v86::ProjectEditV86::of(&Operation::ProjectUvs {
        faces: vec![scene::FaceId::from_raw(0)],
        options: ProjectOptions::default(),
        anchors: None,
    })
    .unwrap();
    let unknown = super::v86::ProjectEditV86 {
        kind: 9,
        ..good.clone()
    };
    assert!(unknown.into_operation().is_err());
    let flat = super::v86::ProjectEditV86 {
        placement: [0.0; 16],
        ..good.clone()
    };
    assert!(flat.into_operation().is_err());
    let mut nan = good.clone();
    nan.placement[3] = f32::NAN;
    assert!(nan.into_operation().is_err());
    let backwards = super::v86::ProjectEditV86 {
        world: Some(-1.0),
        ..good
    };
    assert!(backwards.into_operation().is_err());
}

const SAMPLE_V86: &[u8] = include_bytes!("../../tests/data/sample-v86.avtl");

#[test]
fn a_file_saved_by_format_version_86_opens_with_its_projections() {
    let doc = from_bytes(SAMPLE_V86).expect("the version 86 sample opens");
    assert_eq!(projections(&doc), projections(&rich_document_v86()));
}

#[test]
fn a_version_85_file_holds_no_projections() {
    const SAMPLE_V85: &[u8] = include_bytes!("../../tests/data/sample-v85.avtl");
    let doc = from_bytes(SAMPLE_V85).unwrap();
    assert!(projections(&doc).is_empty());
}
