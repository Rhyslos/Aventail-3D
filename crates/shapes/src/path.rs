//! Paths: points joined by straight or curved sides, open (a line) or
//! closed (a shape with an inside).
//!
//! Places are on the canvas, whose texture square runs from (0, 0) at its
//! top left to (1, 1) at its bottom right, so a path draws the same at
//! any texture size. Each point has two handles, kept as offsets from it:
//! a side is straight when the handles at both its ends are at their
//! points, and curved (a cubic Bézier curve) otherwise.

use glam::Vec2;
use kurbo::{BezPath, ParamCurve, ParamCurveNearest, Point, Shape};
use std::hash::{Hash, Hasher};
use words::text;

/// The most points a path may hold.
pub const MAX_KNOTS: usize = 1024;
/// The farthest from the texture square a point or handle may be, in
/// texture widths from its middle.
pub const MAX_PLACE: f32 = 8.0;
/// How close to the texture's size (a share of it) a point is "on" a
/// side or another point, when finding it.
const ACCURACY: f64 = 1e-4;

/// One point of a path and its two handles.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Knot {
    /// Where it is.
    pub at: Vec2,
    /// Its incoming handle, from `at` (zero: the side arriving is
    /// straight at this end).
    pub inn: Vec2,
    /// Its outgoing handle, from `at` (zero: the side leaving is straight
    /// at this end).
    pub out: Vec2,
}

impl Knot {
    /// A corner at `at`: both sides straight here.
    pub fn corner(at: Vec2) -> Self {
        Self {
            at,
            inn: Vec2::ZERO,
            out: Vec2::ZERO,
        }
    }

    /// A smooth point at `at` whose outgoing handle is `out` and incoming
    /// handle the opposite (what dragging the pen makes).
    pub fn smooth(at: Vec2, out: Vec2) -> Self {
        Self { at, inn: -out, out }
    }

    /// Whether both handles are at the point.
    pub fn is_corner(&self) -> bool {
        self.inn == Vec2::ZERO && self.out == Vec2::ZERO
    }

    /// Whether its place and handles are real numbers within
    /// [`MAX_PLACE`].
    fn is_in_place(&self) -> bool {
        [self.at, self.at + self.inn, self.at + self.out]
            .iter()
            .all(|p| p.is_finite() && (*p - Vec2::splat(0.5)).abs().max_element() <= MAX_PLACE)
    }

    /// Mirrored in the upright line at `x`.
    fn mirrored(&self, x: f32) -> Self {
        let flip = |v: Vec2| Vec2::new(-v.x, v.y);
        Self {
            at: Vec2::new(2.0 * x - self.at.x, self.at.y),
            inn: flip(self.inn),
            out: flip(self.out),
        }
    }
}

/// A turn, a size change and a move, applied to points (`a` maps a
/// point's offset from `about`, then `by` moves it).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Change {
    /// The point turned and sized about.
    pub about: Vec2,
    /// The 2 × 2 matrix turning and sizing, as columns.
    pub matrix: glam::Mat2,
    /// The move after.
    pub by: Vec2,
}

impl Change {
    /// No change at all: every point stays where it is.
    pub const IDENTITY: Self = Self {
        about: Vec2::ZERO,
        matrix: glam::Mat2::IDENTITY,
        by: Vec2::ZERO,
    };

    /// A plain move by `by`.
    pub fn moved(by: Vec2) -> Self {
        Self {
            about: Vec2::ZERO,
            matrix: glam::Mat2::IDENTITY,
            by,
        }
    }

    /// A turn by `angle` (radians, clockwise on screen, where y runs
    /// down) about `about`.
    pub fn turned(about: Vec2, angle: f32) -> Self {
        Self {
            about,
            matrix: glam::Mat2::from_angle(angle),
            by: Vec2::ZERO,
        }
    }

    /// A size change by `scale` (across, down) about `about`.
    pub fn scaled(about: Vec2, scale: Vec2) -> Self {
        Self {
            about,
            matrix: glam::Mat2::from_diagonal(scale),
            by: Vec2::ZERO,
        }
    }

    /// Where the point `p` goes.
    pub fn point(&self, p: Vec2) -> Vec2 {
        self.about + self.matrix * (p - self.about) + self.by
    }

    /// Where the offset `v` (a handle) goes.
    pub fn offset(&self, v: Vec2) -> Vec2 {
        self.matrix * v
    }

    /// Whether it turns a shape over (a mirror image).
    pub fn flips(&self) -> bool {
        self.matrix.determinant() < 0.0
    }

    /// This change followed by `next`, as one change (about the canvas's
    /// corner).
    pub fn then(&self, next: &Change) -> Self {
        // p ↦ M p + t, with M = A and t = about - A about + by.
        let shift = |c: &Change| c.about - c.matrix * c.about + c.by;
        Self {
            about: Vec2::ZERO,
            matrix: next.matrix * self.matrix,
            by: next.matrix * shift(self) + shift(next),
        }
    }

    /// The change undoing this one, if it can be undone (it doesn't
    /// squash shapes flat, and its numbers are real).
    pub fn inverse(&self) -> Option<Self> {
        let det = self.matrix.determinant();
        if !det.is_finite() || det.abs() < MIN_SQUASH || !self.by.is_finite() {
            return None;
        }
        let inv = self.matrix.inverse();
        let shift = self.about - self.matrix * self.about + self.by;
        let out = Self {
            about: Vec2::ZERO,
            matrix: inv,
            by: -(inv * shift),
        };
        (out.matrix.is_finite() && out.by.is_finite()).then_some(out)
    }

    /// Whether its numbers are real and it doesn't squash shapes flat (so
    /// it can be undone).
    pub fn is_usable(&self) -> bool {
        self.matrix.is_finite()
            && self.about.is_finite()
            && self.by.is_finite()
            && self.matrix.determinant().abs() >= MIN_SQUASH
    }
}

/// How small a change may make an area (a share of it) before shapes
/// count as squashed flat.
pub const MIN_SQUASH: f32 = 1e-8;

/// A path: points joined in order by sides, the last back to the first
/// when closed.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Path {
    knots: Vec<Knot>,
    closed: bool,
}

impl Path {
    /// A path through `knots`, closed or open.
    pub fn new(knots: Vec<Knot>, closed: bool) -> Self {
        Self { knots, closed }
    }

    /// A closed path of straight sides through `points`.
    pub fn polygon(points: impl IntoIterator<Item = Vec2>) -> Self {
        Self::new(points.into_iter().map(Knot::corner).collect(), true)
    }

    /// Its points.
    pub fn knots(&self) -> &[Knot] {
        &self.knots
    }

    /// Whether the last point joins back to the first.
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// The same path, closed.
    pub fn closed(mut self) -> Self {
        self.closed = true;
        self
    }

    /// Why it can't be kept, if it can't: too many points, or a point or
    /// handle not a real number or too far out.
    pub fn problem(&self) -> Option<&'static str> {
        if self.knots.len() > MAX_KNOTS {
            Some(text!("a shape has more points than the canvas holds"))
        } else if !self.knots.iter().all(Knot::is_in_place) {
            Some(text!("a point is out of place"))
        } else {
            None
        }
    }

    /// Whether every point and handle is a real number (wherever it is).
    pub fn is_finite(&self) -> bool {
        self.knots
            .iter()
            .all(|k| k.at.is_finite() && k.inn.is_finite() && k.out.is_finite())
    }

    /// The same path with point `index` changed to `knot` (unchanged when
    /// there is no such point).
    pub fn with_knot(&self, index: usize, knot: Knot) -> Self {
        let mut out = self.clone();
        if let Some(k) = out.knots.get_mut(index) {
            *k = knot;
        }
        out
    }

    /// The same path without point `index`.
    pub fn without_knot(&self, index: usize) -> Self {
        let mut out = self.clone();
        if index < out.knots.len() {
            out.knots.remove(index);
        }
        out
    }

    /// The same path with a point added on side `side` (from point `side`
    /// to the next) at `t` (0 to 1 along it), keeping its shape: the
    /// curve is split in two there (de Casteljau).
    pub fn with_knot_on_side(&self, side: usize, t: f32) -> Self {
        let n = self.knots.len();
        let Some((a, b)) = self.side(side) else {
            return self.clone();
        };
        let t = t.clamp(0.0, 1.0);
        let (p0, p1, p2, p3) = (a.at, a.at + a.out, b.at + b.inn, b.at);
        let lerp = |p: Vec2, q: Vec2| p + (q - p) * t;
        let (q0, q1, q2) = (lerp(p0, p1), lerp(p1, p2), lerp(p2, p3));
        let (r0, r1) = (lerp(q0, q1), lerp(q1, q2));
        let mid = lerp(r0, r1);
        let straight = a.out == Vec2::ZERO && b.inn == Vec2::ZERO;
        let mut out = self.clone();
        let next = (side + 1) % n;
        if straight {
            out.knots.insert(side + 1, Knot::corner(mid));
            return out;
        }
        out.knots[side].out = q0 - p0;
        out.knots[next].inn = q2 - p3;
        out.knots.insert(
            side + 1,
            Knot {
                at: mid,
                inn: r0 - mid,
                out: r1 - mid,
            },
        );
        out
    }

    /// Side `index`: its two end points, if there is such a side.
    fn side(&self, index: usize) -> Option<(Knot, Knot)> {
        let n = self.knots.len();
        let sides = if self.closed { n } else { n.saturating_sub(1) };
        if index >= sides || n < 2 {
            return None;
        }
        Some((self.knots[index], self.knots[(index + 1) % n]))
    }

    /// How many sides it has.
    pub fn sides(&self) -> usize {
        let n = self.knots.len();
        if self.closed { n } else { n.saturating_sub(1) }
    }

    /// The same path moved, turned or sized by `change`.
    pub fn changed(&self, change: &Change) -> Self {
        let mut out = self.clone();
        for k in &mut out.knots {
            k.at = change.point(k.at);
            k.inn = change.offset(k.inn);
            k.out = change.offset(k.out);
        }
        out
    }

    /// Its mirror image in the upright line at `x` (the points in the
    /// opposite order, so it runs the same way round).
    pub fn mirrored(&self, x: f32) -> Self {
        let knots = self
            .knots
            .iter()
            .rev()
            .map(|k| {
                let m = k.mirrored(x);
                Knot {
                    at: m.at,
                    inn: m.out,
                    out: m.inn,
                }
            })
            .collect();
        Self {
            knots,
            closed: self.closed,
        }
    }

    /// The smallest rectangle round its points and handles (`None` with
    /// no points).
    pub fn bounds(&self) -> Option<(Vec2, Vec2)> {
        let first = self.knots.first()?.at;
        let mut lo = first;
        let mut hi = first;
        for k in &self.knots {
            for p in [k.at, k.at + k.inn, k.at + k.out] {
                lo = lo.min(p);
                hi = hi.max(p);
            }
        }
        Some((lo, hi))
    }

    /// The middle of its points' rectangle (where it turns and sizes
    /// about).
    pub fn middle(&self) -> Vec2 {
        self.bounds()
            .map_or(Vec2::splat(0.5), |(lo, hi)| (lo + hi) * 0.5)
    }

    /// The path as a curve for measuring.
    pub fn bez(&self) -> BezPath {
        let mut out = BezPath::new();
        let Some(first) = self.knots.first() else {
            return out;
        };
        out.move_to(pt(first.at));
        for i in 0..self.sides() {
            if let Some((a, b)) = self.side(i) {
                if a.out == Vec2::ZERO && b.inn == Vec2::ZERO {
                    out.line_to(pt(b.at));
                } else {
                    out.curve_to(pt(a.at + a.out), pt(b.at + b.inn), pt(b.at));
                }
            }
        }
        if self.closed {
            out.close_path();
        }
        out
    }

    /// The area inside it (0 for an open path). Parts crossing over
    /// themselves count by how they wind.
    pub fn area(&self) -> f32 {
        if !self.closed {
            return 0.0;
        }
        self.bez().area().abs() as f32
    }

    /// Whether `p` is inside it (a closed path; sides crossing over count
    /// as filled, as when it is drawn).
    pub fn contains(&self, p: Vec2) -> bool {
        self.closed && self.knots.len() >= 2 && self.bez().winding(pt(p)) != 0
    }

    /// The nearest place on its sides to `p`: the side, how far along it
    /// (0 to 1) and how far away.
    pub fn nearest(&self, p: Vec2) -> Option<(usize, f32, f32)> {
        let path = self.bez();
        let mut best: Option<(usize, f32, f32)> = None;
        for (i, seg) in path.segments().enumerate() {
            let near = seg.nearest(pt(p), ACCURACY);
            let d = (near.distance_sq.sqrt()) as f32;
            if best.is_none_or(|b| d < b.2) {
                best = Some((i, near.t as f32, d));
            }
        }
        if best.is_none() {
            // One point: how far from it.
            let k = self.knots.first()?;
            return Some((0, 0.0, k.at.distance(p)));
        }
        best
    }

    /// The point `t` (0 to 1) along side `side`.
    pub fn point_on_side(&self, side: usize, t: f32) -> Option<Vec2> {
        let seg = self.bez().get_seg(side + 1)?;
        let at = seg.eval(f64::from(t.clamp(0.0, 1.0)));
        Some(Vec2::new(at.x as f32, at.y as f32))
    }

    /// Its points along its sides as straight steps, curves followed to
    /// within `tolerance` (a share of the texture); a closed path's first
    /// point isn't repeated at its end.
    pub fn flattened(&self, tolerance: f64) -> Vec<Vec2> {
        let mut out = Vec::new();
        kurbo::flatten(self.bez(), tolerance.max(1e-9), |el| match el {
            kurbo::PathEl::MoveTo(p) | kurbo::PathEl::LineTo(p) => {
                out.push(Vec2::new(p.x as f32, p.y as f32));
            }
            _ => {}
        });
        if self.closed && out.len() > 1 && out.first() == out.last() {
            out.pop();
        }
        out
    }

    /// Feeds it into a hasher exactly (numbers by their bits).
    pub(crate) fn hash_into(&self, h: &mut impl Hasher) {
        self.closed.hash(h);
        self.knots.len().hash(h);
        for k in &self.knots {
            for v in [k.at, k.inn, k.out] {
                (v.x.to_bits(), v.y.to_bits()).hash(h);
            }
        }
    }
}

fn pt(v: Vec2) -> Point {
    Point::new(f64::from(v.x), f64::from(v.y))
}

#[cfg(test)]
mod tests;
