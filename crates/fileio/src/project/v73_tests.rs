//! Tests for version 73: version 72 with a smart mask (decision 0195).

use super::v72_tests::rich_document_v72;
use super::*;
use paint::{Mask, SmartKind, SmartMask};

/// Everything version 72 stores, with a smart mask on the top layer. The
/// version 73 sample holds exactly this.
pub(super) fn rich_document_v73() -> Document {
    let mut doc = rich_document_v72();
    let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
    let mut mask = Mask::new(set.size, true).unwrap();
    mask.image.set_pixel(2, 2, [200, 200, 200, 255]);
    mask.smart = Some(SmartMask {
        seed: 7,
        ..SmartMask::of(SmartKind::Cavities)
    });
    set.layers.last_mut().unwrap().mask = Some(mask);
    doc.execute(scene::Command::SetTextureSet {
        material: "Default".into(),
        set: Some(set),
    })
    .unwrap();
    doc
}

#[test]
fn a_smart_mask_survives_a_round_trip() {
    let doc = rich_document_v73();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(doc.scene().paint(), back.scene().paint());
}

const SAMPLE_V73: &[u8] = include_bytes!("../../tests/data/sample-v73.avtl");

#[test]
fn a_file_saved_by_format_version_73_opens_with_its_smart_mask() {
    let doc = from_bytes(SAMPLE_V73).expect("the version 73 sample opens");
    assert_eq!(rich_document_v73().scene().paint(), doc.scene().paint());
}

#[test]
fn an_unknown_kind_or_a_bad_number_is_refused() {
    for (kind, noise) in [(9u8, 0.1f32), (0, 1.5), (0, f32::NAN)] {
        let mut model = v43::ProjectV43::from_document(&rich_document_v73());
        if let Some(p) = model.paint.as_mut()
            && let Some(layer) = p.sets[0].layers.last_mut()
            && let Some(mask) = layer.mask.as_mut()
            && let Some(smart) = mask.smart.as_mut()
        {
            smart.kind = kind;
            smart.noise = noise;
        }
        assert!(model.into_document().is_err(), "{kind} {noise}");
    }
}
