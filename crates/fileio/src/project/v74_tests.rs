//! Tests for version 74: version 73 with a light-and-shade layer
//! (decision 0196).

use super::v73_tests::rich_document_v73;
use super::*;
use paint::{Layer, Lighting};

/// Everything version 73 stores, with a light-and-shade layer on top.
/// The version 74 sample holds exactly this.
pub(super) fn rich_document_v74() -> Document {
    let mut doc = rich_document_v73();
    let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
    let mut image = paint::Image::new(set.size).unwrap();
    image.set_pixel(3, 1, [255, 222, 170, 255]);
    let mut layer = Layer::paint("Light and shade", image);
    layer.lighting = Some(Lighting {
        around: 200.0,
        shadows: false,
        ..Lighting::default()
    });
    layer.opacity = 0.6;
    layer.blend = paint::Blend::Overlay;
    set.layers.push(layer);
    doc.execute(scene::Command::SetTextureSet {
        material: "Default".into(),
        set: Some(set),
    })
    .unwrap();
    doc
}

#[test]
fn a_light_and_shade_layer_survives_a_round_trip() {
    let doc = rich_document_v74();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(doc.scene().paint(), back.scene().paint());
}

const SAMPLE_V74: &[u8] = include_bytes!("../../tests/data/sample-v74.avtl");

#[test]
fn a_file_saved_by_format_version_74_opens_with_its_light_and_shade() {
    let doc = from_bytes(SAMPLE_V74).expect("the version 74 sample opens");
    assert_eq!(rich_document_v74().scene().paint(), doc.scene().paint());
}

#[test]
fn a_bad_sun_or_softness_is_refused() {
    for (around, softness) in [
        (361.0f32, 0.5f32),
        (-1.0, 0.5),
        (90.0, 2.0),
        (f32::NAN, 0.5),
    ] {
        let mut model = v43::ProjectV43::from_document(&rich_document_v74());
        if let Some(p) = model.paint.as_mut()
            && let Some(layer) = p.sets[0].layers.last_mut()
            && let Some(lighting) = layer.lighting.as_mut()
        {
            lighting.around = around;
            lighting.softness = softness;
        }
        assert!(model.into_document().is_err(), "{around} {softness}");
    }
}

#[test]
fn a_layer_that_is_both_a_gradient_and_light_and_shade_is_refused() {
    let mut model = v43::ProjectV43::from_document(&rich_document_v74());
    if let Some(p) = model.paint.as_mut()
        && let Some(layer) = p.sets[0].layers.last_mut()
    {
        layer.gradient = Some(super::v72::gradient_of(&paint::Gradient::default()));
    }
    assert!(model.into_document().is_err());
}
