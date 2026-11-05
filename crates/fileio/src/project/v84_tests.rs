//! Tests for version 84: version 83 with each texture set's glow
//! strength.

use super::v83_tests::rich_document_v83;
use super::*;

/// Everything version 83 stores, with a glow 4 times as bright. The
/// version 84 sample holds exactly this.
pub(super) fn rich_document_v84() -> Document {
    let mut doc = rich_document_v83();
    let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
    set.glow_strength = 4.0;
    doc.execute(scene::Command::SetTextureSet {
        material: "Default".into(),
        set: Some(set),
    })
    .unwrap();
    doc
}

fn strength(doc: &Document) -> f32 {
    doc.scene().paint().set("Default").unwrap().glow_strength
}

#[test]
fn the_glow_strength_survives_a_round_trip() {
    let doc = rich_document_v84();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(strength(&back), 4.0);
}

const SAMPLE_V84: &[u8] = include_bytes!("../../tests/data/sample-v84.avtl");

#[test]
fn a_file_saved_by_format_version_84_opens_with_its_glow_strength() {
    let doc = from_bytes(SAMPLE_V84).expect("the version 84 sample opens");
    assert_eq!(strength(&doc), 4.0);
}

#[test]
fn glows_in_a_version_83_file_are_as_painted() {
    const SAMPLE_V83: &[u8] = include_bytes!("../../tests/data/sample-v83.avtl");
    let doc = from_bytes(SAMPLE_V83).unwrap();
    assert_eq!(strength(&doc), 1.0);
}

#[test]
fn a_glow_strength_out_of_range_is_refused() {
    let mut doc = rich_document_v84();
    let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
    set.glow_strength = 0.5;
    assert!(!set.is_valid());
    set.glow_strength = f32::NAN;
    assert!(!set.is_valid());
    set.glow_strength = paint::MAX_GLOW_STRENGTH;
    assert!(set.is_valid());
    doc.execute(scene::Command::SetTextureSet {
        material: "Default".into(),
        set: Some(set),
    })
    .unwrap();
}
