//! Gradient map: turns greys into colours along a gradient: dark greys
//! take the colours at its left end, light greys those at its right.

use super::info;
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, Label, PlugSpec};
use crate::settings::{ColourGradient, GradientStop, SettingKind, SettingSpec};
use crate::value::Value;
use std::borrow::Cow;
use std::sync::Arc;
use words::text;

/// The Gradient map box.
#[derive(Debug)]
pub(crate) struct GradientMap {
    info: BoxInfo,
}

/// Colours worked out along the gradient, looked up by grey.
const STEPS: usize = 1024;

/// Gradient map's standard gradient: dark earth to warm stone, for a
/// first look that already reads as a material.
pub(crate) fn standard_gradient() -> ColourGradient {
    let stop = |at, colour| GradientStop { at, colour };
    ColourGradient::new(&[
        stop(0.0, [0.12, 0.10, 0.09]),
        stop(0.5, [0.45, 0.41, 0.36]),
        stop(1.0, [0.82, 0.78, 0.70]),
    ])
    .unwrap_or_else(|| ColourGradient::two([0.0; 3], [1.0; 3]))
}

impl GradientMap {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "gradient_map",
                text!("Gradient map"),
                Group::Colour,
                text!(
                    "Colours a height: each grey takes the colour at the same place along the \
                     gradient, black at its left end and white at its right. Plug a height in, \
                     then pick colours: dark for the cracks and dips, light for the tops. This is \
                     the quickest way to colour a material so its colour follows its shape."
                ),
                (
                    vec![PlugSpec::new("in", text!("In"), ValueKind::Height, true)],
                    vec![PlugSpec::new(
                        "colour",
                        text!("Colour"),
                        ValueKind::Colour,
                        false,
                    )],
                ),
                vec![SettingSpec {
                    key: Cow::Borrowed("colours"),
                    name: Label::Key(text!("Colours")),
                    about: Label::Key(text!(
                        "The gradient: the colours dark greys become on the left, light greys on \
                         the right."
                    )),
                    kind: SettingKind::Gradient {
                        standard: standard_gradient(),
                    },
                }],
                &[
                    "hue_saturation",
                    "material_from_height",
                    "mix_materials",
                    "palette_map",
                ],
            ),
        }
    }
}

impl BoxType for GradientMap {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let input = job.needed_grey("in")?;
        let gradient = job.gradient("colours");
        let table: Vec<[f32; 3]> = (0..STEPS)
            .map(|i| gradient.colour_at(i as f32 / (STEPS - 1) as f32))
            .collect();
        let n = i64::from(input.size());
        let out = job.fill_rgb(|x, y| {
            let g = input.at(i64::from(x) % n, i64::from(y) % n);
            let g = if g.is_finite() {
                g.clamp(0.0, 1.0)
            } else {
                0.0
            };
            table[(g * (STEPS - 1) as f32).round() as usize]
        });
        Ok(vec![Value::Colour(Arc::new(out))])
    }
}
