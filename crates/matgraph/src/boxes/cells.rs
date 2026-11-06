//! Cells: a pattern of irregular cells, like cobbles, scales, cracked mud
//! or cell walls.
//!
//! Each square of a grid that wraps round holds one point, moved at
//! random within it (Jitter); every pixel looks for its nearest points
//! (the well-known cellular, or Worley, pattern). Nothing is taken from
//! other programs.

use super::pattern::cell_random;
use super::{choice, info, number, seed, uv, whole};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::value::Value;
use std::sync::Arc;
use words::text;

/// The Cells box.
#[derive(Debug)]
pub(crate) struct Cells {
    info: BoxInfo,
}

/// The looks, in the Look setting's order.
const DISTANCE: u32 = 0;
const BUMPS: u32 = 1;
const EDGES: u32 = 2;
const FLAT: u32 = 3;

impl Cells {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "cells",
                text!("Cells"),
                Group::Make,
                text!(
                    "Makes a pattern of irregular cells: cobbles, scales, cracked mud, crystals. \
                     Look picks what each cell shows: dark middles getting lighter outwards \
                     (Distance), rounded domes (Bumps), the lines between cells (Edges), or one \
                     flat grey per cell (Flat). Random gives each cell its own grey. It repeats \
                     seamlessly."
                ),
                (
                    Vec::new(),
                    vec![
                        PlugSpec::new("height", text!("Height"), ValueKind::Height, false),
                        PlugSpec::new("random", text!("Random"), ValueKind::Height, false),
                    ],
                ),
                vec![
                    whole(
                        "scale",
                        text!("Scale"),
                        text!("How many cells fit across the texture."),
                        1,
                        64,
                        6,
                    ),
                    number(
                        "jitter",
                        text!("Jitter"),
                        text!("How far each cell's middle moves at random: 0 is a neat grid."),
                        0.0,
                        1.0,
                        0.9,
                    ),
                    choice(
                        "look",
                        text!("Look"),
                        text!("What each cell shows."),
                        &[
                            text!("Distance"),
                            text!("Bumps"),
                            text!("Edges"),
                            text!("Flat"),
                        ],
                        BUMPS,
                    ),
                    seed(),
                ],
                &["levels", "bevel", "material_from_height"],
            ),
        }
    }
}

impl BoxType for Cells {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let scale = i64::from(job.whole("scale").max(1));
        let jitter = job.number("jitter").clamp(0.0, 1.0);
        let look = job.choice("look");
        let seed = job.seed();
        let n = job.size().get() as f32;
        let nearest = |x: u32, y: u32| {
            let (u, v) = uv(x, y, n);
            let (px, py) = (u * scale as f32, v * scale as f32);
            let (cx, cy) = (px.floor() as i64, py.floor() as i64);
            let (mut first, mut second, mut owner) = (f32::MAX, f32::MAX, (0, 0));
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (ix, iy) = (cx + dx, cy + dy);
                    let jx = cell_random(seed, 0, ix, iy, scale, scale) - 0.5;
                    let jy = cell_random(seed, 1, ix, iy, scale, scale) - 0.5;
                    let (fx, fy) = (ix as f32 + 0.5 + jx * jitter, iy as f32 + 0.5 + jy * jitter);
                    let d = ((px - fx).powi(2) + (py - fy).powi(2)).sqrt();
                    if d < first {
                        second = first;
                        first = d;
                        owner = (ix, iy);
                    } else if d < second {
                        second = d;
                    }
                }
            }
            (first, second, owner)
        };
        let height = job.fill_grey(|x, y| {
            let (first, second, owner) = nearest(x, y);
            let v = match look {
                DISTANCE => first / 0.8,
                EDGES => 1.0 - ((second - first) / 0.25).min(1.0),
                FLAT => cell_random(seed, 2, owner.0, owner.1, scale, scale),
                _ => 1.0 - first / 0.8,
            };
            v.clamp(0.0, 1.0)
        });
        let random = job.fill_grey(|x, y| {
            let (_, _, owner) = nearest(x, y);
            cell_random(seed, 2, owner.0, owner.1, scale, scale)
        });
        Ok(vec![
            Value::Height(Arc::new(height)),
            Value::Height(Arc::new(random)),
        ])
    }
}
