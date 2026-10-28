//! Tests for version 77: version 76 with painted vertex colours
//! (decision 0205).

use super::v76_tests::rich_document_v76;
use super::*;

/// Everything version 76 stores, with two corners of the first shape
/// painted. The version 77 sample holds exactly this.
pub(super) fn rich_document_v77() -> Document {
    let mut doc = rich_document_v76();
    let id = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).is_some_and(|o| o.kind.has_shape()))
        .unwrap();
    let mesh = doc.scene().evaluated(id).unwrap().mesh;
    let (face, points) = mesh.faces().next().map(|(f, l)| (f, l.to_vec())).unwrap();
    doc.execute(scene::Command::Record {
        id,
        operation: scene::Operation::PaintColours {
            corners: vec![
                (face, points[0], [255, 0, 0, 255]),
                (face, points[1], [0, 80, 0, 128]),
            ],
        },
    })
    .unwrap();
    doc
}

/// Every shape's corner colours.
fn colours(doc: &Document) -> Vec<Option<Vec<[u8; 4]>>> {
    doc.scene()
        .walk()
        .into_iter()
        .filter_map(|(id, _)| doc.scene().evaluated(id))
        .map(|e| mesh::colours::colours(&e.mesh))
        .collect()
}

#[test]
fn painted_vertex_colours_survive_a_round_trip() {
    let doc = rich_document_v77();
    assert!(colours(&doc).iter().any(Option::is_some));
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(colours(&doc), colours(&back));
}

const SAMPLE_V77: &[u8] = include_bytes!("../../tests/data/sample-v77.avtl");

#[test]
fn a_file_saved_by_format_version_77_opens_with_its_colours() {
    let doc = from_bytes(SAMPLE_V77).expect("the version 77 sample opens");
    assert_eq!(colours(&rich_document_v77()), colours(&doc));
}
