//! Bevel: rounds a shape's edges into a height, as if each tile, brick or
//! stone were cut with a rounded, sloped or stepped edge.
//!
//! Each place inside the shape rises with its distance from the outline
//! (an exact distance, wrapping round), up to Width in from it; the edge
//! style shapes that rise, and Roundness softens its corners.

use super::pattern::{distance_inside, soft_blur};
use super::{choice, info, length, number, whole};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::value::Value;
use std::sync::Arc;
use words::text;

/// The Bevel box.
#[derive(Debug)]
pub(crate) struct Bevel {
    info: BoxInfo,
}

/// The edge styles, in the Edge setting's order.
const ROUND: u32 = 0;
const SHARP: u32 = 1;
const STEPS: u32 = 2;

impl Bevel {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "bevel",
                text!("Bevel"),
                Group::Shape,
                text!(
                    "Turns a black-and-white shape into a height with rounded edges, as if each \
                     tile, brick or stone were cut with a rounded, sloped or stepped edge. Width \
                     is how far in from the outline the edge reaches, in centimetres on the \
                     finished material; Height is how high the top is; Roundness softens the \
                     edge's corners. It needs a shape, because only a shape has an inside and an \
                     outside."
                ),
                (
                    vec![PlugSpec::new(
                        "shape",
                        text!("Shape"),
                        ValueKind::Shape,
                        true,
                    )],
                    vec![PlugSpec::new(
                        "height",
                        text!("Height"),
                        ValueKind::Height,
                        false,
                    )],
                ),
                vec![
                    length(
                        "width",
                        text!("Width"),
                        text!("How far in from the outline the edge reaches, in centimetres."),
                        0.0,
                        50.0,
                        2.0,
                    ),
                    number(
                        "roundness",
                        text!("Roundness"),
                        text!("How soft the edge's corners are: 0 is crisp."),
                        0.0,
                        1.0,
                        0.3,
                    ),
                    number(
                        "height",
                        text!("Height"),
                        text!("How high the top of the shape is: 1 is white."),
                        0.0,
                        1.0,
                        1.0,
                    ),
                    choice(
                        "edge",
                        text!("Edge"),
                        text!("The edge's shape: a rounded curve, a straight slope, or steps."),
                        &[text!("Round"), text!("Sharp"), text!("Steps")],
                        ROUND,
                    ),
                    whole(
                        "steps",
                        text!("Steps"),
                        text!("How many steps, when Edge is Steps."),
                        2,
                        12,
                        3,
                    ),
                ],
                &["blend", "warp", "material_from_height"],
            ),
        }
    }
}

impl BoxType for Bevel {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let shape = job.needed_grey("shape")?;
        // Under half a pixel wide is a straight cut.
        let width = job.length_px("width").max(0.5);
        let roundness = job.number("roundness").clamp(0.0, 1.0);
        let top = job.number("height").clamp(0.0, 1.0);
        let edge = job.choice("edge");
        let steps = job.whole("steps").clamp(2, 12) as f32;
        let inside = distance_inside(shape, width, job.threads());
        let raw = job.fill_grey(|x, y| {
            let (x, y) = (i64::from(x), i64::from(y));
            let d = inside.at(x, y);
            if d <= 0.0 {
                return 0.0;
            }
            let t = ((d + 0.5) / width).clamp(0.0, 1.0);
            let rise = match edge {
                SHARP => t,
                STEPS => ((t * steps).floor() / steps).max(1.0 / steps),
                _ => (1.0 - (1.0 - t) * (1.0 - t)).sqrt(),
            };
            rise * top
        });
        let out = soft_blur(&raw, roundness * width * 0.5, job.threads());
        Ok(vec![Value::Height(Arc::new(out))])
    }
}
