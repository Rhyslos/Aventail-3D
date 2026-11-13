//! Scatter: copies of a shape, a colour or a whole material placed in a
//! grid or at random, each with its own size, turn, height and colour
//! (Phase 12, milestone 9; [decision 0271](../../../../docs/decisions/0271-scatter.md)).
//! Good for pebbles, grass tufts, scales and rivets.
//!
//! Each copy is the whole input picture made smaller and turned. Where
//! copies overlap, the higher one is on top (by the shape's height), so
//! a heap of pebbles looks like a heap. Copies wrap over the texture's
//! edges, and every place, size and turn comes from the box's seed, so
//! the texture tiles seamlessly and is the same every time. With nothing
//! plugged in, Scatter places round pebbles.
//!
//! [`placing`] decides where the copies go and keeps a grid of cells so
//! each pixel only looks at the copies near it.

pub(super) mod placing;
#[cfg(test)]
mod tests;

use super::{choice, info, length, moving, number, seed, switch, uv};
use crate::image::{Grey, Image, Rgb};
use crate::kind::ValueKind;
use crate::parallel;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::value::{MaterialMaps, Value};
use placing::{Lookup, Placed, Plan};
use std::sync::Arc;
use words::text;

/// The Scatter box.
#[derive(Debug)]
pub(crate) struct Scatter {
    info: BoxInfo,
}

/// Its outputs' keys, in order.
const OUTPUTS: [&str; 5] = ["material_out", "colour_out", "out", "copies", "random"];

impl Scatter {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "scatter",
                text!("Scatter"),
                Group::Detail,
                text!(
                    "Places copies of a shape, a colour or a whole material at random or in a \
                     grid, each with its own size, turn, height and colour: pebbles, grass tufts, \
                     scales, rivets. Plug the shape of one copy into Shape (a height works too: \
                     its greys become each copy's height); with only a colour or a material, \
                     each copy is the whole picture, square. With nothing plugged in, it places \
                     round pebbles. Where copies overlap, the higher one is on top. Copies reach \
                     over the edges and come back on the other side, so the texture still tiles. \
                     Copies gives where they are, to mix them over something else."
                ),
                (
                    vec![
                        PlugSpec::new("shape", text!("Shape"), ValueKind::Shape, false),
                        PlugSpec::new("colour", text!("Colour"), ValueKind::Colour, false),
                        PlugSpec::new("material", text!("Material"), ValueKind::Material, false),
                        PlugSpec::new("where", text!("Where"), ValueKind::Shape, false),
                    ],
                    vec![
                        PlugSpec::new(OUTPUTS[0], text!("Material"), ValueKind::Material, false),
                        PlugSpec::new(OUTPUTS[1], text!("Colour"), ValueKind::Colour, false),
                        PlugSpec::new(OUTPUTS[2], text!("Height"), ValueKind::Height, false),
                        PlugSpec::new(OUTPUTS[3], text!("Copies"), ValueKind::Shape, false),
                        PlugSpec::new(OUTPUTS[4], text!("Random"), ValueKind::Height, false),
                    ],
                ),
                settings(),
                &[
                    "material_from_height",
                    "gradient_map",
                    "mix_materials",
                    "blend",
                ],
            ),
        }
    }
}

fn settings() -> Vec<crate::settings::SettingSpec> {
    vec![
        choice(
            "arrange",
            text!("Arrange"),
            text!("At random, or in rows and columns (scales, rivets, studs)."),
            &[text!("At random"), text!("In a grid")],
            0,
        ),
        length(
            "spacing",
            text!("Spacing"),
            text!(
                "How far apart the copies are, in centimetres: at random, about one copy in \
                 each square this wide."
            ),
            0.5,
            200.0,
            10.0,
        ),
        length(
            "size",
            text!("Size"),
            text!("How wide the biggest copy is, in centimetres."),
            0.5,
            200.0,
            8.0,
        ),
        number(
            "size_vary",
            text!("Size varies"),
            text!("How much smaller a copy may be, at random: 0.5 is down to half size."),
            0.0,
            0.9,
            0.3,
        ),
        number(
            "turn",
            text!("Turn"),
            text!("How far every copy is turned, in degrees, clockwise."),
            -180.0,
            180.0,
            0.0,
        ),
        number(
            "turn_vary",
            text!("Turn varies"),
            text!("How far each copy may turn either way from that, at random, in degrees."),
            0.0,
            180.0,
            180.0,
        ),
        number(
            "height_vary",
            text!("Height varies"),
            text!("How much lower a copy may be, at random."),
            0.0,
            1.0,
            0.3,
        ),
        number(
            "colour_vary",
            text!("Colour varies"),
            text!("How much each copy's colour may be lighter, darker or tinted, at random."),
            0.0,
            1.0,
            0.2,
        ),
        switch(
            "apart",
            text!("Keep apart"),
            text!(
                "Copies don't overlap: one that would is left out, so there may be fewer than \
                 Spacing asks for."
            ),
        ),
        number(
            "loose",
            text!("Loose"),
            text!("In a grid: how far each copy may stray from its place, as a share of Spacing."),
            0.0,
            1.0,
            0.0,
        ),
        number(
            "shift",
            text!("Shift rows"),
            text!(
                "In a grid: how far every other row moves along, as a share of Spacing (0.5 \
                 for scales)."
            ),
            0.0,
            1.0,
            0.0,
        ),
        seed(),
    ]
}

/// What gives each copy its outline and height.
#[derive(Clone, Copy)]
enum Outline<'a> {
    /// The Shape input: its greys are the copy's height, and it covers
    /// where they aren't black.
    Shape(&'a Grey),
    /// Nothing plugged in at all: a round pebble.
    Pebble,
    /// A colour or material alone: the whole square, as high as the
    /// material (or full height for a colour).
    Square(Option<&'a Grey>),
}

impl Outline<'_> {
    /// The height at `(u, v)` across one copy (0 to 1).
    fn height(self, u: f32, v: f32) -> f32 {
        match self {
            Self::Shape(g) => g.sample(u, v).clamp(0.0, 1.0),
            Self::Pebble => {
                let (a, b) = (u * 2.0 - 1.0, v * 2.0 - 1.0);
                (1.0 - a * a - b * b).max(0.0).sqrt()
            }
            Self::Square(Some(g)) => g.sample(u, v).clamp(0.0, 1.0),
            Self::Square(None) => 1.0,
        }
    }

    /// How much a copy covers where its height is `h` (0 to 1): a shape
    /// fades out over its darkest greys, a square covers all of itself.
    fn cover(self, h: f32) -> f32 {
        match self {
            Self::Shape(_) | Self::Pebble => (h * 10.0).clamp(0.0, 1.0),
            Self::Square(_) => 1.0,
        }
    }

    fn is_square(self) -> bool {
        matches!(self, Self::Square(_))
    }
}

/// What one pixel sees: the copy on top (0 for none, else its number
/// plus one), where on it, how much it covers, and the highest and most
/// covering of every copy there.
#[derive(Debug, Clone, Copy, Default)]
struct Hit {
    copy: u32,
    u: f32,
    v: f32,
    cover: f32,
    height: f32,
    any: f32,
}

impl BoxType for Scatter {
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
        let plan = Plan {
            grid: job.choice("arrange") == 1,
            spacing: job.length_px("spacing").max(1.0) / n,
            loose: job.number("loose"),
            shift: job.number("shift"),
            size: job.length_px("size") / n,
            size_vary: job.number("size_vary"),
            turn: job.number("turn").to_radians(),
            turn_vary: job.number("turn_vary").to_radians(),
            height_vary: job.number("height_vary"),
            colour_vary: job.number("colour_vary"),
            apart: job.switch("apart"),
            seed: job.seed(),
        };
        let copies = placing::place(&plan, job.grey("where").map(|m| &**m));
        let colour = job.input("colour").and_then(Value::colour);
        let material = job.input("material").and_then(Value::material);
        let outline = match (job.grey("shape"), material) {
            (Some(shape), _) => Outline::Shape(shape),
            (None, Some(m)) => Outline::Square(Some(&m.height)),
            (None, None) if colour.is_some() => Outline::Square(None),
            (None, None) => Outline::Pebble,
        };
        let hits = hits(job, &copies, outline);
        let at = |x: u32, y: u32| hits.at(i64::from(x), i64::from(y));
        let top = |h: &Hit| (h.copy > 0).then(|| &copies[h.copy as usize - 1]);
        let height = job.fill_grey(|x, y| at(x, y).height);
        let covered = job.fill_grey(|x, y| at(x, y).any);
        let random = job.fill_grey(|x, y| {
            let h = at(x, y);
            top(&h).map_or(0.0, |c| c.grey * h.cover)
        });
        let colour_out = colour.map(|picture| {
            Value::Colour(Arc::new(job.fill_rgb(|x, y| {
                let h = at(x, y);
                top(&h).map_or([0.0; 3], |c| {
                    tinted(picture.sample_rgb(h.u, h.v), c.tint).map(|v| v * h.cover)
                })
            })))
        });
        let material_out =
            material.map(|m| Value::Material(Arc::new(scattered(job, m, &copies, &hits))));
        Ok(vec![
            material_out,
            colour_out,
            Some(Value::Height(Arc::new(height))),
            Some(Value::Shape(Arc::new(covered))),
            Some(Value::Height(Arc::new(random))),
        ])
    }
}

/// What every pixel sees of `copies`, outlined by `outline`.
fn hits(job: &Job<'_>, copies: &[Placed], outline: Outline<'_>) -> Image<Hit> {
    let n = job.size().get() as f32;
    let lookup = Lookup::new(copies);
    parallel::fill(job.size(), job.threads(), |x, y| {
        let (px, py) = uv(x, y, n);
        let mut hit = Hit::default();
        let mut best = f32::NEG_INFINITY;
        for e in lookup.near(px, py) {
            let c = &copies[e.copy as usize];
            let (dx, dy) = (px - e.x, py - e.y);
            let reach = c.reach();
            if dx.abs() > reach || dy.abs() > reach {
                continue;
            }
            // Turned back and made full size: where on the input it is.
            let lu = (c.cos * dx + c.sin * dy) / c.size;
            let lv = (c.cos * dy - c.sin * dx) / c.size;
            let far = lu.abs().max(lv.abs());
            if far >= 0.5 {
                continue;
            }
            // A soft edge one pixel wide, so square copies aren't jagged.
            let edge = ((0.5 - far) * c.size * n + 0.5).min(1.0);
            let (u, v) = (lu + 0.5, lv + 0.5);
            let h = outline.height(u, v);
            let cover = edge * outline.cover(h);
            if cover <= 0.0 {
                continue;
            }
            let tall = h * c.height;
            hit.height = hit.height.max(tall * cover);
            hit.any = hit.any.max(cover);
            let on_top = if outline.is_square() {
                c.rank
            } else {
                tall + c.rank * 1e-3
            };
            if on_top > best {
                best = on_top;
                hit.copy = e.copy + 1;
                (hit.u, hit.v, hit.cover) = (u, v, cover);
            }
        }
        hit
    })
}

/// `colour` with a copy's `tint`, kept from 0 to 1.
fn tinted(colour: [f32; 3], tint: [f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|i| (colour[i] * tint[i]).clamp(0.0, 1.0))
}

/// The material `m` scattered as `hits` say: each map from the copy on
/// top, its colour tinted, its height lowered, its normals turned with
/// it and steeper for being smaller; black, low and flat between copies.
fn scattered(
    job: &Job<'_>,
    m: &MaterialMaps,
    copies: &[Placed],
    hits: &Image<Hit>,
) -> MaterialMaps {
    let at = |x: u32, y: u32| {
        let h = hits.at(i64::from(x), i64::from(y));
        ((h.copy > 0).then(|| &copies[h.copy as usize - 1]), h)
    };
    let grey = |g: &Grey, gap: f32, lowered: bool| {
        Arc::new(job.fill_grey(|x, y| match at(x, y) {
            (Some(c), h) => {
                let v = g.sample(h.u, h.v) * if lowered { c.height } else { 1.0 };
                gap + (v - gap) * h.cover
            }
            (None, _) => gap,
        }))
    };
    let colour: Rgb = job.fill_rgb(|x, y| match at(x, y) {
        (Some(c), h) => tinted(m.colour.sample_rgb(h.u, h.v), c.tint).map(|v| v * h.cover),
        (None, _) => [0.0; 3],
    });
    let normal: Rgb = job.fill_rgb(|x, y| {
        let (Some(c), h) = at(x, y) else {
            return [0.0, 0.0, 1.0];
        };
        let [nx, ny, nz] = m.normal.sample_rgb(h.u, h.v);
        // Turned clockwise with the copy (y is up the texture), then
        // steeper for a copy smaller than the texture and gentler for a
        // lower one.
        let steeper = c.height / c.size.max(1e-4);
        let tx = (nx * c.cos + ny * c.sin) * steeper;
        let ty = (ny * c.cos - nx * c.sin) * steeper;
        let turned = glam::Vec3::new(tx, ty, nz).normalize_or(glam::Vec3::Z);
        glam::Vec3::Z
            .lerp(turned, h.cover)
            .normalize_or(glam::Vec3::Z)
            .to_array()
    });
    MaterialMaps {
        colour: Arc::new(colour),
        height: grey(&m.height, 0.0, true),
        roughness: grey(&m.roughness, 0.5, false),
        metal: grey(&m.metal, 0.0, false),
        normal: Arc::new(normal),
        occlusion: grey(&m.occlusion, 1.0, false),
        curvature: grey(&m.curvature, 0.5, false),
    }
}
