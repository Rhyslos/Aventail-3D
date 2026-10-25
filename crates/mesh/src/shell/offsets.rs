//! Which way, and how far, each point of a shell moves for a thickness
//! of 1 ([decision 0154](../../../../docs/decisions/0154-shell.md)).
//!
//! A point's **direction** is the mean of its polygons' facings, each
//! counted by the angle of its corner there (so a polygon cut in two
//! doesn't count twice): the usual "point normal".
//!
//! With **even thickness**, the point moves so far that *every* polygon
//! round it ends up the thickness away from where it was: the offset `v`
//! with `v · n = 1` for each polygon facing `n`, found by least squares
//! (weighted by the corner angles) when they can't all be met exactly.
//! On a flat part that is the facing itself; along a box's edge it is
//! the half-way direction, longer by 1 / cos(half the angle); at a box's
//! corner it is (1, 1, 1). Without it, every point moves the thickness
//! along its direction, and sharp corners come out thinner.

use crate::{Mesh, VertexId};
use glam::{DMat3, DVec3, Vec3};
use std::collections::HashMap;

/// How much longer than the thickness an even offset may be, at the
/// sharpest spikes (a spike of about 23° between its sides): beyond
/// that, the point's offset is cut back, so a needle-thin spike can't
/// shoot off into the distance.
pub const MAX_STRETCH: f32 = 5.0;

/// What one point gathers from the polygons round it.
#[derive(Clone, Copy)]
struct Gathered {
    /// The facings, each times its corner angle.
    sum: DVec3,
    /// The facings' outer products, each times its corner angle.
    outer: DMat3,
    /// The corner angles added up.
    weight: f64,
}

impl Default for Gathered {
    // Not derived: a matrix's standard is the identity, not zero.
    fn default() -> Self {
        Self {
            sum: DVec3::ZERO,
            outer: DMat3::ZERO,
            weight: 0.0,
        }
    }
}

/// Each point's offset for a thickness of 1 (see the module's notes),
/// for every point a polygon uses. Points whose polygons have no area
/// (or whose facings cancel out) don't move.
pub fn offsets(mesh: &Mesh, even: bool) -> HashMap<VertexId, Vec3> {
    let mut gathered: HashMap<VertexId, Gathered> = HashMap::new();
    for (_, corners) in mesh.faces() {
        let normal = newell(mesh, corners);
        let n = corners.len();
        for (k, v) in corners.iter().enumerate() {
            let entry = gathered.entry(*v).or_default();
            let Some(normal) = normal else {
                continue;
            };
            let angle = corner_angle(mesh, corners[(k + n - 1) % n], *v, corners[(k + 1) % n]);
            if angle <= 0.0 {
                continue;
            }
            entry.sum += normal * angle;
            entry.outer += outer(normal) * angle;
            entry.weight += angle;
        }
    }
    gathered
        .into_iter()
        .map(|(v, g)| (v, offset_of(&g, even)))
        .collect()
}

/// One point's offset from what it gathered.
fn offset_of(g: &Gathered, even: bool) -> Vec3 {
    let Some(direction) = g.sum.try_normalize() else {
        return Vec3::ZERO;
    };
    if !even {
        return direction.as_vec3();
    }
    // Least squares for v · n = 1 over the polygons round the point. A
    // little of "along the direction" is mixed in, so a flat part (where
    // sideways is free) still gives the facing itself.
    let nudge = g.weight * 1e-6;
    let matrix = g.outer + DMat3::IDENTITY * nudge;
    let target = g.sum + direction * nudge;
    let solved = if matrix.determinant().abs() > 1e-300 {
        matrix.inverse() * target
    } else {
        direction
    };
    let along = solved.dot(direction);
    if !solved.is_finite() || along < 0.5 {
        // Facings that contradict each other (a fold): the plain way.
        return direction.as_vec3();
    }
    let length = solved.length();
    let most = f64::from(MAX_STRETCH);
    let out = if length > most {
        solved * (most / length)
    } else {
        solved
    };
    out.as_vec3()
}

/// A polygon's facing (Newell's method), or `None` if it has no area.
fn newell(mesh: &Mesh, corners: &[VertexId]) -> Option<DVec3> {
    let mut normal = DVec3::ZERO;
    let n = corners.len();
    for k in 0..n {
        let (Some(p), Some(q)) = (
            mesh.position(corners[k]),
            mesh.position(corners[(k + 1) % n]),
        ) else {
            continue;
        };
        let (p, q) = (p.as_dvec3(), q.as_dvec3());
        normal += DVec3::new(
            (p.y - q.y) * (p.z + q.z),
            (p.z - q.z) * (p.x + q.x),
            (p.x - q.x) * (p.y + q.y),
        );
    }
    (normal.length() > 1e-12)
        .then(|| normal.normalize())
        .filter(|n| n.is_finite())
}

/// The angle at `at` between the sides to `before` and `after`, in
/// radians: 0 when a side has no length.
fn corner_angle(mesh: &Mesh, before: VertexId, at: VertexId, after: VertexId) -> f64 {
    let (Some(p), Some(a), Some(b)) = (
        mesh.position(at),
        mesh.position(before),
        mesh.position(after),
    ) else {
        return 0.0;
    };
    let (p, a, b) = (p.as_dvec3(), a.as_dvec3(), b.as_dvec3());
    match ((a - p).try_normalize(), (b - p).try_normalize()) {
        (Some(x), Some(y)) => x.dot(y).clamp(-1.0, 1.0).acos(),
        _ => 0.0,
    }
}

/// `n nᵀ`.
fn outer(n: DVec3) -> DMat3 {
    DMat3::from_cols(n * n.x, n * n.y, n * n.z)
}
