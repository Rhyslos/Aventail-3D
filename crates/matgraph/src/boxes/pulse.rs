//! Pulse: brightens a colour, or strengthens a height, and lets it go
//! again, over and over: embers glowing, a beating heart, a blinking
//! light (milestone 15, [decision 0277](../../../../docs/decisions/0277-animated-boxes.md)).
//!
//! The strength goes from Lowest to Highest and back, Rate times a second,
//! in one of three shapes (smooth, sharp, steps). With nothing plugged in
//! it gives the strength itself as a flat grey, to drive other boxes.

use super::motion::{self, Wave, through};
use super::moving;
use super::{choice, info, number, whole};
use crate::image::Image;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job};
use crate::value::{MaterialMaps, Value};
use std::sync::Arc;
use words::text;

/// The Pulse box.
#[derive(Debug)]
pub(crate) struct Pulse {
    info: BoxInfo,
}

impl Pulse {
    pub(crate) fn new() -> Self {
        Self {
            info: motion::animated(info(
                "pulse",
                text!("Pulse"),
                Group::Move,
                text!(
                    "Brightens a colour (or a material's colour) and strengthens a height, then \
                     lets them go again, over and over: glowing embers, a beating heart, a \
                     blinking light. The strength goes from Lowest to Highest and back Rate \
                     times a second: smoothly, sharply (a quick flash that fades), or in steps. \
                     1 leaves it as it is, 2 doubles it. With nothing plugged in it gives the \
                     strength as a flat grey, to drive other boxes. It moves in the preview \
                     while the player plays; exports bake the moment chosen there."
                ),
                moving::plugs(),
                vec![
                    number(
                        "rate",
                        text!("Rate"),
                        text!("How many pulses a second."),
                        0.01,
                        20.0,
                        1.0,
                    ),
                    choice(
                        "shape",
                        text!("Shape"),
                        text!(
                            "Smooth rises and falls gently; Sharp jumps up and fades, like a \
                             flash; Steps holds at even levels, like a blinking light."
                        ),
                        &[text!("Smooth"), text!("Sharp"), text!("Steps")],
                        0,
                    ),
                    whole(
                        "steps",
                        text!("Steps"),
                        text!("How many levels Steps holds at (2 blinks on and off)."),
                        2,
                        16,
                        2,
                    ),
                    number(
                        "low",
                        text!("Lowest"),
                        text!("The weakest it gets: 1 as it is, 0 dark or flat."),
                        0.0,
                        2.0,
                        0.6,
                    ),
                    number(
                        "high",
                        text!("Highest"),
                        text!("The strongest it gets: 1 as it is, 2 twice as bright or high."),
                        0.0,
                        2.0,
                        1.2,
                    ),
                ],
                &["material_from_height", "blend", "mix_materials"],
            )),
        }
    }
}

/// How strong a Pulse is at its job's moment: from Lowest to Highest.
pub(super) fn strength(job: &Job<'_>) -> f32 {
    let wave = match job.choice("shape") {
        1 => Wave::Sharp,
        2 => Wave::Steps(job.whole("steps").max(2) as u32),
        _ => Wave::Smooth,
    };
    let rate = job.number("rate");
    let p = if rate > 0.0 {
        through(job.time(), 1.0 / rate)
    } else {
        0.0
    };
    let (low, high) = (job.number("low"), job.number("high"));
    low + (high - low) * wave.at(p)
}

impl BoxType for Pulse {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        Ok(moving::filled(self.run_outputs(job)?, job.size()))
    }

    fn run_outputs(&self, job: &Job<'_>) -> Result<Vec<Option<Value>>, Refusal> {
        let f = strength(job);
        let grey = |g: &Image<f32>| Arc::new(job.map_grey(g, |v| (v * f).clamp(0.0, 1.0)));
        let rgb = |c: &Image<[f32; 3]>| {
            Arc::new(job.map_rgb(c, |p| p.map(|v| (v * f).clamp(0.0, 1.0))))
        };
        let height = match job.input("in") {
            Some(v) => v.grey().map(|g| Value::Height(grey(g))),
            // Nothing to strengthen: the strength itself.
            None => Some(Value::Height(Arc::new(Image::filled(
                job.size(),
                f.clamp(0.0, 1.0),
            )))),
        };
        let colour = job
            .input("colour")
            .and_then(Value::colour)
            .map(|c| Value::Colour(rgb(c)));
        let material = job.input("material").and_then(Value::material).map(|m| {
            Value::Material(Arc::new(MaterialMaps {
                colour: rgb(&m.colour),
                ..(**m).clone()
            }))
        });
        Ok(vec![height, colour, material])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxes::tests_each::{input, size, with};
    use crate::kind::ValueKind;
    use crate::settings::SettingValue;

    fn strength_at(seconds: f32, settings: &[(&str, SettingValue)]) -> f32 {
        let pulse = Pulse::new();
        let job = Job::new(pulse.info(), size(16), 0, 1, &with(settings), Vec::new())
            .with_time(seconds);
        strength(&job)
    }

    #[test]
    fn pulse_stays_between_lowest_and_highest() {
        for shape in 0..3 {
            for (low, high) in [(0.6, 1.2), (0.0, 2.0), (1.5, 0.5), (1.0, 1.0)] {
                let settings = [
                    ("shape", SettingValue::Choice(shape)),
                    ("low", SettingValue::Number(low)),
                    ("high", SettingValue::Number(high)),
                    ("rate", SettingValue::Number(1.7)),
                    ("steps", SettingValue::Whole(4)),
                ];
                let (lo, hi) = (low.min(high), low.max(high));
                let mut seen = (f32::MAX, f32::MIN);
                for i in 0..400 {
                    let f = strength_at(i as f32 * 0.013, &settings);
                    assert!(f >= lo - 1e-5 && f <= hi + 1e-5, "{shape} {f}");
                    seen = (seen.0.min(f), seen.1.max(f));
                }
                // It reaches both ends.
                assert!(seen.0 - lo < 0.05 && hi - seen.1 < 0.05, "{shape} {seen:?}");
            }
        }
    }

    #[test]
    fn a_smooth_pulse_loops_at_its_rate() {
        let settings = [("rate", SettingValue::Number(2.0))];
        assert!((strength_at(0.0, &settings) - 0.6).abs() < 1e-5);
        assert!((strength_at(0.25, &settings) - 1.2).abs() < 1e-5);
        assert!((strength_at(0.5, &settings) - strength_at(0.0, &settings)).abs() < 1e-5);
    }

    #[test]
    fn pulse_brightens_what_is_plugged_in_and_stays_in_range() {
        let pulse = Pulse::new();
        let n = size(32);
        let settings = with(&[("low", SettingValue::Number(2.0))]);
        let inputs = vec![
            Some(input(ValueKind::Height, n)),
            Some(input(ValueKind::Colour, n)),
            Some(input(ValueKind::Material, n)),
        ];
        let job = Job::new(pulse.info(), n, 0, 2, &settings, inputs.clone()).with_time(0.0);
        let out = pulse.run_outputs(&job).unwrap();
        let height = out[0].as_ref().unwrap().grey().unwrap();
        let before = inputs[0].as_ref().unwrap().grey().unwrap();
        for (a, b) in before.pixels().iter().zip(height.pixels()) {
            assert!((b - (a * 2.0).min(1.0)).abs() < 1e-5);
        }
        let material = out[2].as_ref().unwrap().material().unwrap();
        let given = inputs[2].as_ref().unwrap().material().unwrap();
        assert_eq!(material.height, given.height, "only the colour brightens");
        assert!(
            material
                .colour
                .pixels()
                .iter()
                .all(|c| c.iter().all(|v| (0.0..=1.0).contains(v)))
        );
        // Nothing plugged in: the strength as a flat grey.
        let job = Job::new(pulse.info(), n, 0, 1, &with(&[]), vec![None, None, None])
            .with_time(0.0);
        let out = pulse.run_outputs(&job).unwrap();
        assert!((out[0].as_ref().unwrap().grey().unwrap().at(5, 5) - 0.6).abs() < 1e-5);
        assert!(out[1].is_none() && out[2].is_none());
    }
}
