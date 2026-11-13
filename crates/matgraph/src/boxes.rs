//! The app's own boxes.
//!
//! Phase 12's milestone 1 made seven (Noise, Tiles, Levels, Blend, Colour
//! fill, Material from height and Material out); milestone 3 made the
//! first full set (decision 0259), in the groups of the experts' working
//! order:
//!
//! - **Make:** Noise, Clouds, Cells, Gradient, Bricks, Tiles, Planks,
//!   Basic shape; Image (milestone 21, decision 0284: a photo or a
//!   painting brought in).
//! - **From the model** (milestone 5, decision 0261): Curvature,
//!   Occlusion, Position, Thickness, Facing up: the model's own baked
//!   maps in the painter, a stand-in of rounded bumps in the Material
//!   mode ([`from_model`]).
//! - **Shape:** Blur, Warp, Slope blur (milestone 10), Levels, Edge
//!   finder, Bevel, Transform, Repeat, Blend; Make seamless, Guess height
//!   and Guess roughness (milestone 21: from a photo to a material).
//! - **Detail:** Scatter (milestone 9, decision 0271): copies of a
//!   shape, a colour or a material at random or in a grid; Cracks, Chips,
//!   Scratches and Fill the gaps (milestone 10, decision 0272); Stamps
//!   (milestone 22, decision 0285: small pictures of your own); Painted
//!   (milestone 20, decision 0282): painted by hand with the painter's
//!   brushes.
//! - **Weathering** (milestone 25, decision 0288): Dirt, Rust, Edge
//!   wear, Dust and Water streaks, which age a whole material and follow
//!   the model in the painter (the `weathering` module).
//! - **Colour:** Colour fill, Gradient map, Hue and saturation, Colour to
//!   shape, Palette map (milestone 19, decision 0281: the project's
//!   palettes); Brush strokes, Edge highlight and Directional light
//!   (milestone 10); Toon bands, Painted highlight and Colour steps
//!   (milestone 23, decision 0286: stylised light for a cel or
//!   hand-painted look); Even out light (milestone 21).
//! - **Moving** (milestone 15, decision 0277): Time, Scroll, Pulse and
//!   Flow, which change over time ([`crate::Job::time`]).
//! - **Finish:** Material from height, Mix materials, Trim sheet
//!   (milestone 26, decision 0289: strips of materials on one texture, on
//!   the project's trim rows), Opacity (milestone 14, decision 0276: alpha
//!   cards), Pixelate (milestone 24, decision 0287: pixel art, a few big
//!   pixels limited to a palette), Material out.
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
mod brush_strokes;
mod cells;
mod chips;
mod clouds;
mod colour_fill;
mod colour_steps;
mod colour_to_shape;
mod cracks;
mod directional_light;
pub mod drawn_shape;
mod edge_highlight;
mod edges;
pub mod even_light;
mod fill_gaps;
mod flow;
pub mod from_model;
mod gradient;
mod gradient_map;
pub mod guess_height;
pub mod guess_roughness;
mod hue_saturation;
pub mod image_box;
mod levels;
mod material_from_height;
mod material_out;
mod mix_materials;
mod motion;
mod moving;
pub mod noise;
pub mod opacity;
pub mod painted;
mod painted_highlight;
mod palette_map;
mod pattern;
mod photo_maths;
pub mod pixelate;
mod planks;
mod pulse;
mod repeat;
mod scatter;
mod scratches;
mod scroll;
pub mod seamless;
mod slope_blur;
pub mod stamps;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_each;
#[cfg(test)]
mod tests_more;
#[cfg(test)]
mod tests_moving;
#[cfg(test)]
mod tests_pixel;
#[cfg(test)]
mod tests_stylised;
mod tiles;
mod time;
mod toon_bands;
mod transform;
pub mod trim_sheet;
mod warp;
mod weathering;

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
        // Make (milestone 11): drawn on the shape canvas.
        Arc::new(drawn_shape::DrawnShape::new()),
        // Make (milestone 21): a photo or painting brought in.
        Arc::new(image_box::ImageBox::new()),
        // From the model (milestone 5).
        Arc::new(from_model::FromModel::curvature()),
        Arc::new(from_model::FromModel::occlusion()),
        Arc::new(from_model::FromModel::position()),
        Arc::new(from_model::FromModel::thickness()),
        Arc::new(from_model::FromModel::facing_up()),
        // Shape.
        Arc::new(blur_box::Blur::new()),
        Arc::new(warp::Warp::new()),
        Arc::new(slope_blur::SlopeBlur::new()),
        Arc::new(levels::Levels::new()),
        Arc::new(edges::Edges::new()),
        Arc::new(bevel::Bevel::new()),
        Arc::new(transform::Transform::new()),
        Arc::new(repeat::Repeat::new()),
        Arc::new(blend::Blend::new()),
        // Shape (milestone 21): from a photo to a material.
        Arc::new(seamless::Seamless::new()),
        Arc::new(guess_height::GuessHeight::new()),
        Arc::new(guess_roughness::GuessRoughness::new()),
        // Detail (milestone 9).
        Arc::new(scatter::Scatter::new()),
        // Detail (milestone 22): small pictures of your own as stamps.
        Arc::new(stamps::StampsBox::new()),
        // Detail (milestone 10).
        Arc::new(cracks::Cracks::new()),
        Arc::new(chips::Chips::new()),
        Arc::new(scratches::Scratches::new()),
        Arc::new(fill_gaps::FillGaps::new()),
        // Detail (milestone 20): painted by hand.
        Arc::new(painted::Painted::new()),
        // Weathering (milestone 25): ageing that follows the model.
        Arc::new(weathering::Dirt::new()),
        Arc::new(weathering::Rust::new()),
        Arc::new(weathering::EdgeWear::new()),
        Arc::new(weathering::Dust::new()),
        Arc::new(weathering::WaterStreaks::new()),
        // Colour.
        Arc::new(colour_fill::ColourFill::new()),
        Arc::new(gradient_map::GradientMap::new()),
        Arc::new(hue_saturation::HueSaturation::new()),
        Arc::new(colour_to_shape::ColourToShape::new()),
        // Colour (milestone 19): the project's palettes.
        Arc::new(palette_map::PaletteMap::new()),
        // Colour (milestone 10).
        Arc::new(brush_strokes::BrushStrokes::new()),
        Arc::new(edge_highlight::EdgeHighlight::new()),
        Arc::new(directional_light::DirectionalLight::new()),
        // Colour (milestone 23): stylised light, for a cel or painted look.
        Arc::new(toon_bands::ToonBands::new()),
        Arc::new(painted_highlight::PaintedHighlight::new()),
        Arc::new(colour_steps::ColourSteps::new()),
        // Colour (milestone 21): light baked into a photo taken out.
        Arc::new(even_light::EvenLight::new()),
        // Moving (milestone 15).
        Arc::new(time::Time::new()),
        Arc::new(scroll::Scroll::new()),
        Arc::new(pulse::Pulse::new()),
        Arc::new(flow::Flow::new()),
        // Finish.
        Arc::new(material_from_height::MaterialFromHeight::new()),
        Arc::new(mix_materials::MixMaterials::new()),
        // Finish (milestone 26): strips of materials on one texture.
        Arc::new(trim_sheet::TrimSheet::new()),
        // Finish (milestone 14): alpha cards.
        Arc::new(opacity::Opacity::new()),
        // Finish (milestone 24): pixel art.
        Arc::new(pixelate::Pixelate::new()),
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
        animated: false,
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
