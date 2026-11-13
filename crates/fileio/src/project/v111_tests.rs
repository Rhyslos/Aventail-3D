//! Tests for version 111: version 110 with Seam blend layers (decision
//! 0293).

use super::v110_tests::rich_document_v110;
use super::*;
use paint::{Layer, SeamBlend, SeamId};
use scene::Command;

/// Everything version 110 stores, with a Seam blend layer on top of the
/// Default material's layers blending two seams of the first object, 12
/// texels wide, made once. The version 111 sample holds exactly this.
pub(crate) fn rich_document_v111() -> Document {
    let mut doc = rich_document_v110();
    let object = doc.scene().walk()[0].0.get();
    let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
    let mut image = paint::Image::new(set.size).unwrap();
    image.set_pixel(3, 4, [120, 60, 200, 128]);
    let mut layer = Layer::paint("Seam blend", image);
    layer.seam_blend = Some(SeamBlend {
        seams: vec![
            SeamId {
                object,
                points: [0, 1],
            },
            SeamId {
                object,
                points: [2, 6],
            },
        ],
        width: 12.0,
        made: 0x5ea3,
    });
    set.layers.push(layer);
    doc.execute(Command::SetTextureSet {
        material: "Default".into(),
        set: Some(set),
    })
    .unwrap();
    doc
}

fn top_blend(doc: &Document) -> SeamBlend {
    doc.scene()
        .paint()
        .set("Default")
        .unwrap()
        .layers
        .last()
        .unwrap()
        .seam_blend
        .clone()
        .unwrap()
}

#[test]
fn a_seam_blend_layer_survives_a_round_trip() {
    let doc = rich_document_v111();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_eq!(doc.scene().paint(), back.scene().paint());
    let blend = top_blend(&back);
    assert_eq!(blend.width, 12.0);
    assert_eq!(blend.made, 0x5ea3);
    assert_eq!(blend.seams.len(), 2);
}

#[test]
fn layers_without_a_seam_blend_store_nothing_new() {
    let doc = rich_document_v110();
    for (_, set) in doc.scene().paint().sets() {
        for layer in &set.layers {
            let text = serde_json::to_string(&v64::layer_of(layer)).unwrap();
            assert!(!text.contains("seam_blend"), "{text}");
        }
    }
}

#[test]
fn stored_seams_out_of_order_are_tidied_and_bad_ones_refused() {
    let doc = rich_document_v111();
    let mut stored = v111::seam_blend_of(&top_blend(&doc));
    let first = stored.seams[0];
    stored.seams.0.reverse();
    stored.seams.push(first);
    stored.seams.push(v111::SeamV111 {
        object: 4,
        points: [9, 3],
    });
    let read = v111::seam_blend_from(stored.clone()).unwrap();
    assert!(read.is_valid());
    assert_eq!(read.seams.len(), 3, "the repeat dropped");
    assert!(read.seams.contains(&SeamId {
        object: 4,
        points: [3, 9]
    }));
    for width in [0.0, 65.0, f32::NAN, f32::INFINITY] {
        let mut bad = stored.clone();
        bad.width = width;
        assert!(v111::seam_blend_from(bad).is_err(), "{width}");
    }
    let mut bad = stored.clone();
    bad.seams.push(v111::SeamV111 {
        object: 1,
        points: [5, 5],
    });
    assert!(v111::seam_blend_from(bad).is_err());
    let mut many = stored;
    many.seams = (0..=paint::seam_blend::MAX_SEAMS as u32)
        .map(|i| v111::SeamV111 {
            object: 1,
            points: [i, i + 1],
        })
        .collect();
    assert!(v111::seam_blend_from(many).is_err());
}

#[test]
fn flipped_bytes_in_a_file_with_a_seam_blend_never_crash_the_reader() {
    let full = to_bytes(&rich_document_v111()).unwrap();
    let mut state = 111u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        state >> 20
    };
    for _ in 0..200 {
        let mut bytes = full.clone();
        for _ in 0..(next() % 3 + 1) {
            let at = (next() as usize) % bytes.len();
            bytes[at] ^= 1 << (next() % 8);
        }
        if let Ok(doc) = from_bytes(&bytes) {
            for (_, set) in doc.scene().paint().sets() {
                assert!(set.is_valid());
            }
        }
    }
}

#[test]
fn a_style_preset_keeps_the_width_but_not_this_projects_seams() {
    let doc = rich_document_v111();
    let layer = doc
        .scene()
        .paint()
        .set("Default")
        .unwrap()
        .layers
        .last()
        .unwrap()
        .clone();
    let text = crate::preset::write_preset(&crate::preset::Preset::Style {
        name: "Blend".into(),
        layer: Box::new(layer),
    });
    let crate::preset::Preset::Style { layer, .. } =
        crate::preset::read_preset(text.as_bytes(), 64).unwrap()
    else {
        panic!("a style preset");
    };
    let blend = layer.seam_blend.unwrap();
    assert!(blend.seams.is_empty());
    assert_eq!(blend.width, 12.0);
}

const SAMPLE_V111: &[u8] = include_bytes!("../../tests/data/sample-v111.avtl");

#[test]
fn a_file_saved_by_format_version_111_opens_with_its_seam_blend() {
    let doc = from_bytes(SAMPLE_V111).expect("the version 111 sample opens");
    let rich = rich_document_v111();
    assert_eq!(doc.scene().paint(), rich.scene().paint());
}

#[test]
fn a_version_110_file_opens_without_seam_blends() {
    const SAMPLE_V110: &[u8] = include_bytes!("../../tests/data/sample-v110.avtl");
    let doc = from_bytes(SAMPLE_V110).unwrap();
    for (_, set) in doc.scene().paint().sets() {
        assert!(set.layers.iter().all(|l| l.seam_blend.is_none()));
    }
}
