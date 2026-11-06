//! Tiles: a grid of white tiles with black gaps between, the start of
//! floors, walls and bricks.

use super::{next, number, whole};
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
                     laid; keep the number of rows even so the texture still repeats seamlessly."
                )),
                inputs: Vec::new(),
                outputs: vec![PlugSpec::new(
                    "shape",
                    text!("Shape"),
                    ValueKind::Shape,
                    false,
                )],
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
                ],
                comes_next: next(&["blend", "levels", "material_from_height"]),
                one_per_graph: false,
            },
        }
    }
}

impl BoxType for Tiles {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let across = job.whole("across").max(1) as f32;
        let down = job.whole("down").max(1) as f32;
        let half_gap = job.number("gap") * 0.5;
        let shift = job.number("shift");
        let n = job.size().get() as f32;
        let image = job.fill_grey(|x, y| {
            let (u, v) = ((x as f32 + 0.5) / n, (y as f32 + 0.5) / n);
            let fy = v * down;
            let row = fy.floor();
            let fx = u * across + if row as i64 % 2 == 1 { shift } else { 0.0 };
            let (tx, ty) = (fx.fract(), fy - row);
            // How far inside the tile, in pixels, from its nearest edge.
            let inside_x = (tx - half_gap).min(1.0 - half_gap - tx) * n / across;
            let inside_y = (ty - half_gap).min(1.0 - half_gap - ty) * n / down;
            (inside_x.min(inside_y) + 0.5).clamp(0.0, 1.0)
        });
        Ok(vec![Value::Shape(Arc::new(image))])
    }
}
