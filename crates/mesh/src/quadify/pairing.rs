//! Judging and choosing pairs of triangles to join into quads, shared by
//! Quadify ([`super`]) and Decimate's *Quads where flat*
//! ([`crate::decimate`]): how a quad made of two triangles looks
//! ([`quad_shape`]), and the squarest pairs first ([`best_pairs`]).

use glam::DVec3;
use std::collections::HashSet;
use std::f64::consts::FRAC_PI_2;

/// The smallest a triangle's doubled area may be (in the square of the
/// mesh's units) and still have a facing.
const NO_AREA: f64 = 1e-14;

/// How a quad made of two triangles looks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct QuadShape {
    /// How far the two triangles lean from each other (the angle between
    /// their facings), in radians: 0 is flat.
    pub(crate) fold: f64,
    /// How far its four corners are from right angles, added up, in
    /// radians: 0 is a rectangle (lower is squarer).
    pub(crate) squareness: f64,
    /// How far its least square corner is from a right angle, in radians.
    pub(crate) worst: f64,
}

/// The shape of the quad with corners `p`, in order round it, made of the
/// triangles `p0 p2 p3` and `p0 p1 p2` (the diagonal `p0`–`p2` goes). `None`
/// when either triangle has no area, the quad has a dent (a corner
/// turning the wrong way, seen along the two facings together), or a
/// place isn't a real number.
pub(crate) fn quad_shape(p: [DVec3; 4]) -> Option<QuadShape> {
    let n1 = (p[2] - p[0]).cross(p[3] - p[0]);
    let n2 = (p[1] - p[0]).cross(p[2] - p[0]);
    let (l1, l2) = (n1.length(), n2.length());
    // A place that isn't a number is refused too.
    if l1.is_nan() || l2.is_nan() || l1 <= NO_AREA || l2 <= NO_AREA {
        return None;
    }
    let fold = (n1.dot(n2) / (l1 * l2)).clamp(-1.0, 1.0).acos();
    let normal = n1 / l1 + n2 / l2;
    if normal.length_squared() <= 1e-12 {
        return None;
    }
    let mut squareness = 0.0;
    let mut worst: f64 = 0.0;
    for i in 0..4 {
        let (prev, here, next) = (p[(i + 3) % 4], p[i], p[(i + 1) % 4]);
        let (e1, e2) = (here - prev, next - here);
        // A corner turning the wrong way is a dent.
        let turn = e1.cross(e2).dot(normal);
        if turn.is_nan() || turn <= 0.0 {
            return None;
        }
        let off = ((-e1).angle_between(e2) - FRAC_PI_2).abs();
        if !off.is_finite() {
            return None;
        }
        squareness += off;
        worst = worst.max(off);
    }
    Some(QuadShape {
        fold,
        squareness,
        worst,
    })
}

/// A pair of triangles that may be joined.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Candidate<T> {
    /// Lower is better.
    pub(crate) score: f64,
    /// The first triangle's number (the lower).
    pub(crate) first: u32,
    /// The second triangle's number.
    pub(crate) second: u32,
    /// What the caller needs to join them.
    pub(crate) item: T,
}

/// The pairs to join: the best scored first, each triangle in one pair
/// at most. Ties go by the triangles' numbers, so the same candidates
/// always give the same pairs, in that order.
pub(crate) fn best_pairs<T>(mut candidates: Vec<Candidate<T>>) -> Vec<Candidate<T>> {
    candidates.sort_by(|x, y| {
        x.score
            .total_cmp(&y.score)
            .then(x.first.cmp(&y.first))
            .then(x.second.cmp(&y.second))
    });
    let mut taken: HashSet<u32> = HashSet::new();
    let mut out = Vec::new();
    for candidate in candidates {
        if taken.contains(&candidate.first) || taken.contains(&candidate.second) {
            continue;
        }
        taken.insert(candidate.first);
        taken.insert(candidate.second);
        out.push(candidate);
    }
    out
}
