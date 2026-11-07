//! The app's own boxes.
//!
//! Phase 12's milestone 1 made seven (Noise, Tiles, Levels, Blend, Colour
//! fill, Material from height and Material out); milestone 3 made the
//! first full set (decision 0259), in the groups of the experts' working
//! order:
//!
//! - **Make:** Noise, Clouds, Cells, Gradient, Bricks, Tiles, Planks,
//!   Basic shape.
//! - **From the model** (milestone 5, decision 0261): Curvature,
//!   Occlusion, Position, Thickness, Facing up: the model's own baked
//!   maps in the painter, a stand-in of rounded bumps in the Material
//!   mode ([`from_model`]).
//! - **Shape:** Blur, Warp, Levels, Edge finder, Bevel, Transform, Repeat,
//!   Blend.
//! - **Colour:** Colour fill, Gradient map, Hue and saturation, Colour to
//!   shape.
//! - **Finish:** Material from height, Mix materials, Material out.
//!
//! Each box is a file here, listed in [`standard`]. Box, plug and setting
//! ids are saved in files: never change them once released (a box may
//! gain plugs and settings). Settings that are sizes are lengths in
//! centimetres, turned into pixels with the graph's real-world size.
//!
//! Every box's pattern repeats exactly across the texture (whole numbers
//! of cells, tiles and noise lattice squares), and boxes that look at
//! neighbours read through [`crate::Image::at`], which wraps round, so
//! everything tiles seamlessly.

mod basic_shape;
mod bevel;
mod blend;
pub mod blur;
mod blur_box;
mod bricks;
mod cells;
mod clouds;
mod colour_fill;
mod colour_to_shape;
mod edges;
pub mod from_model;
mod gradient;
mod gradient_map;
mod hue_saturation;
mod levels;
mod material_from_height;
mod material_out;
mod mix_materials;
pub mod noise;
mod pattern;
mod planks;
mod repeat;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_each;
mod tiles;
mod transform;
mod warp;

use crate::registry::{BoxInfo, BoxType, Group, Label, PlugSpec};
use crate::settings::{SettingKind, SettingSpec};
use std::borrow::Cow;
use std::sync::Arc;
use words::text;

pub use blend::BlendMode;

/// Every standard box, in the order listed within its group.
pub fn standard() -> Vec<Arc<dyn BoxType>> {
    vec![
        // Make.
        Arc::new(noise::Noise::new()),
        Arc::new(clouds::Clouds::new()),
        Arc::new(cells::Cells::new()),
        Arc::new(gradient::Gradient::new()),
        Arc::new(bricks::Bricks::new()),
        Arc::new(tiles::Tiles::new()),
        Arc::new(planks::Planks::new()),
        Arc::new(basic_shape::BasicShape::new()),
        // From the model (milestone 5).
        Arc::new(from_model::FromModel::curvature()),
        Arc::new(from_model::FromModel::occlusion()),
        Arc::new(from_model::FromModel::position()),
        Arc::new(from_model::FromModel::thickness()),
        Arc::new(from_model::FromModel::facing_up()),
        // Shape.
        Arc::new(blur_box::Blur::new()),
        Arc::new(warp::Warp::new()),
        Arc::new(levels::Levels::new()),
        Arc::new(edges::Edges::new()),
        Arc::new(bevel::Bevel::new()),
        Arc::new(transform::Transform::new()),
        Arc::new(repeat::Repeat::new()),
        Arc::new(blend::Blend::new()),
        // Colour.
        Arc::new(colour_fill::ColourFill::new()),
        Arc::new(gradient_map::GradientMap::new()),
        Arc::new(hue_saturation::HueSaturation::new()),
        Arc::new(colour_to_shape::ColourToShape::new()),
        // Finish.
        Arc::new(material_from_height::MaterialFromHeight::new()),
        Arc::new(mix_materials::MixMaterials::new()),
        Arc::new(material_out::MaterialOut::new()),
    ]
}

/// A number setting.
fn number(
    key: &'static str,
    name: &'static str,
    about: &'static str,
    min: f32,
    max: f32,
    standard: f32,
) -> SettingSpec {
    SettingSpec {
        key: Cow::Borrowed(key),
        name: Label::Key(name),
        about: Label::Key(about),
        kind: SettingKind::Number { min, max, standard },
    }
}

/// A length setting, in centimetres (turned into pixels with the
/// graph's real-world size: [`crate::Job::length_px`]).
fn length(
    key: &'static str,
    name: &'static str,
    about: &'static str,
    min: f32,
    max: f32,
    standard: f32,
) -> SettingSpec {
    SettingSpec {
        key: Cow::Borrowed(key),
        name: Label::Key(name),
        about: Label::Key(about),
        kind: SettingKind::Length { min, max, standard },
    }
}

/// A choice setting: `options` are English keys, the first the standard
/// unless `standard` says otherwise.
fn choice(
    key: &'static str,
    name: &'static str,
    about: &'static str,
    options: &[&'static str],
    standard: u32,
) -> SettingSpec {
    SettingSpec {
        key: Cow::Borrowed(key),
        name: Label::Key(name),
        about: Label::Key(about),
        kind: SettingKind::Choice {
            options: options.iter().map(|o| Label::Key(o)).collect(),
            standard,
        },
    }
}

/// A whole-number setting.
fn whole(
    key: &'static str,
    name: &'static str,
    about: &'static str,
    min: i32,
    max: i32,
    standard: i32,
) -> SettingSpec {
    SettingSpec {
        key: Cow::Borrowed(key),
        name: Label::Key(name),
        about: Label::Key(about),
        kind: SettingKind::Whole { min, max, standard },
    }
}

/// A colour setting.
fn colour(
    key: &'static str,
    name: &'static str,
    about: &'static str,
    standard: [f32; 3],
) -> SettingSpec {
    SettingSpec {
        key: Cow::Borrowed(key),
        name: Label::Key(name),
        about: Label::Key(about),
        kind: SettingKind::Colour { standard },
    }
}

/// A switch setting, off to start with.
fn switch(key: &'static str, name: &'static str, about: &'static str) -> SettingSpec {
    SettingSpec {
        key: Cow::Borrowed(key),
        name: Label::Key(name),
        about: Label::Key(about),
        kind: SettingKind::Switch { standard: false },
    }
}

/// A kind of box's [`BoxInfo`] (one of many allowed per graph), from its
/// plain words, plugs (inputs, outputs), settings and what usually comes
/// next.
fn info(
    id: &'static str,
    name: &'static str,
    group: Group,
    about: &'static str,
    plugs: (Vec<PlugSpec>, Vec<PlugSpec>),
    settings: Vec<SettingSpec>,
    comes_next: &[&'static str],
) -> BoxInfo {
    BoxInfo {
        id: Cow::Borrowed(id),
        name: Label::Key(name),
        group,
        about: Label::Key(about),
        inputs: plugs.0,
        outputs: plugs.1,
        settings,
        comes_next: next(comes_next),
        one_per_graph: false,
    }
}

/// The place in the texture of pixel `(x, y)`'s middle, 0 to 1 each way.
fn uv(x: u32, y: u32, n: f32) -> (f32, f32) {
    ((x as f32 + 0.5) / n, (y as f32 + 0.5) / n)
}

/// A box's seed setting, always called Seed.
fn seed() -> SettingSpec {
    SettingSpec {
        key: Cow::Borrowed("seed"),
        name: Label::Key(text!("Seed")),
        about: Label::Key(text!(
            "Another number gives another pattern of the same look. It is mixed with the \
             graph's own seed."
        )),
        kind: SettingKind::Seed,
    }
}

/// Box ids, as listed in "usually comes next".
fn next(ids: &[&'static str]) -> Vec<Cow<'static, str>> {
    ids.iter().map(|id| Cow::Borrowed(*id)).collect()
}
