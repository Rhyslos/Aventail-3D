//! Stamps (Phase 12 milestone 22, [decision 0285](../../../../docs/decisions/0285-stamp-images.md)):
//! small pictures of your own (a rivet, a leaf, a rune) scattered as
//! stamps, as Scatter scatters a shape.
//!
//! The box holds its pictures ([`crate::Stamps`], brought in through
//! Attributes' **Add a stamp…**, see-through parts kept). Copies are
//! placed by Scatter's `placing` (at random or in a grid, keep apart,
//! the Where mask), each picking one of the pictures at random by their
//! weights, with its own size, turn, colour and height. Copies wrap over
//! the texture's edges and everything comes from the seed, so the result
//! tiles and is the same every time.
//!
//! Where copies overlap, the one ranked higher lies on top, its
//! see-through edges blended over the ones under it (the order is the
//! copies' `rank`, as Scatter's square copies use). A copy's height is
//! its picture's outline (or its lightness) **puffed**: rounded up from
//! the edge, like a stamp pressed into clay from behind.
//!
//! The pick of a picture is drawn from its own hash of the seed and the
//! copy's place, never from `placing`'s stream, so Scatter's draw order
//! (and every graph's look) is untouched.

use super::material_from_height::normal;
use super::scatter::placing::{self, Lookup, Placed, Plan};
use super::{choice, info, length, number, seed, switch, uv};
use crate::image::{Grey, Image, Rgb};
use crate::kind::ValueKind;
use crate::painting::lightness;
use crate::parallel;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::seed::{mix, unit};
use crate::stamp::{Stamp, Stamps};
use crate::value::{MaterialMaps, Value};
use std::sync::{Arc, OnceLock};
use words::{text, tr};

/// The box's id.
pub const STAMPS: &str = "stamps";

/// The setting choosing where a stamp's height comes from.
pub const HEIGHT_FROM: &str = "height_from";
/// The setting rounding a stamp's height up from its edge.
pub const PUFF: &str = "puff";

/// What the box says when it holds no stamps yet.
pub const NO_STAMPS: &str = text!(
    "No stamps brought in yet. Choose Add a stamp… in Attributes to pick a small picture (a PNG \
     with see-through parts keeps its outline; JPEG, TGA and BMP work too)."
);

/// How deep a stamp's bumps look on a material (Material from height's
/// standard Depth, in hundredths of the texture's width).
const DEPTH: f32 = 2.0;

/// The most copies blended at one pixel: copies under more than this
/// many others are hidden anyway.
const MOST_LAYERS: usize = 12;

/// Its outputs' keys, in order (Scatter's).
const OUTPUTS: [&str; 5] = ["material_out", "colour_out", "out", "copies", "random"];

/// The Stamps box.
#[derive(Debug)]
pub(crate) struct StampsBox {
    info: BoxInfo,
}

impl StampsBox {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                STAMPS,
                text!("Stamps"),
                Group::Detail,
                text!(
                    "Scatters small pictures of your own (a rivet, a leaf, a rune) as stamps, \
                     at random or in a grid, each copy picking one of the pictures (Add a \
                     stamp… in Attributes) with its own size, turn, colour and height. A PNG's \
                     see-through parts give each stamp its outline. Colour gives the stamps in \
                     their own colours (over what is plugged into Under), Height their height \
                     (their outline or lightness, puffed up from the edge), Copies where they \
                     are. Plug a material in to press the stamps onto it. Copies reach over \
                     the edges and come back on the other side, so the texture still tiles."
                ),
                (
                    vec![
                        PlugSpec::new("colour", text!("Under"), ValueKind::Colour, false),
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
                    "mix_materials",
                    "blend",
                    "hue_saturation",
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
            text!("At random, or in rows and columns (rivets, studs, tiles of runes)."),
            &[text!("At random"), text!("In a grid")],
            0,
        ),
        length(
            "spacing",
            text!("Spacing"),
            text!(
                "How far apart the stamps are, in centimetres: at random, about one stamp in \
                 each square this wide."
            ),
            0.5,
            200.0,
            15.0,
        ),
        length(
            "size",
            text!("Size"),
            text!("How wide the biggest stamp is, in centimetres (its longer side)."),
            0.5,
            200.0,
            10.0,
        ),
        number(
            "size_vary",
            text!("Size varies"),
            text!("How much smaller a stamp may be, at random: 0.5 is down to half size."),
            0.0,
            0.9,
            0.3,
        ),
        number(
            "turn",
            text!("Turn"),
            text!("How far every stamp is turned, in degrees, clockwise."),
            -180.0,
            180.0,
            0.0,
        ),
        number(
            "turn_vary",
            text!("Turn varies"),
            text!(
                "How far each stamp may turn either way from that, at random, in degrees (0 \
                 keeps letters and runes upright)."
            ),
            0.0,
            180.0,
            180.0,
        ),
        number(
            "height_vary",
            text!("Height varies"),
            text!("How much lower a stamp may be, at random."),
            0.0,
            1.0,
            0.3,
        ),
        number(
            "colour_vary",
            text!("Colour varies"),
            text!("How much each stamp's colours may be lighter, darker or tinted, at random."),
            0.0,
            1.0,
            0.2,
        ),
        // On to start with: stamps (rivets, runes) usually sit side by side.
        crate::settings::SettingSpec {
            kind: crate::settings::SettingKind::Switch { standard: true },
            ..switch(
                "apart",
                text!("Keep apart"),
                text!(
                    "Stamps don't overlap: one that would is left out, so there may be fewer \
                     than Spacing asks for. Off, they pile up, the higher-ranked on top."
                ),
            )
        },
        number(
            "loose",
            text!("Loose"),
            text!("In a grid: how far each stamp may stray from its place, as a share of Spacing."),
            0.0,
            1.0,
            0.0,
        ),
        number(
            "shift",
            text!("Shift rows"),
            text!(
                "In a grid: how far every other row moves along, as a share of Spacing (0.5 \
                 for a brick-like layout)."
            ),
            0.0,
            1.0,
            0.0,
        ),
        choice(
            HEIGHT_FROM,
            text!("Height from"),
            text!(
                "Where a stamp's height comes from: its outline (flat on top, as high as it \
                 is solid), or its lightness (light parts higher, so a drawn leaf keeps its \
                 veins)."
            ),
            &[text!("Its outline"), text!("Its lightness")],
            0,
        ),
        number(
            PUFF,
            text!("Puff"),
            text!(
                "How far in from its edge a stamp's height rounds up, as a share of the way to \
                 its middle: 0 is a flat step, 1 a dome."
            ),
            0.0,
            1.0,
            0.3,
        ),
        seed(),
    ]
}

/// One stamp made ready for a job: its pixels (premultiplied colour and
/// alpha) and its height times its alpha, at about the size the biggest
/// copy is drawn, so small copies don't shimmer.
struct Ready {
    width: u32,
    height: u32,
    /// The longer side, in its pixels: the side of a copy's square.
    side: f32,
    /// Red, green, blue (times alpha) and alpha, row by row from the top.
    pixels: Vec<[f32; 4]>,
    /// Height times alpha.
    heights: Vec<f32>,
}

impl Ready {
    /// `stamp` at most `longest` pixels along its longer side, its height
    /// from its lightness (`lit`) or outline, puffed by `puff`.
    fn of(stamp: &Stamp, longest: u32, lit: bool, puff: f32) -> Self {
        let (w, h) = (stamp.width(), stamp.height());
        let long = w.max(h);
        let scale = (longest.max(2) as f32 / long as f32).min(1.0);
        let nw = ((w as f32 * scale).round() as u32).max(1);
        let nh = ((h as f32 * scale).round() as u32).max(1);
        let pixels = shrunk(stamp, nw, nh);
        let inside = distances(&pixels, nw, nh);
        let deepest = inside.iter().copied().fold(0.0f32, f32::max).max(1e-3);
        let puff = if puff.is_finite() {
            puff.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let heights = pixels
            .iter()
            .zip(&inside)
            .map(|(p, d)| {
                let a = p[3];
                if a <= 0.0 {
                    return 0.0;
                }
                let source = if lit {
                    lightness([p[0] / a, p[1] / a, p[2] / a])
                } else {
                    1.0
                };
                let round = if puff > 0.0 {
                    let t = (d / (puff * deepest)).clamp(0.0, 1.0);
                    (1.0 - (1.0 - t) * (1.0 - t)).sqrt()
                } else {
                    1.0
                };
                source * round * a
            })
            .collect();
        Self {
            width: nw,
            height: nh,
            // Half a pixel of room each side, so edges fade inside the
            // copy's square rather than being cut by it.
            side: nw.max(nh) as f32 + 1.0,
            pixels,
            heights,
        }
    }

    /// Colour and alpha (premultiplied) and height times alpha at `(x,
    /// y)` in its pixels (0 its top left corner), blended between the four
    /// nearest; nothing outside it.
    fn sample(&self, x: f32, y: f32) -> ([f32; 4], f32) {
        let (fx, fy) = (x - 0.5, y - 0.5);
        let (x0, y0) = (fx.floor(), fy.floor());
        let (tx, ty) = (fx - x0, fy - y0);
        let (x0, y0) = (x0 as i64, y0 as i64);
        let mut colour = [0.0f32; 4];
        let mut height = 0.0;
        for (dx, dy, wgt) in [
            (0, 0, (1.0 - tx) * (1.0 - ty)),
            (1, 0, tx * (1.0 - ty)),
            (0, 1, (1.0 - tx) * ty),
            (1, 1, tx * ty),
        ] {
            let (px, py) = (x0 + dx, y0 + dy);
            if px < 0 || py < 0 || px >= i64::from(self.width) || py >= i64::from(self.height) {
                continue;
            }
            let at = py as usize * self.width as usize + px as usize;
            let p = self.pixels[at];
            for k in 0..4 {
                colour[k] += p[k] * wgt;
            }
            height += self.heights[at] * wgt;
        }
        (colour, height)
    }
}

/// `stamp` made `w` × `h` pixels (no bigger than it is), each pixel the
/// average of those it covers, premultiplied, 0 to 1.
fn shrunk(stamp: &Stamp, w: u32, h: u32) -> Vec<[f32; 4]> {
    let (sw, sh) = (stamp.width() as usize, stamp.height() as usize);
    let rgba = stamp.rgba();
    let premultiplied = |i: usize| {
        let p = &rgba[i * 4..i * 4 + 4];
        let a = f32::from(p[3]) / 255.0;
        [
            f32::from(p[0]) / 255.0 * a,
            f32::from(p[1]) / 255.0 * a,
            f32::from(p[2]) / 255.0 * a,
            a,
        ]
    };
    if w as usize == sw && h as usize == sh {
        return (0..sw * sh).map(premultiplied).collect();
    }
    let span = |i: u32, n: u32, s: usize| {
        let lo = (i as usize * s) / n as usize;
        let hi = (((i + 1) as usize * s).div_ceil(n as usize)).clamp(lo + 1, s);
        lo..hi
    };
    let mut out = Vec::with_capacity((w * h) as usize);
    for y in 0..h {
        let ys = span(y, h, sh);
        for x in 0..w {
            let xs = span(x, w, sw);
            let mut sum = [0.0f32; 4];
            for sy in ys.clone() {
                for sx in xs.clone() {
                    let p = premultiplied(sy * sw + sx);
                    for k in 0..4 {
                        sum[k] += p[k];
                    }
                }
            }
            let count = (ys.len() * xs.len()) as f32;
            out.push(sum.map(|v| v / count));
        }
    }
    out
}

/// How far each pixel of `pixels` (`w` × `h`) is inside its outline (where
/// it is at least half solid), in pixels: 0 outside, about 0.5 at the
/// edge; past the picture's sides counts as outside.
fn distances(pixels: &[[f32; 4]], w: u32, h: u32) -> Vec<f32> {
    const DIAGONAL: f32 = std::f32::consts::SQRT_2;
    let (w, h) = (w as usize, h as usize);
    let big = (w + h) as f32 * 2.0;
    let mut d: Vec<f32> = pixels
        .iter()
        .map(|p| if p[3] >= 0.5 { big } else { 0.0 })
        .collect();
    let at = |d: &[f32], x: isize, y: isize| -> f32 {
        if x < 0 || y < 0 || x >= w as isize || y >= h as isize {
            0.0
        } else {
            d[y as usize * w + x as usize]
        }
    };
    for y in 0..h as isize {
        for x in 0..w as isize {
            let i = y as usize * w + x as usize;
            if d[i] == 0.0 {
                continue;
            }
            let best = (at(&d, x - 1, y) + 1.0)
                .min(at(&d, x, y - 1) + 1.0)
                .min(at(&d, x - 1, y - 1) + DIAGONAL)
                .min(at(&d, x + 1, y - 1) + DIAGONAL);
            d[i] = d[i].min(best);
        }
    }
    for y in (0..h as isize).rev() {
        for x in (0..w as isize).rev() {
            let i = y as usize * w + x as usize;
            if d[i] == 0.0 {
                continue;
            }
            let best = (at(&d, x + 1, y) + 1.0)
                .min(at(&d, x, y + 1) + 1.0)
                .min(at(&d, x + 1, y + 1) + DIAGONAL)
                .min(at(&d, x - 1, y + 1) + DIAGONAL);
            d[i] = d[i].min(best);
        }
    }
    d.iter_mut().for_each(|v| *v = (*v - 0.5).max(0.0));
    d
}

/// What one pixel sees once every copy over it is blended: colour and
/// coverage (premultiplied), height and the copies' greys times coverage.
#[derive(Debug, Clone, Copy, Default)]
struct Seen {
    rgb: [f32; 3],
    cover: f32,
    height: f32,
    grey: f32,
}

/// One copy's part at a pixel, before blending.
#[derive(Debug, Clone, Copy, Default)]
struct Layer {
    rank: f32,
    /// Orders a big copy's parts seen twice at one pixel (it reaches
    /// round the texture onto itself) the same way wherever they meet.
    tie: f32,
    colour: [f32; 4],
    height: f32,
    grey: f32,
}

/// Whether layer `p` is under layer `q`.
fn below(p: &Layer, q: &Layer) -> std::cmp::Ordering {
    p.rank.total_cmp(&q.rank).then(p.tie.total_cmp(&q.tie))
}

/// The stamp used when the box holds none, for callers wanting every
/// output (the shared box checks): a white round dot.
fn standard_stamps() -> Option<&'static Stamps> {
    static DOT: OnceLock<Option<Stamps>> = OnceLock::new();
    DOT.get_or_init(|| {
        let n = 32u32;
        let mut rgba = Vec::with_capacity((n * n * 4) as usize);
        for y in 0..n {
            for x in 0..n {
                let (dx, dy) = (x as f32 + 0.5 - 16.0, y as f32 + 0.5 - 16.0);
                let a = (14.0 - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0);
                rgba.extend_from_slice(&[255, 255, 255, (a * 255.0).round() as u8]);
            }
        }
        let dot = Stamp::new(n, n, rgba, Vec::new(), "dot").ok()?;
        Stamps::one(dot).ok()
    })
    .as_ref()
}

/// Which stamp copy `c` picks, from its own hash of `seed` and its place
/// (never from the placing's stream).
fn pick(stamps: &Stamps, seed: u64, c: &Placed) -> usize {
    let place = (u64::from(c.x.to_bits()) << 32) | u64::from(c.y.to_bits());
    stamps.pick(unit(mix(mix(seed, 0x057A_49E5), place)))
}

impl BoxType for StampsBox {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn takes_stamps(&self) -> bool {
        true
    }

    /// Every output, with the stamps held or (holding none) a white dot,
    /// so every output can be checked; Material is plain without one
    /// plugged in.
    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let size = job.size();
        let stamps: &Stamps = match job.stamps() {
            Some(s) => s,
            None => standard_stamps().ok_or_else(|| Refusal::new(tr(NO_STAMPS)))?,
        };
        let kinds = self.info.outputs.iter().map(|o| o.kind);
        Ok(stamped(job, stamps)
            .into_iter()
            .zip(kinds)
            .map(|(v, kind)| v.unwrap_or_else(|| super::moving::plain(kind, size)))
            .collect())
    }

    fn run_outputs(&self, job: &Job<'_>) -> Result<Vec<Option<Value>>, Refusal> {
        let Some(stamps) = job.stamps() else {
            return Err(Refusal::new(tr(NO_STAMPS)));
        };
        Ok(stamped(job, stamps))
    }
}

/// The box's outputs with `stamps`: the material only with one plugged
/// in.
fn stamped(job: &Job<'_>, stamps: &Stamps) -> Vec<Option<Value>> {
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
    let picks: Vec<usize> = copies.iter().map(|c| pick(stamps, plan.seed, c)).collect();
    // Each picture at about the size the biggest copy is drawn.
    let longest = (plan.size * n)
        .ceil()
        .clamp(2.0, crate::MAX_STAMP_SIDE as f32) as u32;
    let lit = job.choice(HEIGHT_FROM) == 1;
    let puff = job.number(PUFF);
    let ready: Vec<Ready> = stamps
        .entries()
        .iter()
        .map(|e| Ready::of(&e.stamp, longest, lit, puff))
        .collect();
    let seen = seen(job, &copies, &picks, &ready);
    let at = |x: u32, y: u32| seen.at(i64::from(x), i64::from(y));
    let height = job.fill_grey(|x, y| at(x, y).height);
    let covered = job.fill_grey(|x, y| at(x, y).cover);
    let random = job.fill_grey(|x, y| at(x, y).grey);
    let under = job.input("colour").and_then(Value::colour);
    let over = |s: Seen, below: [f32; 3]| {
        [0, 1, 2].map(|k| (s.rgb[k] + below[k] * (1.0 - s.cover)).clamp(0.0, 1.0))
    };
    let colour = job.fill_rgb(|x, y| {
        let below = under.map_or([0.0; 3], |u| u.at(i64::from(x), i64::from(y)));
        over(at(x, y), below)
    });
    let material = job
        .input("material")
        .and_then(Value::material)
        .map(|m| Value::Material(Arc::new(pressed(job, m, &seen))));
    vec![
        material,
        Some(Value::Colour(Arc::new(colour))),
        Some(Value::Height(Arc::new(height))),
        Some(Value::Shape(Arc::new(covered))),
        Some(Value::Height(Arc::new(random))),
    ]
}

/// What every pixel sees of `copies`, each drawn with its picked stamp,
/// blended in rank order.
fn seen(job: &Job<'_>, copies: &[Placed], picks: &[usize], ready: &[Ready]) -> Image<Seen> {
    let n = job.size().get() as f32;
    let lookup = Lookup::new(copies);
    parallel::fill(job.size(), job.threads(), |x, y| {
        let (px, py) = uv(x, y, n);
        let mut layers = [Layer::default(); MOST_LAYERS];
        let mut count = 0;
        for e in lookup.near(px, py) {
            let c = &copies[e.copy as usize];
            let (dx, dy) = (px - e.x, py - e.y);
            let reach = c.reach();
            if dx.abs() > reach || dy.abs() > reach {
                continue;
            }
            let lu = (c.cos * dx + c.sin * dy) / c.size;
            let lv = (c.cos * dy - c.sin * dx) / c.size;
            if lu.abs() > 0.5 + 1.0 / (c.size * n) || lv.abs() > 0.5 + 1.0 / (c.size * n) {
                continue;
            }
            let r = &ready[picks[e.copy as usize]];
            let sx = lu * r.side + r.width as f32 * 0.5;
            let sy = lv * r.side + r.height as f32 * 0.5;
            let (colour, height) = r.sample(sx, sy);
            if colour[3] <= 1e-4 {
                continue;
            }
            let a = colour[3];
            let layer = Layer {
                rank: c.rank,
                tie: dx * 7.0 + dy,
                colour: [
                    (colour[0] * c.tint[0]).min(a),
                    (colour[1] * c.tint[1]).min(a),
                    (colour[2] * c.tint[2]).min(a),
                    a,
                ],
                height: height * c.height,
                grey: c.grey * a,
            };
            if count < MOST_LAYERS {
                layers[count] = layer;
                count += 1;
            } else if let Some(lowest) = layers
                .iter_mut()
                .min_by(|p, q| below(p, q))
                .filter(|l| below(l, &layer).is_lt())
            {
                *lowest = layer;
            }
        }
        let layers = &mut layers[..count];
        layers.sort_by(below);
        let mut s = Seen::default();
        for l in layers.iter() {
            let keep = 1.0 - l.colour[3];
            for k in 0..3 {
                s.rgb[k] = l.colour[k] + s.rgb[k] * keep;
            }
            s.cover = l.colour[3] + s.cover * keep;
            s.height = l.height + s.height * keep;
            s.grey = l.grey + s.grey * keep;
        }
        s
    })
}

/// Material `m` with the stamps `seen` pressed onto it: their colours
/// over its colour, their height standing on its height, its normals
/// bent by their edges, solid where they are; its shine, metal, occlusion
/// and curvature kept.
fn pressed(job: &Job<'_>, m: &MaterialMaps, seen: &Image<Seen>) -> MaterialMaps {
    let at = |x: u32, y: u32| seen.at(i64::from(x), i64::from(y));
    let colour: Rgb = job.fill_rgb(|x, y| {
        let s = at(x, y);
        let below = m.colour.at(i64::from(x), i64::from(y));
        [0, 1, 2].map(|k| (s.rgb[k] + below[k] * (1.0 - s.cover)).clamp(0.0, 1.0))
    });
    // How much the stamps raise each place: they stand on the material,
    // never above white.
    let raised: Grey = job.fill_grey(|x, y| {
        let base = m.height.at(i64::from(x), i64::from(y)).clamp(0.0, 1.0);
        at(x, y).height.clamp(0.0, 1.0) * (1.0 - base)
    });
    let height = job.fill_grey(|x, y| {
        let (x, y) = (i64::from(x), i64::from(y));
        (m.height.at(x, y) + raised.at(x, y)).clamp(0.0, 1.0)
    });
    let bent = normal(job, &raised, DEPTH);
    let normals: Rgb = job.fill_rgb(|x, y| {
        let (x, y) = (i64::from(x), i64::from(y));
        let [bx, by, bz] = m.normal.at(x, y);
        let [sx, sy, sz] = bent.at(x, y);
        glam::Vec3::new(bx + sx, by + sy, bz * sz)
            .normalize_or(glam::Vec3::Z)
            .to_array()
    });
    let opacity = m.opacity.as_ref().map(|o| {
        Arc::new(job.fill_grey(|x, y| {
            let s = at(x, y);
            let below = o.at(i64::from(x), i64::from(y));
            s.cover + below * (1.0 - s.cover)
        }))
    });
    MaterialMaps {
        colour: Arc::new(colour),
        height: Arc::new(height),
        normal: Arc::new(normals),
        opacity,
        ..m.clone()
    }
}

#[cfg(test)]
mod tests;
