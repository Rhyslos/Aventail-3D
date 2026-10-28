//! Tests for version 81: version 80 with the height channel (decision
//! 0211).

use super::v80_tests::rich_document_v80;
use super::*;
use paint::ChannelPart;

/// Everything version 80 stores, with a raised spot painted on the
/// height of the "Lamp" layer and a fill's height. The version 81 sample
/// holds exactly this.
pub(super) fn rich_document_v81() -> Document {
    let mut doc = rich_document_v80();
    let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
    let mut bump = paint::Image::new(set.size).unwrap();
    bump.set_pixel(6, 6, [230, 230, 230, 255]);
    for layer in &mut set.layers {
        match layer.name.as_str() {
            "Lamp" => layer.channels.height = Some(ChannelPart::Paint(bump.clone())),
            "Steel" => layer.channels.height = Some(ChannelPart::Fill([100, 100, 100, 255])),
            _ => {}
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
fn height_survives_a_round_trip() {
    let doc = rich_document_v81();
    let set = doc.scene().paint().set("Default").unwrap();
    assert!(set.layers.iter().any(|l| l.channels.height.is_some()));
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(doc.scene().paint(), back.scene().paint());
}

const SAMPLE_V81: &[u8] = include_bytes!("../../tests/data/sample-v81.avtl");

#[test]
fn a_file_saved_by_format_version_81_opens_with_its_height() {
    let doc = from_bytes(SAMPLE_V81).expect("the version 81 sample opens");
    assert_eq!(rich_document_v81().scene().paint(), doc.scene().paint());
}
