//! Curves: Bézier splines of points with handles, and the solids made
//! from them ([decision 0138](../../../docs/decisions/0138-curves-sweep-lathe.md)).
//!
//! A [`Curve`] is a list of [`CurvePoint`]s, open or closed. Each point
//! has a place and two handles (offsets from the point): the *in* handle
//! pulls the stretch arriving at the point, the *out* handle the stretch
//! leaving it. Between two points the curve is a cubic Bézier: from the
//! first point, pulled towards its out handle, then towards the second
//! point's in handle, to the second point. A point with both handles at
//! nothing is a sharp corner; a *smooth* point keeps its two handles in
//! one line, so the curve bends through it without a kink.
//!
//! Points have ids ([`CurvePointId`]) that count up and are never reused,
//! as a mesh's do, so a selection of points holds while others are
//! deleted. Curves change only through [`CurveEdit`]s, which give a new
//! curve: the scene records them as commands, so undo works.
//!
//! [`sweep`](sweep::sweep) builds a solid along a curve (a pipe, a cable,
//! a rail), [`lathe`](lathe::lathe) spins one round an axis (a vase, a
//! bottle), and [`ready`] makes the ready-made curves (line, circle, arc,
//! spiral).

use crate::Bounds;
use glam::{Mat4, Vec3};
use words::tr;

mod grid;
pub mod lathe;
mod path;
pub mod ready;
pub mod sweep;

pub use lathe::{LatheAxis, LatheShape};
pub use path::Detail;
pub use ready::{ReadyCurve, ReadyCurveKind};
pub use sweep::{Profile, SweepShape};

#[cfg(test)]
mod tests;

/// The most points a curve may have.
pub const MAX_CURVE_POINTS: usize = 4096;
/// Point ids stay below this, so a curve's points and handles can be
/// named by small numbers (see [`CurvePart::code`]).
pub const MAX_POINT_ID: u32 = 1 << 16;
/// The furthest a point or a handle's end may be from the curve's centre,
/// in centimetres: 10 km.
pub const MAX_COORDINATE: f32 = 1_000_000.0;

/// A curve point's id: stable for the point's whole life, never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CurvePointId(u32);

impl CurvePointId {
    /// The id as a number.
    pub fn get(self) -> u32 {
        self.0
    }

    /// The id with this number, as recorded earlier.
    pub const fn from_raw(number: u32) -> Self {
        Self(number)
    }
}

/// One point of a curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurvePoint {
    /// Its id.
    pub id: CurvePointId,
    /// Where it is, in the curve's own space (centimetres).
    pub position: Vec3,
    /// Its in handle, from the point: pulls the stretch arriving here.
    pub handle_in: Vec3,
    /// Its out handle, from the point: pulls the stretch leaving here.
    pub handle_out: Vec3,
    /// Whether its handles are kept in one line (a smooth bend); a sharp
    /// point's handles move on their own, and are usually nothing.
    pub smooth: bool,
}

impl CurvePoint {
    /// Whether its handles pull at all.
    pub fn has_handles(&self) -> bool {
        self.handle_in.length_squared() > 1e-12 || self.handle_out.length_squared() > 1e-12
    }

    fn is_finite(&self) -> bool {
        let within = |v: Vec3| v.is_finite() && v.abs().max_element() <= MAX_COORDINATE;
        within(self.position)
            && within(self.position + self.handle_in)
            && within(self.position + self.handle_out)
    }
}

/// Which part of a curve point: the point itself or one of its handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PartKind {
    /// The point.
    Point,
    /// Its in handle's end.
    In,
    /// Its out handle's end.
    Out,
}

/// A point of a curve, or the end of one of its handles: what can be
/// selected and moved in the Points mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CurvePart {
    /// The point.
    pub point: CurvePointId,
    /// Which part of it.
    pub kind: PartKind,
}

impl CurvePart {
    /// The point itself.
    pub fn point(id: CurvePointId) -> Self {
        Self {
            point: id,
            kind: PartKind::Point,
        }
    }

    /// A number naming the part: three per point (the point, its in
    /// handle, its out handle). The interface keeps a curve's selected
    /// parts as these, in the place of a mesh's point ids.
    pub fn code(self) -> u32 {
        self.point.0 * 3
            + match self.kind {
                PartKind::Point => 0,
                PartKind::In => 1,
                PartKind::Out => 2,
            }
    }

    /// The part a [`Self::code`] names.
    pub fn from_code(code: u32) -> Self {
        Self {
            point: CurvePointId(code / 3),
            kind: match code % 3 {
                0 => PartKind::Point,
                1 => PartKind::In,
                _ => PartKind::Out,
            },
        }
    }
}

/// A change to a curve. Ids the curve doesn't have are skipped.
#[derive(Debug, Clone, PartialEq)]
pub enum CurveEdit {
    /// Moves, turns or scales parts by a matrix in the curve's own space.
    /// A point's handles come along with it; a handle moved on its own
    /// turns its partner too on a smooth point, keeping the partner's
    /// length.
    Transform {
        /// The parts.
        parts: Vec<CurvePart>,
        /// The change.
        matrix: Mat4,
    },
    /// Deletes points; a handle listed on its own (its point not listed)
    /// is taken back to nothing, making that side of the point sharp.
    Delete {
        /// The parts.
        parts: Vec<CurvePart>,
    },
    /// Makes points smooth (handles in one line; points without handles
    /// get handles that follow their neighbours) or sharp (handles back to
    /// nothing: a corner).
    SetSmooth {
        /// The points.
        points: Vec<CurvePointId>,
        /// Smooth, or sharp.
        smooth: bool,
    },
    /// Closes the curve into a loop, or opens it.
    SetClosed(bool),
}

/// Why a curve can't be made or changed, in plain words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveError {
    /// A position or handle that isn't a number, or too far away.
    NotANumber,
    /// More points than a curve may have.
    TooManyPoints,
    /// A change that would leave fewer than two points.
    TooFewPoints,
    /// Two points with the same id, or an id past the curve's next one.
    BadIds,
}

impl std::fmt::Display for CurveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NotANumber => tr("a curve point isn't a number, or is more than 10 km away"),
            Self::TooManyPoints => tr("a curve can have at most 4096 points"),
            Self::TooFewPoints => {
                tr("a curve needs at least two points: delete the whole object instead")
            }
            Self::BadIds => tr("a curve's points have repeated or impossible ids"),
        })
    }
}

impl std::error::Error for CurveError {}

/// One stretch of a curve between two points: a cubic Bézier.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    /// Where it starts (a point).
    pub start: Vec3,
    /// The first point's out handle's end.
    pub pull_start: Vec3,
    /// The second point's in handle's end.
    pub pull_end: Vec3,
    /// Where it ends (the next point).
    pub end: Vec3,
}

impl Span {
    /// The place `t` of the way along (0 to 1, by its own measure, not
    /// evenly by length).
    pub fn at(&self, t: f32) -> Vec3 {
        let s = 1.0 - t;
        self.start * (s * s * s)
            + self.pull_start * (3.0 * s * s * t)
            + self.pull_end * (3.0 * s * t * t)
            + self.end * (t * t * t)
    }

    /// Which way it goes at `t` (not of unit length; nothing on a span of
    /// no length). Where a handle is nothing, the way it leaves or arrives
    /// is towards the other handle, as the curve itself goes.
    pub fn direction(&self, t: f32) -> Vec3 {
        let s = 1.0 - t;
        let d = (self.pull_start - self.start) * (3.0 * s * s)
            + (self.pull_end - self.pull_start) * (6.0 * s * t)
            + (self.end - self.pull_end) * (3.0 * t * t);
        let scale = self.length_estimate();
        let tiny = |v: Vec3| v.length_squared() <= (1e-6 * scale).powi(2) + 1e-20;
        if !tiny(d) {
            return d;
        }
        // Where a handle is nothing the speed there is nothing too: the way
        // is towards the next control point that isn't on the same spot,
        // worked out exactly (a step along would lose it to rounding).
        let candidates = if t < 0.5 {
            [
                self.pull_start - self.start,
                self.pull_end - self.start,
                self.end - self.start,
            ]
        } else {
            [
                self.end - self.pull_end,
                self.end - self.pull_start,
                self.end - self.start,
            ]
        };
        candidates
            .into_iter()
            .find(|c| !tiny(*c))
            .unwrap_or(self.end - self.start)
    }

    /// Whether it is straight: its handles lie along the line between its
    /// ends (or are nothing).
    pub fn is_straight(&self) -> bool {
        let line = self.end - self.start;
        let length = line.length();
        if length < 1e-6 {
            return self.length_estimate() < 1e-4;
        }
        let way = line / length;
        let off = |p: Vec3| {
            let d = p - self.start;
            (d - way * d.dot(way)).length()
        };
        off(self.pull_start) < 1e-4 * length.max(1.0) && off(self.pull_end) < 1e-4 * length.max(1.0)
    }

    /// The length of its control outline: at least the curve's own.
    pub fn length_estimate(&self) -> f32 {
        self.start.distance(self.pull_start)
            + self.pull_start.distance(self.pull_end)
            + self.pull_end.distance(self.end)
    }
}

/// A Bézier curve: points with handles, open or closed. Always valid:
/// made and changed only through checks.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Curve {
    points: Vec<CurvePoint>,
    closed: bool,
    next_id: u32,
}

impl Curve {
    /// A curve through `points` (each its place, in handle, out handle and
    /// whether it's smooth), in order, with ids from 0.
    pub fn new(
        points: impl IntoIterator<Item = (Vec3, Vec3, Vec3, bool)>,
        closed: bool,
    ) -> Result<Self, CurveError> {
        let points: Vec<CurvePoint> = points
            .into_iter()
            .enumerate()
            .map(
                |(i, (position, handle_in, handle_out, smooth))| CurvePoint {
                    id: CurvePointId(u32::try_from(i).unwrap_or(u32::MAX)),
                    position,
                    handle_in,
                    handle_out,
                    smooth,
                },
            )
            .collect();
        let next_id = u32::try_from(points.len()).unwrap_or(u32::MAX);
        Self::from_parts(points, closed, next_id)
    }

    /// A curve of sharp corners through `positions`.
    pub fn corners(positions: &[Vec3], closed: bool) -> Result<Self, CurveError> {
        Self::new(
            positions
                .iter()
                .map(|p| (*p, Vec3::ZERO, Vec3::ZERO, false)),
            closed,
        )
    }

    /// Rebuilds a curve with its ids (from a file, or the scene), checking
    /// everything: positions real and within [`MAX_COORDINATE`], at most
    /// [`MAX_CURVE_POINTS`] points, ids unique and below `next_id`, which
    /// is at most [`MAX_POINT_ID`].
    pub fn from_parts(
        points: Vec<CurvePoint>,
        closed: bool,
        next_id: u32,
    ) -> Result<Self, CurveError> {
        if points.len() > MAX_CURVE_POINTS {
            return Err(CurveError::TooManyPoints);
        }
        if next_id > MAX_POINT_ID {
            return Err(CurveError::BadIds);
        }
        if !points.iter().all(CurvePoint::is_finite) {
            return Err(CurveError::NotANumber);
        }
        let mut ids: Vec<u32> = points.iter().map(|p| p.id.0).collect();
        ids.sort_unstable();
        if ids.windows(2).any(|w| w[0] == w[1]) || ids.last().is_some_and(|l| *l >= next_id) {
            return Err(CurveError::BadIds);
        }
        Ok(Self {
            points,
            closed,
            next_id,
        })
    }

    /// Its points, in order.
    pub fn points(&self) -> &[CurvePoint] {
        &self.points
    }

    /// Whether it closes into a loop (the last point joins the first).
    pub fn closed(&self) -> bool {
        self.closed
    }

    /// The id the next new point gets.
    pub fn next_id(&self) -> u32 {
        self.next_id
    }

    /// The point with this id.
    pub fn point(&self, id: CurvePointId) -> Option<&CurvePoint> {
        self.points.iter().find(|p| p.id == id)
    }

    /// Where a part is: a point's place, or its handle's end.
    pub fn part_position(&self, part: CurvePart) -> Option<Vec3> {
        let p = self.point(part.point)?;
        Some(match part.kind {
            PartKind::Point => p.position,
            PartKind::In => p.position + p.handle_in,
            PartKind::Out => p.position + p.handle_out,
        })
    }

    /// Every part that can be picked: each point, and the ends of the
    /// handles that pull.
    pub fn parts(&self) -> Vec<(CurvePart, Vec3)> {
        let mut out = Vec::with_capacity(self.points.len() * 3);
        for p in &self.points {
            out.push((CurvePart::point(p.id), p.position));
            for (kind, handle) in [(PartKind::In, p.handle_in), (PartKind::Out, p.handle_out)] {
                if handle.length_squared() > 1e-12 {
                    out.push((CurvePart { point: p.id, kind }, p.position + handle));
                }
            }
        }
        out
    }

    /// Its parts as a mesh of loose points (no polygons), each point's id
    /// the part's [`CurvePart::code`]: so the Points mode can find, select
    /// and move a curve's points and handles as it does a mesh's.
    pub fn parts_mesh(&self) -> crate::Mesh {
        let parts = self.parts();
        let slots = parts
            .iter()
            .map(|(p, _)| p.code() as usize + 1)
            .max()
            .unwrap_or(0);
        let mut points = vec![None; slots];
        for (part, at) in parts {
            points[part.code() as usize] = Some(at);
        }
        crate::Mesh::from_slots(points, Vec::new(), []).unwrap_or_default()
    }

    /// Whether it has fewer than two points apart: there is no line.
    pub fn is_empty(&self) -> bool {
        self.spans().iter().all(|s| s.length_estimate() < 1e-4)
    }

    /// Its stretches, in order (a closed curve's last one back to the
    /// first point).
    pub fn spans(&self) -> Vec<Span> {
        let n = self.points.len();
        if n < 2 {
            return Vec::new();
        }
        let count = if self.closed { n } else { n - 1 };
        (0..count)
            .map(|i| {
                let (a, b) = (&self.points[i], &self.points[(i + 1) % n]);
                Span {
                    start: a.position,
                    pull_start: a.position + a.handle_out,
                    pull_end: b.position + b.handle_in,
                    end: b.position,
                }
            })
            .collect()
    }

    /// The curve as a line of `per_span` pieces per stretch (straight
    /// stretches in one), for drawing: a closed curve ends where it began.
    pub fn polyline(&self, per_span: u32) -> Vec<Vec3> {
        let spans = self.spans();
        let mut out = Vec::new();
        for span in &spans {
            if out.is_empty() {
                out.push(span.start);
            }
            let n = if span.is_straight() {
                1
            } else {
                per_span.max(1)
            };
            for k in 1..=n {
                out.push(span.at(k as f32 / n as f32));
            }
        }
        if out.is_empty()
            && let Some(p) = self.points.first()
        {
            out.push(p.position);
        }
        out
    }

    /// The box around its points and handles.
    pub fn bounds(&self) -> Option<Bounds> {
        Bounds::around(self.parts().into_iter().map(|(_, p)| p))
    }

    /// The same curve moved by `matrix` (its handles turned and scaled
    /// with it). A matrix that isn't real numbers is refused.
    pub fn transformed(&self, matrix: Mat4) -> Result<Self, CurveError> {
        let parts: Vec<CurvePart> = self.points.iter().map(|p| CurvePart::point(p.id)).collect();
        self.edited(&CurveEdit::Transform { parts, matrix })
    }

    /// The curve after `edit`.
    pub fn edited(&self, edit: &CurveEdit) -> Result<Self, CurveError> {
        let mut out = self.clone();
        match edit {
            CurveEdit::Transform { parts, matrix } => {
                if !matrix.is_finite() {
                    return Err(CurveError::NotANumber);
                }
                out.transform(parts, *matrix);
            }
            CurveEdit::Delete { parts } => out.delete(parts)?,
            CurveEdit::SetSmooth { points, smooth } => {
                for id in points {
                    out.set_smooth(*id, *smooth);
                }
            }
            CurveEdit::SetClosed(closed) => out.closed = *closed,
        }
        Self::from_parts(out.points, out.closed, out.next_id)
    }

    fn transform(&mut self, parts: &[CurvePart], matrix: Mat4) {
        for point in &mut self.points {
            let has = |kind| {
                parts.contains(&CurvePart {
                    point: point.id,
                    kind,
                })
            };
            let (all, inn, out) = (has(PartKind::Point), has(PartKind::In), has(PartKind::Out));
            if !(all || inn || out) {
                continue;
            }
            let p = point.position;
            let (end_in, end_out) = (p + point.handle_in, p + point.handle_out);
            if all {
                let moved = matrix.transform_point3(p);
                point.position = moved;
                point.handle_in = matrix.transform_point3(end_in) - moved;
                point.handle_out = matrix.transform_point3(end_out) - moved;
                continue;
            }
            if inn {
                point.handle_in = matrix.transform_point3(end_in) - p;
            }
            if out {
                point.handle_out = matrix.transform_point3(end_out) - p;
            }
            // One handle of a smooth point moved: its partner turns to stay
            // in line, keeping its length.
            if point.smooth && inn != out {
                let (moved, partner) = if inn {
                    (point.handle_in, &mut point.handle_out)
                } else {
                    (point.handle_out, &mut point.handle_in)
                };
                let length = partner.length();
                if let Some(way) = moved.try_normalize() {
                    *partner = -way * length;
                }
            }
        }
    }

    fn delete(&mut self, parts: &[CurvePart]) -> Result<(), CurveError> {
        let gone: Vec<CurvePointId> = parts
            .iter()
            .filter(|p| p.kind == PartKind::Point)
            .map(|p| p.point)
            .collect();
        let left = self.points.iter().filter(|p| !gone.contains(&p.id)).count();
        if left < 2 && left < self.points.len() {
            return Err(CurveError::TooFewPoints);
        }
        self.points.retain(|p| !gone.contains(&p.id));
        for part in parts {
            if let Some(point) = self.points.iter_mut().find(|p| p.id == part.point) {
                match part.kind {
                    PartKind::In => point.handle_in = Vec3::ZERO,
                    PartKind::Out => point.handle_out = Vec3::ZERO,
                    PartKind::Point => {}
                }
                if part.kind != PartKind::Point {
                    point.smooth = false;
                }
            }
        }
        // Two points can't make a loop.
        if self.points.len() < 3 {
            self.closed = false;
        }
        Ok(())
    }

    fn set_smooth(&mut self, id: CurvePointId, smooth: bool) {
        let n = self.points.len();
        let Some(i) = self.points.iter().position(|p| p.id == id) else {
            return;
        };
        if !smooth {
            let point = &mut self.points[i];
            point.smooth = false;
            point.handle_in = Vec3::ZERO;
            point.handle_out = Vec3::ZERO;
            return;
        }
        let here = self.points[i].position;
        let before = if i > 0 {
            Some(self.points[i - 1].position)
        } else if self.closed && n > 1 {
            Some(self.points[n - 1].position)
        } else {
            None
        };
        let after = if i + 1 < n {
            Some(self.points[i + 1].position)
        } else if self.closed && n > 1 {
            Some(self.points[0].position)
        } else {
            None
        };
        let point = &mut self.points[i];
        point.smooth = true;
        let existing = point.handle_out - point.handle_in;
        let way = existing
            .try_normalize()
            .or_else(|| match (before, after) {
                (Some(b), Some(a)) => (a - b).try_normalize(),
                (None, Some(a)) => (a - here).try_normalize(),
                (Some(b), None) => (here - b).try_normalize(),
                (None, None) => None,
            })
            .unwrap_or(Vec3::X);
        // A third of the way to each neighbour, as a smooth curve through
        // them (Catmull-Rom) would pull; or as long as the handles were.
        let reach = |other: Option<Vec3>, handle: Vec3| {
            if handle.length_squared() > 1e-12 {
                handle.length()
            } else {
                other.map_or(0.0, |o| o.distance(here) / 3.0)
            }
        };
        let back = reach(before, point.handle_in);
        let ahead = reach(after, point.handle_out);
        let (back, ahead) = match (back > 0.0, ahead > 0.0) {
            (false, true) => (ahead, ahead),
            (true, false) => (back, back),
            _ => (back, ahead),
        };
        point.handle_in = -way * back;
        point.handle_out = way * ahead;
    }

    /// The curve with one more point at the end (for drawing), refused past
    /// [`MAX_CURVE_POINTS`] or [`MAX_POINT_ID`].
    pub fn with_point(
        &self,
        position: Vec3,
        handle_in: Vec3,
        handle_out: Vec3,
        smooth: bool,
    ) -> Result<Self, CurveError> {
        let mut points = self.points.clone();
        points.push(CurvePoint {
            id: CurvePointId(self.next_id),
            position,
            handle_in,
            handle_out,
            smooth,
        });
        Self::from_parts(points, self.closed, self.next_id.saturating_add(1))
    }
}
