//! Tests for version 78: version 77 with the realism channels (decision
//! 0207).

use super::v77_tests::rich_document_v77;
use super::*;
use paint::{ChannelPart, Layer};

/// Everything version 77 stores, with realism on and a fill and a paint
/// layer changing roughness, metal and glow. The version 78 sample holds
/// exactly this.
pub(super) fn rich_document_v78() -> Document {
    let mut doc = rich_document_v77();
    let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
    set.realism = true;
    let mut fill = Layer::fill("Steel", [140, 140, 150, 255]);
    fill.channels.roughness = Some(ChannelPart::Fill([60, 60, 60, 255]));
    fill.channels.metal = Some(ChannelPart::Fill([255, 255, 255, 255]));
    let mut glow = paint::Image::new(set.size).unwrap();
    glow.set_pixel(2, 2, [255, 160, 0, 255]);
    let mut lamp = Layer::paint("Lamp", paint::Image::new(set.size).unwrap());
    lamp.channels.glow = Some(ChannelPart::Paint(glow));
    set.layers.push(fill);
    set.layers.push(lamp);
    doc.execute(scene::Command::SetTextureSet {
        material: "Default".into(),
        set: Some(set),
    })
    .unwrap();
    doc
}

#[test]
fn realism_channels_survive_a_round_trip() {
    let doc = rich_document_v78();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(doc.scene().paint(), back.scene().paint());
    assert!(back.scene().paint().set("Default").unwrap().realism);
}

const SAMPLE_V78: &[u8] = include_bytes!("../../tests/data/sample-v78.avtl");

#[test]
fn a_file_saved_by_format_version_78_opens_with_its_channels() {
    let doc = from_bytes(SAMPLE_V78).expect("the version 78 sample opens");
    assert_eq!(rich_document_v78().scene().paint(), doc.scene().paint());
}

#[test]
fn a_painted_part_on_a_fill_is_refused() {
    let stored = v78::ChannelsV78 {
        roughness: Some(v78::PartV78 {
            tiles: crate::limits::Bounded::new(),
            fill: None,
        }),
        metal: None,
        glow: None,
        height: None,
    };
    let fill = paint::Content::Fill([0; 4]);
    assert!(v78::channels_from(stored.clone(), &fill, 64).is_err());
    let group = paint::Content::Group(Vec::new());
    assert!(v78::channels_from(stored.clone(), &group, 64).is_err());
    let paint = paint::Content::Paint(paint::Image::new(64).unwrap());
    assert!(v78::channels_from(stored, &paint, 64).is_ok());
}
