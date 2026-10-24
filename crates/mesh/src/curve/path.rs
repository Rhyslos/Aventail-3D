//! Following a curve: the places along it a solid is built at, and the
//! frames (which way is "across" and "up") that turn a cross-section to
//! follow it without flipping.

use super::{Curve, Span};
use glam::{Quat, Vec3};

/// How finely a solid follows its curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Detail {
    /// With `adaptive`, pieces for each quarter turn the curve makes (a
    /// straight stretch is one piece); otherwise pieces for every stretch
    /// between two points. From 1 to [`Detail::MAX_SEGMENTS`].
    pub segments: u32,
    /// Whether bends get more pieces and straight stretches fewer.
    pub adaptive: bool,
}

impl Default for Detail {
    fn default() -> Self {
        Self {
            segments: 6,
            adaptive: true,
        }
    }
}

impl Detail {
    /// The most pieces per stretch (or per quarter turn).
    pub const MAX_SEGMENTS: u32 = 64;

    /// The same detail within limits.
    pub fn clamped(self) -> Self {
        Self {
            segments: self.segments.clamp(1, Self::MAX_SEGMENTS),
            ..self
        }
    }
}

/// Where a sharp corner of the curve turns: the way in and the way out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Corner {
    pub(crate) before: Vec3,
    pub(crate) after: Vec3,
}

impl Corner {
    /// How sharply it turns, in radians.
    pub(crate) fn angle(&self) -> f32 {
        self.before.dot(self.after).clamp(-1.0, 1.0).acos()
    }
}

/// One place along the curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Sample {
    pub(crate) position: Vec3,
    /// Which way the curve goes here (unit; halfway at a corner).
    pub(crate) tangent: Vec3,
    /// Set where the curve turns sharply at a point.
    pub(crate) corner: Option<Corner>,
    /// How far along the curve it is, from the start.
    pub(crate) along: f32,
}

/// The places a solid is built at along a curve.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Samples {
    pub(crate) samples: Vec<Sample>,
    /// Whether the last joins the first (a closed curve).
    pub(crate) closed: bool,
    /// The whole length, round to the start again when closed.
    pub(crate) length: f32,
}

/// Corners turning less than this (radians) are smooth.
const CORNER_FROM: f32 = 0.01;
/// The most pieces one stretch is cut into.
const MAX_PIECES: u32 = 256;

/// How much a stretch turns, in radians: the angles between its
/// directions, a few places along.
fn turning(span: &Span) -> f32 {
    const LOOKS: usize = 16;
    let ways: Vec<Vec3> = (0..=LOOKS)
        .filter_map(|i| span.direction(i as f32 / LOOKS as f32).try_normalize())
        .collect();
    ways.windows(2)
        .map(|w| w[0].dot(w[1]).clamp(-1.0, 1.0).acos())
        .sum()
}

/// The places along `curve` with `detail`; `twist` (radians, over the
/// whole length) asks for more pieces where the cross-section turns.
/// Stretches of no length are skipped; a curve with none left has no
/// places.
pub(crate) fn samples(curve: &Curve, detail: Detail, twist: f32) -> Samples {
    let detail = detail.clamped();
    let spans: Vec<Span> = curve
        .spans()
        .into_iter()
        .filter(|s| s.length_estimate() > 1e-4)
        .collect();
    if spans.is_empty() {
        return Samples::default();
    }
    let closed = curve.closed() && spans.len() >= 2;
    let total: f32 = spans
        .iter()
        .map(Span::length_estimate)
        .sum::<f32>()
        .max(1e-6);
    let quarter = std::f32::consts::FRAC_PI_2;
    let pieces: Vec<u32> = spans
        .iter()
        .map(|span| {
            if !detail.adaptive {
                return detail.segments;
            }
            let bend = (turning(span) / quarter * detail.segments as f32).ceil();
            let share = twist.abs() * span.length_estimate() / total;
            let turn = (share / quarter * detail.segments as f32).ceil();
            (bend.max(turn) as u32).clamp(1, MAX_PIECES)
        })
        .collect();
    let mut samples = Vec::new();
    for (k, span) in spans.iter().enumerate() {
        let n = pieces[k];
        for step in 0..n {
            let t = step as f32 / n as f32;
            let way = span.direction(t).normalize_or(Vec3::X);
            let mut sample = Sample {
                position: span.at(t),
                tangent: way,
                corner: None,
                along: 0.0,
            };
            // At a point: a corner if the way in differs from the way out.
            let previous = if k > 0 {
                Some(&spans[k - 1])
            } else if closed {
                spans.last()
            } else {
                None
            };
            if step == 0
                && let Some(before) = previous.and_then(|p| p.direction(1.0).try_normalize())
            {
                let corner = Corner { before, after: way };
                if corner.angle() > CORNER_FROM {
                    sample.tangent = (before + way).try_normalize().unwrap_or(way);
                    sample.corner = Some(corner);
                } else {
                    sample.tangent = (before + way).normalize_or(way);
                }
            }
            samples.push(sample);
        }
    }
    if !closed && let Some(last) = spans.last() {
        samples.push(Sample {
            position: last.end,
            tangent: last.direction(1.0).normalize_or(Vec3::X),
            corner: None,
            along: 0.0,
        });
    }
    let mut along = 0.0;
    for i in 1..samples.len() {
        along += samples[i].position.distance(samples[i - 1].position);
        samples[i].along = along;
    }
    if closed && let (Some(first), Some(last)) = (samples.first(), samples.last()) {
        along += first.position.distance(last.position);
    }
    Samples {
        samples,
        closed,
        length: along,
    }
}

/// A frame at a place along the curve: `tangent` the way it goes, `up`
/// and `across` square to it and to each other (`across` = `up` ×
/// `tangent`), so a cross-section's x goes across and its y up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Frame {
    pub(crate) tangent: Vec3,
    pub(crate) up: Vec3,
    pub(crate) across: Vec3,
}

/// The first frame's up: the world's up (Z) made square to the way the
/// curve starts, or X where it starts straight up or down.
pub(crate) fn first_up(tangent: Vec3) -> Vec3 {
    let up = Vec3::Z - tangent * tangent.dot(Vec3::Z);
    up.try_normalize()
        .unwrap_or_else(|| (Vec3::X - tangent * tangent.dot(Vec3::X)).normalize_or(Vec3::Y))
}

/// Frames that follow the curve turning as little as possible
/// (rotation-minimising frames, by double reflection: Wang, Jüttler,
/// Zheng and Liu, 2008), so a cross-section never flips or spins on its
/// own. On a closed curve the small turn left over when it comes round
/// is spread evenly along it, so the ends meet. `twist` (radians) turns
/// the cross-section evenly along the length on top.
pub(crate) fn frames(samples: &Samples, twist: f32) -> Vec<Frame> {
    let list = &samples.samples;
    let Some(first) = list.first() else {
        return Vec::new();
    };
    let mut ups = Vec::with_capacity(list.len() + 1);
    ups.push(first_up(first.tangent));
    let step = |from: &Sample, to: (Vec3, Vec3), up: Vec3| -> Vec3 {
        let (position, tangent) = to;
        let v1 = position - from.position;
        let c1 = v1.dot(v1);
        if c1 < 1e-12 {
            // No way between them: turn by the change of direction.
            return Quat::from_rotation_arc(from.tangent, tangent) * up;
        }
        let up_l = up - v1 * (2.0 / c1 * v1.dot(up));
        let t_l = from.tangent - v1 * (2.0 / c1 * v1.dot(from.tangent));
        let v2 = tangent - t_l;
        let c2 = v2.dot(v2);
        let next = if c2 < 1e-12 {
            up_l
        } else {
            up_l - v2 * (2.0 / c2 * v2.dot(up_l))
        };
        // Kept square to the tangent and of unit length against drift.
        (next - tangent * tangent.dot(next)).normalize_or(first_up(tangent))
    };
    for i in 1..list.len() {
        let up = step(
            &list[i - 1],
            (list[i].position, list[i].tangent),
            ups[i - 1],
        );
        ups.push(up);
    }
    let length = samples.length.max(1e-6);
    // Coming round a closed curve, the up may arrive turned: spread the
    // difference along the way.
    let mut closing = 0.0;
    if samples.closed && list.len() > 1 {
        let last = &list[list.len() - 1];
        let back = step(last, (first.position, first.tangent), ups[ups.len() - 1]);
        let across = ups[0].cross(first.tangent);
        closing = -back.dot(across).atan2(back.dot(ups[0]));
    }
    list.iter()
        .zip(&ups)
        .map(|(sample, up)| {
            let share = sample.along / length;
            let turn = Quat::from_axis_angle(sample.tangent, twist * share - closing * share);
            let up = (turn * *up).normalize_or(*up);
            Frame {
                tangent: sample.tangent,
                up,
                across: up.cross(sample.tangent),
            }
        })
        .collect()
}
