//! Painted textures in exports (decision 0204): inside a glTF as PNG,
//! beside an OBJ as PNG or TGA and named in its `.mtl`, padded, left out
//! when asked.

use super::*;
use scene::{Command, ObjectKind, PrimitiveKind};

/// A cube whose material (the standard one) is painted red, with a little
/// blue square at the top left of the texture.
fn painted_cube() -> Document {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let material = scene::Material::standard().name;
    let mut set = paint::TextureSet::new(256).unwrap();
    let mut image = paint::Image::new(256).unwrap();
    for y in 0..256 {
        for x in 0..256 {
            let blue = x < 16 && y < 16;
            image.set_pixel(
                x,
                y,
                if blue {
                    [0, 0, 255, 255]
                } else {
                    [255, 0, 0, 255]
                },
            );
        }
    }
    set.layers.push(paint::Layer::paint("Paint", image));
    doc.execute(Command::SetTextureSet {
        material,
        set: Some(set),
    })
    .unwrap();
    doc
}

fn godot() -> ExportOptions {
    let preset = ExportPreset::Godot.settings();
    ExportOptions {
        units: preset.units,
        up: preset.up,
        triangulate: preset.triangulate,
        godot_names: true,
        ..ExportOptions::default()
    }
}

/// The JSON part of a binary glTF.
fn json_of(glb: &[u8]) -> serde_json::Value {
    let length = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
    serde_json::from_slice(&glb[20..20 + length]).unwrap()
}

#[test]
fn a_gltf_holds_the_painted_texture_as_its_base_colour() {
    let doc = painted_cube();
    let files = export_bytes(&doc, ExportFormat::Glb, &godot(), "App 1.0", "crate.glb").unwrap();
    let json = json_of(&files.main);
    assert_eq!(json["images"][0]["mimeType"], "image/png");
    assert_eq!(json["textures"][0]["source"], 0);
    let material = &json["materials"][0]["pbrMetallicRoughness"];
    assert_eq!(material["baseColorTexture"]["index"], 0);
    assert_eq!(material["baseColorFactor"][0], 1.0);
    // The image's view has no target, as glTF wants for images.
    let view = json["images"][0]["bufferView"].as_u64().unwrap() as usize;
    assert!(json["bufferViews"][view].get("target").is_none());
    assert_eq!(files.textures.len(), 1);
    assert_eq!(files.textures[0].size, 256);
    assert!(files.textures[0].file_bytes > 0);
    assert!(files.beside.is_empty(), "inside the file, not beside it");
}

#[test]
fn an_obj_gets_a_tga_beside_it_named_in_its_materials() {
    let doc = painted_cube();
    let options = ExportOptions {
        textures: TextureFormat::Tga,
        ..ExportOptions::default()
    };
    let files = export_bytes(&doc, ExportFormat::Obj, &options, "App 1.0", "crate.obj").unwrap();
    let (name, tga) = files
        .beside
        .iter()
        .find(|(n, _)| n.ends_with(".tga"))
        .expect("a TGA beside it");
    assert!(name.starts_with("crate_"));
    let mtl = String::from_utf8_lossy(&files.beside[0].1);
    assert!(mtl.contains(&format!("map_Kd {name}")), "{mtl}");
    assert!(mtl.contains("Kd 1 1 1"), "{mtl}");
    let back = crate::image_file::read_image(tga).unwrap();
    assert_eq!((back.width, back.height), (256, 256));
}

#[test]
fn textures_can_be_left_out_and_unpainted_materials_get_none() {
    let doc = painted_cube();
    let options = ExportOptions {
        textures: TextureFormat::None,
        ..godot()
    };
    let files = export_bytes(&doc, ExportFormat::Glb, &options, "App 1.0", "crate.glb").unwrap();
    assert!(json_of(&files.main).get("images").is_none());
    assert!(files.textures.is_empty());
    let mut plain = Document::new();
    plain
        .execute(Command::Create(ObjectKind::Primitive(
            PrimitiveKind::Cube.standard(),
        )))
        .unwrap();
    let files = export_bytes(
        &plain,
        ExportFormat::Obj,
        &ExportOptions::default(),
        "App",
        "a.obj",
    )
    .unwrap();
    assert_eq!(files.beside.len(), 1, "just the .mtl");
}

#[test]
fn padding_spreads_each_piece_past_its_edges() {
    let doc = painted_cube();
    let objects = prepare(&doc, &ExportOptions::default());
    let material = scene::Material::standard().name;
    let covered = textures::coverage(&objects, &material, 256);
    let inside = covered.iter().filter(|c| **c).count();
    assert!(inside > 256 * 256 / 3 && inside < 256 * 256, "{inside}");
    // A texel just outside a piece: uncovered, then filled by padding.
    let mut covered_after = covered.clone();
    let mut rgba = vec![0u8; 256 * 256 * 4];
    for (i, c) in covered.iter().enumerate() {
        if *c {
            rgba[i * 4] = 200;
        }
    }
    textures::pad(&mut rgba, &mut covered_after, 256, 4);
    let grown = covered_after.iter().filter(|c| **c).count();
    assert!(grown > inside);
    let newly = (0..covered.len())
        .find(|i| covered_after[*i] && !covered[*i])
        .unwrap();
    assert_eq!(rgba[newly * 4], 200);
}

#[test]
fn painted_vertex_colours_go_out_as_colour_0() {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let id = doc.selection()[0];
    let mesh = doc.scene().evaluated(id).unwrap().mesh;
    let corners: Vec<_> = mesh
        .faces()
        .next()
        .map(|(f, list)| list.iter().map(|v| (f, *v, [255u8, 0, 0, 255])).collect())
        .unwrap();
    doc.execute(Command::Record {
        id,
        operation: scene::Operation::PaintColours { corners },
    })
    .unwrap();
    let objects = prepare(&doc, &ExportOptions::default());
    assert_eq!(objects[0].built().colours.len(), 2, "red and white");
    let files = export_bytes(&doc, ExportFormat::Glb, &godot(), "App", "a.glb").unwrap();
    let json = json_of(&files.main);
    let attributes = &json["meshes"][0]["primitives"][0]["attributes"];
    let colour = attributes["COLOR_0"].as_u64().expect("COLOR_0") as usize;
    assert_eq!(json["accessors"][colour]["type"], "VEC4");
    // A cube without vertex colours has none.
    let mut plain = Document::new();
    plain
        .execute(Command::Create(ObjectKind::Primitive(
            PrimitiveKind::Cube.standard(),
        )))
        .unwrap();
    let files = export_bytes(&plain, ExportFormat::Glb, &godot(), "App", "a.glb").unwrap();
    let json = json_of(&files.main);
    assert!(
        json["meshes"][0]["primitives"][0]["attributes"]
            .get("COLOR_0")
            .is_none()
    );
}

#[test]
fn the_maps_going_out_follow_what_a_material_uses_and_the_packing() {
    use super::{ChannelPacking, MapsUsed, TextureMap, maps_going_out};
    let names =
        |used, packing, glb| -> Vec<(TextureMap, bool)> { maps_going_out(used, packing, glb) };
    // Colour alone: only the colour, inside a glTF or beside an OBJ.
    let plain = MapsUsed::default();
    assert_eq!(
        names(plain, ChannelPacking::Separate, true),
        vec![(TextureMap::Colour, true)]
    );
    assert_eq!(
        names(plain, ChannelPacking::Standard, false),
        vec![(TextureMap::Colour, false)]
    );
    // A graph's material (decision 0260): roughness, metal and a height.
    let graph = MapsUsed {
        realism: true,
        height: true,
        glow: false,
        ..MapsUsed::default()
    };
    assert_eq!(
        names(graph, ChannelPacking::Standard, true),
        vec![
            (TextureMap::Colour, true),
            (TextureMap::Orm, true),
            (TextureMap::Normal, true)
        ]
    );
    assert_eq!(
        names(graph, ChannelPacking::UnityMask, true),
        vec![
            (TextureMap::Colour, true),
            (TextureMap::Orm, true),
            (TextureMap::MaskMap, false),
            (TextureMap::Normal, true),
            (TextureMap::Normal, false)
        ]
    );
    // Glowing too, every map beside a glTF.
    let all = MapsUsed {
        glow: true,
        ..graph
    };
    let separate = names(all, ChannelPacking::Separate, true);
    assert_eq!(separate.len(), 10, "{separate:?}");
    assert!(separate.contains(&(TextureMap::Emissive, false)));
    assert!(separate.contains(&(TextureMap::Height, false)));
    // An alpha card (decision 0276): its opacity beside only with separate
    // maps, its light through as a translucency or Unity's thickness, all
    // beside the file.
    let card = MapsUsed {
        opacity: true,
        translucency: true,
        ..graph
    };
    let separate = names(card, ChannelPacking::Separate, false);
    assert!(separate.contains(&(TextureMap::Opacity, false)));
    assert!(separate.contains(&(TextureMap::Translucency, false)));
    let unity = names(card, ChannelPacking::UnityMask, true);
    assert!(unity.contains(&(TextureMap::Thickness, false)));
    assert!(!unity.iter().any(|(m, _)| *m == TextureMap::Opacity));
    // Even a material with no realism maps keeps its card's maps.
    let plain_card = MapsUsed {
        realism: false,
        ..card
    };
    assert_eq!(
        names(plain_card, ChannelPacking::Orm, false),
        vec![
            (TextureMap::Colour, false),
            (TextureMap::Translucency, false)
        ]
    );
}
