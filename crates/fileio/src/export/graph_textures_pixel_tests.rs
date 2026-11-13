//! Pixel art going out (decision 0287): a graph ending in a Pixelate box
//! written at exactly its pixel count, each texel its square's value,
//! only the palette's colours, tiling texel for texel, and Godot's
//! material reading it crisp; a model's glTF read crisp when asked.

use super::graph_textures::{CardMaps, SetOptions, texture_set, write_set, written_size};
use super::graph_textures_tests::{folder, read};
use super::textures::{ChannelPacking, TextureFormat, TextureMap};
use glam::Vec2;
use matgraph::boxes::pixelate::cell_middle;
use matgraph::{
    BoxId, Evaluator, Graph, GraphEdit, MaterialMaps, Palette, Palettes, PlugRef, Registry,
    SettingValue, Size,
};
use std::sync::Arc;

const EARTH: [[u8; 3]; 4] = [[230, 220, 200], [60, 50, 40], [150, 120, 90], [90, 110, 70]];

fn registry() -> Registry {
    Registry::with_standard_boxes().with_palettes(Palettes::new([Palette {
        name: "Earth".into(),
        colours: EARTH.to_vec(),
    }]))
}

/// Noise → Material from height → Pixelate (`pixels` choice, Earth,
/// dithered) → Material out.
fn pixel_graph(registry: &Registry, pixels: u32) -> Graph {
    let mut g = Graph::new("Pixel stone");
    for (i, kind) in ["noise", "material_from_height", "pixelate", "material_out"]
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
    let p = |id: u64, plug: &str| PlugRef::new(BoxId(id), plug);
    for (from, to) in [
        (p(1, "height"), p(2, "height")),
        (p(2, "material"), p(3, "material")),
        (p(3, "material_out"), p(4, "material")),
    ] {
        g.apply(registry, &GraphEdit::Connect { from, to }).unwrap();
    }
    let choice = matgraph::PIXEL_COUNTS
        .iter()
        .position(|n| *n == pixels)
        .unwrap() as u32;
    for (key, value) in [
        ("pixels", SettingValue::Choice(choice)),
        ("limit", SettingValue::Switch(true)),
        ("dither", SettingValue::Number(0.5)),
        ("tidy", SettingValue::Switch(true)),
    ] {
        g.apply(
            registry,
            &GraphEdit::SetSetting {
                node: BoxId(3),
                key: key.into(),
                value,
            },
        )
        .unwrap();
    }
    g
}

fn made(pixels: u32, size: u32) -> (Arc<MaterialMaps>, u32) {
    let r = registry();
    let g = pixel_graph(&r, pixels);
    let grid = matgraph::pixel_grid(&r, &g).unwrap();
    let maps = Evaluator::new()
        .material(&r, &g, Size::new(size).unwrap())
        .unwrap();
    (maps, grid)
}

fn options(pixels: Option<u32>, packing: ChannelPacking) -> SetOptions {
    SetOptions {
        stem: "Pixel stone".into(),
        format: TextureFormat::Png,
        packing,
        green_down: false,
        godot: true,
        card: CardMaps::default(),
        pixels,
    }
}

#[test]
fn a_pixel_art_set_is_written_at_exactly_its_pixel_count() {
    for (pixels, size) in [(32, 256), (48, 256), (16, 512), (256, 256)] {
        let (maps, grid) = made(pixels, size);
        assert_eq!(grid, pixels);
        for packing in [
            ChannelPacking::Standard,
            ChannelPacking::UnityMask,
            ChannelPacking::Separate,
        ] {
            let files = texture_set(&maps, &options(Some(grid), packing)).unwrap();
            for file in &files {
                let back = read(&file.bytes);
                assert_eq!(
                    (back.width, back.height),
                    (pixels, pixels),
                    "{}",
                    file.file_name
                );
            }
        }
    }
}

#[test]
fn each_texel_is_its_squares_value_and_only_the_palettes_colours() {
    let (maps, grid) = made(48, 256);
    let files = texture_set(&maps, &options(Some(grid), ChannelPacking::Standard)).unwrap();
    let colour = files.iter().find(|f| f.map == TextureMap::Colour).unwrap();
    let back = read(&colour.bytes);
    for y in 0..grid {
        for x in 0..grid {
            let i = ((y * grid + x) * 4) as usize;
            let texel = [back.rgba[i], back.rgba[i + 1], back.rgba[i + 2]];
            assert!(EARTH.contains(&texel), "{texel:?} at {x}, {y}");
            let (mx, my) = (cell_middle(x, 256, grid), cell_middle(y, 256, grid));
            let c = maps.colour.at(i64::from(mx), i64::from(my));
            assert_eq!(texel, c.map(|v| (v * 255.0).round() as u8));
        }
    }
    // The height too: its square's value.
    let height = files.iter().find(|f| f.map == TextureMap::Height).unwrap();
    let back = read(&height.bytes);
    let h = maps.height.at(
        i64::from(cell_middle(5, 256, grid)),
        i64::from(cell_middle(7, 256, grid)),
    );
    assert_eq!(
        back.rgba[((7 * grid + 5) * 4) as usize],
        (h.clamp(0.0, 1.0) * 255.0).round() as u8
    );
}

#[test]
fn a_pixel_art_set_tiles_texel_for_texel() {
    // Worked out at two sizes, written at the same pixel count: the same
    // texels, so the grid doesn't depend on the size worked at, and
    // texels across the seam are neighbours as any others are.
    let (small, grid) = made(32, 256);
    let (big, _) = made(32, 512);
    let colour = |m: &MaterialMaps| {
        let files = texture_set(m, &options(Some(grid), ChannelPacking::Standard)).unwrap();
        read(&files[0].bytes).rgba
    };
    let (a, b) = (colour(&small), colour(&big));
    let same = a
        .chunks_exact(4)
        .zip(b.chunks_exact(4))
        .filter(|(x, y)| x == y)
        .count();
    // The noise is worked out finer at 512, so a few squares' averages
    // land on another swatch; most agree.
    assert!(same * 10 >= (grid * grid) as usize * 7, "{same}");
}

#[test]
fn the_godot_material_reads_a_pixel_art_set_crisp() {
    let (maps, grid) = made(32, 256);
    let how = options(Some(grid), ChannelPacking::Standard);
    let files = texture_set(&maps, &how).unwrap();
    let out = folder("pixel-art");
    write_set(&out, &how.stem, &files, true, CardMaps::default(), true).unwrap();
    let tres = std::fs::read_to_string(out.join("Pixel_stone.tres")).unwrap();
    assert!(tres.contains("texture_filter = 2"), "{tres}");
    assert!(tres.contains("albedo_texture"));
    write_set(&out, &how.stem, &files, true, CardMaps::default(), false).unwrap();
    let tres = std::fs::read_to_string(out.join("Pixel_stone.tres")).unwrap();
    assert!(!tres.contains("texture_filter"), "{tres}");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn the_written_size_keeps_to_what_was_made() {
    assert_eq!(written_size(256, Some(32)), 32);
    assert_eq!(written_size(256, None), 256);
    // Never bigger than made, never nothing.
    assert_eq!(written_size(16, Some(32)), 16);
    assert_eq!(written_size(256, Some(0)), 256);
    // Not pixel art: written as made, at any packing.
    let (maps, _) = made(32, 256);
    let files = texture_set(&maps, &options(None, ChannelPacking::Orm)).unwrap();
    assert_eq!(read(&files[0].bytes).width, 256);
}

#[test]
fn a_models_gltf_is_read_crisp_when_asked() {
    assert_eq!(crate::gltf::sampler_filters(false), (9729, 9987));
    // NEAREST up close, NEAREST_MIPMAP_LINEAR far away: Godot's own
    // nearest-with-mipmaps and Unity's Point filter.
    assert_eq!(crate::gltf::sampler_filters(true), (9728, 9986));
}
