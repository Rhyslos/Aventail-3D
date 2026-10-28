//! Version 64 of the project file ([decision 0177](../../../../docs/decisions/0177-texture-painter-plan.md)):
//! version 63's, with the project's texture sets (`v43::ProjectV43::paint`,
//! stored as [`PaintV64`]): each material's painted layers, tile by tile,
//! leaving out tiles nobody painted. Files before version 64 hold none.
//!
//! An older app would drop the painting without a word on saving, so it
//! gets a version of its own: the older app says it needs updating
//! instead.

use super::v1::Problem;
use crate::limits::Bounded;
use paint::{Blend, Content, Image, Layer, PaintStore, TILE_BYTES, TextureSet};
use scene::Document;
use serde::de::{SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::sync::Arc;
use words::tr;

/// The texture sets as stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct PaintV64 {
    /// Each material's texture set.
    pub(crate) sets: Bounded<SetV64>,
    /// The project's colour palettes (version 68).
    #[serde(default)]
    pub(crate) palettes: Option<Bounded<super::v68::PaletteV68>>,
}

/// One material's texture set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct SetV64 {
    /// The material's name.
    pub(crate) material: String,
    /// Pixels across its textures.
    pub(crate) size: u32,
    /// Its layers, bottom first.
    pub(crate) layers: Bounded<LayerV64>,
}

/// One layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct LayerV64 {
    /// Its name.
    pub(crate) name: String,
    /// Whether it is mixed in.
    pub(crate) visible: bool,
    /// How much of it shows, 0 to 1.
    pub(crate) opacity: f32,
    /// How it mixes: 0 normal.
    pub(crate) blend: u8,
    /// Its painted tiles.
    pub(crate) tiles: Bounded<TileV64>,
    /// The strokes painted on it (version 65).
    #[serde(default = "Bounded::new")]
    pub(crate) strokes: Bounded<super::v65::StrokeV65>,
    /// Whether it is clipped to the layer below (version 66).
    #[serde(default)]
    pub(crate) clipped: bool,
    /// For a group, whether its layers are listed (version 66).
    #[serde(default)]
    pub(crate) expanded: bool,
    /// A fill layer's colour (version 66): then `tiles` is empty.
    #[serde(default)]
    pub(crate) fill: Option<[u8; 4]>,
    /// A group's layers, bottom first (version 66): then `tiles` is empty.
    #[serde(default)]
    pub(crate) children: Option<Bounded<LayerV64>>,
    /// Its mask (version 67).
    #[serde(default)]
    pub(crate) mask: Option<super::v67::MaskV67>,
    /// A gradient fill's settings (version 72): its tiles are made from
    /// them.
    #[serde(default)]
    pub(crate) gradient: Option<super::v72::GradientV72>,
    /// A light-and-shade layer's settings (version 74): its tiles are
    /// made from them.
    #[serde(default)]
    pub(crate) lighting: Option<super::v74::LightingV74>,
}

/// One painted tile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct TileV64 {
    /// Which tile, row by row from the top left.
    pub(crate) index: u32,
    /// Its pixels.
    pub(crate) pixels: TileBytes,
}

/// A tile's pixels, saved as one block of bytes (not a list of numbers)
/// and read only if it is exactly a tile's size.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TileBytes(pub(crate) Vec<u8>);

impl Serialize for TileBytes {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for TileBytes {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Pixels;

        impl<'de> Visitor<'de> for Pixels {
            type Value = TileBytes;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(tr("a tile's pixels"))
            }

            fn visit_bytes<E: serde::de::Error>(self, bytes: &[u8]) -> Result<TileBytes, E> {
                self.visit_byte_buf(bytes.to_vec())
            }

            fn visit_byte_buf<E: serde::de::Error>(self, bytes: Vec<u8>) -> Result<TileBytes, E> {
                if bytes.len() != TILE_BYTES {
                    return Err(E::custom(tr("a painted tile is the wrong size")));
                }
                if !crate::limits::spend(TILE_BYTES) {
                    return Err(E::custom(tr(crate::limits::TOO_MUCH)));
                }
                Ok(TileBytes(bytes))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<TileBytes, A::Error> {
                // Formats without byte blocks save the pixels as a list.
                let mut bytes = Vec::with_capacity(TILE_BYTES);
                while let Some(byte) = seq.next_element::<u8>()? {
                    if bytes.len() == TILE_BYTES {
                        return Err(serde::de::Error::custom(tr(
                            "a painted tile is the wrong size",
                        )));
                    }
                    bytes.push(byte);
                }
                self.visit_byte_buf(bytes)
            }
        }

        deserializer.deserialize_byte_buf(Pixels)
    }
}

/// The texture sets to store for `document`: `None` when there are none.
pub(crate) fn paint_of(document: &Document) -> Option<PaintV64> {
    let paint = document.scene().paint();
    if paint.is_empty() {
        return None;
    }
    let sets = paint
        .sets()
        .map(|(material, set)| SetV64 {
            material: material.to_string(),
            size: set.size,
            layers: set.layers.iter().map(layer_of).collect(),
        })
        .collect();
    Some(PaintV64 {
        sets,
        palettes: super::v68::palettes_of(paint.palettes()),
    })
}

/// An image's painted tiles as stored.
pub(crate) fn tiles_of(image: &Image) -> Bounded<TileV64> {
    image
        .tiles()
        .iter()
        .enumerate()
        .filter_map(|(index, tile)| {
            Some(TileV64 {
                index: index as u32,
                pixels: TileBytes(tile.as_ref()?.to_vec()),
            })
        })
        .collect()
}

/// An image `size` pixels across from stored tiles, checked: a tile out
/// of place or twice is damage.
pub(crate) fn image_from(tiles: Bounded<TileV64>, size: u32) -> Result<Image, Problem> {
    let damaged = || -> Problem { tr("its painted textures are damaged").into() };
    let mut image = Image::new(size).ok_or_else(damaged)?;
    let mut seen = vec![false; image.tile_count()];
    for tile in tiles {
        let index = tile.index as usize;
        if seen.get(index).is_none_or(|s| *s) {
            return Err(damaged());
        }
        seen[index] = true;
        image.set_tile(index, Some(Arc::new(tile.pixels.0)));
    }
    Ok(image)
}

pub(crate) fn layer_of(layer: &Layer) -> LayerV64 {
    let tiles = layer.image().map(tiles_of).unwrap_or_else(Bounded::new);
    LayerV64 {
        name: layer.name.clone(),
        visible: layer.visible,
        opacity: layer.opacity,
        blend: layer.blend.index(),
        tiles,
        strokes: super::v65::strokes_of(&layer.strokes),
        clipped: layer.clipped,
        expanded: layer.expanded,
        fill: match &layer.content {
            Content::Fill(color) => Some(*color),
            _ => None,
        },
        children: layer
            .children()
            .map(|children| children.iter().map(layer_of).collect()),
        mask: layer.mask.as_ref().map(super::v67::mask_of),
        gradient: layer.gradient.as_ref().map(super::v72::gradient_of),
        lighting: layer.lighting.as_ref().map(super::v74::lighting_of),
    }
}

/// `document` with the stored texture sets, checked: a set with an
/// unknown blend, a tile out of place or twice, a size or layer out of
/// range, or a material twice, is refused.
pub(crate) fn with_paint(
    document: Document,
    stored: Option<PaintV64>,
) -> Result<Document, Problem> {
    let Some(stored) = stored else {
        return Ok(document);
    };
    let damaged = || -> Problem { tr("its painted textures are damaged").into() };
    let mut paint = PaintStore::default();
    for set in stored.sets {
        if set.material.trim().is_empty() || paint.set(&set.material).is_some() {
            return Err(damaged());
        }
        let mut out = TextureSet::new(set.size).ok_or_else(damaged)?;
        for layer in set.layers {
            out.layers.push(layer_from(layer, set.size, 0)?);
        }
        if !out.is_valid() {
            return Err(damaged());
        }
        paint.put(&set.material, Some(Arc::new(out)));
    }
    if let Some(palettes) = stored.palettes {
        paint.set_palettes(super::v68::palettes_from(palettes)?);
    }
    Ok(document.with_paint(paint))
}

/// A stored layer, checked: tiles out of place or twice, a fill or group
/// with tiles, groups nested too deep, or an unknown blend mode, are
/// refused.
pub(crate) fn layer_from(layer: LayerV64, size: u32, depth: usize) -> Result<Layer, Problem> {
    let damaged = || -> Problem { tr("its painted textures are damaged").into() };
    let blend = Blend::from_index(layer.blend).ok_or_else(|| -> Problem {
        tr(
            "it uses a way of mixing paint layers this version doesn't know (made by a newer \
             version?)",
        )
        .into()
    })?;
    let content = match (layer.fill, layer.children) {
        (Some(_), Some(_)) => return Err(damaged()),
        (Some(color), None) => {
            if !layer.tiles.is_empty() {
                return Err(damaged());
            }
            Content::Fill(color)
        }
        (None, Some(children)) => {
            if !layer.tiles.is_empty() || depth + 1 >= paint::set::MAX_DEPTH {
                return Err(damaged());
            }
            let mut out = Vec::new();
            for child in children {
                out.push(layer_from(child, size, depth + 1)?);
            }
            Content::Group(out)
        }
        (None, None) => {
            let image = image_from(layer.tiles, size)?;
            Content::Paint(image)
        }
    };
    Ok(Layer {
        name: layer.name,
        visible: layer.visible,
        opacity: layer.opacity,
        blend,
        clipped: layer.clipped,
        expanded: layer.expanded,
        content,
        mask: layer
            .mask
            .map(|m| super::v67::mask_from(m, size))
            .transpose()?,
        strokes: super::v65::strokes_from(layer.strokes)?,
        gradient: layer.gradient.map(super::v72::gradient_from).transpose()?,
        lighting: layer.lighting.map(super::v74::lighting_from).transpose()?,
    })
}
