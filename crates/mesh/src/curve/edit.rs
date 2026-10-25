//! Changing a curve: moving parts, deleting, handle kinds and auto
//! handles ([decisions 0138](../../../../docs/decisions/0138-curves-sweep-lathe.md)
//! and [0139](../../../../docs/decisions/0139-spline-editing.md)).

use super::{Curve, CurveEdit, CurveError, CurvePart, CurvePointId, PartKind, PointKind};
use glam::{Mat4, Vec3};

/// A handle shorter than this is nothing.
const TINY: f32 = 1e-12;

impl Curve {
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
            CurveEdit::SetKind { points, kind } => {
                for id in points {
                    out.set_kind(*id, *kind);
                }
            }
            CurveEdit::SetClosed(closed) => {
                if *closed && !out.closed && out.points.len() < 3 {
                    return Err(CurveError::LoopTooSmall);
                }
                out.closed = *closed;
            }
            CurveEdit::Insert { after, t } => out.insert(*after, *t)?,
            CurveEdit::Extend { at_start, points } => out.extend(*at_start, points)?,
            CurveEdit::Reverse => out.reverse(),
            CurveEdit::RoundCorners { points, radius } => out.round_corners(points, *radius)?,
            CurveEdit::Resample { count } => out = out.resampled(*count)?,
            CurveEdit::CloseEnds { weld } => out.close_ends(*weld)?,
        }
        Self::from_parts(out.points, out.closed, out.next_id)
    }

    /// The index of the point before `i` and after it (a closed curve
    /// goes round), if any.
    pub(super) fn neighbours(&self, i: usize) -> (Option<usize>, Option<usize>) {
        let n = self.points.len();
        let before = if i > 0 {
            Some(i - 1)
        } else if self.closed && n > 1 {
            Some(n - 1)
        } else {
            None
        };
        let after = if i + 1 < n {
            Some(i + 1)
        } else if self.closed && n > 1 {
            Some(0)
        } else {
            None
        };
        (before, after)
    }

    /// The handles an auto point at `i` gets: along the line from its
    /// neighbour before to its neighbour after, a third of the way to each
    /// (as a Catmull-Rom curve pulls). An end of an open curve points a
    /// third of the way to its one neighbour.
    pub(super) fn auto_handles(&self, i: usize) -> (Vec3, Vec3) {
        let here = self.points[i].position;
        let (before, after) = self.neighbours(i);
        let before = before.map(|b| self.points[b].position);
        let after = after.map(|a| self.points[a].position);
        match (before, after) {
            (Some(b), Some(a)) => {
                let Some(way) = (a - b).try_normalize() else {
                    return (Vec3::ZERO, Vec3::ZERO);
                };
                (
                    -way * (here.distance(b) / 3.0),
                    way * (here.distance(a) / 3.0),
                )
            }
            (None, Some(a)) => (Vec3::ZERO, (a - here) / 3.0),
            (Some(b), None) => ((b - here) / 3.0, Vec3::ZERO),
            (None, None) => (Vec3::ZERO, Vec3::ZERO),
        }
    }

    /// Brings every point in line with its kind: a sharp point has no
    /// handles; an auto point's follow its neighbours.
    pub(super) fn settle(&mut self) {
        for i in 0..self.points.len() {
            match self.points[i].kind {
                PointKind::Sharp => {
                    self.points[i].handle_in = Vec3::ZERO;
                    self.points[i].handle_out = Vec3::ZERO;
                }
                PointKind::Auto => {
                    let (handle_in, handle_out) = self.auto_handles(i);
                    self.points[i].handle_in = handle_in;
                    self.points[i].handle_out = handle_out;
                }
                PointKind::Smooth | PointKind::Broken => {}
            }
        }
    }

    /// An auto point next to a change that should keep the shape keeps
    /// its handles as they are: it becomes smooth.
    pub(super) fn hold_auto(&mut self, i: usize) {
        if let Some(point) = self.points.get_mut(i)
            && point.kind == PointKind::Auto
        {
            point.kind = PointKind::Smooth;
        }
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
            // Pulling an auto point's handle by hand makes it smooth, so the
            // pull stays (decision 0139).
            if point.kind == PointKind::Auto {
                point.kind = PointKind::Smooth;
            }
            // One handle of a smooth point moved: its partner turns to stay
            // in line, keeping its length.
            if point.kind == PointKind::Smooth && inn != out {
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

    /// Deletes points (and takes handles listed on their own back to
    /// nothing), keeping the shape as near as it can: each gap's
    /// neighbours keep their handles' directions, stretched to span the
    /// gap as the deleted stretches did (decision 0139).
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
        if left < self.points.len() {
            self.bridge_gaps(&gone);
        }
        self.points.retain(|p| !gone.contains(&p.id));
        for part in parts {
            if let Some(point) = self.points.iter_mut().find(|p| p.id == part.point) {
                match part.kind {
                    PartKind::In => point.handle_in = Vec3::ZERO,
                    PartKind::Out => point.handle_out = Vec3::ZERO,
                    PartKind::Point => continue,
                }
                // What's left of its handles moves on its own.
                point.kind = if point.has_handles() {
                    PointKind::Broken
                } else {
                    PointKind::Sharp
                };
            }
        }
        // Two points can't make a loop.
        if self.points.len() < 3 {
            self.closed = false;
        }
        Ok(())
    }

    /// Before the points `gone` are deleted: the points either side of each
    /// run of them stretch their handles across the gap.
    fn bridge_gaps(&mut self, gone: &[CurvePointId]) {
        let n = self.points.len();
        let kept = |i: usize| !gone.contains(&self.points[i].id);
        let spans = self.spans();
        // Each run of deleted points follows one kept point; deleted points
        // at the ends of an open curve just go.
        let mut stretches = Vec::new();
        for a in (0..n).filter(|i| kept(*i)) {
            let mut run = vec![a];
            let mut next = self.neighbours(a).1;
            while let Some(k) = next {
                if kept(k) {
                    if run.len() > 1 {
                        run.push(k);
                        stretches.push(run);
                    }
                    break;
                }
                run.push(k);
                if run.len() > n {
                    break;
                }
                next = self.neighbours(k).1;
            }
        }
        for run in stretches {
            self.stretch_across(&run, &spans);
        }
    }

    /// `run`: a kept point, the deleted points after it, and the kept point
    /// after them. The kept points' handles facing the gap grow by how much
    /// longer the whole gap is than their own stretch; for one deleted
    /// smooth point this undoes adding it exactly.
    fn stretch_across(&mut self, run: &[usize], spans: &[super::Span]) {
        let (Some(&a), Some(&b)) = (run.first(), run.last()) else {
            return;
        };
        if a == b {
            return;
        }
        let length = |i: usize| spans.get(i).map_or(0.0, super::Span::length_estimate);
        let lengths: Vec<f32> = run[..run.len() - 1].iter().map(|i| length(*i)).collect();
        let total: f32 = lengths.iter().sum();
        let (first, last) = (lengths[0], lengths[lengths.len() - 1]);
        // How far along the gap the first deleted point sat, and the last.
        let (t_start, t_end) = if run.len() == 3 {
            let middle = &self.points[run[1]];
            let (i, o) = (middle.handle_in.length(), middle.handle_out.length());
            if i > 1e-6 && o > 1e-6 {
                (i / (i + o), i / (i + o))
            } else {
                (first / total.max(1e-9), first / total.max(1e-9))
            }
        } else {
            (first / total.max(1e-9), 1.0 - last / total.max(1e-9))
        };
        let grow = |handle: Vec3, share: f32| {
            if share <= 1e-6 {
                return handle;
            }
            let longer = handle / share;
            // Never far past the gap itself, where a handle would loop.
            let most = (2.0 * total).max(handle.length());
            if longer.length() > most {
                longer.normalize_or_zero() * most
            } else {
                longer
            }
        };
        self.hold_auto(a);
        self.hold_auto(b);
        self.points[a].handle_out = grow(self.points[a].handle_out, t_start);
        self.points[b].handle_in = grow(self.points[b].handle_in, 1.0 - t_end);
    }

    /// Gives point `id` the handle kind `kind`.
    fn set_kind(&mut self, id: CurvePointId, kind: PointKind) {
        let Some(i) = self.points.iter().position(|p| p.id == id) else {
            return;
        };
        let (auto_in, auto_out) = self.auto_handles(i);
        let point = &mut self.points[i];
        point.kind = kind;
        match kind {
            // Settling takes sharp points' handles away and works auto
            // points' out.
            PointKind::Sharp | PointKind::Auto => {}
            PointKind::Broken => {
                if !point.has_handles() {
                    point.handle_in = auto_in;
                    point.handle_out = auto_out;
                }
            }
            PointKind::Smooth => {
                let existing = point.handle_out - point.handle_in;
                let way = existing
                    .try_normalize()
                    .or_else(|| (auto_out - auto_in).try_normalize())
                    .unwrap_or(Vec3::X);
                // As long as the handles were, or a third of the way to
                // each neighbour.
                let reach = |handle: Vec3, auto: Vec3| {
                    if handle.length_squared() > TINY {
                        handle.length()
                    } else {
                        auto.length()
                    }
                };
                let back = reach(point.handle_in, auto_in);
                let ahead = reach(point.handle_out, auto_out);
                let (back, ahead) = match (back > 0.0, ahead > 0.0) {
                    (false, true) => (ahead, ahead),
                    (true, false) => (back, back),
                    _ => (back, ahead),
                };
                point.handle_in = -way * back;
                point.handle_out = way * ahead;
            }
        }
    }
}
