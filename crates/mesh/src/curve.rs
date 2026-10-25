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
//! one line, so the curve bends through it without a kink; a *broken*
//! point's handles move on their own; an *auto* point's handles follow its
//! neighbours ([`PointKind`], [decision 0139](../../../docs/decisions/0139-spline-editing.md)).
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

pub mod bend;
mod edit;
mod grid;
pub mod lathe;
mod path;
pub mod ready;
mod rounding;
mod shaping;
pub mod sweep;

pub use lathe::{LatheAxis, LatheShape};
pub use path::Detail;
pub use ready::{ReadyCurve, ReadyCurveKind};
pub use rounding::MAX_RADIUS;
pub use shaping::{CurveEnd, SAME_SPOT, join, split};
pub use sweep::{Profile, SweepShape};

#[cfg(test)]
mod editing_tests;
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
    /// How its handles behave ([decision 0139](../../../docs/decisions/0139-spline-editing.md)).
    pub kind: PointKind,
}

/// How a curve point's handles behave
/// ([decision 0139](../../../docs/decisions/0139-spline-editing.md)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PointKind {
    /// No handles: a corner.
    #[default]
    Sharp,
    /// Handles kept in one line, so the curve bends through without a
    /// kink: dragging one turns the other to match, keeping its length.
    Smooth,
    /// Handles that move on their own, for a kink with curved sides.
    Broken,
    /// Handles worked out from the neighbours, a third of the way to each
    /// along the line between them: they follow as the neighbours move.
    Auto,
}

impl PointKind {
    /// Every kind, in the order the Attributes panel lists them.
    pub const ALL: [Self; 4] = [Self::Sharp, Self::Smooth, Self::Broken, Self::Auto];

    /// Its name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Sharp => tr("Sharp"),
            Self::Smooth => tr("Smooth"),
            Self::Broken => tr("Broken"),
            Self::Auto => tr("Auto"),
        }
    }

    /// The kind an older smooth switch stood for: smooth, or a sharp point
    /// whose handles (if any) moved on their own.
    pub fn from_smooth(smooth: bool, has_handles: bool) -> Self {
        match (smooth, has_handles) {
            (true, _) => Self::Smooth,
            (false, true) => Self::Broken,
            (false, false) => Self::Sharp,
        }
    }
}

impl CurvePoint {
    /// Whether its handles pull at all.
    pub fn has_handles(&self) -> bool {
        self.handle_in.length_squared() > 1e-12 || self.handle_out.length_squared() > 1e-12
    }

    /// Whether it bends through without a kink of its own making (smooth
    /// or auto).
    pub fn is_smooth(&self) -> bool {
        matches!(self.kind, PointKind::Smooth | PointKind::Auto)
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
    /// Gives points a handle kind: sharp takes the handles back to
    /// nothing; smooth puts them in one line; broken keeps them as they
    /// are; auto works them out from the neighbours. A point without
    /// handles made smooth or broken gets handles that follow its
    /// neighbours.
    SetKind {
        /// The points.
        points: Vec<CurvePointId>,
        /// Their new kind.
        kind: PointKind,
    },
    /// Closes the curve into a loop (it needs three points), or opens it.
    SetClosed(bool),
    /// Adds a point on the stretch leaving point `after`, `t` of the way
    /// along it (0 to 1, by the stretch's own measure), without changing
    /// the curve's shape. The new point gets the next id.
    Insert {
        /// The point the stretch leaves.
        after: CurvePointId,
        /// How far along.
        t: f32,
    },
    /// Adds points at one end of an open curve, in drawing order: from the
    /// end outward. Each is its place, in and out handle (from the point,
    /// as drawn: in points back towards the end), and kind.
    Extend {
        /// Whether they go before the first point (else after the last).
        at_start: bool,
        /// The new points, from the end outward.
        points: Vec<(Vec3, Vec3, Vec3, PointKind)>,
    },
    /// Turns the curve's direction round: its last point becomes its
    /// first (a closed curve keeps its first point).
    Reverse,
    /// Rounds sharp corners into arcs of `radius` (centimetres), each cut
    /// back to what fits between its neighbours. Points that aren't sharp
    /// corners (ends of an open curve, points with handles, points in a
    /// straight line) are left alone.
    RoundCorners {
        /// The points.
        points: Vec<CurvePointId>,
        /// The arcs' radius.
        radius: f32,
    },
    /// Puts `count` points evenly along the curve, with handles that keep
    /// its shape as near as they can (all new ids).
    Resample {
        /// How many points.
        count: usize,
    },
    /// Joins an open curve's two ends: `weld` melts them into one point at
    /// their middle, else a straight stretch joins them. Either way the
    /// curve becomes a loop.
    CloseEnds {
        /// Melt the ends into one, or join them with a stretch.
        weld: bool,
    },
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
    /// A loop of fewer than three points.
    LoopTooSmall,
    /// The curve has used up the point ids it may have.
    OutOfIds,
    /// An end-to-end change on a closed curve, or on points that aren't
    /// its ends.
    NotEnds,
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
            Self::LoopTooSmall => tr("a closed curve needs at least three points"),
            Self::OutOfIds => {
                tr("this curve has had 65536 points made over time, the most it can: draw it again")
            }
            Self::NotEnds => tr("that works on the two ends of open curves"),
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
                    kind: PointKind::from_smooth(
                        smooth,
                        handle_in.length_squared() > 1e-12 || handle_out.length_squared() > 1e-12,
                    ),
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
        let mut curve = Self {
            points,
            closed,
            next_id,
        };
        curve.settle();
        if !curve.points.iter().all(CurvePoint::is_finite) {
            return Err(CurveError::NotANumber);
        }
        Ok(curve)
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
            kind: PointKind::from_smooth(
                smooth,
                handle_in.length_squared() > 1e-12 || handle_out.length_squared() > 1e-12,
            ),
        });
        Self::from_parts(points, self.closed, self.next_id.saturating_add(1))
    }
}
