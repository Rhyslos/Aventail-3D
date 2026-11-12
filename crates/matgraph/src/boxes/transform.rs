//! Transform: moves, turns and flips a height, a colour or a whole
//! material without breaking its tiling (colours and materials from
//! decision 0270, [`super::moving`]).

use super::moving::{self, Source};
use super::{choice, info, number, switch, uv};
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job};
use crate::value::Value;
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
                    "Moves, turns or flips a height, a colour or a whole material: plug in any \
                     of them (or all three, moved together) and take each from its output. \
                     Whatever moves off one edge comes back on the other, so the texture still \
                     tiles. It turns in quarter turns only: any other angle would break the \
                     tiling. A material's bumps turn with it. To make a pattern smaller, use \
                     Repeat."
                ),
                super::moving::plugs(),
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
        Ok(moving::filled(self.run_outputs(job)?, job.size()))
    }

    fn needs_an_input(&self) -> bool {
        true
    }

    fn run_outputs(&self, job: &Job<'_>) -> Result<Vec<Option<Value>>, Refusal> {
        let (across, down) = (job.number("across"), job.number("down"));
        let turn = job.choice("turn") % 4;
        let (flip_x, flip_y) = (job.switch("flip_across"), job.switch("flip_down"));
        let n = job.size().get() as f32;
        let at = |x, y| {
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
            Source { u, v, scale: 1.0 }
        };
        // A normal (x right, y up the texture) flips, then turns
        // clockwise, as the picture does.
        let normal = |[mut nx, mut ny, nz]: [f32; 3], _: &Source| {
            if flip_x {
                nx = -nx;
            }
            if flip_y {
                ny = -ny;
            }
            match turn {
                1 => [ny, -nx, nz],
                2 => [-nx, -ny, nz],
                3 => [-ny, nx, nz],
                _ => [nx, ny, nz],
            }
        };
        moving::moved_all(job, &at, &normal)
    }
}
