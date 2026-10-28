//! Preset files ([decision 0206](../../../docs/decisions/0206-presets-and-starting-layouts.md)):
//! brushes, a style (one paint layer or group, as a recipe) or a material
//! (its colour and all its layers, as a recipe) kept in a small file to
//! use in other projects or share.
//!
//! A preset is written as readable JSON. Paint layers and painted masks
//! keep their settings but not their pixels: the pixels belong to one
//! model's UVs, while fills, gradients, light and shade, and smart masks
//! are made again on whatever model the preset is used on.

use crate::limits::Bounded;
use crate::project::v64::{LayerV64, layer_from, layer_of};
use paint::Layer;
use serde::{Deserialize, Serialize};
use words::tr;

/// The extension of preset files, without the dot. Generic on purpose:
/// it doesn't name the app, so it survives a rename (RENAMING.md).
pub const PRESET_EXTENSION: &str = "paintpreset";

/// The largest preset file read, in bytes.
pub const MAX_PRESET_FILE: u64 = 4 << 20;

/// The version of the preset format written. Newer files are refused
/// with a message rather than read wrongly.
pub const PRESET_FORMAT: u32 = 1;

/// The most brushes one file holds.
pub const MAX_BRUSHES: usize = 200;

/// What a preset file holds, as the program uses it.
#[derive(Debug, Clone, PartialEq)]
pub enum Preset {
    /// Brushes, as the brush bar keeps them.
    Brushes(Vec<BrushRecipe>),
    /// One layer or group, made at the size asked for.
    Style {
        /// The style's name.
        name: String,
        /// The layer.
        layer: Box<Layer>,
    },
    /// A material's colour and its layers, bottom first.
    Material {
        /// The material's name when it was saved (for the file's name).
        name: String,
        /// Its colour (red, green, blue, 0 to 1).
        colour: [f32; 3],
        /// Its layers, bottom first.
        layers: Vec<Layer>,
    },
}

/// One brush in a preset file: the same numbers the brush bar keeps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrushRecipe {
    /// Its name.
    pub name: String,
    /// Its tip, by number (`paint::tip::Tip::index`).
    pub tip: u8,
    /// Hardness, 0 to 1.
    pub hardness: f32,
    /// Spacing, as a share of its width.
    pub spacing: f32,
    /// Flow, 0 to 1.
    pub flow: f32,
    /// Whether pressure changes the size, and its least share.
    #[serde(default)]
    pub size_pressure: Option<f32>,
    /// Whether pressure changes the flow, and its least share.
    #[serde(default)]
    pub flow_pressure: Option<f32>,
    /// What it does, by number (`paint::BrushKind::index`).
    #[serde(default)]
    pub kind: u8,
}

/// The file as stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct PresetFile {
    /// [`PRESET_FORMAT`] when written.
    format: u32,
    /// What it holds.
    #[serde(flatten)]
    content: Stored,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Stored {
    Brushes {
        brushes: Bounded<BrushRecipe>,
    },
    Style {
        name: String,
        layer: Box<LayerV64>,
    },
    Material {
        name: String,
        colour: [f32; 3],
        layers: Bounded<LayerV64>,
    },
}

/// `layer` as a recipe: its settings, without painted pixels or the
/// strokes that made them.
fn recipe_of(layer: &Layer) -> LayerV64 {
    let mut stored = layer_of(layer);
    strip(&mut stored);
    stored
}

fn strip(layer: &mut LayerV64) {
    layer.tiles = Bounded::new();
    layer.strokes = Bounded::new();
    if let Some(mask) = &mut layer.mask {
        mask.tiles = Bounded::new();
        mask.strokes = Bounded::new();
    }
    if let Some(channels) = &mut layer.channels {
        for part in [
            &mut channels.roughness,
            &mut channels.metal,
            &mut channels.glow,
            &mut channels.height,
        ]
        .into_iter()
        .flatten()
        {
            part.tiles = Bounded::new();
        }
    }
    if let Some(children) = &mut layer.children {
        children.0.iter_mut().for_each(strip);
    }
}

/// The text of a preset file holding `preset`.
pub fn write_preset(preset: &Preset) -> String {
    let content = match preset {
        Preset::Brushes(brushes) => Stored::Brushes {
            brushes: Bounded(brushes.clone()),
        },
        Preset::Style { name, layer } => Stored::Style {
            name: name.clone(),
            layer: Box::new(recipe_of(layer)),
        },
        Preset::Material {
            name,
            colour,
            layers,
        } => Stored::Material {
            name: name.clone(),
            colour: *colour,
            layers: Bounded(layers.iter().map(recipe_of).collect()),
        },
    };
    let file = PresetFile {
        format: PRESET_FORMAT,
        content,
    };
    serde_json::to_string_pretty(&file).unwrap_or_default()
}

/// The preset in `bytes`, with its layers made `size` pixels across, or
/// why it can't be read.
pub fn read_preset(bytes: &[u8], size: u32) -> Result<Preset, String> {
    let unreadable = || tr("it isn't a preset file, or it is damaged").to_string();
    if bytes.len() as u64 > MAX_PRESET_FILE {
        return Err(tr("it is too large for a preset").to_string());
    }
    if paint::TextureSet::new(size).is_none() {
        return Err(unreadable());
    }
    // The format number first, so a newer file says so.
    #[derive(Deserialize)]
    struct Version {
        format: u32,
    }
    let version: Version = serde_json::from_slice(bytes).map_err(|_| unreadable())?;
    if version.format > PRESET_FORMAT {
        return Err(tr("it was made by a newer version of the app").to_string());
    }
    let file: PresetFile =
        crate::limits::within_budget(|| serde_json::from_slice(bytes)).map_err(|_| {
            if crate::limits::limit_hit() {
                tr(crate::limits::TOO_MUCH).to_string()
            } else {
                unreadable()
            }
        })?;
    let layer = |stored: LayerV64| -> Result<Layer, String> {
        let layer = layer_from(stored, size, 0)?;
        if layer.is_valid() && layer.depth() < paint::set::MAX_DEPTH {
            Ok(layer)
        } else {
            Err(unreadable())
        }
    };
    let name_ok = |name: &str| name.chars().count() <= paint::set::MAX_NAME;
    match file.content {
        Stored::Brushes { brushes } => {
            if brushes.0.len() > MAX_BRUSHES || brushes.0.iter().any(|b| !name_ok(&b.name)) {
                return Err(unreadable());
            }
            Ok(Preset::Brushes(brushes.0))
        }
        Stored::Style {
            name,
            layer: stored,
        } => {
            if !name_ok(&name) {
                return Err(unreadable());
            }
            let layer = layer(*stored)?;
            if layer.count() > paint::set::MAX_LAYERS {
                return Err(unreadable());
            }
            Ok(Preset::Style {
                name,
                layer: Box::new(layer),
            })
        }
        Stored::Material {
            name,
            colour,
            layers: stored,
        } => {
            if !name_ok(&name) || !colour.iter().all(|c| (0.0..=1.0).contains(c)) {
                return Err(unreadable());
            }
            let layers = stored
                .0
                .into_iter()
                .map(layer)
                .collect::<Result<Vec<_>, _>>()?;
            let set = paint::TextureSet {
                size,
                layers: layers.clone(),
                realism: paint::channel::any_channels(&layers),
            };
            if !set.is_valid() {
                return Err(unreadable());
            }
            Ok(Preset::Material {
                name,
                colour,
                layers,
            })
        }
    }
}

#[cfg(test)]
#[path = "preset_tests.rs"]
mod tests;
