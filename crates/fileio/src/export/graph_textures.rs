//! A material graph's texture set on its own (Phase 12, milestone 6,
//! [decision 0262](../../../../docs/decisions/0262-baked-export.md)):
//! the material's maps written as seamless picture files, for tiling it
//! in other apps and engines (Material mode → Export textures…).
//!
//! The maps follow the same rule as a model's export
//! ([`super::maps_going_out`], decision 0208), all beside, as files: the
//! colour, the realism maps packed as the engine wants them (an ORM for
//! Godot, Unreal Engine, Solis and Substance Painter; Unity's mask map;
//! or each map on its own), and the normal map (green up, or down for
//! Unreal Engine). A texture set always takes its height too, as grey, and
//! with separate maps its occlusion, since tools that read texture sets
//! use them. For Godot, a material file (`.tres`) naming them comes
//! with it.
//!
//! Nothing is padded: a graph's texture repeats seamlessly, so its edges
//! already lead into each other.
//!
//! **Alpha cards** (milestone 14, decision 0276): a material made
//! see-through by the Opacity box carries its opacity in the colour map's
//! alpha (every engine reads it there), and as its own grey map too with
//! separate maps; the light through it goes beside as a translucency map
//! (Unity: a thickness map). Godot's material file then cuts the card out
//! (alpha scissor at a half), shows both sides and lights it from behind
//! with the translucency (backlight).
//!
//! **Pixel art** (milestone 24, decision 0287): a graph ending in a
//! Pixelate box goes out at exactly its pixel count
//! ([`SetOptions::pixels`]), one texel a square, each the middle of its
//! square as worked out ([`matgraph::boxes::pixelate::cell_middle`]), so
//! nothing is blended. Godot's material file then reads its textures
//! crisp: `texture_filter = 2`, nearest with mipmaps (sharp squares up
//! close, smooth far away so they don't shimmer).

use super::textures::{ChannelPacking, MapsUsed, TextureFormat, TextureMap, encode, file_safe};
use matgraph::MaterialMaps;
use std::path::{Path, PathBuf};
use words::{tr, trf};

/// How a texture set goes out.
#[derive(Debug, Clone, PartialEq)]
pub struct SetOptions {
    /// The start of every file's name (the graph's name, made safe).
    pub stem: String,
    /// PNG or TGA (left out counts as PNG: the set is asked for).
    pub format: TextureFormat,
    /// How the realism maps are packed.
    pub packing: ChannelPacking,
    /// Whether the normal map's green points down (Unreal Engine).
    pub green_down: bool,
    /// Whether a Godot material file comes with it.
    pub godot: bool,
    /// The alpha card's maps the material has, for naming the files
    /// before it is made ([`texture_set`] goes by the material itself).
    pub card: CardMaps,
    /// Pixel art (decision 0287): the squares across the material (its
    /// Pixelate box's Pixels), written as exactly that many texels and
    /// shown crisp; `None` writes the material at the size it was made.
    pub pixels: Option<u32>,
}

/// An alpha card's maps a material has (decision 0276).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CardMaps {
    /// See-through in places (the Opacity box).
    pub opacity: bool,
    /// Light passing through.
    pub translucency: bool,
}

impl CardMaps {
    /// The ones `maps` has.
    pub fn of(maps: &MaterialMaps) -> Self {
        Self {
            opacity: maps.opacity.is_some(),
            translucency: maps.translucency.is_some(),
        }
    }
}

/// One file of a texture set.
#[derive(Debug, Clone, PartialEq)]
pub struct SetFile {
    /// Which map it is.
    pub map: TextureMap,
    /// Its file's name.
    pub file_name: String,
    /// Its encoded bytes.
    pub bytes: Vec<u8>,
}

/// The maps a texture set packed with `packing` goes out as, in order,
/// with an alpha card's `card` maps.
pub fn set_maps(packing: ChannelPacking, card: CardMaps) -> Vec<TextureMap> {
    let used = MapsUsed {
        realism: true,
        height: true,
        glow: false,
        opacity: card.opacity,
        translucency: card.translucency,
    };
    let mut maps: Vec<TextureMap> = Vec::new();
    for (map, _) in super::maps_going_out(used, packing, false) {
        if !maps.contains(&map) {
            maps.push(map);
        }
    }
    if !maps.contains(&TextureMap::Height) {
        maps.push(TextureMap::Height);
    }
    if packing == ChannelPacking::Separate {
        maps.push(TextureMap::Occlusion);
    }
    maps
}

/// The format a set's pictures are written in: PNG unless TGA is asked.
fn picture_format(format: TextureFormat) -> TextureFormat {
    match format {
        TextureFormat::Tga => TextureFormat::Tga,
        TextureFormat::Png | TextureFormat::None => TextureFormat::Png,
    }
}

/// The file names a set with `options` writes, pictures first, then the
/// Godot material if asked: for the export check, before anything is
/// made.
pub fn file_names(options: &SetOptions) -> Vec<String> {
    let stem = file_safe(&options.stem);
    let ext = picture_format(options.format).extension().unwrap_or("png");
    let mut out: Vec<String> = set_maps(options.packing, options.card)
        .into_iter()
        .map(|map| format!("{stem}{}.{ext}", map.suffix()))
        .collect();
    if options.godot {
        out.push(format!("{stem}.tres"));
    }
    out
}

/// `maps` as the pictures of a texture set made with `options`, or why
/// they can't be (maps of different sizes, a picture that couldn't be
/// encoded).
pub fn texture_set(maps: &MaterialMaps, options: &SetOptions) -> Result<Vec<SetFile>, String> {
    let made = maps.size();
    let greys = [&maps.height, &maps.roughness, &maps.metal, &maps.occlusion];
    let card_greys = [&maps.opacity, &maps.translucency];
    if maps.normal.size() != made
        || greys.iter().any(|g| g.size() != made)
        || card_greys
            .iter()
            .flat_map(|g| g.iter())
            .any(|g| g.size() != made)
    {
        return Err(tr("the material's maps aren't all the same size").to_string());
    }
    // Pixel art: one texel a square, from the square's middle.
    let size = written_size(made, options.pixels);
    let from: Vec<usize> = (0..size * size)
        .map(|i| {
            let (x, y) = (i % size, i / size);
            let at = |v: u32| {
                if size == made {
                    v
                } else {
                    matgraph::boxes::pixelate::cell_middle(v, made, size)
                }
            };
            (at(y) * made + at(x)) as usize
        })
        .collect();
    let stem = file_safe(&options.stem);
    let format = picture_format(options.format);
    let ext = format.extension().unwrap_or("png");
    set_maps(options.packing, CardMaps::of(maps))
        .into_iter()
        .map(|map| {
            let rgba = pixels(maps, map, options.green_down, &from);
            let bytes = encode(&rgba, size, format)
                .ok_or_else(|| tr("a picture couldn't be written").to_string())?;
            Ok(SetFile {
                map,
                file_name: format!("{stem}{}.{ext}", map.suffix()),
                bytes,
            })
        })
        .collect()
}

/// A 0 to 1 value as a byte.
fn byte(v: f32) -> u8 {
    if v.is_finite() {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    } else {
        0
    }
}

/// The texels across a set made at `made` pixels is written at: `pixels`
/// for pixel art (when the material was made at least that big), else
/// as made.
pub fn written_size(made: u32, pixels: Option<u32>) -> u32 {
    pixels.filter(|n| *n > 0 && *n <= made).unwrap_or(made)
}

/// Map `map` of `maps` as RGBA bytes, row by row from the top: each
/// texel the material's pixel `from` names.
fn pixels(maps: &MaterialMaps, map: TextureMap, green_down: bool, from: &[usize]) -> Vec<u8> {
    let mut out = Vec::with_capacity(from.len() * 4);
    for &i in from {
        let grey = |g: &matgraph::Grey| byte(g.pixels()[i]);
        let pixel = match map {
            TextureMap::Colour => {
                let c = maps.colour.pixels()[i];
                let alpha = maps.opacity.as_ref().map_or(255, |g| grey(g));
                [byte(c[0]), byte(c[1]), byte(c[2]), alpha]
            }
            TextureMap::Opacity => {
                let v = maps.opacity.as_ref().map_or(255, |g| grey(g));
                [v, v, v, 255]
            }
            TextureMap::Translucency => {
                let t = maps.translucency.as_ref().map_or(0.0, |g| g.pixels()[i]);
                let c = maps.colour.pixels()[i];
                [byte(c[0] * t), byte(c[1] * t), byte(c[2] * t), 255]
            }
            TextureMap::Thickness => {
                let t = maps.translucency.as_ref().map_or(0.0, |g| g.pixels()[i]);
                let v = byte(1.0 - t);
                [v, v, v, 255]
            }
            TextureMap::Normal => {
                let n = maps.normal.pixels()[i];
                let y = if green_down { -n[1] } else { n[1] };
                let half = |v: f32| byte(v * 0.5 + 0.5);
                [half(n[0]), half(y), half(n[2]), 255]
            }
            TextureMap::Orm => [
                grey(&maps.occlusion),
                grey(&maps.roughness),
                grey(&maps.metal),
                255,
            ],
            TextureMap::MaskMap => [
                grey(&maps.metal),
                grey(&maps.occlusion),
                0,
                255 - grey(&maps.roughness),
            ],
            TextureMap::Roughness => {
                let v = grey(&maps.roughness);
                [v, v, v, 255]
            }
            TextureMap::Metallic => {
                let v = grey(&maps.metal);
                [v, v, v, 255]
            }
            TextureMap::Height => {
                let v = grey(&maps.height);
                [v, v, v, 255]
            }
            TextureMap::Occlusion => {
                let v = grey(&maps.occlusion);
                [v, v, v, 255]
            }
            // Nothing in a graph glows yet.
            TextureMap::Emissive => [0, 0, 0, 255],
        };
        out.extend_from_slice(&pixel);
    }
    out
}

/// Writes `files` into `folder` (made if it isn't there), each safely
/// (a temporary file renamed over it), and with `godot` a Godot material
/// naming the colour, normal map and ORM; for an alpha card (`card`, the
/// material's [`CardMaps`]) cut out, two-sided and lit from behind by
/// the translucency; `crisp` (pixel art) read with no smoothing. The
/// files written, or why not.
pub fn write_set(
    folder: &Path,
    stem: &str,
    files: &[SetFile],
    godot: bool,
    card: CardMaps,
    crisp: bool,
) -> Result<Vec<PathBuf>, String> {
    std::fs::create_dir_all(folder)
        .map_err(|e| trf!("couldn't make the folder {}: {}", folder.display(), e))?;
    let mut written = Vec::new();
    for file in files {
        let path = folder.join(&file.file_name);
        crate::project::safe_write(&path, &file.bytes)
            .map_err(|e| trf!("couldn't write {}: {}", file.file_name, e))?;
        written.push(path);
    }
    if godot {
        let path_of = |map: TextureMap| {
            written
                .iter()
                .zip(files)
                .find(|(_, f)| f.map == map)
                .map(|(p, _)| crate::bake_file::godot_path(p))
        };
        let text = crate::bake_file::godot_card_material(
            path_of(TextureMap::Colour).as_deref(),
            path_of(TextureMap::Normal).as_deref(),
            path_of(TextureMap::Orm).as_deref(),
            crate::bake_file::GodotCard {
                cut_out: card.opacity,
                backlight: path_of(TextureMap::Translucency),
                crisp,
            },
        );
        let name = format!("{}.tres", file_safe(stem));
        let path = folder.join(&name);
        crate::project::safe_write(&path, text.as_bytes())
            .map_err(|e| trf!("couldn't write {}: {}", name, e))?;
        written.push(path);
    }
    Ok(written)
}
