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
use paint::{Blend, Image, Layer, PaintStore, TILE_BYTES, TextureSet};
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
    Some(PaintV64 { sets })
}

fn layer_of(layer: &Layer) -> LayerV64 {
    let tiles = layer
        .image()
        .map(|image| {
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
        })
        .unwrap_or_else(Bounded::new);
    LayerV64 {
        name: layer.name.clone(),
        visible: layer.visible,
        opacity: layer.opacity,
        blend: match layer.blend {
            Blend::Normal => 0,
        },
        tiles,
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
            let mut image = Image::new(set.size).ok_or_else(damaged)?;
            let mut seen = vec![false; image.tile_count()];
            for tile in layer.tiles {
                let index = tile.index as usize;
                if seen.get(index).is_none_or(|s| *s) {
                    return Err(damaged());
                }
                seen[index] = true;
                image.set_tile(index, Some(Arc::new(tile.pixels.0)));
            }
            let blend = match layer.blend {
                0 => Blend::Normal,
                _ => {
                    return Err(tr(
                        "it uses a way of mixing paint layers this version doesn't know (made \
                         by a newer version?)",
                    )
                    .into());
                }
            };
            out.layers.push(Layer {
                name: layer.name,
                visible: layer.visible,
                opacity: layer.opacity,
                blend,
                content: paint::Content::Paint(image),
            });
        }
        if !out.is_valid() {
            return Err(damaged());
        }
        paint.put(&set.material, Some(Arc::new(out)));
    }
    Ok(document.with_paint(paint))
}
