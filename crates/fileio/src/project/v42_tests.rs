//! Tests for version 42: version 41 with improved bevels (decision 0141).

use super::tests::{assert_same, header};
use super::v41_tests::rich_document_v41;
use super::*;
use scene::{BevelOptions, BevelShape, Command, ObjectId, ObjectKind, Operation, Primitive};

/// Every bevel of edges' settings in `doc`, in order.
fn bevel_options(doc: &Document) -> Vec<BevelOptions> {
    doc.scene()
        .walk()
        .iter()
        .flat_map(|(id, _)| doc.scene().object(*id).unwrap().stack.layers().to_vec())
        .flat_map(|layer| layer.entries.clone())
        .filter_map(|entry| match entry.operation.inner() {
            Operation::BevelEdges { shape, .. } => Some(shape.options),
            _ => None,
        })
        .collect()
}

/// Everything version 41 stores, plus a box with every edge given an
/// improved, rounded bevel, and a mirrored one on another: the version 42
/// sample holds exactly this.
pub(super) fn rich_document_v42() -> Document {
    let mut doc = rich_document_v41();
    for (name, segments, mirrored) in [("Rounded box", 3, false), ("Mirrored box", 2, true)] {
        doc.execute(Command::Create(ObjectKind::Primitive(Primitive::Cube(
            Default::default(),
        ))))
        .unwrap();
        let id = doc.selection()[0];
        doc.execute(Command::Rename {
            id,
            name: name.into(),
        })
        .unwrap();
        let shape = doc.scene().evaluated(id).unwrap().mesh;
        let bevel = Operation::BevelEdges {
            edges: shape.edges().to_vec(),
            shape: BevelShape {
                width: 12.0,
                segments,
                profile: 1.0,
                options: BevelOptions::default(),
            },
            stretch: glam::Vec3::ONE,
            anchors: None,
        };
        let operation = if mirrored {
            Operation::Mirrored {
                op: Box::new(bevel),
                mirror: mesh::symmetry::Mirror {
                    axes: [true, false, false],
                    centre: glam::Vec3::ZERO,
                    clip: false,
                },
            }
        } else {
            bevel
        };
        doc.execute(Command::Record { id, operation }).unwrap();
    }
    doc
}

fn named(doc: &Document, name: &str) -> ObjectId {
    doc.scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == name)
        .unwrap()
}

#[test]
fn improved_bevels_survive_a_round_trip() {
    let doc = rich_document_v42();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], 42u32.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_same(&doc, &back);
    assert_eq!(bevel_options(&doc), bevel_options(&back));
    // The new ones improved, the older ones (from version 23's part of
    // the document) not.
    let options = bevel_options(&back);
    assert_eq!(options.iter().filter(|o| o.improved).count(), 2);
    assert!(options.iter().any(|o| !o.improved));
    let rounded = named(&back, "Rounded box");
    assert_eq!(
        back.scene().evaluated(rounded).map(|e| e.mesh),
        doc.scene().evaluated(rounded).map(|e| e.mesh),
        "the rounded box comes out the same"
    );
}

const SAMPLE_V42: &[u8] = include_bytes!("../../tests/data/sample-v42.avtl");

#[test]
fn a_file_saved_by_format_version_42_opens_with_its_improved_bevels() {
    let doc = from_bytes(SAMPLE_V42).expect("the version 42 sample opens");
    let expected = rich_document_v42();
    assert_same(&expected, &doc);
    assert_eq!(bevel_options(&expected), bevel_options(&doc));
}

const SAMPLE_V41: &[u8] = include_bytes!("../../tests/data/sample-v41.avtl");

#[test]
fn bevels_in_older_files_open_as_they_were_made() {
    // Every bevel saved before version 42, even with the settings new
    // bevels then had (even, stop at overlaps, weld), keeps its old shape.
    let doc = from_bytes(SAMPLE_V41).unwrap();
    let options = bevel_options(&doc);
    assert!(!options.is_empty());
    assert!(options.iter().all(|o| !o.improved), "{options:?}");
    // And saving it again keeps them so.
    let again = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(bevel_options(&again), options);
    assert_same(&doc, &again);
}

#[test]
fn an_older_app_is_told_it_needs_updating() {
    // The header says 42: an app that reads up to 41 refuses it as too new
    // rather than dropping the bevel (checked here as the next version).
    let error = super::tests::refused(&header(FORMAT_VERSION + 1, 0, 0));
    assert!(matches!(error, ProjectError::TooNew { .. }));
}

#[test]
fn random_version_42_bodies_never_panic() {
    let mut seed = 0x6c07_8965_d3a1_2f43_u64;
    let good = v41::encode(&v41::ProjectV41::from_document(&rich_document_v42())).unwrap();
    for round in 0..300 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut body = good.clone();
        let at = (seed as usize) % body.len();
        body[at] ^= (seed >> 32) as u8 | 1;
        if round % 3 == 0 {
            body.truncate(at);
        }
        let mut bytes = header(42, 0, body.len() as u64);
        bytes.extend_from_slice(&body);
        let _ = from_bytes(&bytes);
    }
}
