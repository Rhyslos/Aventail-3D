//! Guess roughness (Phase 12 milestone 21, [decision 0283](../../../../docs/decisions/0283-image-to-material.md)):
//! a roughness guessed from how busy a picture is.
//!
//! A smooth, even part of a photo (polished stone, a worn step, wet mud)
//! is usually the shiny part; a busy, grainy one (rough rock, bark, dry
//! earth) scatters light and is matt. The box measures how much the
//! lightness changes within **Detail size** of each pixel (its spread
//! about the local average), stretches that so the calmest and busiest
//! parts reach the ends, softens it, and lays it between **Shiniest** and
//! **Roughest**. **Invert** gives the busy parts the shine instead. The
//! blurs wrap round, so a seamless picture gives a seamless roughness.

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
pub const GUESS_ROUGHNESS: &str = "guess_roughness";

/// The Guess roughness box.
#[derive(Debug)]
pub(crate) struct GuessRoughness {
    info: BoxInfo,
}

impl GuessRoughness {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                GUESS_ROUGHNESS,
                text!("Guess roughness"),
                Group::Shape,
                text!(
                    "Guesses how rough or shiny each part of a picture is: busy, grainy parts \
                     (rough rock, bark) are taken as matt, smooth even parts (polished or wet \
                     patches) as shiny. It gives greys from Shiniest to Roughest: plug it \
                     into Material from height's Roughness. It is a guess: look at the lit \
                     preview and set the range."
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
                        text!("Roughness"),
                        ValueKind::Height,
                        false,
                    )],
                ),
                vec![
                    number(
                        "low",
                        text!("Shiniest"),
                        text!("The roughness of the smoothest parts: 0 is a mirror, 1 fully matt."),
                        0.0,
                        1.0,
                        0.45,
                    ),
                    number(
                        "high",
                        text!("Roughest"),
                        text!("The roughness of the busiest parts: 0 is a mirror, 1 fully matt."),
                        0.0,
                        1.0,
                        0.9,
                    ),
                    number(
                        "detail",
                        text!("Detail size"),
                        text!(
                            "How far round each spot to look for grain, as a share of the \
                             texture's width: about the size of the grain that makes a part \
                             rough."
                        ),
                        0.005,
                        0.2,
                        0.02,
                    ),
                    switch(
                        "invert",
                        text!("Invert"),
                        text!("Busy parts are shiny and smooth parts rough instead."),
                    ),
                ],
                &["material_from_height", "levels"],
            ),
        }
    }
}

impl BoxType for GuessRoughness {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let colour = job.needed_colour("colour")?.clone();
        let threads = job.threads();
        let n = job.size().get() as f32;
        let radius = (job.number("detail").clamp(0.005, 0.2) * n).max(1.0);
        let (low, high) = (
            job.number("low").clamp(0.0, 1.0),
            job.number("high").clamp(0.0, 1.0),
        );
        let invert = job.switch("invert");
        let light = lightness_of(&colour, threads);
        let squared = light.map(|v| v * v);
        let average = soft_blur(&light, radius, threads);
        let average_sq = soft_blur(&squared, radius, threads);
        let busy = job.fill_grey(|x, y| {
            let (x, y) = (i64::from(x), i64::from(y));
            let m = average.at(x, y);
            (average_sq.at(x, y) - m * m).max(0.0).sqrt()
        });
        let busy = soft_blur(&stretched(&busy, threads), radius * 0.5, threads);
        let out = job.map_grey(&busy, |t| {
            let t = if invert { 1.0 - t } else { t };
            (low + (high - low) * t.clamp(0.0, 1.0)).clamp(0.0, 1.0)
        });
        Ok(vec![Value::Height(Arc::new(out))])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxes::tests_each::{registry, with};
    use crate::image::{Image, Rgb, Size};
    use crate::settings::{SettingValue, Settings};

    fn run(colour: Rgb, settings: &Settings) -> crate::Grey {
        let info = registry().get(GUESS_ROUGHNESS).unwrap().info();
        let size = Size::new(colour.size()).unwrap();
        let job = Job::new(
            info,
            size,
            1,
            2,
            settings,
            vec![Some(Value::Colour(Arc::new(colour)))],
        );
        let out = registry().get(GUESS_ROUGHNESS).unwrap().run(&job).unwrap();
        out[0].grey().unwrap().as_ref().clone()
    }

    /// Grainy on the left half, smooth on the right.
    fn half_grainy(n: u32) -> Rgb {
        crate::parallel::fill(Size::new(n).unwrap(), 1, |x, y| {
            let v = if x < n / 2 {
                0.3 + 0.4 * crate::seed::unit(crate::seed::lattice(5, 0, x.into(), y.into()))
            } else {
                0.5
            };
            [v; 3]
        })
    }

    #[test]
    fn grainy_parts_are_rough_and_smooth_ones_shiny() {
        let r = run(half_grainy(128), &Settings::new());
        assert!(r.at(32, 64) > 0.8, "{}", r.at(32, 64));
        assert!(r.at(96, 64) < 0.5, "{}", r.at(96, 64));
        let inverted = run(
            half_grainy(128),
            &with(&[("invert", SettingValue::Switch(true))]),
        );
        assert!(inverted.at(32, 64) < inverted.at(96, 64));
    }

    #[test]
    fn it_stays_inside_the_range_chosen() {
        for (low, high) in [(0.2, 0.6), (0.0, 1.0), (0.7, 0.3), (0.5, 0.5)] {
            let s = with(&[
                ("low", SettingValue::Number(low)),
                ("high", SettingValue::Number(high)),
            ]);
            let r = run(half_grainy(64), &s);
            let (lo, hi) = (low.min(high), low.max(high));
            for v in r.pixels() {
                assert!(*v >= lo - 1e-5 && *v <= hi + 1e-5, "{low} {high}: {v}");
            }
        }
    }

    #[test]
    fn a_flat_picture_gives_one_roughness() {
        let flat: Rgb = Image::filled(Size::new(32).unwrap(), [0.6; 3]);
        let r = run(flat, &Settings::new());
        let first = r.at(0, 0);
        assert!(r.pixels().iter().all(|v| (*v - first).abs() < 1e-5));
    }
}
