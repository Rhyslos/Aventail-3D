//! The realism channels in exports (decision 0208): glTF's own
//! metallic-roughness (ORM) and emissive maps inside a glTF, and what is
//! asked for beside the file (an ORM, Unity's mask map, or separate
//! roughness and metal), for OBJ named in its `.mtl`.

use super::*;
use paint::ChannelPart;
use scene::{Command, ObjectKind, PrimitiveKind};

/// A cube painted with a steel fill (roughness 25%, metal 100%), with
/// glow when `glowing`.
fn steel_cube(glowing: bool) -> Document {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let mut set = paint::TextureSet::new(64).unwrap();
    set.realism = true;
    let mut steel = paint::Layer::fill("Steel", [150, 150, 160, 255]);
    steel.channels.roughness = Some(ChannelPart::Fill([64, 64, 64, 255]));
    steel.channels.metal = Some(ChannelPart::Fill([255, 255, 255, 255]));
    if glowing {
        steel.channels.glow = Some(ChannelPart::Fill([255, 128, 0, 255]));
    }
    set.layers.push(steel);
    doc.execute(Command::SetTextureSet {
        material: scene::Material::standard().name,
        set: Some(set),
    })
    .unwrap();
    doc
}

fn json_of(glb: &[u8]) -> serde_json::Value {
    let length = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
    serde_json::from_slice(&glb[20..20 + length]).unwrap()
}

fn options(packing: ChannelPacking) -> ExportOptions {
    ExportOptions {
        packing,
        ..ExportOptions::default()
    }
}

/// The pixel in the middle of an encoded image.
fn middle(bytes: &[u8]) -> [u8; 4] {
    let image = crate::image_file::read_image(bytes).unwrap();
    let (w, h) = (image.width, image.height);
    let at = ((h / 2 * w + w / 2) * 4) as usize;
    image.rgba[at..at + 4].try_into().unwrap()
}

#[test]
fn a_gltf_holds_roughness_and_metal_as_its_own_orm() {
    let doc = steel_cube(false);
    let files = export_bytes(
        &doc,
        ExportFormat::Glb,
        &options(ChannelPacking::Standard),
        "App",
        "crate.glb",
    )
    .unwrap();
    let json = json_of(&files.main);
    let pbr = &json["materials"][0]["pbrMetallicRoughness"];
    let index = pbr["metallicRoughnessTexture"]["index"].as_u64().unwrap() as usize;
    assert_eq!(pbr["metallicFactor"], 1.0);
    assert_eq!(pbr["roughnessFactor"], 1.0);
    let name = json["images"][index]["name"].as_str().unwrap();
    assert!(name.ends_with("_ORM.png"), "{name}");
    let orm = files
        .textures
        .iter()
        .find(|t| t.map == TextureMap::Orm)
        .unwrap();
    assert!(orm.inside);
    assert!(json.get("extensionsUsed").is_none());
    assert!(
        json["materials"][0].get("emissiveTexture").is_none(),
        "no glow"
    );
    assert!(files.beside.is_empty(), "nothing beside, as standard");
}

#[test]
fn the_orm_holds_white_occlusion_roughness_and_metal() {
    let doc = steel_cube(false);
    let files = export_bytes(
        &doc,
        ExportFormat::Obj,
        &options(ChannelPacking::Standard),
        "App",
        "crate.obj",
    )
    .unwrap();
    let (name, orm) = files
        .beside
        .iter()
        .find(|(n, _)| n.ends_with("_ORM.png"))
        .expect("an ORM beside an OBJ");
    assert_eq!(middle(orm), [255, 64, 255, 255]);
    let mtl = String::from_utf8_lossy(&files.beside[0].1);
    assert!(mtl.contains(&format!("map_Pr -imfchan g {name}")), "{mtl}");
    assert!(mtl.contains(&format!("map_Pm -imfchan b {name}")), "{mtl}");
}

#[test]
fn glow_goes_out_as_emissive() {
    let doc = steel_cube(true);
    let files = export_bytes(
        &doc,
        ExportFormat::Glb,
        &options(ChannelPacking::Standard),
        "App",
        "lamp.glb",
    )
    .unwrap();
    let json = json_of(&files.main);
    assert_eq!(json["materials"][0]["emissiveFactor"][0], 1.0);
    assert!(json["materials"][0]["emissiveTexture"]["index"].is_u64());
    let obj = export_bytes(
        &doc,
        ExportFormat::Obj,
        &options(ChannelPacking::Standard),
        "App",
        "lamp.obj",
    )
    .unwrap();
    let (_, glow) = obj
        .beside
        .iter()
        .find(|(n, _)| n.ends_with("_Emissive.png"))
        .unwrap();
    assert_eq!(middle(glow), [255, 128, 0, 255]);
    assert!(String::from_utf8_lossy(&obj.beside[0].1).contains("map_Ke"));
}

#[test]
fn unitys_mask_map_goes_beside_a_gltf() {
    let doc = steel_cube(false);
    assert_eq!(
        ExportPreset::Unity.settings().packing,
        ChannelPacking::UnityMask
    );
    let files = export_bytes(
        &doc,
        ExportFormat::Glb,
        &options(ChannelPacking::UnityMask),
        "App",
        "crate.glb",
    )
    .unwrap();
    let (_, mask) = files
        .beside
        .iter()
        .find(|(n, _)| n.ends_with("_MaskMap.png"))
        .expect("the mask map beside the glTF");
    // Metal, occlusion, detail mask, smoothness (1 - roughness).
    assert_eq!(middle(mask), [255, 255, 0, 191]);
    // The glTF still holds its own ORM.
    assert!(
        json_of(&files.main)["materials"][0]["pbrMetallicRoughness"]
            .get("metallicRoughnessTexture")
            .is_some()
    );
}

#[test]
fn separate_maps_are_grey_and_named_in_the_mtl() {
    let doc = steel_cube(false);
    let files = export_bytes(
        &doc,
        ExportFormat::Obj,
        &options(ChannelPacking::Separate),
        "App",
        "crate.obj",
    )
    .unwrap();
    let find = |end: &str| {
        files
            .beside
            .iter()
            .find(|(n, _)| n.ends_with(end))
            .unwrap_or_else(|| panic!("{end}"))
    };
    let (rough_name, rough) = find("_Roughness.png");
    let (metal_name, metal) = find("_Metallic.png");
    assert_eq!(middle(rough), [64, 64, 64, 255]);
    assert_eq!(middle(metal), [255, 255, 255, 255]);
    let mtl = String::from_utf8_lossy(&files.beside[0].1);
    assert!(mtl.contains(&format!("map_Pr {rough_name}")), "{mtl}");
    assert!(mtl.contains(&format!("map_Pm {metal_name}")), "{mtl}");
    assert!(!files.beside.iter().any(|(n, _)| n.ends_with("_ORM.png")));
}

#[test]
fn hand_painted_sets_send_out_only_their_colour() {
    let mut doc = steel_cube(true);
    let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
    set.realism = false;
    set.layers[0].channels = paint::Channels::default();
    doc.execute(Command::SetTextureSet {
        material: "Default".into(),
        set: Some(set),
    })
    .unwrap();
    let report = textures::plan(
        &doc,
        &prepare(&doc, &ExportOptions::default()),
        TextureFormat::Png,
        "crate",
        ChannelPacking::Separate,
        true,
    );
    assert_eq!(report.len(), 1);
    assert_eq!(report[0].map, TextureMap::Colour);
}

/// A cube whose steel fill has a raised square painted on its height.
fn bumpy_cube() -> Document {
    let mut doc = steel_cube(false);
    let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
    let mut height = paint::Image::new(set.size).unwrap();
    for y in 20..44 {
        for x in 20..44 {
            height.set_pixel(x, y, [255, 255, 255, 255]);
        }
    }
    let mut bumps = paint::Layer::paint("Bumps", paint::Image::new(set.size).unwrap());
    bumps.channels.height = Some(ChannelPart::Paint(height));
    set.layers.push(bumps);
    doc.execute(Command::SetTextureSet {
        material: "Default".into(),
        set: Some(set),
    })
    .unwrap();
    doc
}

#[test]
fn height_goes_out_as_a_normal_map_green_up_inside_a_gltf() {
    let doc = bumpy_cube();
    let files = export_bytes(
        &doc,
        ExportFormat::Glb,
        &options(ChannelPacking::Standard),
        "App",
        "crate.glb",
    )
    .unwrap();
    let json = json_of(&files.main);
    assert!(json["materials"][0]["normalTexture"]["index"].is_u64());
    let normal = files
        .textures
        .iter()
        .find(|t| t.map == TextureMap::Normal)
        .unwrap();
    assert!(normal.inside);
    // Flat where nothing changes: straight out.
    let set = doc.scene().paint().set("Default").unwrap();
    let style = NormalStyle {
        texel_cm: 1.0,
        green_down: false,
    };
    let pixels = textures::map_pixels(set, TextureMap::Normal, [1.0; 3], style).unwrap();
    let at = |x: usize, y: usize| &pixels[(y * 64 + x) * 4..(y * 64 + x) * 4 + 4];
    assert_eq!(at(5, 5), [128, 128, 255, 255]);
    // Along the square's top edge (rows run down), the slope faces up:
    // green above the middle; its left edge faces left: red below.
    assert!(at(30, 20)[1] > 140, "{:?}", at(30, 20));
    assert!(at(20, 30)[0] < 116, "{:?}", at(20, 30));
    // DirectX's green down flips green only.
    let down = textures::map_pixels(
        set,
        TextureMap::Normal,
        [1.0; 3],
        NormalStyle {
            green_down: true,
            ..style
        },
    )
    .unwrap();
    let i = (20 * 64 + 30) * 4;
    assert_eq!(down[i], pixels[i]);
    assert!(down[i + 1] < 116);
}

#[test]
fn an_obj_names_its_normal_map_and_unreal_wants_green_down() {
    let doc = bumpy_cube();
    let files = export_bytes(
        &doc,
        ExportFormat::Obj,
        &options(ChannelPacking::Standard),
        "App",
        "crate.obj",
    )
    .unwrap();
    let (name, _) = files
        .beside
        .iter()
        .find(|(n, _)| n.ends_with("_Normal.png"))
        .expect("a normal map beside the OBJ");
    let mtl = String::from_utf8_lossy(&files.beside[0].1);
    assert!(mtl.contains(&format!("norm {name}")), "{mtl}");
    let unreal = ExportPreset::UnrealEngine.settings();
    assert!(unreal.green_down);
    assert_eq!(unreal.packing, ChannelPacking::Orm);
    // Beside a glTF too, when maps go beside it.
    let glb = export_bytes(
        &doc,
        ExportFormat::Glb,
        &ExportOptions {
            packing: ChannelPacking::Orm,
            green_down: true,
            ..ExportOptions::default()
        },
        "App",
        "crate.glb",
    )
    .unwrap();
    assert!(glb.beside.iter().any(|(n, _)| n.ends_with("_Normal.png")));
}
