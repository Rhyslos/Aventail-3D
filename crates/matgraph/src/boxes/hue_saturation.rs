//! Hue and saturation: shifts a colour picture's hues, makes it more or
//! less colourful, lighter or darker; through a shape if one is given.
//!
//! The colours are turned into hue, saturation and lightness (the usual
//! HSL model), changed, and turned back.

use super::{info, number};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::value::Value;
use std::sync::Arc;
use words::text;

/// The Hue and saturation box.
#[derive(Debug)]
pub(crate) struct HueSaturation {
    info: BoxInfo,
}

impl HueSaturation {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "hue_saturation",
                text!("Hue and saturation"),
                Group::Colour,
                text!(
                    "Changes a colour picture: Hue turns every colour round the colour wheel (red \
                     towards yellow, green, blue), Saturation makes it more colourful or greyer, \
                     Lightness lighter or darker. With a shape in Where, only the white parts \
                     change: say, to make the moss greener but not the stone."
                ),
                (
                    vec![
                        PlugSpec::new("in", text!("In"), ValueKind::Colour, true),
                        PlugSpec::new("where", text!("Where"), ValueKind::Shape, false),
                    ],
                    vec![PlugSpec::new("out", text!("Out"), ValueKind::Colour, false)],
                ),
                vec![
                    number(
                        "hue",
                        text!("Hue"),
                        text!("How far round the colour wheel every colour turns, in degrees."),
                        -180.0,
                        180.0,
                        0.0,
                    ),
                    number(
                        "saturation",
                        text!("Saturation"),
                        text!("0 is grey, 1 as it is, 2 twice as colourful."),
                        0.0,
                        2.0,
                        1.0,
                    ),
                    number(
                        "lightness",
                        text!("Lightness"),
                        text!("Below 0 darker, above 0 lighter."),
                        -1.0,
                        1.0,
                        0.0,
                    ),
                ],
                &["material_from_height", "mix_materials"],
            ),
        }
    }
}

impl BoxType for HueSaturation {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let input = job.needed_colour("in")?;
        let mask = job.grey("where");
        let hue = job.number("hue") / 360.0;
        let saturation = job.number("saturation").max(0.0);
        let lightness = job.number("lightness").clamp(-1.0, 1.0);
        let out = job.fill_rgb(|x, y| {
            let (x, y) = (i64::from(x), i64::from(y));
            let before = input.at(x, y);
            let (h, s, l) = to_hsl(before);
            let s = (s * saturation).clamp(0.0, 1.0);
            let l = if lightness >= 0.0 {
                l + (1.0 - l) * lightness
            } else {
                l * (1.0 + lightness)
            };
            let after = from_hsl((h + hue).rem_euclid(1.0), s, l.clamp(0.0, 1.0));
            let share = mask.map_or(1.0, |m| m.at(x, y).clamp(0.0, 1.0));
            std::array::from_fn(|i| (before[i] + (after[i] - before[i]) * share).clamp(0.0, 1.0))
        });
        Ok(vec![Value::Colour(Arc::new(out))])
    }
}

/// Hue (0 to 1 round the wheel), saturation and lightness of `c`.
pub(crate) fn to_hsl(c: [f32; 3]) -> (f32, f32, f32) {
    let [r, g, b] = c.map(|v| {
        if v.is_finite() {
            v.clamp(0.0, 1.0)
        } else {
            0.0
        }
    });
    let (most, least) = (r.max(g).max(b), r.min(g).min(b));
    let l = (most + least) * 0.5;
    let span = most - least;
    if span <= 1e-6 {
        return (0.0, 0.0, l);
    }
    let s = span / (1.0 - (2.0 * l - 1.0).abs()).max(1e-6);
    let h = if most == r {
        ((g - b) / span).rem_euclid(6.0)
    } else if most == g {
        (b - r) / span + 2.0
    } else {
        (r - g) / span + 4.0
    };
    (h / 6.0, s.min(1.0), l)
}

/// The colour of hue `h` (0 to 1), saturation `s` and lightness `l`.
pub(crate) fn from_hsl(h: f32, s: f32, l: f32) -> [f32; 3] {
    let chroma = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let h6 = h.rem_euclid(1.0) * 6.0;
    let x = chroma * (1.0 - (h6 % 2.0 - 1.0).abs());
    let (r, g, b) = match h6 as u32 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let m = l - chroma * 0.5;
    [r + m, g + m, b + m].map(|v| v.clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_survive_the_round_trip_through_hsl() {
        for c in [
            [0.2, 0.5, 0.8],
            [1.0, 0.0, 0.0],
            [0.3, 0.3, 0.3],
            [0.9, 0.85, 0.1],
            [0.0, 1.0, 0.5],
        ] {
            let (h, s, l) = to_hsl(c);
            let back = from_hsl(h, s, l);
            for i in 0..3 {
                assert!((back[i] - c[i]).abs() < 1e-4, "{c:?} {back:?}");
            }
        }
        // Red turned a third of the way round is green.
        let (h, s, l) = to_hsl([1.0, 0.0, 0.0]);
        let green = from_hsl(h + 1.0 / 3.0, s, l);
        assert!((green[1] - 1.0).abs() < 1e-5 && green[0] < 1e-5);
        assert_eq!(to_hsl([f32::NAN, 2.0, -1.0]).2, 0.5);
    }
}
