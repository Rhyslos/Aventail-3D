//! Tests for version 71: version 70 with strokes of the effect brushes
//! (decision 0186).

use super::v70_tests::rich_document_v70;
use super::*;
use paint::BrushKind;
use std::sync::Arc;

/// Everything version 70 stores, with a blur stroke beside the first.
/// The version 71 sample holds exactly this.
pub(super) fn rich_document_v71() -> Document {
    let mut doc = rich_document_v70();
    let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
    let mut blur = (*set.layers[0].strokes[0]).clone();
    blur.kind = BrushKind::Blur;
    set.layers[0].strokes.push(Arc::new(blur));
    doc.execute(scene::Command::SetTextureSet {
        material: "Default".into(),
        set: Some(set),
    })
    .unwrap();
    doc
}

#[test]
fn every_kind_of_stroke_survives_a_round_trip() {
    for kind in BrushKind::ALL {
        let mut doc = rich_document_v71();
        let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
        let mut stroke = (*set.layers[0].strokes[0]).clone();
        stroke.kind = kind;
        set.layers[0].strokes[0] = Arc::new(stroke);
        doc.execute(scene::Command::SetTextureSet {
            material: "Default".into(),
            set: Some(set),
        })
        .unwrap();
        let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
        assert_eq!(doc.scene().paint(), back.scene().paint(), "{kind:?}");
    }
}

const SAMPLE_V71: &[u8] = include_bytes!("../../tests/data/sample-v71.avtl");

#[test]
fn a_file_saved_by_format_version_71_opens_with_its_effect_strokes() {
    let doc = from_bytes(SAMPLE_V71).expect("the version 71 sample opens");
    assert_eq!(rich_document_v71().scene().paint(), doc.scene().paint());
}

#[test]
fn an_unknown_kind_is_refused() {
    let mut model = v43::ProjectV43::from_document(&rich_document_v71());
    if let Some(p) = model.paint.as_mut() {
        p.sets[0].layers[0].strokes[0].kind = 8;
    }
    assert!(model.into_document().is_err());
}
