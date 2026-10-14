//! Cutting polygons into triangles to draw and pick
//! ([decision 0057](../../../docs/decisions/0057-after-phase-4-testing.md)).
//!
//! A screen draws triangles, so every polygon is cut into them. The cut is
//! worked out from where the corners are:
//!
//! - **Triangles** stay as they are, and **quads** are cut corner 0 to 2
//!   unless that line would run outside a dented quad (then 1 to 3).
//! - **Flat polygons without dents**, like a cylinder's cap, are cut as a
//!   fan from their first corner: quick, and right for them.
//! - **Anything else** (a polygon with a dent, or a bent one, as Dissolve
//!   makes across a corner) is laid flat on the plane it is closest to and
//!   cut along lines that stay inside it. With up to [`BEST_CUT_UP_TO`]
//!   corners the shortest cut is chosen, which follows the fold of a bent
//!   polygon; bigger ones are cut ear by ear.
//!
//! The same polygon always gets the same cut, so drawing, picking and
//! (later) export agree. If nothing works (a polygon folded flat onto
//! itself), the fan is used.

use crate::{Mesh, VertexId};
use glam::{Vec2, Vec3};

/// Up to how many corners a polygon gets the shortest cut (worked out in
/// time growing with the cube of the corners); bigger ones are cut ear by
/// ear.
pub const BEST_CUT_UP_TO: usize = 32;

/// How far (a share of the polygon's size) a corner may be off the
/// polygon's plane for it to count as flat.
const FLAT: f32 = 1e-4;

/// Cuts a polygon with `n` corners, the corner `i` at `at(i)`, into
/// `n - 2` triangles, calling `out` with each one's three corner places
/// (counter-clockwise as the polygon runs). Polygons with fewer than three
/// corners give nothing.
pub fn triangulate(n: usize, at: impl Fn(usize) -> Vec3, mut out: impl FnMut([usize; 3])) {
    match n {
        0..=2 => {}
        3 => out([0, 1, 2]),
        4 => {
            let p = [at(0), at(1), at(2), at(3)];
            let normal = newell(&p);
            // A corner turning the wrong way is a dent; the line between
            // the other two corners would run outside the polygon.
            let dent = |i: usize| {
                let (a, b, c) = (p[(i + 3) % 4], p[i], p[(i + 1) % 4]);
                (b - a).cross(c - b).dot(normal) < 0.0
            };
            if dent(1) || dent(3) {
                out([0, 1, 3]);
                out([1, 2, 3]);
            } else {
                out([0, 1, 2]);
                out([0, 2, 3]);
            }
        }
        _ => {
            let points: Vec<Vec3> = (0..n).map(&at).collect();
            for triangle in triangles(&points) {
                out(triangle);
            }
        }
    }
}

/// Cuts the polygon with `corners` of `mesh` into triangles (see
/// [`triangulate`]), calling `out` with each one's three corner places.
pub fn face_triangles(mesh: &Mesh, corners: &[VertexId], out: impl FnMut([usize; 3])) {
    triangulate(
        corners.len(),
        |i| mesh.position(corners[i]).unwrap_or(Vec3::ZERO),
        out,
    );
}

/// The triangles of a polygon with five or more corners (see the module
/// notes).
fn triangles(points: &[Vec3]) -> Vec<[usize; 3]> {
    let n = points.len();
    let fan = || (1..n - 1).map(|i| [0, i, i + 1]).collect::<Vec<_>>();
    let normal = newell(points);
    let Some(normal) = normal.try_normalize() else {
        return fan();
    };
    let flat_points = flatten(points, normal);
    let size = points
        .iter()
        .map(|p| p.distance(points[0]))
        .fold(0.0f32, f32::max)
        .max(1e-6);
    let middle = points.iter().copied().sum::<Vec3>() / n as f32;
    let flat = points
        .iter()
        .all(|p| (*p - middle).dot(normal).abs() <= size * FLAT);
    if flat && convex(&flat_points) {
        return fan();
    }
    let cut = if n <= BEST_CUT_UP_TO {
        shortest_cut(points, &flat_points)
    } else {
        ear_cut(&flat_points)
    };
    cut.unwrap_or_else(fan)
}

/// Twice the polygon's area, along its facing (Newell's method): its
/// normal, worked out so a bent or dented polygon still gets a fair one.
fn newell(points: &[Vec3]) -> Vec3 {
    let middle = points.iter().copied().sum::<Vec3>() / points.len().max(1) as f32;
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(a, b)| (*a - middle).cross(*b - middle))
        .sum()
}

/// The points laid flat on the plane facing `normal`, seen so the polygon
/// runs counter-clockwise.
fn flatten(points: &[Vec3], normal: Vec3) -> Vec<Vec2> {
    let x = normal.any_orthonormal_vector();
    let y = normal.cross(x);
    points
        .iter()
        .map(|p| Vec2::new(p.dot(x), p.dot(y)))
        .collect()
}

/// Twice the signed area of the triangle `a b c`: positive when it runs
/// counter-clockwise.
fn turn(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    (b - a).perp_dot(c - a)
}

/// Whether the flat polygon has no dents (no corner turning clockwise).
fn convex(points: &[Vec2]) -> bool {
    let n = points.len();
    let scale = points
        .iter()
        .map(|p| p.distance_squared(points[0]))
        .fold(0.0f32, f32::max)
        .max(1e-12);
    (0..n).all(|i| turn(points[(i + n - 1) % n], points[i], points[(i + 1) % n]) >= -scale * 1e-6)
}

/// Whether the straight line between corners `i` and `j` of the flat
/// polygon runs inside it: it starts and ends into the polygon, and
/// crosses none of its sides.
fn inside(points: &[Vec2], i: usize, j: usize) -> bool {
    let n = points.len();
    let (a, b) = (points[i], points[j]);
    if a.distance_squared(b) <= 0.0 {
        return false;
    }
    // Into the polygon at both ends: between the sides meeting there.
    let in_cone = |k: usize, towards: Vec2| {
        let (prev, here, next) = (points[(k + n - 1) % n], points[k], points[(k + 1) % n]);
        if turn(prev, here, next) >= 0.0 {
            // A corner turning left: inside means left of both sides.
            turn(here, next, towards) > 0.0 && turn(prev, here, towards) > 0.0
        } else {
            // A dent: anywhere but the outside wedge.
            !(turn(here, next, towards) <= 0.0 && turn(prev, here, towards) <= 0.0)
        }
    };
    if !in_cone(i, b) || !in_cone(j, a) {
        return false;
    }
    // Crossing no side that doesn't touch either end.
    (0..n).all(|k| {
        let l = (k + 1) % n;
        if k == i || k == j || l == i || l == j {
            return true;
        }
        !crosses(a, b, points[k], points[l])
    })
}

/// Whether the segments `a b` and `c d` cross or touch.
fn crosses(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> bool {
    let (d1, d2) = (turn(a, b, c), turn(a, b, d));
    let (d3, d4) = (turn(c, d, a), turn(c, d, b));
    if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
    {
        return true;
    }
    let on = |p: Vec2, q: Vec2, r: Vec2, t: f32| {
        t == 0.0
            && r.x >= p.x.min(q.x)
            && r.x <= p.x.max(q.x)
            && r.y >= p.y.min(q.y)
            && r.y <= p.y.max(q.y)
    };
    on(a, b, c, d1) || on(a, b, d, d2) || on(c, d, a, d3) || on(c, d, b, d4)
}

/// The cut whose inside lines are shortest in all (lengths in 3D), using
/// only lines that stay inside the flat polygon. A bent polygon is cut
/// along its fold, as the fold is the shortest way across. `None` when no
/// cut stays inside.
fn shortest_cut(points: &[Vec3], flat: &[Vec2]) -> Option<Vec<[usize; 3]>> {
    let n = points.len();
    // What a line between two corners costs: nothing along a side, its
    // length inside, and no way at all outside.
    let mut weight = vec![f32::INFINITY; n * n];
    for i in 0..n {
        for j in i + 1..n {
            let w = if j == i + 1 || (i == 0 && j == n - 1) {
                0.0
            } else if inside(flat, i, j) {
                points[i].distance(points[j])
            } else {
                f32::INFINITY
            };
            weight[i * n + j] = w;
        }
    }
    // best[i][j]: the cheapest cut of the corners i to j (a polygon closed
    // by the line j to i), and the corner k its triangle on that line uses.
    let mut best = vec![f32::INFINITY; n * n];
    let mut pick = vec![usize::MAX; n * n];
    for i in 0..n - 1 {
        best[i * n + i + 1] = 0.0;
    }
    for span in 2..n {
        for i in 0..n - span {
            let j = i + span;
            if weight[i * n + j].is_infinite() {
                continue;
            }
            for k in i + 1..j {
                let cost =
                    best[i * n + k] + best[k * n + j] + weight[i * n + k] + weight[k * n + j];
                if cost < best[i * n + j] {
                    best[i * n + j] = cost;
                    pick[i * n + j] = k;
                }
            }
        }
    }
    if best[n - 1].is_infinite() {
        return None;
    }
    let mut out = Vec::with_capacity(n - 2);
    let mut todo = vec![(0, n - 1)];
    while let Some((i, j)) = todo.pop() {
        if j <= i + 1 {
            continue;
        }
        let k = pick[i * n + j];
        if k == usize::MAX {
            return None;
        }
        out.push([i, k, j]);
        todo.push((i, k));
        todo.push((k, j));
    }
    Some(out)
}

/// Cuts the flat polygon ear by ear: a corner whose triangle with its two
/// neighbours lies inside the polygon is cut off, until a triangle is
/// left. `None` when it gets stuck (the polygon crosses itself).
fn ear_cut(flat: &[Vec2]) -> Option<Vec<[usize; 3]>> {
    let mut left: Vec<usize> = (0..flat.len()).collect();
    let mut out = Vec::with_capacity(flat.len() - 2);
    let mut since_cut = 0;
    let mut i = 0;
    while left.len() > 3 {
        let m = left.len();
        let (a, b, c) = (left[(i + m - 1) % m], left[i % m], left[(i + 1) % m]);
        let ear = turn(flat[a], flat[b], flat[c]) > 0.0
            && left.iter().all(|&p| {
                p == a || p == b || p == c || !in_triangle(flat[p], flat[a], flat[b], flat[c])
            });
        if ear {
            out.push([a, b, c]);
            left.remove(i % m);
            since_cut = 0;
        } else {
            i += 1;
            since_cut += 1;
            if since_cut > m {
                return None;
            }
        }
        i %= left.len();
    }
    out.push([left[0], left[1], left[2]]);
    Some(out)
}

/// Whether `p` is inside (or on) the counter-clockwise triangle `a b c`.
fn in_triangle(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    turn(a, b, p) >= 0.0 && turn(b, c, p) >= 0.0 && turn(c, a, p) >= 0.0
}

#[cfg(test)]
mod tests;
