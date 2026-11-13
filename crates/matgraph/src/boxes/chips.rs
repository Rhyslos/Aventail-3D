//! Chips: bites taken out of a shape's edges, as on old bricks, worn
//! tiles and chipped paint (Phase 12, milestone 10; decision 0272).
//!
//! A chip is a piece of a coarse, angular cell pattern (cells a whole
//! number of times across, so it tiles) that reaches in from the
//! outline: each cell gets a random bite, and the place is chipped where
//! it is nearer the outline than its cell's bite. The distance to the
//! outline is the exact one shared with Bevel and Edge finder.

use super::noise::layered;
use super::pattern::{cell_random, distance_inside, smooth, whole_repeats};
use super::{info, length, number, seed, uv};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::seed::mix;
use crate::value::Value;
use std::sync::Arc;
use words::text;

/// The Chips box.
#[derive(Debug)]
pub(crate) struct Chips {
    info: BoxInfo,
}

/// The most chip cells across the texture.
const MOST_CELLS: i64 = 512;

impl Chips {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "chips",
                text!("Chips"),
                Group::Detail,
                text!(
                    "Takes bites out of a shape's edges, as on old bricks, worn tiles or chipped \
                     paint. Chip size is how wide a chip is and Reach how far in from the edge \
                     the biggest one bites, both in centimetres on the finished material; Amount \
                     is how many there are. It gives the chipped shape, a height with the chips \
                     cut down (from the height in Height, or the shape itself), and the chips \
                     alone. It needs a shape, because only a shape has edges to chip."
                ),
                (
                    vec![
                        PlugSpec::new("shape", text!("Shape"), ValueKind::Shape, true),
                        PlugSpec::new("height", text!("Height"), ValueKind::Height, false),
                    ],
                    vec![
                        PlugSpec::new("shape_out", text!("Shape"), ValueKind::Shape, false),
                        PlugSpec::new("out", text!("Height"), ValueKind::Height, false),
                        PlugSpec::new("chips", text!("Chips"), ValueKind::Shape, false),
                    ],
                ),
                vec![
                    length(
                        "size",
                        text!("Chip size"),
                        text!("How wide a chip is, in centimetres."),
                        0.2,
                        50.0,
                        3.0,
                    ),
                    length(
                        "reach",
                        text!("Reach"),
                        text!("How far in from the edge the biggest chip bites, in centimetres."),
                        0.1,
                        50.0,
                        1.5,
                    ),
                    number(
                        "amount",
                        text!("Amount"),
                        text!("How many chips: 1 chips the edges all the way round."),
                        0.0,
                        1.0,
                        0.45,
                    ),
                    number(
                        "depth",
                        text!("Depth"),
                        text!("How far down a chip cuts the height: 1 is right to the bottom."),
                        0.0,
                        1.0,
                        0.6,
                    ),
                    seed(),
                ],
                &["bevel", "material_from_height", "edges"],
            ),
        }
    }
}

impl BoxType for Chips {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let shape = job.needed_grey("shape")?;
        let height = job.grey("height");
        let n = job.size().get() as f32;
        let cells = whole_repeats(n, job.length_px("size"), MOST_CELLS);
        let reach = job.length_px("reach").max(0.5);
        let amount = job.number("amount").clamp(0.0, 1.0);
        let depth = job.number("depth").clamp(0.0, 1.0);
        let seed = job.seed();
        let inside = distance_inside(shape, reach + 2.0, job.threads());
        let wobble = (cells as u32).saturating_mul(3).min(1 << 12);
        let chips = job.fill_grey(|x, y| {
            let d = inside.at(i64::from(x), i64::from(y));
            if d <= 0.0 || amount <= 0.0 {
                return 0.0;
            }
            let (u, v) = uv(x, y, n);
            // A little noise on the chips' sides, so they aren't straight.
            let jag = layered(u, v, wobble, 2, 0.5, mix(seed, 7)) - 0.5;
            let bite = cell_bite(seed, cells, u, v, amount) * (1.0 + jag * 0.5);
            if bite <= 0.0 {
                return 0.0;
            }
            1.0 - smooth(bite * reach - 0.6, bite * reach + 0.6, d)
        });
        let at = |img: &crate::image::Grey, x: u32, y: u32| img.at(i64::from(x), i64::from(y));
        let chipped =
            job.fill_grey(|x, y| (at(shape, x, y) * (1.0 - at(&chips, x, y))).clamp(0.0, 1.0));
        let under = height.unwrap_or(shape);
        let cut = job
            .fill_grey(|x, y| (at(under, x, y) * (1.0 - depth * at(&chips, x, y))).clamp(0.0, 1.0));
        Ok(vec![
            Value::Shape(Arc::new(chipped)),
            Value::Height(Arc::new(cut)),
            Value::Shape(Arc::new(chips)),
        ])
    }
}

/// How far the chip of the cell at `(u, v)` bites, as a share of the
/// reach: 0 for a cell without a chip. The cells are the nearest of
/// jittered middles, so chips have straight, angular sides.
fn cell_bite(seed: u64, cells: i64, u: f32, v: f32, amount: f32) -> f32 {
    let (px, py) = (u * cells as f32, v * cells as f32);
    let (cx, cy) = (px.floor() as i64, py.floor() as i64);
    let (mut best, mut owner) = (f32::MAX, (cx, cy));
    for dy in -1..=1 {
        for dx in -1..=1 {
            let (ix, iy) = (cx + dx, cy + dy);
            let jx = cell_random(seed, 0, ix, iy, cells, cells) - 0.5;
            let jy = cell_random(seed, 1, ix, iy, cells, cells) - 0.5;
            let (mx, my) = (ix as f32 + 0.5 + jx * 0.9, iy as f32 + 0.5 + jy * 0.9);
            let d = (mx - px).powi(2) + (my - py).powi(2);
            if d < best {
                (best, owner) = (d, (ix, iy));
            }
        }
    }
    let r = cell_random(seed, 2, owner.0, owner.1, cells, cells);
    if r >= amount {
        return 0.0;
    }
    // Bigger bites are rarer: the size comes from a second number.
    let size = cell_random(seed, 3, owner.0, owner.1, cells, cells);
    0.35 + 0.65 * size * size
}
