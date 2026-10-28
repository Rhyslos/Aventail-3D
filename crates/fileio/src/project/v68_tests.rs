//! Tests for version 68: version 67 with the project's colour palettes
//! (decision 0183).

use super::v67_tests::rich_document_v67;
use super::*;
use paint::palette::Palette;

/// Everything version 67 stores, with two palettes. The version 68
/// sample holds exactly this.
pub(super) fn rich_document_v68() -> Document {
    let mut doc = rich_document_v67();
    doc.execute(scene::Command::SetPalettes(vec![
        Palette {
            name: "Crate".into(),
            colours: vec![[227, 170, 102], [126, 76, 39]],
        },
        Palette::new("Empty"),
    ]))
    .unwrap();
    doc
}

#[test]
fn palettes_survive_a_round_trip() {
    let doc = rich_document_v68();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_eq!(doc.scene().paint(), back.scene().paint());
}

#[test]
fn palettes_alone_are_kept() {
    let mut doc = Document::new();
    doc.execute(scene::Command::SetPalettes(vec![Palette::new("Only")]))
        .unwrap();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(back.scene().paint().palettes().len(), 1);
}

const SAMPLE_V68: &[u8] = include_bytes!("../../tests/data/sample-v68.avtl");

#[test]
fn a_file_saved_by_format_version_68_opens_with_its_palettes() {
    let doc = from_bytes(SAMPLE_V68).expect("the version 68 sample opens");
    assert_eq!(rich_document_v68().scene().paint(), doc.scene().paint());
}

#[test]
fn a_version_67_file_opens_without_palettes() {
    const SAMPLE_V67: &[u8] = include_bytes!("../../tests/data/sample-v67.avtl");
    let doc = from_bytes(SAMPLE_V67).unwrap();
    assert!(doc.scene().paint().palettes().is_empty());
}

#[test]
fn damaged_palettes_are_refused() {
    let good = v43::ProjectV43::from_document(&rich_document_v68());
    let mut model = good.clone();
    if let Some(p) = model.paint.as_mut() {
        let mut palettes: Vec<_> = p.palettes.clone().unwrap().into_iter().collect();
        palettes[0].name = "n".repeat(paint::palette::MAX_NAME + 1);
        p.palettes = Some(palettes.into_iter().collect());
    }
    assert!(model.into_document().is_err());
    assert!(good.into_document().is_ok());
}
