//! Alpha cards in a graph's texture set (decision 0276): the maps each
//! engine gets, the opacity in the colour's alpha (sharp and right at the
//! edges, no dark fringe), the translucency and Unity's thickness, and
//! the Godot material's alpha settings.

use super::graph_textures::{CardMaps, file_names, set_maps, texture_set, write_set};
use super::graph_textures_tests::{flat, folder, options, read};
use super::textures::{ChannelPacking, TextureMap};
use super::{ExportPreset, PresetSettings};
use glam::Vec2;
use matgraph::{Evaluator, Graph, GraphEdit, Grey, MaterialMaps, PlugRef, Registry, Size};
use std::sync::Arc;

const CARD: CardMaps = CardMaps {
    opacity: true,
    translucency: true,
};

/// Basic shape (a circle) → Material from height → Opacity (cut by the
/// circle) → Material out, worked out at `size`.
fn leaf(size: u32) -> Arc<MaterialMaps> {
    let registry = Registry::standard();
    let mut g = Graph::new("Leaf");
    for (i, kind) in [
        "basic_shape",
        "material_from_height",
        "opacity",
        "material_out",
    ]
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
        (p(1, "shape"), p(2, "height")),
        (p(2, "material"), p(3, "material")),
        (p(1, "shape"), p(3, "shape")),
        (p(3, "material_out"), p(4, "material")),
    ] {
        g.apply(registry, &GraphEdit::Connect { from, to }).unwrap();
    }
    Evaluator::new()
        .material(registry, &g, Size::new(size).unwrap())
        .unwrap()
}

#[test]
fn a_card_goes_out_with_the_maps_each_engine_reads() {
    use TextureMap::*;
    let maps = |p: ExportPreset| {
        let PresetSettings { packing, .. } = p.settings();
        set_maps(packing, CARD)
    };
    let orm = vec![Colour, Orm, Normal, Translucency, Height];
    assert_eq!(maps(ExportPreset::Godot), orm);
    assert_eq!(maps(ExportPreset::Solis), orm);
    assert_eq!(maps(ExportPreset::UnrealEngine), orm);
    assert_eq!(maps(ExportPreset::SubstancePainter), orm);
    assert_eq!(
        maps(ExportPreset::Unity),
        vec![Colour, MaskMap, Normal, Thickness, Height]
    );
    assert_eq!(
        set_maps(ChannelPacking::Separate, CARD),
        vec![
            Colour,
            Roughness,
            Metallic,
            Height,
            Normal,
            Opacity,
            Translucency,
            Occlusion
        ]
    );
    // A card letting no light through has no translucency map.
    let solid_light = CardMaps {
        opacity: true,
        translucency: false,
    };
    assert_eq!(
        set_maps(ChannelPacking::Orm, solid_light),
        vec![Colour, Orm, Normal, Height]
    );
    // The names follow.
    let mut how = options(ChannelPacking::UnityMask);
    how.card = CARD;
    assert!(file_names(&how).contains(&"Stone_wall_Thickness.png".to_string()));
}

#[test]
fn the_opacity_is_the_colours_alpha_solid_inside_clear_outside_and_soft_at_the_edge() {
    let maps = leaf(64);
    assert_eq!(CardMaps::of(&maps), CARD);
    let files = texture_set(&maps, &options(ChannelPacking::Separate)).unwrap();
    let colour = read(
        &files
            .iter()
            .find(|f| f.map == TextureMap::Colour)
            .unwrap()
            .bytes,
    );
    let opacity = read(
        &files
            .iter()
            .find(|f| f.map == TextureMap::Opacity)
            .unwrap()
            .bytes,
    );
    let n = colour.width as usize;
    let alpha = |x: usize, y: usize| colour.rgba[(y * n + x) * 4 + 3];
    // The middle of the circle is solid, the corners clear.
    assert_eq!(alpha(32, 32), 255);
    assert_eq!(alpha(0, 0), 0);
    assert_eq!(alpha(63, 63), 0);
    // Along a row through the middle: clear, then a pixel or two between,
    // then solid; never solid then clear then solid again.
    let row: Vec<u8> = (0..n).map(|x| alpha(x, 32)).collect();
    let partial = row.iter().filter(|a| **a > 0 && **a < 255).count();
    assert!((1..=4).contains(&partial), "{row:?}");
    let first_solid = row.iter().position(|a| *a == 255).unwrap();
    assert!(
        row[..first_solid].windows(2).all(|w| w[0] <= w[1]),
        "{row:?}"
    );
    // The opacity map is the same as the alpha.
    for i in 0..n * n {
        assert_eq!(opacity.rgba[i * 4], colour.rgba[i * 4 + 3]);
    }
    // No dark fringe: the colour where the card is clear is the leaf's
    // colour (Material from height's), not black.
    let rgb = |x: usize, y: usize| &colour.rgba[(y * n + x) * 4..(y * n + x) * 4 + 3];
    assert_eq!(rgb(0, 0), rgb(32, 32));
}

#[test]
fn the_light_through_is_the_cards_colour_and_unitys_thickness_its_opposite() {
    let size = Size::new(16).unwrap();
    let mut maps = flat(16, 0.5, 0.0, 1.0, [0.0, 0.0, 1.0]);
    maps.opacity = Some(Arc::new(Grey::filled(size, 1.0)));
    maps.translucency = Some(Arc::new(Grey::filled(size, 0.5)));
    let pixel = |packing: ChannelPacking, map: TextureMap| {
        let files = texture_set(&maps, &options(packing)).unwrap();
        let file = files.iter().find(|f| f.map == map).unwrap();
        read(&file.bytes).rgba[..4].to_vec()
    };
    // The colour (0.2, 0.4, 0.6) times a half.
    assert_eq!(
        pixel(ChannelPacking::Orm, TextureMap::Translucency),
        [26, 51, 77, 255]
    );
    assert_eq!(
        pixel(ChannelPacking::UnityMask, TextureMap::Thickness),
        [128, 128, 128, 255]
    );
    // A solid material's colour stays opaque, and its maps are unchanged.
    let solid = flat(16, 0.5, 0.0, 1.0, [0.0, 0.0, 1.0]);
    assert_eq!(CardMaps::of(&solid), CardMaps::default());
    let files = texture_set(&solid, &options(ChannelPacking::Orm)).unwrap();
    assert!(read(&files[0].bytes).rgba.chunks(4).all(|p| p[3] == 255));
    // Card maps of another size are refused, in plain words.
    maps.translucency = Some(Arc::new(Grey::filled(Size::new(32).unwrap(), 0.5)));
    let why = texture_set(&maps, &options(ChannelPacking::Orm)).unwrap_err();
    assert!(why.contains("same size"), "{why}");
}

#[test]
fn godots_material_cuts_the_card_out_shows_both_sides_and_lights_it_from_behind() {
    let maps = leaf(32);
    let mut how = options(ChannelPacking::Orm);
    how.godot = true;
    how.card = CardMaps::of(&maps);
    let files = texture_set(&maps, &how).unwrap();
    let out = folder("card");
    let written = write_set(&out, &how.stem, &files, true, how.card, false).unwrap();
    assert_eq!(written.len(), files.len() + 1);
    for (path, name) in written.iter().zip(file_names(&how)) {
        assert_eq!(path.file_name().unwrap().to_string_lossy(), name);
    }
    let tres = std::fs::read_to_string(out.join("Stone_wall.tres")).unwrap();
    for line in [
        "load_steps=5",
        "transparency = 2\n",
        "alpha_scissor_threshold = 0.5\n",
        "cull_mode = 2\n",
        "backlight_enabled = true\n",
        "backlight = Color(0, 0, 0, 1)\n",
        "path=\"Stone_wall_Translucency.png\" id=\"4\"",
        "backlight_texture = ExtResource(\"4\")\n",
        "albedo_texture = ExtResource(\"1\")\n",
    ] {
        assert!(tres.contains(line), "{line} in\n{tres}");
    }
    // A solid material's Godot file has none of it.
    let solid = flat(16, 0.5, 0.0, 1.0, [0.0, 0.0, 1.0]);
    let files = texture_set(&solid, &options(ChannelPacking::Orm)).unwrap();
    write_set(&out, &how.stem, &files, true, CardMaps::default(), false).unwrap();
    let tres = std::fs::read_to_string(out.join("Stone_wall.tres")).unwrap();
    assert!(!tres.contains("transparency") && !tres.contains("backlight"));
    assert!(!tres.contains("cull_mode"));
    let _ = std::fs::remove_dir_all(&out);
}
