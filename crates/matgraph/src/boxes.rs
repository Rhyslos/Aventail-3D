//! The app's own boxes.
//!
//! These first seven are enough to make a material from nothing and test
//! the whole engine: Colour fill, Noise, Tiles, Levels, Blend, Material
//! from height and Material out. The full first set (about 25) comes with
//! Phase 12's milestone 3; each new box is a file here, listed in
//! [`standard`].
//!
//! Every box's pattern repeats exactly across the texture (whole numbers
//! of cells, tiles and noise lattice squares), and boxes that look at
//! neighbours read through [`crate::Image::at`], which wraps round, so
//! everything tiles seamlessly.

mod blend;
pub mod blur;
mod colour_fill;
mod levels;
mod material_from_height;
mod material_out;
pub mod noise;
#[cfg(test)]
mod tests;
mod tiles;

use crate::registry::{BoxType, Label};
use crate::settings::{SettingKind, SettingSpec};
use std::borrow::Cow;
use std::sync::Arc;
use words::text;

pub use blend::BlendMode;

/// Every standard box, in the order listed within its group.
pub fn standard() -> Vec<Arc<dyn BoxType>> {
    vec![
        Arc::new(noise::Noise::new()),
        Arc::new(tiles::Tiles::new()),
        Arc::new(levels::Levels::new()),
        Arc::new(blend::Blend::new()),
        Arc::new(colour_fill::ColourFill::new()),
        Arc::new(material_from_height::MaterialFromHeight::new()),
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

/// A switch setting.
fn switch(key: &'static str, name: &'static str, about: &'static str) -> SettingSpec {
    SettingSpec {
        key: Cow::Borrowed(key),
        name: Label::Key(name),
        about: Label::Key(about),
        kind: SettingKind::Switch { standard: false },
    }
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
