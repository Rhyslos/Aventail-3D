//! Tests for version 80: version 79 with a material layer (decision
//! 0210).

use super::v79_tests::rich_document_v79;
use super::*;
use paint::{Layer, MaterialFill, MaterialKind};

/// Everything version 79 stores, with a wood material layer on top. The
/// version 80 sample holds exactly this.
pub(super) fn rich_document_v80() -> Document {
    let mut doc = rich_document_v79();
    let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
    let mut image = paint::Image::new(set.size).unwrap();
    image.set_pixel(4, 4, [150, 104, 62, 255]);
    let mut wood = Layer::paint("Wood", image);
    wood.material = Some(MaterialFill {
        scale: 25.0,
        seed: 3,
        ..MaterialFill::of(MaterialKind::Wood)
    });
    set.layers.push(wood);
    doc.execute(scene::Command::SetTextureSet {
        material: "Default".into(),
        set: Some(set),
    })
    .unwrap();
    doc
}

#[test]
fn a_material_layer_survives_a_round_trip() {
    for kind in MaterialKind::ALL {
        let mut doc = rich_document_v80();
        let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
        set.layers.last_mut().unwrap().material = Some(MaterialFill::of(kind));
        doc.execute(scene::Command::SetTextureSet {
            material: "Default".into(),
            set: Some(set),
        })
        .unwrap();
        let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
        assert_eq!(doc.scene().paint(), back.scene().paint(), "{kind:?}");
    }
}

#[test]
fn a_material_out_of_range_is_refused() {
    let mut stored = v80::material_of(&MaterialFill::of(MaterialKind::Stone));
    assert!(v80::material_from(stored.clone()).is_ok());
    stored.scale = 0.0;
    assert!(v80::material_from(stored.clone()).is_err());
    stored.scale = 10.0;
    stored.kind = 40;
    assert!(v80::material_from(stored).is_err());
}

const SAMPLE_V80: &[u8] = include_bytes!("../../tests/data/sample-v80.avtl");

#[test]
fn a_file_saved_by_format_version_80_opens_with_its_material_layer() {
    let doc = from_bytes(SAMPLE_V80).expect("the version 80 sample opens");
    assert_eq!(rich_document_v80().scene().paint(), doc.scene().paint());
}
