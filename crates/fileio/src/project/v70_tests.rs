//! Tests for version 70: version 69 with each stroke's tip (decision
//! 0185).

use super::v69_tests::rich_document_v69;
use super::*;
use std::sync::Arc;

/// Everything version 69 stores, with the first stroke painted with the
/// bristle tip. The version 70 sample holds exactly this.
pub(super) fn rich_document_v70() -> Document {
    let mut doc = rich_document_v69();
    let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
    let mut stroke = (*set.layers[0].strokes[0]).clone();
    stroke.tip = paint::Tip::Bristle;
    set.layers[0].strokes[0] = Arc::new(stroke);
    doc.execute(scene::Command::SetTextureSet {
        material: "Default".into(),
        set: Some(set),
    })
    .unwrap();
    doc
}

#[test]
fn tips_survive_a_round_trip() {
    let doc = rich_document_v70();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_eq!(doc.scene().paint(), back.scene().paint());
}

const SAMPLE_V70: &[u8] = include_bytes!("../../tests/data/sample-v70.avtl");

#[test]
fn a_file_saved_by_format_version_70_opens_with_its_tips() {
    let doc = from_bytes(SAMPLE_V70).expect("the version 70 sample opens");
    assert_eq!(rich_document_v70().scene().paint(), doc.scene().paint());
}

#[test]
fn a_version_69_file_is_painted_round_and_unknown_tips_are_refused() {
    const SAMPLE_V69: &[u8] = include_bytes!("../../tests/data/sample-v69.avtl");
    let doc = from_bytes(SAMPLE_V69).unwrap();
    let set = doc.scene().paint().set("Default").unwrap();
    assert_eq!(set.layers[0].strokes[0].tip, paint::Tip::Round);
    let mut model = v43::ProjectV43::from_document(&rich_document_v70());
    if let Some(p) = model.paint.as_mut() {
        p.sets[0].layers[0].strokes[0].tip = 99;
    }
    assert!(model.into_document().is_err());
}
