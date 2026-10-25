//! Rounding a curve's corners and spacing its points evenly
//! ([decision 0139](../../../../docs/decisions/0139-spline-editing.md)).

use super::{
    Curve, CurveError, CurvePoint, CurvePointId, MAX_CURVE_POINTS, MAX_POINT_ID, PointKind,
};
use glam::Vec3;
use std::f32::consts::PI;

/// The largest radius a rounded corner may ask for (10 km); it is cut back
/// to what fits anyway.
pub const MAX_RADIUS: f32 = super::MAX_COORDINATE;

/// How finely each stretch is measured when spacing points evenly.
const MEASURE_STEPS: usize = 64;

impl Curve {
    /// Whether point `id` is a corner [`CurveEdit::RoundCorners`] rounds:
    /// sharp, with a neighbour on each side, not in a straight line with
    /// them.
    ///
    /// [`CurveEdit::RoundCorners`]: super::CurveEdit::RoundCorners
    pub fn can_round(&self, id: CurvePointId) -> bool {
        self.points
            .iter()
            .position(|p| p.id == id)
            .is_some_and(|i| self.is_roundable(i))
    }

    /// Whether point `i` is a corner that can be rounded: sharp, with a
    /// neighbour on each side, not in a straight line with them.
    fn is_roundable(&self, i: usize) -> bool {
        let point = &self.points[i];
        if point.kind != PointKind::Sharp {
            return false;
        }
        let (Some(b), Some(a)) = self.neighbours(i) else {
            return false;
        };
        let here = point.position;
        let (Some(u), Some(v)) = (
            (self.points[b].position - here).try_normalize(),
            (self.points[a].position - here).try_normalize(),
        ) else {
            return false;
        };
        let angle = u.dot(v).clamp(-1.0, 1.0).acos();
        angle < PI - 1e-3 && angle > 1e-4
    }

    /// Rounds the sharp corners `ids` into arcs of `radius`, each cut back
    /// so it fits on its legs (half a leg where the next corner is rounded
    /// too). The first point of each arc keeps the corner's id.
    pub(super) fn round_corners(
        &mut self,
        ids: &[CurvePointId],
        radius: f32,
    ) -> Result<(), CurveError> {
        if !radius.is_finite() {
            return Err(CurveError::NotANumber);
        }
        let radius = radius.clamp(0.0, MAX_RADIUS);
        if radius <= 0.0 {
            return Ok(());
        }
        let n = self.points.len();
        let chosen: Vec<bool> = (0..n)
            .map(|i| ids.contains(&self.points[i].id) && self.is_roundable(i))
            .collect();
        let count = chosen.iter().filter(|c| **c).count();
        if n + count > MAX_CURVE_POINTS {
            return Err(CurveError::TooManyPoints);
        }
        let mut arcs: Vec<Option<[CurvePoint; 2]>> = vec![None; n];
        for i in (0..n).filter(|i| chosen[*i]) {
            let (Some(b), Some(a)) = self.neighbours(i) else {
                continue;
            };
            let here = self.points[i].position;
            let (before, after) = (self.points[b].position, self.points[a].position);
            let (u, v) = (
                (before - here).normalize_or_zero(),
                (after - here).normalize_or_zero(),
            );
            let angle = u.dot(v).clamp(-1.0, 1.0).acos();
            let half_tan = (angle / 2.0).tan();
            // How far along each leg the arc may start: all of it, or half
            // where the corner at its other end is rounded too.
            let share = |other: usize| if chosen[other] { 0.5 } else { 0.999 };
            let room = (here.distance(before) * share(b)).min(here.distance(after) * share(a));
            let reach = (radius / half_tan.max(1e-6)).min(room);
            let fitted = reach * half_tan;
            // The handles of a Bézier arc through `turn` radians.
            let turn = PI - angle;
            let pull = 4.0 / 3.0 * (turn / 4.0).tan() * fitted;
            let id = self.new_id()?;
            let start = CurvePoint {
                id: self.points[i].id,
                position: here + u * reach,
                handle_in: Vec3::ZERO,
                handle_out: -u * pull,
                kind: PointKind::Smooth,
            };
            let end = CurvePoint {
                id,
                position: here + v * reach,
                handle_in: -v * pull,
                handle_out: Vec3::ZERO,
                kind: PointKind::Smooth,
            };
            arcs[i] = Some([start, end]);
        }
        let old = std::mem::take(&mut self.points);
        for (i, point) in old.into_iter().enumerate() {
            match arcs[i] {
                Some(pair) => self.points.extend(pair),
                None => self.points.push(point),
            }
        }
        Ok(())
    }

    /// The curve with `count` points spread evenly along its length (at
    /// least two, three for a loop): each with handles along the curve a
    /// third of the way to its neighbours, or none where it sits on a
    /// straight stretch. All new ids; the first point stays where the
    /// curve began.
    pub(super) fn resampled(&self, count: usize) -> Result<Self, CurveError> {
        let least = if self.closed { 3 } else { 2 };
        let count = count.clamp(least, MAX_CURVE_POINTS);
        if self.next_id as usize + count > MAX_POINT_ID as usize {
            return Err(CurveError::OutOfIds);
        }
        let spans = self.spans();
        if spans.is_empty() {
            return Ok(self.clone());
        }
        // The curve measured finely: (stretch, t, length so far).
        let mut marks = vec![(0usize, 0.0f32, 0.0f32)];
        let mut total = 0.0;
        for (s, span) in spans.iter().enumerate() {
            let mut last = span.start;
            for k in 1..=MEASURE_STEPS {
                let t = k as f32 / MEASURE_STEPS as f32;
                let here = span.at(t);
                total += here.distance(last);
                last = here;
                marks.push((s, t, total));
            }
        }
        let gaps = if self.closed { count } else { count - 1 };
        let spacing = total / gaps as f32;
        let mut points = Vec::with_capacity(count);
        for k in 0..count {
            let want = spacing * k as f32;
            let (s, t) = find(&marks, want);
            let span = spans[s];
            let position = span.at(t);
            let way = span.direction(t).normalize_or_zero() * (spacing / 3.0);
            let flat = span.is_straight() || way == Vec3::ZERO;
            let mut point = CurvePoint {
                id: CurvePointId(self.next_id + k as u32),
                position,
                handle_in: -way,
                handle_out: way,
                kind: if flat {
                    PointKind::Sharp
                } else {
                    PointKind::Smooth
                },
            };
            if !self.closed && k == 0 {
                point.handle_in = Vec3::ZERO;
            }
            if !self.closed && k + 1 == count {
                point.handle_out = Vec3::ZERO;
            }
            points.push(point);
        }
        Self::from_parts(points, self.closed, self.next_id + count as u32)
    }
}

/// The stretch and its `t` at `length` along the curve, from `marks`.
fn find(marks: &[(usize, f32, f32)], length: f32) -> (usize, f32) {
    let k = marks
        .partition_point(|m| m.2 < length)
        .clamp(1, marks.len() - 1);
    let (s0, t0, l0) = marks[k - 1];
    let (s1, t1, l1) = marks[k];
    let step = l1 - l0;
    let f = if step > 1e-9 {
        ((length - l0) / step).clamp(0.0, 1.0)
    } else {
        0.0
    };
    if s0 == s1 {
        (s1, t0 + (t1 - t0) * f)
    } else {
        // Across a stretch's start: `t0` was the end of the one before.
        (s1, t1 * f)
    }
}
