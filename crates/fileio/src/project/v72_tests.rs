//! Tests for version 72: version 71 with a gradient fill layer
//! (decision 0194).

use super::v71_tests::rich_document_v71;
use super::*;
use paint::{Gradient, GradientAxis, Layer};

/// Everything version 71 stores, with a gradient fill layer on top. The
/// version 72 sample holds exactly this.
pub(super) fn rich_document_v72() -> Document {
    let mut doc = rich_document_v71();
    let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
    let mut image = paint::Image::new(set.size).unwrap();
    image.set_pixel(1, 1, [10, 20, 30, 255]);
    let mut layer = Layer::paint("Gradient", image);
    layer.gradient = Some(Gradient {
        from: [10, 20, 30, 255],
        to: [200, 210, 220, 255],
        axis: GradientAxis::X,
        start: 0.25,
        end: 0.75,
        span: paint::GradientSpan::Object,
    });
    set.layers.push(layer);
    doc.execute(scene::Command::SetTextureSet {
        material: "Default".into(),
        set: Some(set),
    })
    .unwrap();
    doc
}

#[test]
fn a_gradient_layer_survives_a_round_trip() {
    let doc = rich_document_v72();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(doc.scene().paint(), back.scene().paint());
    let set = back.scene().paint().set("Default").unwrap();
    assert_eq!(
        set.layers.last().unwrap().gradient.unwrap().axis,
        GradientAxis::X
    );
}

const SAMPLE_V72: &[u8] = include_bytes!("../../tests/data/sample-v72.avtl");

#[test]
fn a_file_saved_by_format_version_72_opens_with_its_gradient() {
    let doc = from_bytes(SAMPLE_V72).expect("the version 72 sample opens");
    assert_eq!(rich_document_v72().scene().paint(), doc.scene().paint());
}

#[test]
fn an_unknown_axis_or_a_bad_range_is_refused() {
    for (axis, start) in [(3u8, 0.0f32), (0, 2.0), (0, f32::NAN)] {
        let mut model = v43::ProjectV43::from_document(&rich_document_v72());
        if let Some(p) = model.paint.as_mut()
            && let Some(layer) = p.sets[0].layers.last_mut()
            && let Some(g) = layer.gradient.as_mut()
        {
            g.axis = axis;
            g.start = start;
        }
        assert!(model.into_document().is_err(), "{axis} {start}");
    }
}
