//! The *Beauty* cut ([decision 0161](../../../../docs/decisions/0161-triangulate.md)):
//! a polygon's triangles evened out, so long thin slivers become
//! triangles closer to equal-sided.
//!
//! It starts from the cut the export uses ([`super::triangulate`]) and
//! turns the line between two triangles to the other diagonal of the
//! four-sided shape they make whenever that gives a larger smallest
//! corner (Lawson's flips, as a Delaunay triangulation is made). A line
//! is only turned when both triangles lie flat with each other (within
//! [`FLAT_DEGREES`]) and the other diagonal runs inside them, so a bent
//! polygon keeps the fold the export's cut follows and no triangle ever
//! faces the other way. The same polygon always gets the same cut.

use super::{flatten, newell, turn};
use glam::{Vec2, Vec3};
use std::collections::BTreeMap;

/// How far, in degrees, two triangles may lean from each other and still
/// count as flat for turning the line between them.
pub const FLAT_DEGREES: f32 = 1.0;

/// How much larger (in radians) the smallest corner must get for a line
/// to be turned: two equal choices keep the export's.
const BETTER_BY: f32 = 1e-4;

/// The most rounds of turning lines; each round turns every line that
/// improves, and a polygon is usually done in a few.
const MAX_ROUNDS: usize = 64;

/// `triangles` (of the polygon with corners at `points`) with lines
/// turned as the module notes say. Triangles keep their place in the list.
pub(super) fn beautify(points: &[Vec3], triangles: &mut [[usize; 3]]) {
    if triangles.len() < 2 {
        return;
    }
    let Some(normal) = newell(points).try_normalize() else {
        return;
    };
    let flat = flatten(points, normal);
    let flat_cos = FLAT_DEGREES.to_radians().cos();
    for _ in 0..MAX_ROUNDS {
        let mut turned = false;
        for (a, b) in shared_lines(triangles) {
            if let Some((first, second)) =
                turn_line(points, &flat, triangles[a], triangles[b], flat_cos)
            {
                triangles[a] = first;
                triangles[b] = second;
                turned = true;
            }
        }
        if !turned {
            break;
        }
    }
}

/// Every pair of triangles sharing a line, by their places in the list,
/// in a fixed order.
fn shared_lines(triangles: &[[usize; 3]]) -> Vec<(usize, usize)> {
    let mut by_line: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new();
    for (t, [a, b, c]) in triangles.iter().enumerate() {
        for (x, y) in [(*a, *b), (*b, *c), (*c, *a)] {
            by_line.entry((x.min(y), x.max(y))).or_default().push(t);
        }
    }
    by_line
        .into_values()
        .filter(|ts| ts.len() == 2)
        .map(|ts| (ts[0], ts[1]))
        .collect()
}

/// The two triangles with the line between them turned, when that is
/// allowed and better; `None` otherwise (also when the two triangles
/// changed this round and no longer share a line).
fn turn_line(
    points: &[Vec3],
    flat: &[Vec2],
    first: [usize; 3],
    second: [usize; 3],
    flat_cos: f32,
) -> Option<([usize; 3], [usize; 3])> {
    // The shared line i → j as `first` runs, k its far corner; `second`
    // runs j → i, l its far corner.
    let (i, j, k) = (0..3).find_map(|s| {
        let (i, j, k) = (first[s], first[(s + 1) % 3], first[(s + 2) % 3]);
        (0..3)
            .any(|r| second[r] == j && second[(r + 1) % 3] == i)
            .then_some((i, j, k))
    })?;
    let l = *second.iter().find(|c| **c != i && **c != j)?;
    if k == l {
        return None;
    }
    // Flat with each other (a triangle with no area counts as flat).
    let facing = |[a, b, c]: [usize; 3]| (points[b] - points[a]).cross(points[c] - points[a]);
    let (n1, n2) = (facing([i, j, k]), facing([j, i, l]));
    if let (Some(n1), Some(n2)) = (n1.try_normalize(), n2.try_normalize())
        && n1.dot(n2) < flat_cos
    {
        return None;
    }
    // The shape runs i → l → j → k round: the new triangles keep its way.
    // They must both turn that way with some area, so the shape has no
    // dent at i or j and the other diagonal runs inside it (a straight
    // corner at k or l, where a sliver had no area, is fine).
    let (new_first, new_second) = ([k, i, l], [l, j, k]);
    let size = [i, j, k, l]
        .iter()
        .map(|p| flat[*p].distance_squared(flat[i]))
        .fold(0.0f32, f32::max);
    let least = size * 1e-7;
    if !(turn(flat[k], flat[i], flat[l]) > least && turn(flat[l], flat[j], flat[k]) > least) {
        return None;
    }
    let now = smallest_corner(flat, [i, j, k]).min(smallest_corner(flat, [j, i, l]));
    let then = smallest_corner(flat, new_first).min(smallest_corner(flat, new_second));
    (then > now + BETTER_BY).then_some((new_first, new_second))
}

/// The smallest corner of a flat triangle, in radians (0 for one with no
/// area).
fn smallest_corner(flat: &[Vec2], [a, b, c]: [usize; 3]) -> f32 {
    let corner = |p: Vec2, q: Vec2, r: Vec2| {
        let angle = (q - p).angle_to(r - p).abs();
        if angle.is_finite() { angle } else { 0.0 }
    };
    let (pa, pb, pc) = (flat[a], flat[b], flat[c]);
    corner(pa, pb, pc)
        .min(corner(pb, pc, pa))
        .min(corner(pc, pa, pb))
}
