//! Tests for version 64: version 63 with the project's texture sets
//! (decision 0177).

use super::tests::assert_same;
use super::v63_tests::rich_document_v63;
use super::*;
use paint::{Image, Layer, TextureSet};
use scene::Command;

/// A texture set 128 pixels across with two layers: a painted corner
/// and a half-hidden layer with one pixel.
fn a_texture_set() -> TextureSet {
    let mut base = Image::new(128).unwrap();
    for y in 0..10 {
        for x in 0..10 {
            base.set_pixel(x, y, [200, 120, 40, 255]);
        }
    }
    let mut top = Image::new(128).unwrap();
    top.set_pixel(127, 127, [10, 20, 30, 128]);
    let mut set = TextureSet::new(128).unwrap();
    set.layers.push(Layer::paint("Base", base));
    let mut layer = Layer::paint("Dirt", top);
    layer.opacity = 0.5;
    layer.visible = false;
    set.layers.push(layer);
    set
}

/// Everything version 63 stores, plus a texture set for the material
/// "Default". The version 64 sample holds exactly this.
pub(super) fn rich_document_v64() -> Document {
    let mut doc = rich_document_v63();
    doc.execute(Command::SetTextureSet {
        material: "Default".into(),
        set: Some(a_texture_set()),
    })
    .unwrap();
    doc
}

#[test]
fn texture_sets_survive_a_round_trip() {
    let doc = rich_document_v64();
    let bytes = to_bytes(&doc).unwrap();
    let back = from_bytes(&bytes).unwrap();
    assert_same(&doc, &back);
    let set = back.scene().paint().set("Default").unwrap();
    assert_eq!(**set, a_texture_set());
    assert_eq!(set.layers[0].image().unwrap().painted_tiles(), 1);
}

#[test]
fn a_project_without_painting_stores_none() {
    let doc = rich_document_v63();
    assert!(v43::ProjectV43::from_document(&doc).paint.is_none());
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert!(back.scene().paint().is_empty());
}

const SAMPLE_V64: &[u8] = include_bytes!("../../tests/data/sample-v64.avtl");

#[test]
fn a_file_saved_by_format_version_64_opens_with_its_painting() {
    let doc = from_bytes(SAMPLE_V64).expect("the version 64 sample opens");
    assert_same(&rich_document_v64(), &doc);
}

#[test]
fn damaged_painting_is_refused() {
    let good = v43::ProjectV43::from_document(&rich_document_v64());
    let paint = |model: &v43::ProjectV43| model.paint.clone().unwrap();
    let with = |p: super::v64::PaintV64| {
        let mut model = good.clone();
        model.paint = Some(p);
        model.into_document()
    };
    // A tile past the end.
    let mut far = paint(&good);
    far.sets[0].layers[0].tiles[0].index = 9_999;
    assert!(with(far).is_err());
    // The same tile twice.
    let mut twice = paint(&good);
    let tile = twice.sets[0].layers[0].tiles[0].clone();
    twice.sets[0].layers[0].tiles.push(tile);
    assert!(with(twice).is_err());
    // A blend this version doesn't know.
    let mut blend = paint(&good);
    blend.sets[0].layers[0].blend = 200;
    assert!(with(blend).is_err());
    // An opacity that isn't a number, and a size out of range.
    let mut opacity = paint(&good);
    opacity.sets[0].layers[0].opacity = f32::NAN;
    assert!(with(opacity).is_err());
    let mut size = paint(&good);
    size.sets[0].size = 3;
    assert!(with(size).is_err());
    // The same material twice, and a nameless one.
    let mut again = paint(&good);
    let set = again.sets[0].clone();
    again.sets.push(set);
    assert!(with(again).is_err());
    let mut nameless = paint(&good);
    nameless.sets[0].material = String::new();
    assert!(with(nameless).is_err());
    // The good one opens.
    assert!(with(paint(&good)).is_ok());
}

#[test]
fn a_tile_of_the_wrong_size_doesnt_open() {
    let good = v43::ProjectV43::from_document(&rich_document_v64());
    let mut model = good.clone();
    if let Some(p) = model.paint.as_mut() {
        p.sets[0].layers[0].tiles[0].pixels.0.truncate(100);
    }
    let bytes = v43::encode(&model).unwrap();
    assert!(ciborium::from_reader::<v43::ProjectV43, _>(&bytes[..]).is_err());
}
