//! Tests for version 94: version 93 with the Bake window's settings.

use super::v93_tests::rich_document_v93;
use super::*;

/// The Bake window settings text in the version 94 sample.
const SETTINGS: &str = "(maps: [true, false, true, false, false, false, true], godot: true)";

/// Everything version 93 stores, with Bake window settings. The version
/// 94 sample holds exactly this.
pub(super) fn rich_document_v94() -> Document {
    let mut doc = rich_document_v93();
    assert!(doc.set_bake_settings(SETTINGS));
    doc
}

#[test]
fn bake_settings_survive_a_round_trip() {
    let doc = rich_document_v94();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(back.scene().bake_settings(), SETTINGS);
    // None stored reads back as none.
    let plain = rich_document_v93();
    let back = from_bytes(&to_bytes(&plain).unwrap()).unwrap();
    assert_eq!(back.scene().bake_settings(), "");
}

#[test]
fn too_long_bake_settings_are_refused() {
    let long = "x".repeat(scene::MAX_BAKE_SETTINGS + 1);
    assert!(super::v94::with_bake(Document::new(), long).is_err());
    let fits = "x".repeat(scene::MAX_BAKE_SETTINGS);
    let doc = super::v94::with_bake(Document::new(), fits.clone()).unwrap();
    assert_eq!(doc.scene().bake_settings(), fits);
}

const SAMPLE_V94: &[u8] = include_bytes!("../../tests/data/sample-v94.avtl");

#[test]
fn a_file_saved_by_format_version_94_opens_with_its_bake_settings() {
    let doc = from_bytes(SAMPLE_V94).expect("the version 94 sample opens");
    assert_eq!(doc.scene().bake_settings(), SETTINGS);
}

#[test]
fn a_version_93_file_holds_no_bake_settings() {
    const SAMPLE_V93: &[u8] = include_bytes!("../../tests/data/sample-v93.avtl");
    let doc = from_bytes(SAMPLE_V93).unwrap();
    assert_eq!(doc.scene().bake_settings(), "");
}
