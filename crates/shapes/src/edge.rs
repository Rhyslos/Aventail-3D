//! The edge randomiser (Phase 12 milestone 13, decision 0275): an edge
//! setting on a shape that roughens its outline into teeth, saw teeth,
//! waves or a torn edge, the same way every time for one seed.
//!
//! The outline is walked at even steps and each step is pushed out or in
//! along the outline's outward side by a pattern repeating a whole number
//! of times round it, so a closed outline stays closed with no seam. The
//! result is a new path (vectors), so it is as sharp at 4096 pixels as at
//! 64. A shape's edge is shared by its linked copies (it belongs with the
//! shape); varied copies mix their own seed into it.

use crate::path::{Knot, Path};
use glam::Vec2;
use std::ops::RangeInclusive;
use words::text;

/// How deep the teeth are: 1 makes them as deep as they are wide.
pub const AMOUNT: RangeInclusive<f32> = 0.0..=1.0;
/// How wide one tooth is along the outline, as a share of the texture.
pub const SIZE: RangeInclusive<f32> = 0.004..=0.25;
/// The most points a roughened outline gets: past this its teeth are
/// made wider so the count stays within it.
pub const MAX_POINTS: usize = 2048;
/// How closely curves are followed before roughening, a share of the
/// texture (a tenth of a pixel at 8192).
const FLATNESS: f64 = 1e-5;
/// Teeth are never deeper than this share of the shape's narrower side,
/// so a deep edge on a thin shape can't turn it inside out.
const MAX_DEPTH_SHARE: f32 = 0.25;

/// What an edge does to an outline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EdgeKind {
    /// Left as drawn.
    #[default]
    Smooth,
    /// Sharp teeth of uneven heights, like a holly or nettle leaf.
    Jagged,
    /// Saw teeth leaning one way, like a rose or elm leaf.
    Serrated,
    /// Soft, even waves.
    Wavy,
    /// A rough, ragged edge, like torn paper or a bitten leaf.
    Torn,
}

impl EdgeKind {
    /// Every kind, in the order shown.
    pub const ALL: [Self; 5] = [
        Self::Smooth,
        Self::Jagged,
        Self::Serrated,
        Self::Wavy,
        Self::Torn,
    ];

    /// Its plain name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Smooth => text!("Smooth"),
            Self::Jagged => text!("Jagged"),
            Self::Serrated => text!("Serrated"),
            Self::Wavy => text!("Wavy"),
            Self::Torn => text!("Torn"),
        }
    }

    /// What it looks like, in plain words.
    pub fn describe(self) -> &'static str {
        match self {
            Self::Smooth => text!("The outline as you drew it."),
            Self::Jagged => text!("Sharp teeth of uneven heights, like a holly or nettle leaf."),
            Self::Serrated => text!("Saw teeth leaning one way, like a rose or elm leaf."),
            Self::Wavy => text!("Soft, even waves along the outline."),
            Self::Torn => text!("A rough, ragged edge, like torn paper or a nibbled leaf."),
        }
    }

    /// Steps along the outline per tooth (tooth corners fall on steps).
    fn steps(self) -> usize {
        match self {
            Self::Smooth => 1,
            Self::Jagged | Self::Serrated => 4,
            Self::Wavy => 8,
            Self::Torn => 16,
        }
    }

    /// Whether the steps are corners (sharp) or smooth points.
    fn sharp(self) -> bool {
        !matches!(self, Self::Wavy)
    }
}

/// A shape's edge: what kind, how deep, how wide its teeth, and its seed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edge {
    /// What it does.
    pub kind: EdgeKind,
    /// How deep the teeth are ([`AMOUNT`]): 0 leaves the outline as drawn.
    pub amount: f32,
    /// How wide one tooth is along the outline ([`SIZE`], a share of the
    /// texture).
    pub size: f32,
    /// Which teeth: another seed gives other teeth of the same kind.
    pub seed: u32,
}

impl Default for Edge {
    fn default() -> Self {
        Self::SMOOTH
    }
}

impl Edge {
    /// A new shape's edge: smooth, ready to roughen when a kind is picked.
    pub const SMOOTH: Self = Self {
        kind: EdgeKind::Smooth,
        amount: 0.5,
        size: 0.03,
        seed: 1,
    };

    /// Whether it leaves the outline as drawn.
    pub fn is_smooth(&self) -> bool {
        self.kind == EdgeKind::Smooth || self.amount <= 0.0
    }

    /// The same edge with its numbers brought within their ranges, or
    /// `None` when one isn't a number.
    pub fn fitted(self) -> Option<Self> {
        (self.amount.is_finite() && self.size.is_finite()).then(|| Self {
            amount: self.amount.clamp(*AMOUNT.start(), *AMOUNT.end()),
            size: self.size.clamp(*SIZE.start(), *SIZE.end()),
            ..self
        })
    }
}

/// `path` with `edge` along it, its teeth picked by `seed`. Open paths
/// (lines), smooth edges and paths too small to roughen come back as
/// they are. With `fade` (the mirror line's place across), teeth fade out
/// toward that upright line, so a half drawn against the mirror line
/// still meets its mirror image there.
pub fn edged(path: &Path, edge: &Edge, seed: u32, fade: Option<f32>) -> Path {
    let Some(edge) = edge.fitted() else {
        return path.clone();
    };
    if edge.is_smooth() || !path.is_closed() {
        return path.clone();
    }
    let points = path.flattened(FLATNESS);
    let Some(outline) = Outline::new(points) else {
        return path.clone();
    };
    let steps = edge.kind.steps();
    let teeth = ((outline.length / edge.size).round() as usize)
        .max(3)
        .min(MAX_POINTS / steps);
    let count = teeth * steps;
    let tooth = outline.length / teeth as f32;
    let depth = (edge.amount * tooth).min(outline.narrow * MAX_DEPTH_SHARE);
    let pattern = Pattern {
        kind: edge.kind,
        teeth,
        seed,
    };
    let mut at = Vec::with_capacity(count);
    for j in 0..count {
        let s = outline.length * j as f32 / count as f32;
        let (p, outward) = outline.at(s, tooth / steps as f32);
        let weight = fade.map_or(1.0, |x| smoothstep((p.x - x).abs() / tooth));
        let push = pattern.at(j / steps, (j % steps) as f32 / steps as f32);
        at.push(p + outward * (push * depth * 0.5 * weight));
    }
    let knots = if edge.kind.sharp() {
        at.iter().map(|p| Knot::corner(*p)).collect()
    } else {
        smooth_knots(&at)
    };
    Path::new(knots, true)
}

/// A closed outline as straight steps, measured along.
struct Outline {
    points: Vec<Vec2>,
    /// How far along each point is.
    along: Vec<f32>,
    length: f32,
    /// Which way is outward: +1 or -1 times each side's left.
    side: f32,
    /// The narrower side of its rectangle.
    narrow: f32,
}

impl Outline {
    fn new(mut points: Vec<Vec2>) -> Option<Self> {
        points.dedup_by(|a, b| a.distance(*b) < 1e-7);
        while points.len() > 1 && points[0].distance(points[points.len() - 1]) < 1e-7 {
            points.pop();
        }
        if points.len() < 3 || !points.iter().all(|p| p.is_finite()) {
            return None;
        }
        let mut along = Vec::with_capacity(points.len() + 1);
        let mut length = 0.0f32;
        let mut area = 0.0f32;
        let (mut lo, mut hi) = (points[0], points[0]);
        for (i, p) in points.iter().enumerate() {
            let q = points[(i + 1) % points.len()];
            along.push(length);
            length += p.distance(q);
            area += p.perp_dot(q);
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
        along.push(length);
        let narrow = (hi - lo).min_element();
        if length < 1e-6 || narrow < 1e-6 || area.abs() < 1e-12 {
            return None;
        }
        Some(Self {
            points,
            along,
            length,
            // `perp` turns left of the way along: outward when the
            // outline runs round the other way from its area's sign.
            side: if area > 0.0 { -1.0 } else { 1.0 },
            narrow,
        })
    }

    /// The point `s` along (wrapping), and the outward direction there,
    /// from the outline `reach` either side of it.
    fn at(&self, s: f32, reach: f32) -> (Vec2, Vec2) {
        let p = self.point(s);
        let way = self.point(s + reach) - self.point(s - reach);
        let outward = way.perp().normalize_or_zero() * self.side;
        (p, outward)
    }

    fn point(&self, s: f32) -> Vec2 {
        let s = s.rem_euclid(self.length);
        let i = self
            .along
            .partition_point(|a| *a <= s)
            .saturating_sub(1)
            .min(self.points.len() - 1);
        let (a, b) = (self.points[i], self.points[(i + 1) % self.points.len()]);
        let span = self.along[i + 1] - self.along[i];
        let t = if span > 0.0 {
            (s - self.along[i]) / span
        } else {
            0.0
        };
        a + (b - a) * t.clamp(0.0, 1.0)
    }
}

/// The pattern pushing the outline out (positive) or in, -1 to 1, tooth
/// by tooth; it repeats every `teeth` teeth, so it closes round.
struct Pattern {
    kind: EdgeKind,
    teeth: usize,
    seed: u32,
}

impl Pattern {
    /// How far out at `f` (0 to 1) through tooth `i`.
    fn at(&self, i: usize, f: f32) -> f32 {
        let r = |i: usize, k: u32| self.random(i % self.teeth.max(1), k);
        match self.kind {
            EdgeKind::Smooth => 0.0,
            EdgeKind::Jagged => {
                // A valley at the tooth's start, its tip half way.
                let valley = |i| -(0.4 + 0.6 * r(i, 0));
                let tip = 0.4 + 0.6 * r(i, 1);
                if f < 0.5 {
                    lerp(valley(i), tip, f * 2.0)
                } else {
                    lerp(tip, valley(i + 1), (f - 0.5) * 2.0)
                }
            }
            EdgeKind::Serrated => {
                // A slow rise to the tip three quarters along, then a drop.
                let high = |i| 0.75 + 0.25 * r(i, 2);
                if f < 0.75 {
                    lerp(-high(i), high(i), f / 0.75)
                } else {
                    lerp(high(i), -high(i + 1), (f - 0.75) / 0.25)
                }
            }
            EdgeKind::Wavy => {
                let t = 0.5 - 0.5 * (f * std::f32::consts::PI).cos();
                let size = lerp(0.75 + 0.25 * r(i, 3), 0.75 + 0.25 * r(i + 1, 3), t);
                (f * std::f32::consts::TAU).sin() * size
            }
            EdgeKind::Torn => {
                // Rough noise at four scales, the coarsest a tooth wide.
                let x = i as f32 + f;
                let mut sum = 0.0;
                let mut weight = 1.0;
                let mut all = 0.0;
                for octave in 0..4u32 {
                    let scale = (1usize << octave) as f32;
                    sum += self.noise(x * scale, self.teeth << octave, 4 + octave) * weight;
                    all += weight;
                    weight *= 0.5;
                }
                (sum / all * 1.6).clamp(-1.0, 1.0)
            }
        }
    }

    /// Smooth noise, -1 to 1, repeating every `period`.
    fn noise(&self, x: f32, period: usize, k: u32) -> f32 {
        let i = x.floor();
        let t = smoothstep(x - i);
        let i = i as usize;
        let at = |i: usize| self.random(i % period.max(1), k) * 2.0 - 1.0;
        lerp(at(i), at(i + 1), t)
    }

    /// A number from 0 to 1 for step `i` and use `k`.
    fn random(&self, i: usize, k: u32) -> f32 {
        let h = mix(mix(u64::from(self.seed), i as u64), u64::from(k));
        (h >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// Smooth points through `at` (a closed loop), each with handles along
/// the line between its neighbours, so the curve passes through them all.
fn smooth_knots(at: &[Vec2]) -> Vec<Knot> {
    let n = at.len();
    (0..n)
        .map(|i| {
            let (prev, next) = (at[(i + n - 1) % n], at[(i + 1) % n]);
            let out = (next - prev) / 6.0;
            Knot {
                at: at[i],
                inn: -out,
                out,
            }
        })
        .collect()
}

/// Mixes two numbers into a well-spread one (the same each time).
pub(crate) fn mix(a: u64, b: u64) -> u64 {
    let mut z = a ^ b.wrapping_mul(0x9E37_79B9_7F4A_7C15).rotate_left(17);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests;
