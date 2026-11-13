//! Directional light: paints light and shade into a colour from a
//! height, as if a sun shone across the material, warm where it reaches
//! and cool in the shade (Phase 12, milestone 10; decision 0272).
//!
//! It is the texture painter's light-and-shade helper
//! ([decision 0196](../../../../docs/decisions/0196-light-and-shade.md))
//! for a material: the same sun angles, colours, softness and warmth,
//! and the same turn from light to shade ([`crate::light`], which the
//! painter calls too). The slopes come from the height, Depth deep; with
//! Shadows on, the height's bumps also cast shadows along the texture
//! (wrapping round, so it still tiles). Toon bands and painted
//! highlights are for later stylised light boxes; this is the plain one.

use super::{colour, info, length, moving, number, switch};
use crate::image::{Grey, Rgb};
use crate::kind::ValueKind;
use crate::light::{lit, sun, warmed};
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::settings::{SettingKind, SettingSpec};
use crate::value::{MaterialMaps, Value};
use std::sync::Arc;
use words::text;

/// The Directional light box.
#[derive(Debug)]
pub(crate) struct DirectionalLight {
    info: BoxInfo,
}

/// The most pixels a shadow is looked for along.
const MOST_SHADOW_STEPS: u32 = 160;

impl DirectionalLight {
    pub(crate) fn new() -> Self {
        let mut shadows = switch(
            "shadows",
            text!("Shadows"),
            text!("Whether the height's bumps cast shadows on the rest."),
        );
        shadows.kind = SettingKind::Switch { standard: true };
        Self {
            info: info(
                "directional_light",
                text!("Directional light"),
                Group::Colour,
                text!(
                    "Paints light and shade into a colour from a height, as if a sun shone \
                     across the material: warm where it reaches, cool in the shade. It is the \
                     painter's light and shade helper for a material, with the same settings. \
                     Plug the height into Height and the colour (or a whole material) to light; \
                     with no colour, it gives the light and shade colours alone. Around and Sun \
                     height place the sun (0 from the right, 90 from the top of the texture); \
                     Depth is how deep the height is from black to white, in centimetres on the \
                     finished material, so its slopes and shadows are the right size. Light \
                     gives how lit each place is, as a height."
                ),
                (
                    vec![
                        PlugSpec::new("height", text!("Height"), ValueKind::Height, true),
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
                        PlugSpec::new("light", text!("Light"), ValueKind::Height, false),
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
                    colour(
                        "light_colour",
                        text!("Light colour"),
                        text!("The colour where the sun reaches."),
                        [1.0, 222.0 / 255.0, 170.0 / 255.0],
                    ),
                    colour(
                        "shade_colour",
                        text!("Shade colour"),
                        text!("The colour in the shade."),
                        [60.0 / 255.0, 80.0 / 255.0, 140.0 / 255.0],
                    ),
                    number(
                        "softness",
                        text!("Softness"),
                        text!(
                            "How soft the turn from light to shade is: 0 is a hard line, as in \
                             toon shading."
                        ),
                        0.0,
                        1.0,
                        0.5,
                    ),
                    number(
                        "warmth",
                        text!("Warmth"),
                        text!(
                            "How warm the light and how cool the shade: 0 makes both neutral \
                             greys."
                        ),
                        0.0,
                        1.0,
                        1.0,
                    ),
                    shadows,
                    number(
                        "amount",
                        text!("Amount"),
                        text!("How strongly the light and shade are painted over the colour."),
                        0.0,
                        1.0,
                        0.8,
                    ),
                ],
                &["edge_highlight", "brush_strokes", "material_from_height"],
            ),
        }
    }
}

/// An angle setting, in degrees from 0 to `most`.
fn degrees(
    key: &'static str,
    name: &'static str,
    about: &'static str,
    most: f32,
    standard: f32,
) -> SettingSpec {
    number(key, name, about, 0.0, most, standard)
}

impl BoxType for DirectionalLight {
    fn info(&self) -> &BoxInfo {
        &self.info
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
        let height = job.needed_grey("height")?;
        let toward = sun(
            job.number("around").clamp(0.0, 360.0),
            job.number("up").clamp(0.0, 90.0),
        );
        let depth = job.length_px("depth").max(0.0);
        let softness = job.number("softness").clamp(0.0, 1.0);
        let warmth = job.number("warmth");
        let light_colour = warmed(job.colour("light_colour"), warmth);
        let shade_colour = warmed(job.colour("shade_colour"), warmth);
        let amount = job.number("amount").clamp(0.0, 1.0);
        let shadows = job.switch("shadows");
        let light = job.fill_grey(|x, y| {
            let (x, y) = (i64::from(x), i64::from(y));
            // Slopes in pixels of height per pixel along; y is up the
            // texture, so the row below is "down".
            let dx = (height.at(x + 1, y) - height.at(x - 1, y)) * 0.5 * depth;
            let dy = (height.at(x, y - 1) - height.at(x, y + 1)) * 0.5 * depth;
            let normal = glam::Vec3::new(-dx, -dy, 1.0).normalize_or(glam::Vec3::Z);
            let facing = normal.dot(glam::Vec3::from_array(toward));
            let mut amount_lit = lit(facing, softness);
            if shadows && amount_lit > 0.0 {
                amount_lit *= 1.0 - shadow(height, x, y, toward, depth, softness);
            }
            amount_lit.clamp(0.0, 1.0)
        });
        let tone = |k: f32| -> [f32; 3] {
            [0, 1, 2].map(|i| shade_colour[i] + (light_colour[i] - shade_colour[i]) * k)
        };
        let painted = |c: &Rgb| -> Rgb {
            job.fill_rgb(|x, y| {
                let (x, y) = (i64::from(x), i64::from(y));
                let base = c.at(x, y);
                let t = tone(light.at(x, y));
                [0, 1, 2].map(|i| {
                    let (a, b) = (base[i].clamp(0.0, 1.0), t[i].clamp(0.0, 1.0));
                    // Soft light: mid grey leaves the colour, lighter
                    // tones lighten it and darker ones darken it.
                    let lit = (1.0 - 2.0 * b) * a * a + 2.0 * b * a;
                    (a + (lit - a) * amount).clamp(0.0, 1.0)
                })
            })
        };
        let colour_out = match job.input("colour").and_then(Value::colour) {
            Some(c) => painted(c),
            // The light and shade colours alone, as the painter's helper.
            None => job.fill_rgb(|x, y| {
                let t = tone(light.at(i64::from(x), i64::from(y)));
                t.map(|v| (0.5 + (v - 0.5) * amount).clamp(0.0, 1.0))
            }),
        };
        let material_out = job.input("material").and_then(Value::material).map(|m| {
            Value::Material(Arc::new(MaterialMaps {
                colour: Arc::new(painted(&m.colour)),
                ..(**m).clone()
            }))
        });
        Ok(vec![
            Some(Value::Colour(Arc::new(colour_out))),
            material_out,
            Some(Value::Height(Arc::new(light))),
        ])
    }
}

/// How much of the sun is blocked at pixel `(x, y)` (0 to 1): looking
/// along the texture toward the sun, whether the height (`depth` pixels
/// from black to white) rises above the line to it. A soft light gives
/// softer shadows.
fn shadow(height: &Grey, x: i64, y: i64, toward: [f32; 3], depth: f32, softness: f32) -> f32 {
    let flat = (toward[0] * toward[0] + toward[1] * toward[1]).sqrt();
    if flat < 1e-4 || depth <= 0.0 {
        return 0.0;
    }
    // Across and down the texture per step (y is up, rows go down).
    let (sx, sy) = (toward[0] / flat, -toward[1] / flat);
    let rise = toward[2] / flat;
    let start = height.at(x, y) * depth;
    // Past the height's full depth nothing can block the sun.
    let reach = if rise <= 1e-4 {
        MOST_SHADOW_STEPS
    } else {
        ((depth / rise).ceil() as u32).min(MOST_SHADOW_STEPS)
    };
    let soft = 0.5 + softness * 2.0;
    let mut most: f32 = 0.0;
    for k in 1..=reach {
        let t = k as f32;
        let (qx, qy) = (x as f32 + sx * t, y as f32 + sy * t);
        let n = height.size() as f32;
        let h = height.sample((qx + 0.5) / n, (qy + 0.5) / n) * depth;
        let above = h - (start + rise * t);
        if above > 0.0 {
            most = most.max((above / soft).min(1.0));
            if most >= 1.0 {
                break;
            }
        }
    }
    most
}
