//! Painted highlight (Phase 12, milestone 23, [decision 0286](../../../../docs/decisions/0286-stylised-light-boxes.md)):
//! a stroke of light along the edges that face a chosen light, as hand
//! painters put a rim or top light on stones, planks and leaves.
//!
//! An edge is where the height stands above the average round it, Width
//! wide (Edge highlight's idea); which way an edge faces is the slope of
//! that average, so it is steady along an edge. Only edges turned toward
//! the light (Around, the same angles as Directional light and the
//! painter's helper, [`crate::light::sun`]) are lit, and Spread is how far
//! round the edges the stroke reaches. Softness 0 paints a crisp stroke.

use super::directional_light::degrees;
use super::pattern::soft_blur;
use super::{colour, info, length, moving, number};
use crate::image::{Grey, Rgb};
use crate::kind::ValueKind;
use crate::light::sun;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::value::{MaterialMaps, Value};
use std::sync::Arc;
use words::{text, trf};

/// How strongly a step in height reads as an edge (as Edge highlight).
const GAIN: f32 = 8.0;

/// How strongly an edge's slope must show for its direction to count:
/// flat tops have no direction.
const SURE: f32 = 4.0;

/// The Painted highlight box.
#[derive(Debug)]
pub(crate) struct PaintedHighlight {
    info: BoxInfo,
}

impl PaintedHighlight {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "painted_highlight",
                text!("Painted highlight"),
                Group::Colour,
                text!(
                    "Paints a stroke of light along the edges that face a light, as hand \
                     painters put a rim or top light on stones, planks and leaves. Plug the \
                     height into Height (or a whole material: its own height is used) and the \
                     colour or material to paint on. Around is where the light comes from (90 \
                     is from the top of the texture); Spread is how far round the edges the \
                     stroke reaches, Width how wide it is, in centimetres. Highlight gives the \
                     stroke alone, to use as a shape elsewhere."
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
                        PlugSpec::new("highlight", text!("Highlight"), ValueKind::Height, false),
                    ],
                ),
                vec![
                    degrees(
                        "around",
                        text!("Around"),
                        text!(
                            "Where the light comes from, in degrees: 0 from the right, 90 from \
                             the top of the texture, 180 from the left, 270 from the bottom."
                        ),
                        360.0,
                        90.0,
                    ),
                    number(
                        "spread",
                        text!("Spread"),
                        text!(
                            "How far round the edges the stroke reaches: 0 only the edges facing \
                             the light straight on, 1 every edge."
                        ),
                        0.0,
                        1.0,
                        0.35,
                    ),
                    length(
                        "width",
                        text!("Width"),
                        text!("How far in from an edge the stroke reaches, in centimetres."),
                        0.05,
                        50.0,
                        1.0,
                    ),
                    number(
                        "softness",
                        text!("Softness"),
                        text!(
                            "How soft the stroke's sides are: 0 is a crisp, painted stroke, 1 a \
                             gentle glow."
                        ),
                        0.0,
                        1.0,
                        0.4,
                    ),
                    colour(
                        "highlight_colour",
                        text!("Highlight colour"),
                        text!("The colour the stroke paints."),
                        [1.0, 0.95, 0.8],
                    ),
                    number(
                        "amount",
                        text!("Amount"),
                        text!("How strongly the stroke is painted over the colour."),
                        0.0,
                        1.0,
                        0.7,
                    ),
                ],
                &["toon_bands", "colour_steps", "material_from_height"],
            ),
        }
    }
}

/// How much of a highlight an edge whose outside faces `facing` (the
/// dot of its direction with the light's, -1 to 1) gets, for `spread`
/// (0 only straight on, 1 all round) and `softness`.
pub(crate) fn facing_weight(facing: f32, spread: f32, softness: f32) -> f32 {
    let centre = 1.0 - 2.2 * spread.clamp(0.0, 1.0);
    let half = 0.15 + 0.35 * softness.clamp(0.0, 1.0);
    let t = ((facing - centre + half) / (2.0 * half)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The stroke's strength across an edge, from how raised it is there
/// (0 to 1): crisp at `softness` 0, a gentle ramp at 1.
fn stroke(raised: f32, softness: f32) -> f32 {
    let half = 0.02 + 0.3 * softness.clamp(0.0, 1.0);
    let t = ((raised - 0.3 + half) / (2.0 * half)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl BoxType for PaintedHighlight {
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
                "{} needs a height or a material to find the edges: plug one into Height or \
                 Material.",
                job.box_name()
            )));
        };
        let toward = sun(job.number("around").clamp(0.0, 360.0), 0.0);
        let spread = job.number("spread").clamp(0.0, 1.0);
        let width = job.length_px("width").max(0.5);
        let softness = job.number("softness").clamp(0.0, 1.0);
        let paint = job.colour("highlight_colour").map(|v| v.clamp(0.0, 1.0));
        let amount = job.number("amount").clamp(0.0, 1.0);
        let average = soft_blur(height, width, job.threads());
        let highlight = job.fill_grey(|x, y| {
            let (x, y) = (i64::from(x), i64::from(y));
            let raised = ((height.at(x, y) - average.at(x, y)) * GAIN).clamp(0.0, 1.0);
            if raised <= 0.0 {
                return 0.0;
            }
            // Uphill across and up the texture (rows go down); an edge's
            // outside faces downhill.
            let gx = average.at(x + 1, y) - average.at(x - 1, y);
            let gy = average.at(x, y - 1) - average.at(x, y + 1);
            let slope = (gx * gx + gy * gy).sqrt();
            if slope <= 1e-6 {
                return 0.0;
            }
            let facing = (-gx * toward[0] - gy * toward[1]) / slope;
            let sure = (slope * width.max(2.0) * SURE).clamp(0.0, 1.0);
            (stroke(raised, softness) * facing_weight(facing, spread, softness) * sure)
                .clamp(0.0, 1.0)
        });
        let painted = |c: &Rgb| -> Rgb {
            job.fill_rgb(|x, y| {
                let (x, y) = (i64::from(x), i64::from(y));
                let k = highlight.at(x, y) * amount;
                let base = c.at(x, y);
                [0, 1, 2].map(|i| {
                    let b = base[i].clamp(0.0, 1.0);
                    (b + (paint[i] - b) * k).clamp(0.0, 1.0)
                })
            })
        };
        let colour_out = job
            .input("colour")
            .and_then(Value::colour)
            .map(|c| Value::Colour(Arc::new(painted(c))));
        let material_out = material.map(|m| {
            Value::Material(Arc::new(MaterialMaps {
                colour: Arc::new(painted(&m.colour)),
                ..(**m).clone()
            }))
        });
        Ok(vec![
            colour_out,
            material_out,
            Some(Value::Height(Arc::new(highlight))),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edges_facing_the_light_get_it_and_others_dont() {
        assert!(facing_weight(1.0, 0.35, 0.4) > 0.99);
        assert_eq!(facing_weight(-1.0, 0.35, 0.4), 0.0);
        assert_eq!(facing_weight(0.0, 0.0, 0.0), 0.0);
        // Spread 1 reaches every edge.
        assert!(facing_weight(-1.0, 1.0, 0.0) > 0.99);
        // Never backwards.
        let mut last = 0.0;
        for i in 0..=200 {
            let w = facing_weight(i as f32 / 100.0 - 1.0, 0.5, 0.5);
            assert!(w >= last - 1e-6);
            last = w;
        }
    }

    #[test]
    fn a_crisp_stroke_is_all_or_nothing() {
        assert_eq!(stroke(0.1, 0.0), 0.0);
        assert_eq!(stroke(0.5, 0.0), 1.0);
        let soft = stroke(0.2, 1.0);
        assert!(soft > 0.0 && soft < 1.0);
    }
}
