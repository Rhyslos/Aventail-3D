//! Toon bands (Phase 12, milestone 23, [decision 0286](../../../../docs/decisions/0286-stylised-light-boxes.md)):
//! light and shade in a few flat bands, as cel-shaded games and
//! cartoons draw them.
//!
//! The light is worked out as Directional light works it out (the same
//! sun angles, the same slopes from a height and the same turn from
//! light to shade, [`crate::light`]), from a height or a material's own
//! normals, then cut into Bands even steps: each step is one flat tone,
//! from the Shade colour to the Light colour. Softness blurs only where
//! one band meets the next. With Palette, the tones keep to one of the
//! project's palettes (Palette map's Nearest, decision 0281).

use super::directional_light::{degrees, shadow, slope_normal};
use super::palette_map::{DARK_TO_LIGHT, MAX_SWATCH, nearest, oklab, swatches};
use super::{choice, colour, info, length, moving, number, whole};
use crate::image::{Grey, Rgb};
use crate::kind::ValueKind;
use crate::light::{lit, sun};
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, Label, PlugSpec};
use crate::settings::{SettingKind, SettingSpec};
use crate::value::{MaterialMaps, Value};
use std::borrow::Cow;
use std::sync::Arc;
use words::{text, trf};

/// The most bands.
pub(crate) const MAX_BANDS: i32 = 8;

/// The Colours setting's choices.
const OWN_COLOURS: u32 = 0;
const PALETTE: u32 = 1;

/// The Toon bands box.
#[derive(Debug)]
pub(crate) struct ToonBands {
    info: BoxInfo,
}

impl ToonBands {
    pub(crate) fn new() -> Self {
        let mut shadows = super::switch(
            "shadows",
            text!("Shadows"),
            text!("Whether the height's bumps cast shadows, which fall into the darkest band."),
        );
        shadows.kind = SettingKind::Switch { standard: true };
        Self {
            info: info(
                "toon_bands",
                text!("Toon bands"),
                Group::Colour,
                text!(
                    "Light and shade in a few flat bands, as cel-shaded games and cartoons draw \
                     them. Plug a height into Height (or a whole material: its own bumps are \
                     used) and the colour or material to shade. The sun is placed as in \
                     Directional light; Bands is how many tones from shade to light, and \
                     Softness how soft the line between two bands is. With no colour, it gives \
                     the tones alone. Bands gives which band each place is in, as a height."
                ),
                (
                    vec![
                        PlugSpec::new("height", text!("Height"), ValueKind::Height, false),
                        PlugSpec::new("colour", text!("Colour"), ValueKind::Colour, false),
                        PlugSpec::new("material", text!("Material"), ValueKind::Material, false),
                    ],
                    vec![
                        PlugSpec::new("colour_out", text!("Colour"), ValueKind::Colour, false),
                        PlugSpec::new(
                            "material_out",
                            text!("Material"),
                            ValueKind::Material,
                            false,
                        ),
                        PlugSpec::new("bands", text!("Bands"), ValueKind::Height, false),
                    ],
                ),
                vec![
                    degrees(
                        "around",
                        text!("Around"),
                        text!(
                            "Which way the sun is, in degrees: 0 from the right, 90 from the top \
                             of the texture, 180 from the left, 270 from the bottom."
                        ),
                        360.0,
                        240.0,
                    ),
                    degrees(
                        "up",
                        text!("Sun height"),
                        text!("How high the sun is, in degrees: 0 on the horizon, 90 overhead."),
                        90.0,
                        50.0,
                    ),
                    length(
                        "depth",
                        text!("Depth"),
                        text!(
                            "How deep the height is from black to white, in centimetres: \
                             deeper makes steeper slopes and longer shadows."
                        ),
                        0.0,
                        50.0,
                        2.0,
                    ),
                    whole(
                        "bands",
                        text!("Bands"),
                        text!("How many flat tones from shade to light: 2 is light and shade."),
                        2,
                        MAX_BANDS,
                        3,
                    ),
                    number(
                        "softness",
                        text!("Softness"),
                        text!(
                            "How soft the line between two bands is: 0 is a hard line, as in \
                             cel shading."
                        ),
                        0.0,
                        1.0,
                        0.15,
                    ),
                    choice(
                        "colours",
                        text!("Colours"),
                        text!(
                            "Shade and light: the bands go from the Shade colour to the Light \
                             colour. Palette: every colour is then snapped to the closest of a \
                             palette's swatches (with no colour plugged in, each band is one \
                             swatch, dark to light)."
                        ),
                        &[text!("Shade and light"), text!("Palette")],
                        OWN_COLOURS,
                    ),
                    colour(
                        "shade_colour",
                        text!("Shade colour"),
                        text!("The darkest band's colour, multiplied over the colour."),
                        [0.42, 0.38, 0.62],
                    ),
                    colour(
                        "light_colour",
                        text!("Light colour"),
                        text!("The lightest band's colour, multiplied over the colour."),
                        [1.0, 0.97, 0.9],
                    ),
                    SettingSpec {
                        key: Cow::Borrowed("palette"),
                        name: Label::Key(text!("Palette")),
                        about: Label::Key(text!(
                            "Which of the project's palettes the colours keep to, with Colours \
                             at Palette."
                        )),
                        kind: SettingKind::Palette {
                            standard: String::new(),
                        },
                    },
                    shadows,
                    number(
                        "amount",
                        text!("Amount"),
                        text!("How strongly the bands are painted over the colour."),
                        0.0,
                        1.0,
                        1.0,
                    ),
                ],
                &["painted_highlight", "colour_steps", "material_from_height"],
            ),
        }
    }
}

/// Light `l` (0 to 1) cut into `bands` even steps, as 0 (the darkest) to
/// 1 (the lightest), each line between two softened `softness` (0 hard,
/// 1 a fade half a band wide each side).
pub(crate) fn banded(l: f32, bands: u32, softness: f32) -> f32 {
    let n = bands.max(2) as f32;
    let half = softness.clamp(0.0, 1.0) * 0.5 / n;
    let mut b = 0.0;
    for j in 1..bands.max(2) {
        let line = j as f32 / n;
        b += if half <= 0.0 {
            if l >= line { 1.0 } else { 0.0 }
        } else {
            let t = ((l - line + half) / (2.0 * half)).clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        };
    }
    (b / (n - 1.0)).clamp(0.0, 1.0)
}

impl BoxType for ToonBands {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn needs_an_input(&self) -> bool {
        true
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let size = job.size();
        let kinds = self.info.outputs.iter().map(|o| o.kind);
        Ok(self
            .run_outputs(job)?
            .into_iter()
            .zip(kinds)
            .map(|(v, kind)| v.unwrap_or_else(|| moving::plain(kind, size)))
            .collect())
    }

    fn run_outputs(&self, job: &Job<'_>) -> Result<Vec<Option<Value>>, Refusal> {
        let material = job.input("material").and_then(Value::material);
        let height: Option<&Grey> = job
            .grey("height")
            .map(|g| &**g)
            .or_else(|| material.map(|m| &*m.height));
        let Some(height) = height else {
            return Err(Refusal::new(trf!(
                "{} needs a height or a material to know which way each place faces: plug one \
                 into Height or Material.",
                job.box_name()
            )));
        };
        // A material's own normals when no height is plugged in.
        let normals: Option<&Rgb> = match job.grey("height") {
            Some(_) => None,
            None => material.map(|m| &*m.normal),
        };
        let toward = sun(
            job.number("around").clamp(0.0, 360.0),
            job.number("up").clamp(0.0, 90.0),
        );
        let depth = job.length_px("depth").max(0.0);
        let bands = job.whole("bands").clamp(2, MAX_BANDS) as u32;
        let softness = job.number("softness").clamp(0.0, 1.0);
        let amount = job.number("amount").clamp(0.0, 1.0);
        let shadows = job.switch("shadows");
        let shade = job.colour("shade_colour").map(|v| v.clamp(0.0, 1.0));
        let light = job.colour("light_colour").map(|v| v.clamp(0.0, 1.0));
        let palette = if job.choice("colours") == PALETTE {
            let name = job.palette("palette");
            let p = job.palettes().usable(&job.box_name(), "palette", &name)?;
            Some(swatches(&p.colours, 1, MAX_SWATCH, DARK_TO_LIGHT))
        } else {
            None
        };
        let labs: Option<Vec<[f32; 3]>> = palette
            .as_ref()
            .map(|p| p.iter().map(|c| oklab(*c)).collect());
        let toward_v = glam::Vec3::from_array(toward);
        let level = job.fill_grey(|x, y| {
            let (x, y) = (i64::from(x), i64::from(y));
            let normal = match normals {
                Some(n) => glam::Vec3::from_array(n.at(x, y)).normalize_or(glam::Vec3::Z),
                None => slope_normal(height, x, y, depth),
            };
            // The painter's turn at its softest, a smooth ramp from shade
            // to light for the bands to cut.
            let mut l = lit(normal.dot(toward_v), 1.0);
            if shadows && l > 0.0 {
                l *= 1.0 - shadow(height, x, y, toward, depth, 0.0);
            }
            banded(l.clamp(0.0, 1.0), bands, softness)
        });
        let tone = |t: f32| -> [f32; 3] { [0, 1, 2].map(|i| shade[i] + (light[i] - shade[i]) * t) };
        let snapped = |c: [f32; 3]| -> [f32; 3] {
            match (&palette, &labs) {
                (Some(p), Some(l)) if !p.is_empty() => p[nearest(l, oklab(c))],
                _ => c,
            }
        };
        let shaded = |c: &Rgb| -> Rgb {
            job.fill_rgb(|x, y| {
                let (x, y) = (i64::from(x), i64::from(y));
                let base = c.at(x, y).map(|v| v.clamp(0.0, 1.0));
                let t = tone(level.at(x, y));
                snapped([0, 1, 2].map(|i| base[i] + (base[i] * t[i] - base[i]) * amount))
            })
        };
        let colour_out = match job.input("colour").and_then(Value::colour) {
            Some(c) => shaded(c),
            None => match &palette {
                // Each band one swatch, dark to light.
                Some(p) if !p.is_empty() => job.fill_rgb(|x, y| {
                    let t = level.at(i64::from(x), i64::from(y));
                    let i = (t * (p.len() - 1) as f32).round() as usize;
                    p[i.min(p.len() - 1)]
                }),
                // The tones alone: the bands over white.
                _ => job.fill_rgb(|x, y| {
                    let t = tone(level.at(i64::from(x), i64::from(y)));
                    t.map(|v| 1.0 + (v - 1.0) * amount)
                }),
            },
        };
        let material_out = material.map(|m| {
            Value::Material(Arc::new(MaterialMaps {
                colour: Arc::new(shaded(&m.colour)),
                ..(**m).clone()
            }))
        });
        Ok(vec![
            Some(Value::Colour(Arc::new(colour_out))),
            material_out,
            Some(Value::Height(Arc::new(level))),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hard_bands_give_exactly_that_many_tones() {
        for bands in 2..=MAX_BANDS as u32 {
            let mut seen: Vec<f32> = (0..=1000)
                .map(|i| banded(i as f32 / 1000.0, bands, 0.0))
                .collect();
            seen.dedup();
            assert_eq!(seen.len(), bands as usize, "{bands}");
            assert_eq!(seen[0], 0.0);
            assert_eq!(*seen.last().unwrap(), 1.0);
        }
    }

    #[test]
    fn softness_blends_only_near_the_lines() {
        // Three bands: lines at a third and two thirds.
        assert_eq!(banded(0.1, 3, 0.5), 0.0);
        assert_eq!(banded(0.5, 3, 0.5), 0.5);
        assert_eq!(banded(0.9, 3, 0.5), 1.0);
        let at_line = banded(1.0 / 3.0, 3, 0.5);
        assert!((at_line - 0.25).abs() < 1e-4, "{at_line}");
        // Never backwards.
        let mut last = 0.0;
        for i in 0..=1000 {
            let b = banded(i as f32 / 1000.0, 4, 0.7);
            assert!(b >= last - 1e-6);
            last = b;
        }
    }
}
