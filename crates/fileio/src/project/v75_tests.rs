//! Tests for version 75: version 74 with a gradient across UV pieces and
//! light and shade at half warmth (decision 0199).

use super::v74_tests::rich_document_v74;
use super::*;
use paint::GradientSpan;

/// Everything version 74 stores, with the gradient across UV pieces and
/// the light and shade at half warmth. The version 75 sample holds
/// exactly this.
pub(super) fn rich_document_v75() -> Document {
    let mut doc = rich_document_v74();
    let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
    for layer in &mut set.layers {
        if let Some(g) = layer.gradient.as_mut() {
            g.span = GradientSpan::UvPiece;
        }
        if let Some(l) = layer.lighting.as_mut() {
            l.warmth = 0.5;
        }
    }
    doc.execute(scene::Command::SetTextureSet {
        material: "Default".into(),
        set: Some(set),
    })
    .unwrap();
    doc
}

#[test]
fn a_gradient_span_and_warmth_survive_a_round_trip() {
    let doc = rich_document_v75();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(doc.scene().paint(), back.scene().paint());
}

const SAMPLE_V75: &[u8] = include_bytes!("../../tests/data/sample-v75.avtl");

#[test]
fn a_file_saved_by_format_version_75_opens_with_its_span_and_warmth() {
    let doc = from_bytes(SAMPLE_V75).expect("the version 75 sample opens");
    assert_eq!(rich_document_v75().scene().paint(), doc.scene().paint());
}

#[test]
fn older_files_read_as_each_object_and_full_warmth() {
    let doc = from_bytes(include_bytes!("../../tests/data/sample-v74.avtl")).unwrap();
    let set = doc.scene().paint().set("Default").unwrap();
    let gradient = set.layers.iter().find_map(|l| l.gradient).unwrap();
    let lighting = set.layers.iter().find_map(|l| l.lighting).unwrap();
    assert_eq!(gradient.span, GradientSpan::Object);
    assert_eq!(lighting.warmth, 1.0);
}

#[test]
fn an_unknown_span_or_a_bad_warmth_is_refused() {
    for (span, warmth) in [(3u8, 1.0f32), (0, 1.5), (0, f32::NAN)] {
        let mut model = v43::ProjectV43::from_document(&rich_document_v75());
        if let Some(p) = model.paint.as_mut() {
            for layer in p.sets[0].layers.iter_mut() {
                if let Some(g) = layer.gradient.as_mut() {
                    g.span = span;
                }
                if let Some(l) = layer.lighting.as_mut() {
                    l.warmth = warmth;
                }
            }
        }
        assert!(model.into_document().is_err(), "{span} {warmth}");
    }
}
