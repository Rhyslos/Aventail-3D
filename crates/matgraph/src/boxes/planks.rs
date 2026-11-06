//! Planks: rows of boards with their ends at random places along each
//! row, a random grey for each board, and a wood grain running along
//! them.

use super::noise::layered;
use super::pattern::cell_random;
use super::{info, number, seed, uv, whole};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::value::Value;
use std::sync::Arc;
use words::text;

/// The Planks box.
#[derive(Debug)]
pub(crate) struct Planks {
    info: BoxInfo,
}

/// How many times finer the grain is down a board than along it.
const GRAIN_STRETCH: u32 = 12;

impl Planks {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "planks",
                text!("Planks"),
                Group::Make,
                text!(
                    "Makes rows of wooden boards, for floors, decks, walls and crates. Each row's \
                     board ends sit at a random place, as boards are laid. Random gives each \
                     board its own grey; Grain gives long streaks running along the boards, \
                     different on each. It repeats seamlessly."
                ),
                (
                    Vec::new(),
                    vec![
                        PlugSpec::new("shape", text!("Shape"), ValueKind::Shape, false),
                        PlugSpec::new("random", text!("Random"), ValueKind::Height, false),
                        PlugSpec::new("grain", text!("Grain"), ValueKind::Height, false),
                    ],
                ),
                vec![
                    whole(
                        "rows",
                        text!("Boards down"),
                        text!("Rows of boards down the texture."),
                        1,
                        64,
                        6,
                    ),
                    whole(
                        "across",
                        text!("Boards per row"),
                        text!("How many boards make up each row."),
                        1,
                        16,
                        2,
                    ),
                    number(
                        "gap",
                        text!("Gap"),
                        text!(
                            "How wide the gaps between boards are, as a share of a board's width."
                        ),
                        0.0,
                        0.45,
                        0.06,
                    ),
                    whole(
                        "grain",
                        text!("Grain size"),
                        text!("How many grain streaks run along a board: more is finer."),
                        1,
                        32,
                        3,
                    ),
                    seed(),
                ],
                &["blend", "gradient_map", "material_from_height"],
            ),
        }
    }
}

impl BoxType for Planks {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let rows = job.whole("rows").max(1);
        let across = job.whole("across").max(1);
        let half_gap = job.number("gap") * 0.5;
        let grain = job.whole("grain").max(1) as u32;
        let seed = job.seed();
        let n = job.size().get() as f32;
        let (rows_f, across_f) = (rows as f32, across as f32);
        // Board (column, row), and how far inside it in pixels.
        let board = |x: u32, y: u32| {
            let (u, v) = uv(x, y, n);
            let fy = v * rows_f;
            let row = fy.floor();
            // Each row's ends are moved along at random.
            let offset = cell_random(seed, 3, 0, row as i64, 1, i64::from(rows));
            let fx = u * across_f + offset;
            let col = fx.floor();
            let (tx, ty) = (fx - col, fy - row);
            let board_height = n / rows_f;
            let gap_px = half_gap * board_height;
            let inside_x = tx.min(1.0 - tx) * n / across_f - gap_px;
            let inside_y = ty.min(1.0 - ty) * board_height - gap_px;
            let inside = (inside_x.min(inside_y) + 0.5).clamp(0.0, 1.0);
            (inside, col as i64, row as i64, u, ty)
        };
        let shape = job.fill_grey(|x, y| board(x, y).0);
        let random = job.fill_grey(|x, y| {
            let (inside, col, row, ..) = board(x, y);
            inside * cell_random(seed, 4, col, row, i64::from(across), i64::from(rows))
        });
        let grain_image = job.fill_grey(|x, y| {
            let (_, col, row, u, ty) = board(x, y);
            // Each board reads the grain from its own place, so neighbours
            // differ; along the board it repeats with the texture.
            let shift = cell_random(seed, 5, col, row, i64::from(across), i64::from(rows));
            let down = (ty + shift * 7.0) / rows_f;
            layered(
                u,
                down * GRAIN_STRETCH as f32,
                grain,
                4,
                0.5,
                seed ^ 0x6772_6169,
            )
        });
        Ok(vec![
            Value::Shape(Arc::new(shape)),
            Value::Height(Arc::new(random)),
            Value::Height(Arc::new(grain_image)),
        ])
    }
}
