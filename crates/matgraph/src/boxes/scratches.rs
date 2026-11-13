//! Scratches: thin, tapering scratches at random places and angles,
//! scored into a height, for worn metal, scuffed wood and old paint
//! (Phase 12, milestone 10; decision 0272).
//!
//! Each scratch is placed like one of Scatter's copies (the same
//! [`placing`] from the box's seed, so the same seed gives the same
//! scratches): its middle, its length (the copy's size), its turn and its
//! strength (the copy's height). Each pixel looks only at the scratches
//! near it through [`placing::Lookup`], which lists scratches reaching
//! over an edge on the other side too, so the texture tiles.

use super::pattern::smooth;
use super::scatter::placing::{self, Lookup, Plan};
use super::{info, length, number, seed, uv};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::value::Value;
use std::sync::Arc;
use words::text;

/// The Scratches box.
#[derive(Debug)]
pub(crate) struct Scratches {
    info: BoxInfo,
}

impl Scratches {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "scratches",
                text!("Scratches"),
                Group::Detail,
                text!(
                    "Scores thin scratches into a height, each tapering to a point: worn metal, \
                     scuffed wood, old paint. Spacing says how many there are (about one in \
                     each square this wide), Length and Width how big each is, all in \
                     centimetres on the finished material. Direction turns them all, and \
                     Direction varies lets each stray from it, so they can run one way (brushed \
                     metal) or every way (a floor). With a height plugged in, the scratches are \
                     cut into it; with nothing, into flat white. Scratches gives the lines alone, \
                     to lighten them on a colour. With a shape in Where, scratches only start \
                     where it is white."
                ),
                (
                    vec![
                        PlugSpec::new("in", text!("Height"), ValueKind::Height, false),
                        PlugSpec::new("where", text!("Where"), ValueKind::Shape, false),
                    ],
                    vec![
                        PlugSpec::new("out", text!("Height"), ValueKind::Height, false),
                        PlugSpec::new("scratches", text!("Scratches"), ValueKind::Shape, false),
                    ],
                ),
                vec![
                    length(
                        "spacing",
                        text!("Spacing"),
                        text!(
                            "How far apart the scratches are, in centimetres: about one in each \
                             square this wide."
                        ),
                        0.5,
                        200.0,
                        6.0,
                    ),
                    length(
                        "length",
                        text!("Length"),
                        text!("How long the longest scratch is, in centimetres."),
                        0.5,
                        200.0,
                        12.0,
                    ),
                    number(
                        "length_vary",
                        text!("Length varies"),
                        text!("How much shorter a scratch may be, at random."),
                        0.0,
                        0.9,
                        0.6,
                    ),
                    length(
                        "width",
                        text!("Width"),
                        text!("How wide a scratch is at its middle, in centimetres."),
                        0.01,
                        5.0,
                        0.12,
                    ),
                    number(
                        "direction",
                        text!("Direction"),
                        text!("Which way the scratches run, in degrees, clockwise from across."),
                        -180.0,
                        180.0,
                        30.0,
                    ),
                    number(
                        "direction_vary",
                        text!("Direction varies"),
                        text!("How far each may stray from that, either way, in degrees."),
                        0.0,
                        180.0,
                        25.0,
                    ),
                    number(
                        "strength_vary",
                        text!("Strength varies"),
                        text!("How much fainter some scratches are, at random."),
                        0.0,
                        1.0,
                        0.6,
                    ),
                    number(
                        "depth",
                        text!("Depth"),
                        text!("How deep the scratches cut into the height."),
                        0.0,
                        1.0,
                        0.3,
                    ),
                    seed(),
                ],
                &["blend", "material_from_height", "hue_saturation"],
            ),
        }
    }
}

impl BoxType for Scratches {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let n = job.size().get() as f32;
        let plan = Plan {
            grid: false,
            spacing: job.length_px("spacing").max(1.0) / n,
            loose: 0.0,
            shift: 0.0,
            size: job.length_px("length") / n,
            size_vary: job.number("length_vary"),
            turn: job.number("direction").to_radians(),
            turn_vary: job.number("direction_vary").to_radians(),
            height_vary: job.number("strength_vary"),
            colour_vary: 0.0,
            apart: false,
            seed: job.seed(),
        };
        let lines = placing::place(&plan, job.grey("where").map(|m| &**m));
        let lookup = Lookup::new(&lines);
        let half_width = (job.length_px("width") * 0.5).max(0.0);
        let depth = job.number("depth").clamp(0.0, 1.0);
        let scratches = job.fill_grey(|x, y| {
            let (px, py) = uv(x, y, n);
            let mut most: f32 = 0.0;
            for e in lookup.near(px, py) {
                let c = &lines[e.copy as usize];
                let (dx, dy) = (px - e.x, py - e.y);
                let half = c.size * 0.5;
                if half <= 0.0 {
                    continue;
                }
                let along = c.cos * dx + c.sin * dy;
                let t = along / half;
                if t.abs() >= 1.0 {
                    continue;
                }
                // A gentle bow, its own for each scratch.
                let bow = (c.rank - 0.5) * 0.12 * half * (1.0 - t * t);
                let across = (c.cos * dy - c.sin * dx - bow).abs() * n;
                // Narrowing to a point at both ends.
                let here = half_width * (1.0 - t * t).sqrt();
                let cover = 1.0 - smooth(here * 0.4, here + 0.75, across);
                if cover <= 0.0 {
                    continue;
                }
                // Thinner than a pixel shows fainter, never as nothing.
                let strength = (here * 2.0 + 0.35).min(1.0) * c.height;
                most = most.max(cover * strength);
            }
            most.clamp(0.0, 1.0)
        });
        let base = job.grey("in");
        let carved = job.fill_grey(|x, y| {
            let (x, y) = (i64::from(x), i64::from(y));
            let under = base.map_or(1.0, |b| b.at(x, y));
            (under - depth * scratches.at(x, y)).clamp(0.0, 1.0)
        });
        Ok(vec![
            Value::Height(Arc::new(carved)),
            Value::Shape(Arc::new(scratches)),
        ])
    }
}
