//! Guess height (Phase 12 milestone 21, [decision 0283](../../../../docs/decisions/0283-image-to-material.md)):
//! a height guessed from a picture's lightness.
//!
//! In most photos of stone, bark, ground or fabric, what sticks out
//! catches the light and the gaps between are in shade: light is high,
//! dark is low. The box takes the picture's lightness, keeps only the
//! changes smaller than **Detail size** (the picture's lightness less a
//! blur of it over that size, so a darker half or a shadow across the
//! photo isn't taken for a slope), and stretches the result so its
//! darkest and lightest few hundredths reach black and white.
//! **Invert** turns it over for pictures where dark is high (dark knots
//! on pale wood, dark paint on a light wall). **Big shapes** adds back
//! some of the plain lightness, for pictures whose large parts (whole
//! stones) should stand up as a whole.
//!
//! It is a guess: no picture says for sure how deep anything is. The
//! blur wraps round, so a seamless picture gives a seamless height.

use super::pattern::soft_blur;
use super::photo_maths::{lightness_of, stretched};
use super::{info, number, switch};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::value::Value;
use std::sync::Arc;
use words::text;

/// The box's id.
pub const GUESS_HEIGHT: &str = "guess_height";

/// The Guess height box.
#[derive(Debug)]
pub(crate) struct GuessHeight {
    info: BoxInfo,
}

impl GuessHeight {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                GUESS_HEIGHT,
                text!("Guess height"),
                Group::Shape,
                text!(
                    "Guesses a height from a picture: light parts are taken as high and dark \
                     ones as low, as on most stone, bark or ground, where what sticks out \
                     catches the light. Only changes smaller than Detail size count, so a \
                     shadow across the photo isn't taken for a slope. Invert it where dark \
                     means high. It is a guess: plug it into Material from height and look \
                     at the preview."
                ),
                (
                    vec![PlugSpec::new(
                        "colour",
                        text!("Colour"),
                        ValueKind::Colour,
                        true,
                    )],
                    vec![PlugSpec::new(
                        "out",
                        text!("Height"),
                        ValueKind::Height,
                        false,
                    )],
                ),
                vec![
                    number(
                        "detail",
                        text!("Detail size"),
                        text!(
                            "The biggest bumps to find, as a share of the texture's width: \
                             about the size of one stone, plank or tuft. Changes of light \
                             bigger than this are left out."
                        ),
                        0.01,
                        0.5,
                        0.1,
                    ),
                    number(
                        "shapes",
                        text!("Big shapes"),
                        text!(
                            "How much of the plain lightness is added back, so large light \
                             parts (whole stones) stand up as a whole: 0 none, 1 as much as \
                             the detail."
                        ),
                        0.0,
                        1.0,
                        0.25,
                    ),
                    number(
                        "smooth",
                        text!("Smoothing"),
                        text!(
                            "Softens the height a little, so speckles and grain in the photo \
                             don't come out as sharp spikes: 0 none, 1 a soft blur."
                        ),
                        0.0,
                        1.0,
                        0.3,
                    ),
                    switch(
                        "invert",
                        text!("Invert"),
                        text!("Dark is high instead of low."),
                    ),
                ],
                &["material_from_height", "levels", "blur"],
            ),
        }
    }
}

impl BoxType for GuessHeight {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let colour = job.needed_colour("colour")?.clone();
        let threads = job.threads();
        let n = job.size().get() as f32;
        let light = lightness_of(&colour, threads);
        let detail = job.number("detail").clamp(0.01, 0.5) * n * 0.5;
        let big = job.number("shapes").clamp(0.0, 1.0);
        let invert = job.switch("invert");
        // Smoothing up to about one 256th of the texture.
        let smooth = job.number("smooth").clamp(0.0, 1.0) * n / 256.0 * 2.0;
        let soft = soft_blur(&light, smooth, threads);
        let wide = soft_blur(&soft, detail, threads);
        let raw = job.fill_grey(|x, y| {
            let (x, y) = (i64::from(x), i64::from(y));
            let here = soft.at(x, y);
            let h = (here - wide.at(x, y)) + big * (here - 0.5) * 0.5;
            if invert { -h } else { h }
        });
        Ok(vec![Value::Height(Arc::new(stretched(&raw, threads)))])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxes::tests_each::{registry, with};
    use crate::image::{Image, Rgb, Size};
    use crate::settings::{SettingValue, Settings};

    fn run(colour: Rgb, settings: &Settings) -> crate::Grey {
        let info = registry().get(GUESS_HEIGHT).unwrap().info();
        let size = Size::new(colour.size()).unwrap();
        let job = Job::new(
            info,
            size,
            1,
            2,
            settings,
            vec![Some(Value::Colour(Arc::new(colour)))],
        );
        let out = registry().get(GUESS_HEIGHT).unwrap().run(&job).unwrap();
        out[0].grey().unwrap().as_ref().clone()
    }

    /// Light stones on dark gaps: a grid of squares, with a shadow
    /// across the whole picture from left to right.
    fn stones(n: u32) -> Rgb {
        crate::parallel::fill(Size::new(n).unwrap(), 1, |x, y| {
            let stone = x % 16 > 2 && y % 16 > 2;
            let shade = 0.6 + 0.4 * x as f32 / n as f32;
            let v = if stone { 0.8 } else { 0.2 } * shade;
            [v, v * 0.9, v * 0.8]
        })
    }

    #[test]
    fn light_stones_stand_up_and_the_shadow_is_ignored() {
        let h = run(stones(128), &Settings::new());
        assert!(h.pixels().iter().all(|v| (0.0..=1.0).contains(v)));
        // The full range is used.
        let lo = h.pixels().iter().copied().fold(1.0, f32::min);
        let hi = h.pixels().iter().copied().fold(0.0, f32::max);
        assert!(lo < 0.05 && hi > 0.95, "{lo} {hi}");
        // A stone is higher than a gap, in the shade as in the light.
        for x0 in [0, 112] {
            assert!(h.at(x0 + 9, 9) > h.at(x0 + 1, 9) + 0.3, "{x0}");
        }
        // The stones in the shade come out about as high as those in the
        // light (the shadow isn't a slope).
        assert!((h.at(9, 9) - h.at(121, 9)).abs() < 0.2);
    }

    #[test]
    fn invert_turns_it_over() {
        let a = run(stones(64), &Settings::new());
        let b = run(stones(64), &with(&[("invert", SettingValue::Switch(true))]));
        assert!(a.at(9, 9) > a.at(1, 9));
        assert!(b.at(9, 9) < b.at(1, 9));
    }

    #[test]
    fn a_flat_picture_gives_a_flat_height() {
        let flat: Rgb = Image::filled(Size::new(32).unwrap(), [0.3, 0.4, 0.5]);
        let h = run(flat, &Settings::new());
        assert!(h.pixels().iter().all(|v| *v == 0.5));
    }
}
