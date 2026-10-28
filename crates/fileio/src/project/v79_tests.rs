//! Tests for version 79: version 78 with the wear, dirt and rust smart
//! masks (decision 0209).

use super::v78_tests::rich_document_v78;
use super::*;
use paint::{Layer, Mask, SmartKind, SmartMask};

/// Everything version 78 stores, with a rust layer through a rust smart
/// mask on top. The version 79 sample holds exactly this.
pub(super) fn rich_document_v79() -> Document {
    let mut doc = rich_document_v78();
    let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
    let mut mask = Mask::new(set.size, false).unwrap();
    mask.smart = Some(SmartMask::of(SmartKind::Rust));
    let mut rust = Layer::fill("Rust", [118, 52, 24, 255]);
    rust.mask = Some(mask);
    set.layers.push(rust);
    doc.execute(scene::Command::SetTextureSet {
        material: "Default".into(),
        set: Some(set),
    })
    .unwrap();
    doc
}

#[test]
fn every_kind_of_smart_mask_survives_a_round_trip() {
    for kind in SmartKind::ALL {
        let mut doc = rich_document_v79();
        let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
        let top = set.layers.last_mut().unwrap();
        top.mask.as_mut().unwrap().smart = Some(SmartMask::of(kind));
        doc.execute(scene::Command::SetTextureSet {
            material: "Default".into(),
            set: Some(set),
        })
        .unwrap();
        let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
        assert_eq!(doc.scene().paint(), back.scene().paint(), "{kind:?}");
    }
}

const SAMPLE_V79: &[u8] = include_bytes!("../../tests/data/sample-v79.avtl");

#[test]
fn a_file_saved_by_format_version_79_opens_with_its_rust_mask() {
    let doc = from_bytes(SAMPLE_V79).expect("the version 79 sample opens");
    assert_eq!(rich_document_v79().scene().paint(), doc.scene().paint());
}
