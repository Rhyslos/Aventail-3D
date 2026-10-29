//! Tests for preset files (decision 0206).

use super::*;
use paint::{Blend, Image, Mask, SmartKind, SmartMask};

fn painted(size: u32) -> Layer {
    let mut image = Image::new(size).unwrap();
    image.set_pixel(3, 4, [10, 20, 30, 255]);
    Layer::paint("Paint", image)
}

fn styled(size: u32) -> Layer {
    let mut mask = Mask::new(size, false).unwrap();
    mask.smart = Some(SmartMask::of(SmartKind::Cavities));
    let mut dirt = Layer::fill("Dirt", [50, 40, 30, 255]);
    dirt.mask = Some(mask);
    dirt.blend = Blend::Multiply;
    dirt.opacity = 0.5;
    let mut light = Layer::paint("Light", Image::new(size).unwrap());
    light.lighting = Some(paint::Lighting::default());
    Layer::group("Grime", vec![dirt, light, painted(size)])
}

#[test]
fn a_style_keeps_its_settings_but_not_its_pixels() {
    let layer = styled(256);
    let text = write_preset(&Preset::Style {
        name: "Grime".into(),
        layer: Box::new(layer.clone()),
    });
    assert!(text.contains("\"kind\": \"style\""), "readable: {text}");
    let Preset::Style { name, layer: read } = read_preset(text.as_bytes(), 256).unwrap() else {
        panic!("a style");
    };
    assert_eq!(name, "Grime");
    let children = read.children().unwrap();
    assert_eq!(children.len(), 3);
    assert_eq!(children[0].blend, Blend::Multiply);
    assert_eq!(children[0].opacity, 0.5);
    assert_eq!(
        children[0].mask.as_ref().unwrap().smart,
        Some(SmartMask::of(SmartKind::Cavities))
    );
    assert!(children[1].lighting.is_some());
    // The painted pixel isn't kept.
    assert_eq!(children[2].image().unwrap().pixel(3, 4), [0; 4]);
}

#[test]
fn a_preset_is_made_at_the_size_it_is_loaded_into() {
    let text = write_preset(&Preset::Style {
        name: "Grime".into(),
        layer: Box::new(styled(256)),
    });
    let Preset::Style { layer, .. } = read_preset(text.as_bytes(), 1024).unwrap() else {
        panic!("a style");
    };
    let set = paint::TextureSet {
        realism: false,
        size: 1024,
        layers: vec![*layer],
    };
    assert!(set.is_valid());
}

#[test]
fn a_material_keeps_its_colour_and_layers() {
    let preset = Preset::Material {
        name: "Wood".into(),
        colour: [0.5, 0.25, 0.1],
        layers: vec![Layer::fill("Base", [128, 64, 25, 255]), styled(512)],
    };
    let read = read_preset(write_preset(&preset).as_bytes(), 512).unwrap();
    let Preset::Material {
        name,
        colour,
        layers,
    } = read
    else {
        panic!("a material");
    };
    assert_eq!((name.as_str(), colour), ("Wood", [0.5, 0.25, 0.1]));
    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0].name, "Base");
}

#[test]
fn brushes_round_trip() {
    let brush = BrushRecipe {
        name: "Chunky".into(),
        tip: 1,
        hardness: 0.7,
        spacing: 0.1,
        flow: 0.8,
        size_pressure: Some(0.2),
        flow_pressure: None,
        kind: 0,
        alignment: Some(2),
    };
    let preset = Preset::Brushes(vec![brush.clone(), brush]);
    assert_eq!(
        read_preset(write_preset(&preset).as_bytes(), 256).unwrap(),
        preset
    );
}

#[test]
fn newer_damaged_and_wrong_files_are_refused_with_a_reason() {
    let newer = br#"{"format": 99, "kind": "brushes", "brushes": []}"#;
    assert!(read_preset(newer, 256).unwrap_err().contains("newer"));
    for bad in [
        &b""[..],
        b"not json",
        br#"{"format": 1, "kind": "teapot"}"#,
        br#"{"format": 1, "kind": "material", "name": "x", "colour": [2, 0, 0], "layers": []}"#,
        br#"{"kind": "brushes", "brushes": []}"#,
    ] {
        assert!(
            read_preset(bad, 256).is_err(),
            "{}",
            String::from_utf8_lossy(bad)
        );
    }
    // A size that isn't a texture size.
    let fine = write_preset(&Preset::Brushes(Vec::new()));
    assert!(read_preset(fine.as_bytes(), 8).is_err());
    // Too large.
    let huge = vec![b' '; MAX_PRESET_FILE as usize + 1];
    assert!(read_preset(&huge, 256).unwrap_err().contains("large"));
}

#[test]
fn a_layer_out_of_range_is_refused() {
    let mut layer = Layer::fill("Base", [0; 4]);
    layer.opacity = 0.5;
    let text = write_preset(&Preset::Style {
        name: "x".into(),
        layer: Box::new(layer),
    })
    .replace("0.5", "7.0");
    assert!(read_preset(text.as_bytes(), 256).is_err());
}

#[test]
fn fuzz_changed_bytes_read_or_are_refused_without_panicking() {
    let text = write_preset(&Preset::Material {
        name: "Wood".into(),
        colour: [0.5, 0.25, 0.1],
        layers: vec![styled(256)],
    });
    let bytes = text.as_bytes();
    let mut seed = 0x2545_F491_4F6C_DD1Du64;
    for _ in 0..2000 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut changed = bytes.to_vec();
        let at = (seed as usize) % changed.len();
        changed[at] = (seed >> 32) as u8;
        if seed.is_multiple_of(5) {
            changed.truncate(at);
        }
        let _ = read_preset(&changed, 256);
    }
}
