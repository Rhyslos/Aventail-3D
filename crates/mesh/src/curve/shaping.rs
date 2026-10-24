//! Reshaping curves ([decision 0139](../../../../docs/decisions/0139-spline-editing.md)):
//! adding a point without changing the shape, carrying a curve on from an
//! end, turning it round, rounding corners, spacing points evenly,
//! closing, joining and splitting.

use super::{
    Curve, CurveError, CurvePoint, CurvePointId, MAX_CURVE_POINTS, MAX_POINT_ID, PointKind, Span,
};
use glam::Vec3;

/// One end of an open curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CurveEnd {
    /// Its first point.
    Start,
    /// Its last point.
    End,
}

/// Ends closer than this (centimetres) are on the same spot: joining them
/// melts them into one point.
pub const SAME_SPOT: f32 = 1e-3;

/// Where `a` is `t` of the way to `b`.
fn lerp(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    a + (b - a) * t
}

/// The kind two handles meeting at a melted point make: none, in one line
/// (smooth) or apart (broken).
fn kind_of_handles(handle_in: Vec3, handle_out: Vec3) -> PointKind {
    let (i, o) = (handle_in.try_normalize(), handle_out.try_normalize());
    match (i, o) {
        (None, None) => PointKind::Sharp,
        (Some(i), Some(o)) if i.dot(o) < -0.9999 => PointKind::Smooth,
        _ => PointKind::Broken,
    }
}

impl Curve {
    /// A new id, refused once the curve has used them all.
    pub(super) fn new_id(&mut self) -> Result<CurvePointId, CurveError> {
        if self.next_id >= MAX_POINT_ID {
            return Err(CurveError::OutOfIds);
        }
        let id = CurvePointId(self.next_id);
        self.next_id += 1;
        Ok(id)
    }

    /// Which end of the open curve point `id` is, if it is one.
    pub fn end_of(&self, id: CurvePointId) -> Option<CurveEnd> {
        if self.closed || self.points.len() < 2 {
            return None;
        }
        if self.points.first().is_some_and(|p| p.id == id) {
            Some(CurveEnd::Start)
        } else if self.points.last().is_some_and(|p| p.id == id) {
            Some(CurveEnd::End)
        } else {
            None
        }
    }

    /// Splits the stretch leaving point `after` at `t` (de Casteljau): the
    /// new point and the handles either side are set so the shape is
    /// exactly the same.
    pub(super) fn insert(&mut self, after: CurvePointId, t: f32) -> Result<(), CurveError> {
        if !t.is_finite() {
            return Err(CurveError::NotANumber);
        }
        let n = self.points.len();
        let Some(i) = self.points.iter().position(|p| p.id == after) else {
            return Ok(());
        };
        let Some(j) = self.neighbours(i).1 else {
            return Ok(());
        };
        if n >= MAX_CURVE_POINTS {
            return Err(CurveError::TooManyPoints);
        }
        let id = self.new_id()?;
        let t = t.clamp(0.0, 1.0);
        let (a, b) = (self.points[i], self.points[j]);
        let span = Span {
            start: a.position,
            pull_start: a.position + a.handle_out,
            pull_end: b.position + b.handle_in,
            end: b.position,
        };
        let straight = span.is_straight();
        let q0 = lerp(span.start, span.pull_start, t);
        let q1 = lerp(span.pull_start, span.pull_end, t);
        let q2 = lerp(span.pull_end, span.end, t);
        let r0 = lerp(q0, q1, t);
        let r1 = lerp(q1, q2, t);
        let at = lerp(r0, r1, t);
        // Its neighbours' handles change length: auto ones would move, so
        // they keep what they have.
        self.hold_auto(i);
        self.hold_auto(j);
        self.points[i].handle_out = q0 - a.position;
        self.points[j].handle_in = q2 - b.position;
        let point = if straight {
            // On a straight stretch the new point is a corner on the line.
            CurvePoint {
                id,
                position: at,
                handle_in: Vec3::ZERO,
                handle_out: Vec3::ZERO,
                kind: PointKind::Sharp,
            }
        } else {
            CurvePoint {
                id,
                position: at,
                handle_in: r0 - at,
                handle_out: r1 - at,
                kind: PointKind::Smooth,
            }
        };
        self.points.insert(i + 1, point);
        Ok(())
    }

    /// Adds `points` at one end, from the end outward.
    pub(super) fn extend(
        &mut self,
        at_start: bool,
        points: &[(Vec3, Vec3, Vec3, PointKind)],
    ) -> Result<(), CurveError> {
        if self.closed {
            return Err(CurveError::NotEnds);
        }
        if self.points.len() + points.len() > MAX_CURVE_POINTS {
            return Err(CurveError::TooManyPoints);
        }
        for (position, handle_in, handle_out, kind) in points {
            let id = self.new_id()?;
            // Drawn outward from the start, the curve runs the other way:
            // in and out swap.
            let (handle_in, handle_out) = if at_start {
                (*handle_out, *handle_in)
            } else {
                (*handle_in, *handle_out)
            };
            let point = CurvePoint {
                id,
                position: *position,
                handle_in,
                handle_out,
                kind: *kind,
            };
            if at_start {
                self.points.insert(0, point);
            } else {
                self.points.push(point);
            }
        }
        Ok(())
    }

    /// Turns the curve round: points in the other order, in and out
    /// handles swapped. A closed curve keeps its first point.
    pub(super) fn reverse(&mut self) {
        self.points.reverse();
        if self.closed && !self.points.is_empty() {
            self.points.rotate_right(1);
        }
        for p in &mut self.points {
            std::mem::swap(&mut p.handle_in, &mut p.handle_out);
        }
    }

    /// Joins the ends of an open curve, making it a loop.
    pub(super) fn close_ends(&mut self, weld: bool) -> Result<(), CurveError> {
        if self.closed || self.points.len() < 2 {
            return Err(CurveError::NotEnds);
        }
        let needed = if weld { 4 } else { 3 };
        if self.points.len() < needed {
            return Err(CurveError::LoopTooSmall);
        }
        if weld {
            let n = self.points.len();
            let (first, last) = (self.points[0], self.points[n - 1]);
            let middle = first.position.lerp(last.position, 0.5);
            let point = &mut self.points[0];
            point.position = middle;
            point.handle_in = last.handle_in;
            point.kind = kind_of_handles(point.handle_in, point.handle_out);
            self.points.pop();
            self.hold_auto(0);
        } else {
            // A straight stretch: the ends' handles facing it go.
            let n = self.points.len();
            self.points[0].handle_in = Vec3::ZERO;
            self.points[n - 1].handle_out = Vec3::ZERO;
        }
        self.closed = true;
        Ok(())
    }

    /// The curve turned so `end` is its last point.
    fn with_end_last(&self, end: CurveEnd) -> Self {
        let mut out = self.clone();
        if end == CurveEnd::Start {
            out.reverse();
        }
        out
    }
}

/// Joins open curve `b` to open curve `a`, `a`'s end `a_end` to `b`'s end
/// `b_end`, both in the same space. `weld` melts the two ends into one
/// point at their middle; otherwise a straight stretch joins them, unless
/// they are on the same spot ([`SAME_SPOT`]), when they melt anyway. The
/// result runs as `a` does (from `a`'s other end, unless `a_end` is its
/// start, when it runs the other way); `a`'s points keep their ids and
/// `b`'s get new ones. If its two ends then meet on one spot, they melt
/// and it closes.
pub fn join(
    a: &Curve,
    a_end: CurveEnd,
    b: &Curve,
    b_end: CurveEnd,
    weld: bool,
) -> Result<Curve, CurveError> {
    if a.closed || b.closed || a.points.is_empty() || b.points.is_empty() {
        return Err(CurveError::NotEnds);
    }
    let mut out = a.with_end_last(a_end);
    // `b` turned so its joining end comes first.
    let b = match b_end {
        CurveEnd::Start => b.clone(),
        CurveEnd::End => b.with_end_last(CurveEnd::Start),
    };
    if out.points.len() + b.points.len() > MAX_CURVE_POINTS {
        return Err(CurveError::TooManyPoints);
    }
    let last = out.points.len() - 1;
    let (a_last, b_first) = (out.points[last], b.points[0]);
    let melt = weld || a_last.position.distance(b_first.position) <= SAME_SPOT;
    let mut rest = b.points.as_slice();
    if melt {
        let middle = a_last.position.lerp(b_first.position, 0.5);
        let point = &mut out.points[last];
        point.position = middle;
        point.handle_out = b_first.handle_out;
        point.kind = kind_of_handles(point.handle_in, point.handle_out);
        rest = &rest[1..];
    } else {
        out.points[last].handle_out = Vec3::ZERO;
    }
    out.hold_auto(last);
    for (k, p) in rest.iter().enumerate() {
        let id = out.new_id()?;
        let mut p = *p;
        p.id = id;
        if k == 0 && !melt {
            p.handle_in = Vec3::ZERO;
        }
        out.points.push(p);
    }
    let n = out.points.len();
    if n >= 4 && out.points[0].position.distance(out.points[n - 1].position) <= SAME_SPOT {
        out.close_ends(true)?;
    }
    Curve::from_parts(out.points, out.closed, out.next_id)
}

/// `curve` cut in pieces at points `at`: each one listed ends one piece
/// and starts the next (the ends of an open curve cut nothing). A closed
/// curve cut at one point opens there; cut at more, it falls into that
/// many pieces. The first piece keeps the curve's ids; the copy of a cut
/// point that starts the next piece keeps its id too (each piece is its
/// own curve), except on a closed curve opened at one point, whose copy
/// gets a new id. The cut points' outer handles go, so each piece keeps
/// its shape. Fewer than two pieces: the curve as it was, or opened.
pub fn split(curve: &Curve, at: &[CurvePointId]) -> Result<Vec<Curve>, CurveError> {
    let n = curve.points.len();
    let cut = |i: usize| at.contains(&curve.points[i].id);
    let cuts: Vec<usize> = (0..n)
        .filter(|i| cut(*i) && (curve.closed || (*i > 0 && *i + 1 < n)))
        .collect();
    let Some(&first_cut) = cuts.first() else {
        return Ok(vec![curve.clone()]);
    };
    // The points in order, starting at a cut on a closed curve.
    let order: Vec<usize> = if curve.closed {
        (0..=n).map(|k| (first_cut + k) % n).collect()
    } else {
        (0..n).collect()
    };
    let mut pieces: Vec<Vec<CurvePoint>> = vec![Vec::new()];
    for (k, &i) in order.iter().enumerate() {
        let point = curve.points[i];
        if let Some(piece) = pieces.last_mut() {
            piece.push(point);
        }
        let inner = k > 0 && k + 1 < order.len();
        if inner && cuts.contains(&i) {
            pieces.push(vec![point]);
        }
    }
    let mut next_id = curve.next_id;
    let mut out = Vec::with_capacity(pieces.len());
    for (k, mut points) in pieces.into_iter().enumerate() {
        if points.len() < 2 {
            continue;
        }
        // A closed curve opened at one point: its copy needs an id of its
        // own.
        if curve.closed && cuts.len() == 1 && k == 0 {
            if next_id >= MAX_POINT_ID {
                return Err(CurveError::OutOfIds);
            }
            if let Some(last) = points.last_mut() {
                last.id = CurvePointId(next_id);
            }
            next_id += 1;
        }
        let last = points.len() - 1;
        for i in [0, last] {
            if points[i].kind == PointKind::Auto {
                points[i].kind = PointKind::Smooth;
            }
        }
        points[0].handle_in = Vec3::ZERO;
        points[last].handle_out = Vec3::ZERO;
        out.push(Curve::from_parts(points, false, next_id)?);
    }
    Ok(out)
}
