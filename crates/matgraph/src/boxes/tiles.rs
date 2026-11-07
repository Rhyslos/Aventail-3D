//! Tiles: a grid of white tiles with black gaps between, the start of
//! floors, walls and bricks; and a random grey for each tile.

use super::pattern::cell_random;
use super::{next, number, seed, uv, whole};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, Label, PlugSpec};
use crate::value::Value;
use std::borrow::Cow;
use std::sync::Arc;
use words::text;

/// The Tiles box.
#[derive(Debug)]
pub(crate) struct Tiles {
    info: BoxInfo,
}

impl Tiles {
    pub(crate) fn new() -> Self {
        Self {
            info: BoxInfo {
                id: Cow::Borrowed("tiles"),
                name: Label::Key(text!("Tiles")),
                group: Group::Make,
                about: Label::Key(text!(
                    "Makes a grid of white tiles with black gaps between them: the start of \
                     floors, walls and bricks. Shift moves every other row along, as bricks are \
                     laid; keep the number of rows even so the texture still repeats seamlessly. \
                     Random gives each tile its own grey (black in the gaps), to make each tile a \
                     little higher, lower or differently coloured."
                )),
                inputs: Vec::new(),
                outputs: vec![
                    PlugSpec::new("shape", text!("Shape"), ValueKind::Shape, false),
                    // Added in Phase 12's milestone 3.
                    PlugSpec::new("random", text!("Random"), ValueKind::Height, false),
                ],
                settings: vec![
                    whole(
                        "across",
                        text!("Across"),
                        text!("Tiles across the texture."),
                        1,
                        64,
                        4,
                    ),
                    whole(
                        "down",
                        text!("Down"),
                        text!("Tiles down the texture."),
                        1,
                        64,
                        4,
                    ),
                    number(
                        "gap",
                        text!("Gap"),
                        text!("How wide the gaps are, as a share of a tile."),
                        0.0,
                        0.45,
                        0.05,
                    ),
                    number(
                        "shift",
                        text!("Shift"),
                        text!("How far every other row moves along, as a share of a tile."),
                        0.0,
                        1.0,
                        0.0,
                    ),
                    // Added in Phase 12's milestone 3, for Random.
                    seed(),
                ],
                comes_next: next(&["bevel", "warp", "blend", "material_from_height"]),
                one_per_graph: false,
                animated: false,
            },
        }
    }
}

impl BoxType for Tiles {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let across = job.whole("across").max(1);
        let down = job.whole("down").max(1);
        let grid = Grid {
            across: across as f32,
            down: down as f32,
            gap: [job.number("gap") * 0.5; 2],
            shift: job.number("shift"),
            n: job.size().get() as f32,
        };
        let seed = job.seed();
        let shape = job.fill_grey(|x, y| grid.at(x, y).0);
        let random = job.fill_grey(|x, y| {
            let (inside, (col, row)) = grid.at(x, y);
            inside * cell_random(seed, 0, col, row, i64::from(across), i64::from(down))
        });
        Ok(vec![
            Value::Shape(Arc::new(shape)),
            Value::Height(Arc::new(random)),
        ])
    }
}

/// A grid of tiles laid in rows, every other row moved along by `shift`
/// (a share of a tile), as Tiles and Bricks lay them.
pub(crate) struct Grid {
    /// Tiles across the texture.
    pub(crate) across: f32,
    /// Rows down the texture.
    pub(crate) down: f32,
    /// Half the gap across and down, as a share of a tile's width and
    /// height.
    pub(crate) gap: [f32; 2],
    /// How far every other row moves along, as a share of a tile.
    pub(crate) shift: f32,
    /// Pixels across the texture.
    pub(crate) n: f32,
}

impl Grid {
    /// How far pixel `(x, y)` is inside its tile (1 inside, 0 in the gap,
    /// blended over a pixel at the edge), and which tile it is in
    /// (column, row).
    pub(crate) fn at(&self, x: u32, y: u32) -> (f32, (i64, i64)) {
        let (u, v) = uv(x, y, self.n);
        let fy = v * self.down;
        let row = fy.floor();
        let fx = u * self.across + if row as i64 % 2 == 1 { self.shift } else { 0.0 };
        let col = fx.floor();
        let (tx, ty) = (fx - col, fy - row);
        let [gx, gy] = self.gap;
        // How far inside the tile, in pixels, from its nearest edge.
        let inside_x = (tx - gx).min(1.0 - gx - tx) * self.n / self.across;
        let inside_y = (ty - gy).min(1.0 - gy - ty) * self.n / self.down;
        let inside = (inside_x.min(inside_y) + 0.5).clamp(0.0, 1.0);
        (inside, (col as i64, row as i64))
    }
}
