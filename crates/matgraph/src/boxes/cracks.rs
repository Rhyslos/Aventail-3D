//! Cracks: a network of thin, wandering cracks carved into a height,
//! for dried mud, old plaster, cracked stone and paint (Phase 12,
//! milestone 10; decision 0272).
//!
//! The pieces between the cracks are cells of a jittered grid that
//! repeats a whole number of times across the texture; the cracks run
//! along the cells' borders, pushed about by the box's own noise so they
//! wander. Each border is cracked or not by a random number of its own
//! (Amount says how many), and the crack narrows and widens along its
//! length. The exact distance to a border comes from the two-pass cell
//! search (first the nearest middle, then every border round it), so
//! cracks keep their width in centimetres. Everything wraps, so the
//! texture tiles.

use super::noise::layered;
use super::pattern::{cell_random, smooth, whole_repeats};
use super::{info, length, number, seed, uv};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::seed::{mix, unit};
use crate::value::Value;
use std::sync::Arc;
use words::text;

/// The Cracks box.
#[derive(Debug)]
pub(crate) struct Cracks {
    info: BoxInfo,
}

/// The most pieces across the texture.
const MOST_PIECES: i64 = 256;

/// How far a piece's middle strays from its place in the grid.
const JITTER: f32 = 0.85;

impl Cracks {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "cracks",
                text!("Cracks"),
                Group::Detail,
                text!(
                    "Carves a network of thin, wandering cracks into a height: dried mud, old \
                     plaster, cracked stone or paint. Piece size is how big the pieces between \
                     the cracks are and Width how wide a crack is, both in centimetres on the \
                     finished material. Amount says how many of the pieces' borders crack. With \
                     a height plugged in, the cracks are cut into it; with nothing, into flat \
                     white. Cracks gives the lines alone, to colour them or mix in dirt. With a \
                     shape in Where, cracks only show where it is white."
                ),
                (
                    vec![
                        PlugSpec::new("in", text!("Height"), ValueKind::Height, false),
                        PlugSpec::new("where", text!("Where"), ValueKind::Shape, false),
                    ],
                    vec![
                        PlugSpec::new("out", text!("Height"), ValueKind::Height, false),
                        PlugSpec::new("cracks", text!("Cracks"), ValueKind::Shape, false),
                    ],
                ),
                vec![
                    length(
                        "size",
                        text!("Piece size"),
                        text!("How big the pieces between the cracks are, in centimetres."),
                        1.0,
                        200.0,
                        25.0,
                    ),
                    length(
                        "width",
                        text!("Width"),
                        text!("How wide a crack is at its widest, in centimetres."),
                        0.02,
                        10.0,
                        0.4,
                    ),
                    number(
                        "amount",
                        text!("Amount"),
                        text!("How many of the pieces' borders crack: 1 is every one."),
                        0.0,
                        1.0,
                        0.65,
                    ),
                    number(
                        "wiggle",
                        text!("Wiggle"),
                        text!("How much the cracks wander: 0 is straight lines."),
                        0.0,
                        1.0,
                        0.5,
                    ),
                    number(
                        "depth",
                        text!("Depth"),
                        text!("How deep the cracks cut into the height."),
                        0.0,
                        1.0,
                        0.6,
                    ),
                    seed(),
                ],
                &["material_from_height", "fill_gaps", "blend"],
            ),
        }
    }
}

impl BoxType for Cracks {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let n = job.size().get() as f32;
        let pieces = whole_repeats(n, job.length_px("size"), MOST_PIECES);
        let half_width = (job.length_px("width") * 0.5).max(0.0);
        let amount = job.number("amount").clamp(0.0, 1.0);
        let wiggle = job.number("wiggle").clamp(0.0, 1.0);
        let depth = job.number("depth").clamp(0.0, 1.0);
        let seed = job.seed();
        let base = job.grey("in");
        let mask = job.grey("where");
        let pixels_per_piece = n / pieces as f32;
        let wobble = (pieces as u32).saturating_mul(2).min(1 << 12);
        let cracks = job.fill_grey(|x, y| {
            let (u, v) = uv(x, y, n);
            let push = |layer: u64| layered(u, v, wobble, 3, 0.5, mix(seed, layer)) - 0.5;
            let px = u * pieces as f32 + push(1) * wiggle * 0.9;
            let py = v * pieces as f32 + push(2) * wiggle * 0.9;
            // How wide here: cracks narrow and widen along their length.
            let taper = layered(u, v, wobble, 2, 0.5, mix(seed, 3));
            let here = half_width * smooth(0.2, 0.75, taper).max(0.3);
            let line = border_line(seed, pieces, amount, px, py, pixels_per_piece, here);
            let fade = mask.map_or(1.0, |m| m.at(i64::from(x), i64::from(y)).clamp(0.0, 1.0));
            (line * fade).clamp(0.0, 1.0)
        });
        let carved = job.fill_grey(|x, y| {
            let (x, y) = (i64::from(x), i64::from(y));
            let under = base.map_or(1.0, |b| b.at(x, y));
            (under - depth * cracks.at(x, y)).clamp(0.0, 1.0)
        });
        Ok(vec![
            Value::Height(Arc::new(carved)),
            Value::Shape(Arc::new(cracks)),
        ])
    }
}

/// A piece's middle, in the grid's cells (wrapping round `pieces`).
fn middle(seed: u64, pieces: i64, ix: i64, iy: i64) -> (f32, f32) {
    let jx = cell_random(seed, 0, ix, iy, pieces, pieces) - 0.5;
    let jy = cell_random(seed, 1, ix, iy, pieces, pieces) - 0.5;
    (ix as f32 + 0.5 + jx * JITTER, iy as f32 + 0.5 + jy * JITTER)
}

/// Whether the border between pieces `a` and `b` cracks: the same
/// answer from either side.
fn cracked(seed: u64, pieces: i64, a: (i64, i64), b: (i64, i64), amount: f32) -> bool {
    let id = |(x, y): (i64, i64)| (y.rem_euclid(pieces) * pieces + x.rem_euclid(pieces)) as u64;
    let (p, q) = (id(a), id(b));
    let (lo, hi) = (p.min(q), p.max(q));
    unit(mix(mix(seed, 0xC4AC), mix(lo, hi.rotate_left(29)))) < amount
}

/// How much of a crack covers the place `(px, py)` (in pieces): 1 in the
/// middle of a cracked border, fading to nothing `half_width` pixels
/// out and a little more, so the line is smooth.
fn border_line(
    seed: u64,
    pieces: i64,
    amount: f32,
    px: f32,
    py: f32,
    pixels_per_piece: f32,
    half_width: f32,
) -> f32 {
    let (cx, cy) = (px.floor() as i64, py.floor() as i64);
    // The nearest middle.
    let (mut best, mut owner, mut to_owner) = (f32::MAX, (cx, cy), (0.0, 0.0));
    for dy in -1..=1 {
        for dx in -1..=1 {
            let (ix, iy) = (cx + dx, cy + dy);
            let (mx, my) = middle(seed, pieces, ix, iy);
            let (rx, ry) = (mx - px, my - py);
            let d = rx * rx + ry * ry;
            if d < best {
                (best, owner, to_owner) = (d, (ix, iy), (rx, ry));
            }
        }
    }
    // Every border round it: the distance to each, in pixels.
    let mut line: f32 = 0.0;
    for dy in -2..=2 {
        for dx in -2..=2 {
            let other = (owner.0 + dx, owner.1 + dy);
            if other == owner {
                continue;
            }
            let (mx, my) = middle(seed, pieces, other.0, other.1);
            let (rx, ry) = (mx - px, my - py);
            let (ax, ay) = (rx - to_owner.0, ry - to_owner.1);
            let len = (ax * ax + ay * ay).sqrt();
            if len < 1e-6 || !cracked(seed, pieces, owner, other, amount) {
                continue;
            }
            let along = ((to_owner.0 + rx) * 0.5 * ax + (to_owner.1 + ry) * 0.5 * ay) / len;
            let d = along.max(0.0) * pixels_per_piece;
            // At least a faint line under a pixel wide, never nothing.
            let strength = (half_width * 2.0 + 0.3).min(1.0);
            let cover = 1.0 - smooth(half_width * 0.5, half_width + 0.75, d);
            line = line.max(cover * strength);
        }
    }
    line
}
