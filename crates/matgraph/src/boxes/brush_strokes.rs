//! Brush strokes: repaints a colour (or a material's colour) as
//! overlapping strokes of a brush, for a hand-painted look (Phase 12,
//! milestone 10; decision 0272).
//!
//! Each stroke is placed like one of Scatter's copies ([`placing`], from
//! the box's seed): its middle, its length (the copy's size), its turn
//! and a little change of colour. A stroke takes the colour under its
//! middle and lays it flat along its length, with faint streaks from the
//! brush's bristles, so fine detail turns into dabs of paint. Where
//! strokes overlap, the later one is on top. Strokes wrap over the
//! edges, so the texture tiles. The Height output is the strokes' own
//! paint ridges.

use super::moving;
use super::pattern::smooth;
use super::scatter::placing::{self, Lookup, Placed, Plan};
use super::{info, length, number, seed, uv};
use crate::image::{Image, Rgb};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::seed::{mix, unit};
use crate::value::{MaterialMaps, Value};
use std::sync::Arc;
use words::text;

/// The Brush strokes box.
#[derive(Debug)]
pub(crate) struct BrushStrokes {
    info: BoxInfo,
}

/// How many bristle streaks run along a stroke.
const BRISTLES: f32 = 9.0;

impl BrushStrokes {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "brush_strokes",
                text!("Brush strokes"),
                Group::Colour,
                text!(
                    "Repaints a colour as overlapping strokes of a brush, for a hand-painted \
                     look: fine detail becomes dabs of paint, each a little lighter or darker, \
                     with faint streaks from the bristles. Plug in a colour (or a whole \
                     material, whose colour is repainted). Length and Width are each stroke's \
                     size, in centimetres on the finished material; Direction turns them all \
                     and Direction varies lets each stray. Detail kept lets some of the colour \
                     under a stroke show through. Height gives the strokes' paint ridges, to \
                     blend into a height."
                ),
                (
                    vec![
                        PlugSpec::new("colour", text!("Colour"), ValueKind::Colour, false),
                        PlugSpec::new("material", text!("Material"), ValueKind::Material, false),
                    ],
                    vec![
                        PlugSpec::new("colour_out", text!("Colour"), ValueKind::Colour, false),
                        PlugSpec::new(
                            "material_out",
                            text!("Material"),
                            ValueKind::Material,
                            false,
                        ),
                        PlugSpec::new("out", text!("Height"), ValueKind::Height, false),
                    ],
                ),
                vec![
                    length(
                        "length",
                        text!("Length"),
                        text!("How long a stroke is, in centimetres."),
                        0.5,
                        200.0,
                        10.0,
                    ),
                    length(
                        "width",
                        text!("Width"),
                        text!("How wide a stroke is, in centimetres."),
                        0.2,
                        100.0,
                        3.0,
                    ),
                    number(
                        "size_vary",
                        text!("Size varies"),
                        text!("How much smaller a stroke may be, at random."),
                        0.0,
                        0.9,
                        0.3,
                    ),
                    number(
                        "direction",
                        text!("Direction"),
                        text!("Which way the strokes run, in degrees, clockwise from across."),
                        -180.0,
                        180.0,
                        -35.0,
                    ),
                    number(
                        "direction_vary",
                        text!("Direction varies"),
                        text!("How far each may stray from that, either way, in degrees."),
                        0.0,
                        180.0,
                        20.0,
                    ),
                    number(
                        "keep_detail",
                        text!("Detail kept"),
                        text!(
                            "How much of the colour under a stroke shows through: 0 is flat \
                             dabs."
                        ),
                        0.0,
                        1.0,
                        0.25,
                    ),
                    number(
                        "colour_vary",
                        text!("Colour varies"),
                        text!("How much lighter, darker or tinted each stroke is, at random."),
                        0.0,
                        1.0,
                        0.25,
                    ),
                    number(
                        "bristles",
                        text!("Bristles"),
                        text!("How strong the streaks from the brush's bristles are."),
                        0.0,
                        1.0,
                        0.4,
                    ),
                    seed(),
                ],
                &["hue_saturation", "edge_highlight", "material_from_height"],
            ),
        }
    }
}

/// The strokes at one pixel: the top one and the one under it (their
/// number plus one, 0 for none) and how much of each covers it, and the
/// paint's height there.
#[derive(Debug, Clone, Copy, Default)]
struct Hit {
    top: u32,
    top_cover: f32,
    top_shade: f32,
    under: u32,
    under_cover: f32,
    under_shade: f32,
    height: f32,
}

impl BoxType for BrushStrokes {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let size = job.size();
        let kinds = self.info.outputs.iter().map(|o| o.kind);
        Ok(self
            .run_outputs(job)?
            .into_iter()
            .zip(kinds)
            .map(|(v, kind)| v.unwrap_or_else(|| moving::plain(kind, size)))
            .collect())
    }

    fn run_outputs(&self, job: &Job<'_>) -> Result<Vec<Option<Value>>, Refusal> {
        let n = job.size().get() as f32;
        let long = job.length_px("length").max(1.0) / n;
        let wide = (job.length_px("width").max(0.5) / n).min(long);
        // About three strokes over each place, so few gaps are left.
        let spacing = (long * wide).sqrt() * 0.6;
        let plan = Plan {
            grid: false,
            spacing: spacing.max(1.0 / n),
            loose: 0.0,
            shift: 0.0,
            size: long,
            size_vary: job.number("size_vary"),
            turn: job.number("direction").to_radians(),
            turn_vary: job.number("direction_vary").to_radians(),
            height_vary: 0.0,
            colour_vary: job.number("colour_vary"),
            apart: false,
            seed: job.seed(),
        };
        let strokes = placing::place(&plan, None);
        let ratio = wide / long;
        let bristles = job.number("bristles").clamp(0.0, 1.0);
        let keep = job.number("keep_detail").clamp(0.0, 1.0);
        let seed = job.seed();
        let hits = hits(job, &strokes, ratio, bristles, seed);
        let height = job.fill_grey(|x, y| hits.at(i64::from(x), i64::from(y)).height);
        let painted = |c: &Rgb| -> Rgb {
            job.fill_rgb(|x, y| {
                let h = hits.at(i64::from(x), i64::from(y));
                let (u, v) = uv(x, y, n);
                let here = c.sample_rgb(u, v);
                let dab = |copy: u32, shade: f32| {
                    let s = &strokes[copy as usize - 1];
                    let flat = c.sample_rgb(s.x, s.y);
                    [0, 1, 2].map(|i| {
                        let base = flat[i] + (here[i] - flat[i]) * keep;
                        base * s.tint[i] * shade
                    })
                };
                let mut out = here;
                for (copy, cover, shade) in [
                    (h.under, h.under_cover, h.under_shade),
                    (h.top, h.top_cover, h.top_shade),
                ] {
                    if copy > 0 {
                        let d = dab(copy, shade);
                        out = [0, 1, 2].map(|i| out[i] + (d[i] - out[i]) * cover);
                    }
                }
                out.map(|v| v.clamp(0.0, 1.0))
            })
        };
        let colour_out = job
            .input("colour")
            .and_then(Value::colour)
            .map(|c| Value::Colour(Arc::new(painted(c))));
        let material_out = job.input("material").and_then(Value::material).map(|m| {
            Value::Material(Arc::new(MaterialMaps {
                colour: Arc::new(painted(&m.colour)),
                ..(**m).clone()
            }))
        });
        Ok(vec![
            colour_out,
            material_out,
            Some(Value::Height(Arc::new(height))),
        ])
    }
}

/// What every pixel sees of `strokes`, each `ratio` as wide as long.
fn hits(job: &Job<'_>, strokes: &[Placed], ratio: f32, bristles: f32, seed: u64) -> Image<Hit> {
    let n = job.size().get() as f32;
    let lookup = Lookup::new(strokes);
    crate::parallel::fill(job.size(), job.threads(), |x, y| {
        let (px, py) = uv(x, y, n);
        let mut hit = Hit::default();
        let (mut best, mut second) = (f32::NEG_INFINITY, f32::NEG_INFINITY);
        for e in lookup.near(px, py) {
            let s = &strokes[e.copy as usize];
            let (dx, dy) = (px - e.x, py - e.y);
            let half_long = s.size * 0.5;
            let half_wide = half_long * ratio;
            if half_wide <= 0.0 {
                continue;
            }
            let a = (s.cos * dx + s.sin * dy) / half_long;
            let b = (s.cos * dy - s.sin * dx) / half_wide;
            // A long rounded shape, squarer along than across.
            let f = a.powi(4) + b * b;
            if f >= 1.0 {
                continue;
            }
            // About how many pixels in from the outline, for a soft edge
            // (narrower on a stroke only a few pixels wide, so it still
            // covers its middle).
            let wide_px = half_wide * n;
            let inside = (1.0 - f) * wide_px * 0.5;
            let cover = smooth(0.0, (wide_px * 0.5).clamp(0.05, 1.5), inside);
            if cover <= 0.0 {
                continue;
            }
            let streak = bristle(seed, e.copy, b, a);
            let shade = 1.0 + (streak - 0.5) * bristles * 0.3;
            // The paint is thicker in the middle and ridged by bristles.
            let thick = cover * (0.55 + 0.25 * (1.0 - b * b) + 0.2 * (streak - 0.5) * bristles);
            hit.height = hit.height.max(thick.clamp(0.0, 1.0));
            // Later strokes (higher rank) are on top.
            let rank = s.rank;
            if rank > best {
                (second, hit.under, hit.under_cover, hit.under_shade) =
                    (best, hit.top, hit.top_cover, hit.top_shade);
                best = rank;
                (hit.top, hit.top_cover, hit.top_shade) = (e.copy + 1, cover, shade);
            } else if rank > second {
                second = rank;
                (hit.under, hit.under_cover, hit.under_shade) = (e.copy + 1, cover, shade);
            }
        }
        hit
    })
}

/// The bristles' streak at `across` (-1 to 1) on stroke `copy`, `along`
/// it (-1 to 1): 0 to 1, a value per bristle, blended between them and
/// fading toward the stroke's tail, where the brush runs dry.
fn bristle(seed: u64, copy: u32, across: f32, along: f32) -> f32 {
    let at = (across * 0.5 + 0.5) * BRISTLES;
    let i = at.floor();
    let t = at - i;
    let value = |k: f32| unit(mix(mix(seed, 0xB215), (u64::from(copy) << 8) ^ k as u64));
    let v = value(i) + (value(i + 1.0) - value(i)) * t * t * (3.0 - 2.0 * t);
    // Toward the tail (along 1), the streaks show more.
    let dry = 0.6 + 0.4 * along.clamp(0.0, 1.0);
    0.5 + (v - 0.5) * dry
}
