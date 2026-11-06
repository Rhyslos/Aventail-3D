//! Repeat: copies a height in a grid, smaller, so a pattern repeats more
//! often; each copy can be a little lighter or darker.

use super::pattern::cell_random;
use super::{info, number, seed, uv, whole};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::value::Value;
use std::sync::Arc;
use words::text;

/// The Repeat box.
#[derive(Debug)]
pub(crate) struct Repeat {
    info: BoxInfo,
}

impl Repeat {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "repeat",
                text!("Repeat"),
                Group::Shape,
                text!(
                    "Copies a height in a grid, smaller, so it repeats more often: one stud \
                     becomes a sheet of studs, one tile a whole floor. Shift moves every other \
                     row along (keep Down even so the texture still repeats seamlessly). Vary \
                     makes each copy a little darker at random, so the copies don't look stamped."
                ),
                (
                    vec![PlugSpec::new("in", text!("In"), ValueKind::Height, true)],
                    vec![PlugSpec::new("out", text!("Out"), ValueKind::Height, false)],
                ),
                vec![
                    whole(
                        "across",
                        text!("Across"),
                        text!("Copies across the texture."),
                        1,
                        32,
                        2,
                    ),
                    whole(
                        "down",
                        text!("Down"),
                        text!("Copies down the texture."),
                        1,
                        32,
                        2,
                    ),
                    number(
                        "shift",
                        text!("Shift"),
                        text!("How far every other row moves along, as a share of a copy."),
                        0.0,
                        1.0,
                        0.0,
                    ),
                    number(
                        "vary",
                        text!("Vary"),
                        text!("How much darker each copy may be, at random."),
                        0.0,
                        1.0,
                        0.0,
                    ),
                    seed(),
                ],
                &["blend", "warp", "material_from_height"],
            ),
        }
    }
}

impl BoxType for Repeat {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let input = job.needed_grey("in")?;
        let across = job.whole("across").max(1);
        let down = job.whole("down").max(1);
        let shift = job.number("shift");
        let vary = job.number("vary").clamp(0.0, 1.0);
        let seed = job.seed();
        let n = job.size().get() as f32;
        let out = job.fill_grey(|x, y| {
            let (u, v) = uv(x, y, n);
            let fy = v * down as f32;
            let row = fy.floor();
            let fx = u * across as f32 + if row as i64 % 2 == 1 { shift } else { 0.0 };
            let col = fx.floor();
            let value = input.sample(fx - col, fy - row);
            let dark = cell_random(
                seed,
                0,
                col as i64,
                row as i64,
                i64::from(across),
                i64::from(down),
            );
            value * (1.0 - vary * dark)
        });
        Ok(vec![Value::Height(Arc::new(out))])
    }
}
