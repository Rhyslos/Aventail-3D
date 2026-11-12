//! Repeat: copies a height, a colour or a whole material in a grid,
//! smaller, so a pattern repeats more often; each copy can be a little
//! darker (colours and materials from decision 0270, [`super::moving`]).

use super::moving::{self, Source};
use super::pattern::cell_random;
use super::{info, number, seed, uv, whole};
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job};
use crate::value::Value;
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
                    "Copies a height, a colour or a whole material in a grid, smaller, so it \
                     repeats more often: one stud becomes a sheet of studs, one tile a whole \
                     floor. Plug in any of them (or all three, copied together) and take each \
                     from its output. Shift moves every other row along (keep Down even so the \
                     texture still repeats seamlessly). Vary makes each copy a little darker \
                     (and a height lower) at random, so the copies don't look stamped."
                ),
                moving::plugs(),
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
        Ok(moving::filled(self.run_outputs(job)?, job.size()))
    }

    fn needs_an_input(&self) -> bool {
        true
    }

    fn run_outputs(&self, job: &Job<'_>) -> Result<Vec<Option<Value>>, Refusal> {
        let across = job.whole("across").max(1);
        let down = job.whole("down").max(1);
        let shift = job.number("shift");
        let vary = job.number("vary").clamp(0.0, 1.0);
        let seed = job.seed();
        let n = job.size().get() as f32;
        let at = |x, y| {
            let (u, v) = uv(x, y, n);
            let fy = v * down as f32;
            let row = fy.floor();
            let fx = u * across as f32 + if row as i64 % 2 == 1 { shift } else { 0.0 };
            let col = fx.floor();
            let dark = cell_random(
                seed,
                0,
                col as i64,
                row as i64,
                i64::from(across),
                i64::from(down),
            );
            Source {
                u: fx - col,
                v: fy - row,
                scale: 1.0 - vary * dark,
            }
        };
        // Copies smaller across and down have slopes steeper by as much,
        // and a copy made lower has gentler ones.
        let (sx, sy) = (across as f32, down as f32);
        let normal =
            |[nx, ny, nz]: [f32; 3], s: &Source| [nx * sx * s.scale, ny * sy * s.scale, nz];
        moving::moved_all(job, &at, &normal)
    }
}
