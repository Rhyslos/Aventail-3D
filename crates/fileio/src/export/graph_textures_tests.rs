//! A material graph's texture set on its own (decision 0262): the maps
//! each engine's packing asks for, files written and read back at their
//! size, the packing's channels, the normal map's green, seamless edges,
//! the Godot material, and failures in plain words.

use super::graph_textures::{
    CardMaps, SetOptions, file_names, set_maps, texture_set, write_set,
};
use super::textures::{ChannelPacking, TextureFormat, TextureMap};
use super::{ExportPreset, PresetSettings};
use glam::Vec2;
use matgraph::{
    Evaluator, Graph, GraphEdit, Grey, Image, MaterialMaps, PlugRef, Registry, Rgb, Size,
};
use std::sync::Arc;

/// Noise → Material from height → Material out, worked out at `size`.
fn stone(size: u32) -> Arc<MaterialMaps> {
    let registry = Registry::standard();
    let mut g = Graph::new("Stone");
    for (i, kind) in ["noise", "material_from_height", "material_out"]
        .into_iter()
        .enumerate()
    {
        g.apply(
            registry,
            &GraphEdit::AddBox {
                kind: kind.into(),
                at: Vec2::new(i as f32 * 200.0, 0.0),
            },
        )
        .unwrap();
    }
    let p = |id: u64, plug: &str| PlugRef::new(matgraph::BoxId(id), plug);
    for (from, to) in [
        (p(1, "height"), p(2, "height")),
        (p(2, "material"), p(3, "material")),
    ] {
        g.apply(registry, &GraphEdit::Connect { from, to }).unwrap();
    }
    Evaluator::new()
        .material(registry, &g, Size::new(size).unwrap())
        .unwrap()
}

/// A flat material: one value in each map.
pub(super) fn flat(size: u32, rough: f32, metal: f32, open: f32, normal: [f32; 3]) -> MaterialMaps {
    let s = Size::new(size).unwrap();
    let grey = |v: f32| Arc::new(Grey::filled(s, v));
    MaterialMaps {
        colour: Arc::new(Rgb::filled(s, [0.2, 0.4, 0.6])),
        height: grey(0.5),
        roughness: grey(rough),
        metal: grey(metal),
        normal: Arc::new(Rgb::filled(s, normal)),
        occlusion: grey(open),
        curvature: grey(0.5),
        opacity: None,
        translucency: None,
    }
}

pub(super) fn options(packing: ChannelPacking) -> SetOptions {
    SetOptions {
        stem: "Stone wall".into(),
        format: TextureFormat::Png,
        packing,
        green_down: false,
        godot: false,
        card: CardMaps::default(),
    }
}

pub(super) fn read(bytes: &[u8]) -> crate::image_file::ReadImage {
    crate::image_file::read_image(bytes).unwrap()
}

pub(super) fn folder(name: &str) -> std::path::PathBuf {
    let f = std::env::temp_dir().join(format!("graph-set-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&f);
    f
}

#[test]
fn every_engine_gets_its_own_packing() {
    let maps = |p: ExportPreset| {
        let PresetSettings { packing, .. } = p.settings();
        set_maps(packing, CardMaps::default())
    };
    use TextureMap::*;
    assert_eq!(maps(ExportPreset::Godot), vec![Colour, Orm, Normal, Height]);
    assert_eq!(maps(ExportPreset::Solis), vec![Colour, Orm, Normal, Height]);
    assert_eq!(
        maps(ExportPreset::UnrealEngine),
        vec![Colour, Orm, Normal, Height]
    );
    assert_eq!(
        maps(ExportPreset::Unity),
        vec![Colour, MaskMap, Normal, Height]
    );
    assert_eq!(
        maps(ExportPreset::SubstancePainter),
        vec![Colour, Orm, Normal, Height]
    );
    assert_eq!(
        set_maps(ChannelPacking::Separate, CardMaps::default()),
        vec![Colour, Roughness, Metallic, Height, Normal, Occlusion]
    );
    // Unreal's normal map points green down.
    assert!(ExportPreset::UnrealEngine.settings().green_down);
}

#[test]
fn files_are_written_at_the_size_asked_and_named_after_the_graph() {
    let maps = stone(64);
    let mut how = options(ChannelPacking::Standard);
    how.godot = true;
    let files = texture_set(&maps, &how).unwrap();
    let names: Vec<&str> = files.iter().map(|f| f.file_name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Stone_wall.png",
            "Stone_wall_ORM.png",
            "Stone_wall_Normal.png",
            "Stone_wall_Height.png"
        ]
    );
    assert_eq!(file_names(&how).last().unwrap(), "Stone_wall.tres");
    let out = folder("written");
    let written = write_set(&out, &how.stem, &files, true, CardMaps::default()).unwrap();
    assert_eq!(written.len(), 5);
    for (path, name) in written.iter().zip(file_names(&how)) {
        assert_eq!(path.file_name().unwrap().to_string_lossy(), name);
        assert!(path.is_file());
    }
    for file in &files {
        let back = read(&std::fs::read(out.join(&file.file_name)).unwrap());
        assert_eq!((back.width, back.height), (64, 64), "{}", file.file_name);
    }
    let tres = std::fs::read_to_string(out.join("Stone_wall.tres")).unwrap();
    assert!(tres.contains("path=\"Stone_wall.png\""), "{tres}");
    assert!(tres.contains("albedo_texture"));
    assert!(tres.contains("path=\"Stone_wall_Normal.png\""));
    assert!(tres.contains("path=\"Stone_wall_ORM.png\""));
    assert!(tres.contains("ao_enabled = true"));
    // Written again: replaced, not added to.
    write_set(&out, &how.stem, &files, true, CardMaps::default()).unwrap();
    assert_eq!(std::fs::read_dir(&out).unwrap().count(), 5);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn the_realism_maps_are_packed_as_each_engine_reads_them() {
    let maps = flat(16, 0.8, 0.2, 0.6, [0.0, 0.0, 1.0]);
    let pixel = |packing: ChannelPacking, map: TextureMap| {
        let files = texture_set(&maps, &options(packing)).unwrap();
        let file = files.iter().find(|f| f.map == map).unwrap();
        let back = read(&file.bytes);
        back.rgba[..4].to_vec()
    };
    // Occlusion, roughness, metal in red, green, blue.
    assert_eq!(
        pixel(ChannelPacking::Standard, TextureMap::Orm),
        [153, 204, 51, 255]
    );
    // Unity: metal, occlusion, detail 0, smoothness (1 − roughness).
    assert_eq!(
        pixel(ChannelPacking::UnityMask, TextureMap::MaskMap),
        [51, 153, 0, 51]
    );
    assert_eq!(
        pixel(ChannelPacking::Separate, TextureMap::Roughness),
        [204, 204, 204, 255]
    );
    assert_eq!(
        pixel(ChannelPacking::Separate, TextureMap::Metallic),
        [51, 51, 51, 255]
    );
    assert_eq!(
        pixel(ChannelPacking::Separate, TextureMap::Occlusion),
        [153, 153, 153, 255]
    );
    assert_eq!(
        pixel(ChannelPacking::Separate, TextureMap::Height),
        [128, 128, 128, 255]
    );
    assert_eq!(
        pixel(ChannelPacking::Standard, TextureMap::Colour),
        [51, 102, 153, 255]
    );
}

#[test]
fn the_normal_maps_green_points_up_or_down_as_asked() {
    // A surface tilted up the texture.
    let maps = flat(16, 0.5, 0.0, 1.0, [0.0, 0.6, 0.8]);
    let green = |down: bool| {
        let mut how = options(ChannelPacking::Standard);
        how.green_down = down;
        let files = texture_set(&maps, &how).unwrap();
        let normal = files.iter().find(|f| f.map == TextureMap::Normal).unwrap();
        read(&normal.bytes).rgba[1]
    };
    assert_eq!(green(false), 204);
    assert_eq!(green(true), 51);
}

#[test]
fn the_set_tiles_seamlessly() {
    let maps = stone(64);
    let files = texture_set(&maps, &options(ChannelPacking::Standard)).unwrap();
    for file in &files {
        let back = read(&file.bytes);
        let n = back.width as usize;
        let at = |x: usize, y: usize, c: usize| f32::from(back.rgba[(y * n + x) * 4 + c]);
        // The step across the join (right edge to left) against the steps
        // between neighbours inside.
        let (mut join, mut inside) = (0.0, 0.0);
        for y in 0..n {
            for c in 0..3 {
                join += (at(n - 1, y, c) - at(0, y, c)).abs();
                for x in 0..n - 1 {
                    inside += (at(x, y, c) - at(x + 1, y, c)).abs();
                }
            }
        }
        let inside = inside / (n - 1) as f32;
        assert!(
            join <= inside * 3.0 + 3.0 * n as f32,
            "{}: join {join}, inside {inside}",
            file.file_name
        );
    }
}

#[test]
fn tga_is_written_when_asked_and_png_when_textures_are_left_out() {
    let maps = flat(16, 0.5, 0.0, 1.0, [0.0, 0.0, 1.0]);
    let mut how = options(ChannelPacking::Standard);
    how.format = TextureFormat::Tga;
    let files = texture_set(&maps, &how).unwrap();
    assert!(files.iter().all(|f| f.file_name.ends_with(".tga")));
    assert_eq!(read(&files[0].bytes).width, 16);
    how.format = TextureFormat::None;
    assert!(
        texture_set(&maps, &how)
            .unwrap()
            .iter()
            .all(|f| f.file_name.ends_with(".png"))
    );
}

#[test]
fn problems_are_said_in_plain_words() {
    // Maps of different sizes.
    let mut maps = flat(16, 0.5, 0.0, 1.0, [0.0, 0.0, 1.0]);
    maps.height = Arc::new(Image::filled(Size::new(32).unwrap(), 0.5));
    let why = texture_set(&maps, &options(ChannelPacking::Standard)).unwrap_err();
    assert!(why.contains("same size"), "{why}");
    // A folder that is a file.
    let out = folder("blocked");
    std::fs::write(&out, b"in the way").unwrap();
    let maps = flat(16, 0.5, 0.0, 1.0, [0.0, 0.0, 1.0]);
    let files = texture_set(&maps, &options(ChannelPacking::Standard)).unwrap();
    let why = write_set(&out.join("inside"), "Stone", &files, false, CardMaps::default()).unwrap_err();
    assert!(why.starts_with("couldn't make the folder"), "{why}");
    let _ = std::fs::remove_file(&out);
    // Damaged numbers come out as bytes in range, never a crash.
    let odd = flat(16, f32::NAN, 2.0, -1.0, [f32::INFINITY, 0.0, 1.0]);
    assert!(texture_set(&odd, &options(ChannelPacking::Separate)).is_ok());
    // A name with nothing safe in it still makes files.
    let mut how = options(ChannelPacking::Standard);
    how.stem = String::new();
    assert!(file_names(&how)[0].starts_with("material"));
}
