//! Gradient: greys running smoothly from dark to light, repeated a whole
//! number of times so the texture still tiles.

use super::{choice, info, uv, whole};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::value::Value;
use std::sync::Arc;
use words::text;

/// The Gradient box.
#[derive(Debug)]
pub(crate) struct Gradient {
    info: BoxInfo,
}

/// The styles, in the Style setting's order.
const UP_AND_DOWN: u32 = 0;
const RAMP: u32 = 1;
const ROUND: u32 = 2;

impl Gradient {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "gradient",
                text!("Gradient"),
                Group::Make,
                text!(
                    "Makes greys running smoothly from dark to light: for stripes, waves, fading \
                     an effect in, or (with Round) a soft spot in each repeat. Up and down goes \
                     dark, light, dark again, so it never has a sharp step; Ramp goes dark to \
                     light then starts again with a sharp step. It repeats a whole number of \
                     times, so the texture still tiles."
                ),
                (
                    Vec::new(),
                    vec![PlugSpec::new(
                        "height",
                        text!("Height"),
                        ValueKind::Height,
                        false,
                    )],
                ),
                vec![
                    choice(
                        "style",
                        text!("Style"),
                        text!("How the greys run."),
                        &[text!("Up and down"), text!("Ramp"), text!("Round")],
                        UP_AND_DOWN,
                    ),
                    choice(
                        "direction",
                        text!("Direction"),
                        text!("Which way the greys run (Round spreads out every way)."),
                        &[text!("Across"), text!("Down"), text!("Diagonal")],
                        0,
                    ),
                    whole(
                        "repeats",
                        text!("Repeats"),
                        text!("How many times it repeats across the texture."),
                        1,
                        64,
                        1,
                    ),
                ],
                &["levels", "blend", "warp"],
            ),
        }
    }
}

impl BoxType for Gradient {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let style = job.choice("style");
        let direction = job.choice("direction");
        let repeats = job.whole("repeats").max(1) as f32;
        let n = job.size().get() as f32;
        let image = job.fill_grey(|x, y| {
            let (u, v) = uv(x, y, n);
            if style == ROUND {
                let (fx, fy) = ((u * repeats).fract() - 0.5, (v * repeats).fract() - 0.5);
                // 1 in the middle, 0 at the nearest edge of the repeat.
                return (1.0 - (fx * fx + fy * fy).sqrt() * 2.0).clamp(0.0, 1.0);
            }
            let along = match direction {
                1 => v,
                2 => u + v,
                _ => u,
            };
            let t = (along * repeats).fract();
            let value = if style == RAMP {
                t
            } else {
                1.0 - (2.0 * t - 1.0).abs()
            };
            value.clamp(0.0, 1.0)
        });
        Ok(vec![Value::Height(Arc::new(image))])
    }
}
