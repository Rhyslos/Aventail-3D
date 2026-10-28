//! Tests for version 76: version 75 with an object's outline (decision
//! 0201).

use super::v75_tests::rich_document_v75;
use super::*;

/// Everything version 75 stores, with an outline on the first object.
/// The version 76 sample holds exactly this.
pub(super) fn rich_document_v76() -> Document {
    let mut doc = rich_document_v75();
    let id = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).is_some_and(|o| o.kind.has_shape()))
        .unwrap();
    doc.execute(scene::Command::SetOutline {
        ids: vec![id],
        outline: Some(scene::Outline {
            width: 2.5,
            colour: [200, 10, 30],
        }),
    })
    .unwrap();
    doc
}

fn outlines(doc: &Document) -> Vec<Option<scene::Outline>> {
    doc.scene()
        .walk()
        .into_iter()
        .map(|(id, _)| doc.scene().object(id).unwrap().outline)
        .collect()
}

#[test]
fn an_outline_survives_a_round_trip() {
    let doc = rich_document_v76();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(outlines(&doc), outlines(&back));
    assert!(outlines(&back).iter().any(Option::is_some));
}

const SAMPLE_V76: &[u8] = include_bytes!("../../tests/data/sample-v76.avtl");

#[test]
fn a_file_saved_by_format_version_76_opens_with_its_outline() {
    let doc = from_bytes(SAMPLE_V76).expect("the version 76 sample opens");
    assert_eq!(outlines(&rich_document_v76()), outlines(&doc));
}

#[test]
fn a_bad_outline_width_is_refused() {
    for width in [f32::NAN, -1.0, 1000.0] {
        let mut model = v43::ProjectV43::from_document(&rich_document_v76());
        for object in model.project.project.project.objects.iter_mut() {
            if let Some(o) = object.outline.as_mut() {
                o.width = width;
            }
        }
        assert!(model.into_document().is_err(), "{width}");
    }
}
