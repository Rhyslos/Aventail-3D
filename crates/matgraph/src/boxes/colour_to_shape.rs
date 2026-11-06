//! Colour to shape: turns a colour picture into a black-and-white shape,
//! by how light it is or how close it is to a chosen colour. The box the
//! refusal "a colour has no inside or outside" offers to put between.

use super::{choice, colour, info, number};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::value::Value;
use std::sync::Arc;
use words::text;

/// The Colour to shape box.
#[derive(Debug)]
pub(crate) struct ColourToShape {
    info: BoxInfo,
}

/// The ways to choose, in the By setting's order.
const LIGHTNESS: u32 = 0;
const CLOSE_TO: u32 = 1;

impl ColourToShape {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "colour_to_shape",
                text!("Colour to shape"),
                Group::Colour,
                text!(
                    "Turns a colour picture into a black-and-white shape, so boxes that need a \
                     shape (Bevel, Edge finder, a Where plug) can use it. A colour on its own has \
                     no inside or outside: this box decides one. By Lightness, light colours \
                     become white; by Close to a colour, colours like the chosen one become \
                     white. Height gives the same greys as a height."
                ),
                (
                    vec![PlugSpec::new("in", text!("In"), ValueKind::Colour, true)],
                    vec![
                        PlugSpec::new("shape", text!("Shape"), ValueKind::Shape, false),
                        PlugSpec::new("height", text!("Height"), ValueKind::Height, false),
                    ],
                ),
                vec![
                    choice(
                        "by",
                        text!("By"),
                        text!("What makes a place white."),
                        &[text!("Lightness"), text!("Close to a colour")],
                        LIGHTNESS,
                    ),
                    colour(
                        "colour",
                        text!("Colour"),
                        text!("The colour that becomes white, for Close to a colour."),
                        [1.0, 1.0, 1.0],
                    ),
                    number(
                        "threshold",
                        text!("Threshold"),
                        text!(
                            "By Lightness: how light a colour must be to become white. By Close \
                             to a colour: how different a colour may be and still count."
                        ),
                        0.0,
                        1.0,
                        0.5,
                    ),
                    number(
                        "softness",
                        text!("Softness"),
                        text!("How gently black turns to white: 0 is a sharp edge."),
                        0.0,
                        1.0,
                        0.1,
                    ),
                ],
                &["bevel", "edges", "blend"],
            ),
        }
    }
}

impl BoxType for ColourToShape {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let input = job.needed_colour("in")?;
        let by = job.choice("by");
        let target = job.colour("colour");
        let threshold = job.number("threshold").clamp(0.0, 1.0);
        let soft = job.number("softness").clamp(0.0, 1.0).max(1e-3) * 0.5;
        let n = i64::from(input.size());
        let out = job.fill_grey(|x, y| {
            let c = input.at(i64::from(x) % n, i64::from(y) % n);
            let c = c.map(|v| {
                if v.is_finite() {
                    v.clamp(0.0, 1.0)
                } else {
                    0.0
                }
            });
            // How far past the threshold, towards white.
            let past = if by == CLOSE_TO {
                let apart = (0..3)
                    .map(|i| (c[i] - target[i]).powi(2))
                    .sum::<f32>()
                    .sqrt()
                    / 3f32.sqrt();
                threshold - apart
            } else {
                0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2] - threshold
            };
            (0.5 + past / (2.0 * soft)).clamp(0.0, 1.0)
        });
        let out = Arc::new(out);
        Ok(vec![Value::Shape(out.clone()), Value::Height(out)])
    }
}
