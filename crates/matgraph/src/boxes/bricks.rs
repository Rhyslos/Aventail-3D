//! Bricks: bricks laid in rows, every other row moved along, with mortar
//! gaps the same width all round; and a random grey for each brick.

use super::pattern::cell_random;
use super::tiles::Grid;
use super::{info, number, seed, whole};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::value::Value;
use std::sync::Arc;
use words::text;

/// The Bricks box.
#[derive(Debug)]
pub(crate) struct Bricks {
    info: BoxInfo,
}

impl Bricks {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "bricks",
                text!("Bricks"),
                Group::Make,
                text!(
                    "Makes bricks laid in rows, every other row moved along by half a brick (or \
                     as far as Shift says), with mortar gaps the same width all round. Random \
                     gives each brick its own grey, to make some stick out or look a different \
                     colour. An odd number of rows is made one more, so the texture still repeats \
                     seamlessly."
                ),
                (
                    Vec::new(),
                    vec![
                        PlugSpec::new("shape", text!("Shape"), ValueKind::Shape, false),
                        PlugSpec::new("random", text!("Random"), ValueKind::Height, false),
                    ],
                ),
                vec![
                    whole(
                        "rows",
                        text!("Rows"),
                        text!("Rows of bricks down the texture (made even)."),
                        2,
                        64,
                        8,
                    ),
                    whole(
                        "across",
                        text!("Bricks per row"),
                        text!("Bricks in each row."),
                        1,
                        32,
                        4,
                    ),
                    number(
                        "gap",
                        text!("Mortar"),
                        text!("How wide the mortar gaps are, as a share of a brick's height."),
                        0.0,
                        0.45,
                        0.08,
                    ),
                    number(
                        "shift",
                        text!("Shift"),
                        text!("How far every other row moves along, as a share of a brick."),
                        0.0,
                        1.0,
                        0.5,
                    ),
                    seed(),
                ],
                &["bevel", "warp", "blend", "material_from_height"],
            ),
        }
    }
}

impl BoxType for Bricks {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let rows = job.whole("rows").max(2);
        let rows = rows + rows % 2;
        let across = job.whole("across").max(1);
        let half_gap = job.number("gap") * 0.5;
        // The same gap in pixels across as down: bricks are wider than tall.
        let wide = rows as f32 / across as f32;
        let grid = Grid {
            across: across as f32,
            down: rows as f32,
            gap: [half_gap / wide, half_gap],
            shift: job.number("shift"),
            n: job.size().get() as f32,
        };
        let seed = job.seed();
        let shape = job.fill_grey(|x, y| grid.at(x, y).0);
        let random = job.fill_grey(|x, y| {
            let (inside, (col, row)) = grid.at(x, y);
            inside * cell_random(seed, 0, col, row, i64::from(across), i64::from(rows))
        });
        Ok(vec![
            Value::Shape(Arc::new(shape)),
            Value::Height(Arc::new(random)),
        ])
    }
}
