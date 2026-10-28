//! Tests for version 67: version 66 with layer masks and polygon fills
//! kept with strokes (decision 0182).

use super::tests::assert_same;
use super::v66_tests::rich_document_v66;
use super::*;
use paint::{FillBy, FillRecord, Mask};
use std::sync::Arc;

/// Everything version 66 stores, with an inverted black mask (a pixel
/// painted on it) on the first layer, and a stroke with a polygon fill.
/// The version 67 sample holds exactly this.
pub(super) fn rich_document_v67() -> Document {
    let mut doc = rich_document_v66();
    let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
    let mut mask = Mask::new(set.size, false).unwrap();
    mask.image.set_pixel(3, 4, [255, 255, 255, 255]);
    mask.inverted = true;
    let mut stroke = (*set.layers[0].strokes[0]).clone();
    stroke.fills.push(FillRecord {
        face: 2,
        by: FillBy::Piece,
    });
    mask.strokes.push(Arc::new(stroke));
    set.layers[0].mask = Some(mask);
    doc.execute(scene::Command::SetTextureSet {
        material: "Default".into(),
        set: Some(set),
    })
    .unwrap();
    doc
}

#[test]
fn masks_and_fills_survive_a_round_trip() {
    let doc = rich_document_v67();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_same(&doc, &back);
    assert_eq!(
        doc.scene().paint().set("Default"),
        back.scene().paint().set("Default")
    );
}

const SAMPLE_V67: &[u8] = include_bytes!("../../tests/data/sample-v67.avtl");

#[test]
fn a_file_saved_by_format_version_67_opens_with_its_masks() {
    let doc = from_bytes(SAMPLE_V67).expect("the version 67 sample opens");
    assert_eq!(
        rich_document_v67().scene().paint().set("Default"),
        doc.scene().paint().set("Default")
    );
}

#[test]
fn a_version_66_file_opens_without_masks() {
    const SAMPLE_V66: &[u8] = include_bytes!("../../tests/data/sample-v66.avtl");
    let doc = from_bytes(SAMPLE_V66).unwrap();
    let set = doc.scene().paint().set("Default").unwrap();
    assert!(set.layers.iter().all(|l| l.mask.is_none()));
}

#[test]
fn damaged_masks_and_fills_are_refused() {
    let good = v43::ProjectV43::from_document(&rich_document_v67());
    let with = |change: &dyn Fn(&mut super::v67::MaskV67)| {
        let mut model = good.clone();
        if let Some(p) = model.paint.as_mut() {
            let mut layers: Vec<_> = p.sets[0].layers.iter().cloned().collect();
            if let Some(mask) = layers[0].mask.as_mut() {
                change(mask);
            }
            p.sets[0].layers = layers.into_iter().collect();
        }
        model.into_document()
    };
    // A tile twice.
    assert!(
        with(&|m| {
            let tile = m.tiles[0].clone();
            m.tiles = [tile.clone(), tile].into_iter().collect();
        })
        .is_err()
    );
    // An unknown kind of fill, from a newer version.
    assert!(
        with(&|m| {
            let mut strokes: Vec<_> = m.strokes.iter().cloned().collect();
            strokes[0].fills = std::iter::once((2, 9)).collect();
            m.strokes = strokes.into_iter().collect();
        })
        .is_err()
    );
    assert!(with(&|_| ()).is_ok());
}
