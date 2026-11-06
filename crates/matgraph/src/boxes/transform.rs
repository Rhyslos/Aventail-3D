//! Transform: moves, turns and flips a height without breaking its
//! tiling.

use super::{choice, info, number, switch, uv};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::value::Value;
use std::sync::Arc;
use words::text;

/// The Transform box.
#[derive(Debug)]
pub(crate) struct Transform {
    info: BoxInfo,
}

impl Transform {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "transform",
                text!("Transform"),
                Group::Shape,
                text!(
                    "Moves, turns or flips a height. Whatever moves off one edge comes back on \
                     the other, so the texture still tiles. It turns in quarter turns only: any \
                     other angle would break the tiling. To make a pattern smaller, use Repeat."
                ),
                (
                    vec![PlugSpec::new("in", text!("In"), ValueKind::Height, true)],
                    vec![PlugSpec::new("out", text!("Out"), ValueKind::Height, false)],
                ),
                vec![
                    number(
                        "across",
                        text!("Move across"),
                        text!("How far it moves right, as a share of the texture."),
                        -1.0,
                        1.0,
                        0.0,
                    ),
                    number(
                        "down",
                        text!("Move down"),
                        text!("How far it moves down, as a share of the texture."),
                        -1.0,
                        1.0,
                        0.0,
                    ),
                    choice(
                        "turn",
                        text!("Turn"),
                        text!("How far it turns, clockwise."),
                        &[
                            text!("None"),
                            text!("A quarter"),
                            text!("A half"),
                            text!("Three quarters"),
                        ],
                        0,
                    ),
                    switch(
                        "flip_across",
                        text!("Flip left to right"),
                        text!("Mirrors it left to right."),
                    ),
                    switch(
                        "flip_down",
                        text!("Flip top to bottom"),
                        text!("Mirrors it top to bottom."),
                    ),
                ],
                &["blend", "warp", "levels"],
            ),
        }
    }
}

impl BoxType for Transform {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let input = job.needed_grey("in")?;
        let (across, down) = (job.number("across"), job.number("down"));
        let turn = job.choice("turn") % 4;
        let (flip_x, flip_y) = (job.switch("flip_across"), job.switch("flip_down"));
        let n = job.size().get() as f32;
        let out = job.fill_grey(|x, y| {
            // Work back from where a pixel lands to where it came from.
            let (mut u, mut v) = uv(x, y, n);
            u -= across;
            v -= down;
            let (cu, cv) = (u - 0.5, v - 0.5);
            // Turned clockwise by `turn` quarters: undo it.
            let (cu, cv) = match turn {
                1 => (cv, -cu),
                2 => (-cu, -cv),
                3 => (-cv, cu),
                _ => (cu, cv),
            };
            let (mut u, mut v) = (cu + 0.5, cv + 0.5);
            if flip_x {
                u = 1.0 - u;
            }
            if flip_y {
                v = 1.0 - v;
            }
            input.sample(u, v)
        });
        Ok(vec![Value::Height(Arc::new(out))])
    }
}
