//! Colour steps (Phase 12, milestone 23, [decision 0286](../../../../docs/decisions/0286-stylised-light-boxes.md)):
//! colours cut into a few flat steps (posterised), for a cel or poster
//! look, or a height cut into terraces.
//!
//! - **Each channel:** red, green and blue are each cut into Steps even
//!   steps, which flattens and shifts colours, as a poster print does.
//! - **Lightness only:** only how light each colour is (by eye, in the
//!   Oklab colour space) is cut into steps, each the middle of its band;
//!   its hue and strength are kept, so a material keeps its colours in
//!   flat tones.
//!
//! Dither is Palette map's fixed 4 × 4 pattern (it repeats every four
//! pixels, so the texture still tiles), and Snap to a palette then turns
//! each colour into the palette's closest swatch, with Palette map's
//! Nearest (decision 0281). It moves a height, a colour or a whole
//! material (its colour) as Transform's plugs do ([`super::moving`]).

use super::palette_map::{bayer, from_oklab_kept, nearest, oklab};
use super::{choice, info, moving, number, whole};
use crate::image::Rgb;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, Label};
use crate::settings::{SettingKind, SettingSpec};
use crate::value::{MaterialMaps, Value};
use std::borrow::Cow;
use std::sync::Arc;
use words::text;

/// The most steps.
pub(crate) const MAX_STEPS: i32 = 32;

/// The Mode setting's choices: Each channel (0), Lightness only.
const LIGHTNESS: u32 = 1;

/// The Colour steps box.
#[derive(Debug)]
pub(crate) struct ColourSteps {
    info: BoxInfo,
}

impl ColourSteps {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "colour_steps",
                text!("Colour steps"),
                Group::Colour,
                text!(
                    "Cuts colours into a few flat steps (posterises them), for a cel or poster \
                     look. Lightness only keeps each colour's hue and cuts only how light it \
                     is; Each channel cuts red, green and blue apart, which shifts colours as \
                     a poster print does. Snap to a palette then keeps every colour to one of \
                     the project's palettes. Plug in a colour or a whole material; a height \
                     comes out in terraces."
                ),
                moving::plugs(),
                vec![
                    whole(
                        "steps",
                        text!("Steps"),
                        text!("How many flat steps from dark to light."),
                        2,
                        MAX_STEPS,
                        4,
                    ),
                    choice(
                        "mode",
                        text!("Mode"),
                        text!(
                            "Lightness only: each colour keeps its hue, only how light it is is \
                             cut into steps. Each channel: red, green and blue are each cut, \
                             for a poster look."
                        ),
                        &[text!("Each channel"), text!("Lightness only")],
                        LIGHTNESS,
                    ),
                    number(
                        "dither",
                        text!("Dither"),
                        text!(
                            "Mixes neighbouring steps in a fine checked pattern where one meets \
                             the next, for a retro or pixel-art look."
                        ),
                        0.0,
                        1.0,
                        0.0,
                    ),
                    super::switch(
                        "snap",
                        text!("Snap to a palette"),
                        text!("Turns every colour into the closest of a palette's swatches."),
                    ),
                    SettingSpec {
                        key: Cow::Borrowed("palette"),
                        name: Label::Key(text!("Palette")),
                        about: Label::Key(text!(
                            "Which of the project's palettes to snap to, with Snap to a palette \
                             on."
                        )),
                        kind: SettingKind::Palette {
                            standard: String::new(),
                        },
                    },
                ],
                &["toon_bands", "palette_map", "material_from_height"],
            ),
        }
    }
}

/// `v` (0 to 1) cut into `steps` even steps, the first 0 and the last 1.
pub(crate) fn stepped(v: f32, steps: u32) -> f32 {
    let n = steps.max(2) as f32;
    let v = if v.is_finite() { v } else { 0.0 };
    ((v * n).floor().clamp(0.0, n - 1.0)) / (n - 1.0)
}

/// `v` (0 to 1) cut into `steps` even steps, each the middle of its
/// band: how light a colour is never goes all the way to black or white,
/// so its hue still shows.
pub(crate) fn stepped_middle(v: f32, steps: u32) -> f32 {
    let n = steps.max(2) as f32;
    let v = if v.is_finite() { v } else { 0.0 };
    ((v * n).floor().clamp(0.0, n - 1.0) + 0.5) / n
}

impl BoxType for ColourSteps {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn needs_an_input(&self) -> bool {
        true
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        Ok(moving::filled(self.run_outputs(job)?, job.size()))
    }

    fn run_outputs(&self, job: &Job<'_>) -> Result<Vec<Option<Value>>, Refusal> {
        let steps = job.whole("steps").clamp(2, MAX_STEPS) as u32;
        let n = steps as f32;
        let lightness = job.choice("mode") == LIGHTNESS;
        let dither = job.number("dither").clamp(0.0, 1.0);
        let palette: Option<Vec<[f32; 3]>> = if job.switch("snap") {
            let name = job.palette("palette");
            let p = job.palettes().usable(&job.box_name(), "palette", &name)?;
            Some(
                p.colours
                    .iter()
                    .map(|c| c.map(|v| f32::from(v) / 255.0))
                    .collect(),
            )
        } else {
            None
        };
        let labs: Option<Vec<[f32; 3]>> = palette
            .as_ref()
            .map(|p| p.iter().map(|c| oklab(*c)).collect());
        // The dither's push at a pixel, a step wide at most.
        let push = |x: u32, y: u32| bayer(x, y) * dither / n;
        let colours = |c: &Rgb| -> Rgb {
            job.fill_rgb(|x, y| {
                let base = c.at(i64::from(x), i64::from(y)).map(|v| v.clamp(0.0, 1.0));
                let d = push(x, y);
                let out = if lightness {
                    let [l, a, b] = oklab(base);
                    from_oklab_kept([stepped_middle(l + d, steps), a, b])
                } else {
                    base.map(|v| stepped(v + d, steps))
                };
                match (&palette, &labs) {
                    (Some(p), Some(l)) if !p.is_empty() => p[nearest(l, oklab(out))],
                    _ => out,
                }
            })
        };
        let out = moving::INPUTS
            .iter()
            .map(|key| {
                job.input(key).map(|v| match v {
                    Value::Shape(g) | Value::Height(g) => {
                        Value::Height(Arc::new(job.fill_grey(|x, y| {
                            stepped(g.at(i64::from(x), i64::from(y)) + push(x, y), steps)
                        })))
                    }
                    Value::Colour(c) => Value::Colour(Arc::new(colours(c))),
                    Value::Material(m) => Value::Material(Arc::new(MaterialMaps {
                        colour: Arc::new(colours(&m.colour)),
                        ..(**m).clone()
                    })),
                })
            })
            .collect::<Vec<_>>();
        if out.iter().all(Option::is_none) {
            return Err(Refusal::new(words::trf!(
                "{} has nothing plugged in.",
                job.box_name()
            )));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_give_that_many_values_from_black_to_white() {
        for steps in 2..=MAX_STEPS as u32 {
            let mut seen: Vec<f32> = (0..=2000)
                .map(|i| stepped(i as f32 / 2000.0, steps))
                .collect();
            seen.dedup();
            assert_eq!(seen.len(), steps as usize, "{steps}");
            assert_eq!(seen[0], 0.0);
            assert_eq!(*seen.last().unwrap(), 1.0);
        }
        assert_eq!(stepped(f32::NAN, 4), 0.0);
        assert_eq!(stepped(-3.0, 4), 0.0);
        assert_eq!(stepped(7.0, 4), 1.0);
        // Lightness takes each band's middle.
        assert_eq!(stepped_middle(0.0, 4), 0.125);
        assert_eq!(stepped_middle(1.0, 4), 0.875);
        assert_eq!(stepped_middle(0.3, 2), 0.25);
    }
}
