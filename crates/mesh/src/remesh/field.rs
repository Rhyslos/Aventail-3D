//! The remesh grid's values: which points are inside the shape, and how
//! far each point near the surface is from it.

use super::{Grid, Soup};
use crate::bvh::closest_on_triangle;
use glam::{Vec2, Vec3};

/// How far from the surface (in grid steps) distances are measured; past
/// it, a point only needs to know which side it is on.
const BAND: f32 = 1.5;

/// A point outside made inside to clean up a corner-only touch sits this
/// far (in grid steps) inside the new surface.
const FILLED: f32 = 0.01;

/// The most clean-up passes over the grid. Each pass only fills points
/// in, so it always ends; real shapes need one to three.
const MAX_PASSES: usize = 64;

/// No triangle is nearer: a point far from the surface.
pub(super) const NO_TRIANGLE: u32 = u32::MAX;

/// The grid's values.
pub(super) struct Field {
    /// Signed distance to the surface, in centimetres: below zero inside.
    pub(super) value: Vec<f32>,
    /// Whether each point is inside (the sign of `value`, kept apart so a
    /// point on the surface is still on one side).
    pub(super) inside: Vec<bool>,
    /// The triangle nearest each point near the surface, or
    /// [`NO_TRIANGLE`].
    pub(super) nearest: Vec<u32>,
}

impl Field {
    /// The values of `grid` round the shape `soup`.
    pub(super) fn new(grid: &Grid, soup: &Soup) -> Self {
        let (distance, nearest) = distances(grid, soup);
        let mut inside = parity(grid, soup);
        let before = inside.clone();
        clean_corners(grid, &mut inside);
        let far = BAND * grid.step;
        let value = distance
            .iter()
            .zip(inside.iter().zip(&before))
            .map(|(d, (now, was))| match (now, was) {
                (true, false) => -FILLED * grid.step,
                (true, true) => -d.min(far),
                _ => d.min(far),
            })
            .collect();
        Self {
            value,
            inside,
            nearest,
        }
    }
}

/// Each grid point's distance to the surface where it is within
/// [`BAND`] steps of it (elsewhere infinity), and the triangle nearest.
fn distances(grid: &Grid, soup: &Soup) -> (Vec<f32>, Vec<u32>) {
    let mut distance = vec![f32::INFINITY; grid.len()];
    let mut nearest = vec![NO_TRIANGLE; grid.len()];
    let band = BAND * grid.step;
    for (t, [a, b, c]) in soup.triangles.iter().enumerate() {
        let low = a.min(*b).min(*c) - Vec3::splat(band);
        let high = a.max(*b).max(*c) + Vec3::splat(band);
        let [i0, j0, k0] = cell_index(grid, low, f32::ceil);
        let [i1, j1, k1] = cell_index(grid, high, f32::floor);
        let normal = (*b - *a).cross(*c - *a);
        let flat = normal.length_squared() <= 1e-12 * (*b - *a).length_squared().max(1e-12);
        let unit = if flat { Vec3::ZERO } else { normal.normalize() };
        for k in k0..=k1 {
            for j in j0..=j1 {
                for i in i0..=i1 {
                    let p = grid.point(i, j, k);
                    // Quick skip: too far from the triangle's plane.
                    if !flat && unit.dot(p - *a).abs() >= band {
                        continue;
                    }
                    let d = if flat {
                        segment_distance(p, *a, *b)
                            .min(segment_distance(p, *b, *c))
                            .min(segment_distance(p, *c, *a))
                    } else {
                        p.distance(closest_on_triangle(p, *a, *b, *c))
                    };
                    let at = grid.index(i, j, k);
                    if d < distance[at] {
                        distance[at] = d;
                        nearest[at] = t as u32;
                    }
                }
            }
        }
    }
    (distance, nearest)
}

/// The grid index along each axis of the point nearest `p`, rounded with
/// `round` and kept within the grid.
fn cell_index(grid: &Grid, p: Vec3, round: fn(f32) -> f32) -> [usize; 3] {
    let at = (p - grid.origin) / grid.step;
    let mut out = [0; 3];
    for (axis, slot) in out.iter_mut().enumerate() {
        let max = grid.size[axis].saturating_sub(1) as f32;
        *slot = round(at[axis]).clamp(0.0, max) as usize;
    }
    out
}

/// The distance from `p` to the segment from `a` to `b`.
fn segment_distance(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let length = ab.length_squared();
    let t = if length > 0.0 {
        ((p - a).dot(ab) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.distance(a + ab * t)
}

/// Twice the signed area of the 2D triangle `a b c`.
fn cross(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    (b - a).perp_dot(c - a)
}

/// Which grid points are inside the shape: along each row of points in
/// the X direction, the line through it goes into the surface (through a
/// triangle facing back along X) more often than out of it before each
/// inside point (the *winding number* isn't zero). So shapes that overlap
/// count as one solid, their overlap inside too. The line is moved a
/// tiny, odd fraction of a step off the row, so it never runs exactly
/// through a triangle's edge or corner.
pub(crate) fn parity(grid: &Grid, soup: &Soup) -> Vec<bool> {
    let [nx, ny, nz] = grid.size;
    let nudge = Vec2::new(0.001_234_5, 0.000_765_4) * grid.step;
    // Where a row's line crosses the surface: (row, x).
    // Where a row's line crosses the surface, and whether it goes in (+1)
    // or out (-1) there: (row, x, way).
    let mut hits: Vec<(u32, f32, i32)> = Vec::new();
    for [a, b, c] in &soup.triangles {
        let (pa, pb, pc) = (
            Vec2::new(a.y, a.z),
            Vec2::new(b.y, b.z),
            Vec2::new(c.y, c.z),
        );
        let low = pa.min(pb).min(pc);
        let high = pa.max(pb).max(pc);
        let from = |v: f32, origin: f32, n: usize, round: fn(f32) -> f32| {
            round((v - origin) / grid.step).clamp(0.0, n.saturating_sub(1) as f32) as usize
        };
        let (j0, j1) = (
            from(low.x - nudge.x, grid.origin.y, ny, f32::ceil),
            from(high.x - nudge.x, grid.origin.y, ny, f32::floor),
        );
        let (k0, k1) = (
            from(low.y - nudge.y, grid.origin.z, nz, f32::ceil),
            from(high.y - nudge.y, grid.origin.z, nz, f32::floor),
        );
        let area = cross(pa, pb, pc);
        if area.abs() <= f32::MIN_POSITIVE {
            continue;
        }
        for k in k0..=k1 {
            for j in j0..=j1 {
                let q = Vec2::new(
                    grid.origin.y + j as f32 * grid.step,
                    grid.origin.z + k as f32 * grid.step,
                ) + nudge;
                let (wa, wb, wc) = (cross(pb, pc, q), cross(pc, pa, q), cross(pa, pb, q));
                let all_in = if area > 0.0 {
                    wa > 0.0 && wb > 0.0 && wc > 0.0
                } else {
                    wa < 0.0 && wb < 0.0 && wc < 0.0
                };
                if all_in {
                    let x = (wa * a.x + wb * b.x + wc * c.x) / area;
                    // The triangle's normal along X has the sign of its
                    // area seen along X: facing back, the line goes in.
                    let way = if area < 0.0 { 1 } else { -1 };
                    hits.push(((j + ny * k) as u32, x, way));
                }
            }
        }
    }
    hits.sort_by(|p, q| p.0.cmp(&q.0).then(p.1.total_cmp(&q.1)));
    let mut inside = vec![false; grid.len()];
    let mut start = 0;
    while start < hits.len() {
        let row = hits[start].0;
        let mut end = start;
        while end < hits.len() && hits[end].0 == row {
            end += 1;
        }
        let row_hits = &hits[start..end];
        // A row that doesn't come out as often as it goes in (only by
        // rounding, on a closed shape) would be inside out to the grid's
        // edge: it is counted by crossings instead, its last one dropped
        // if odd.
        let balanced = row_hits.iter().map(|h| h.2).sum::<i32>() == 0;
        let count = if balanced {
            row_hits.len()
        } else {
            row_hits.len() - row_hits.len() % 2
        };
        let xs = &row_hits[..count];
        let (j, k) = (row as usize % ny, row as usize / ny);
        let mut crossed = 0;
        let mut winding = 0;
        for i in 0..nx {
            let x = grid.origin.x + i as f32 * grid.step;
            while crossed < xs.len() && xs[crossed].1 < x {
                winding += xs[crossed].2;
                crossed += 1;
            }
            inside[grid.index(i, j, k)] = if balanced {
                winding != 0
            } else {
                crossed % 2 == 1
            };
        }
        start = end;
    }
    inside
}

/// Fills in outside points where inside points touch only along a
/// square's diagonal or a cube's long diagonal: there the new surface
/// would pinch to a line or a point, and edges would be shared by four
/// quads. Only points are filled in, so it always ends.
fn clean_corners(grid: &Grid, inside: &mut [bool]) {
    let [nx, ny, nz] = grid.size;
    if nx < 2 || ny < 2 || nz < 2 {
        return;
    }
    for _ in 0..MAX_PASSES {
        let mut changed = false;
        for k in 0..nz - 1 {
            for j in 0..ny - 1 {
                for i in 0..nx - 1 {
                    let at = |b: usize| grid.index(i + (b & 1), j + ((b >> 1) & 1), k + (b >> 2));
                    let corners: [bool; 8] = std::array::from_fn(|b| inside[at(b)]);
                    if let Some(b) = fill_for(&corners) {
                        inside[at(b)] = true;
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
}

/// The corner (bit 0 along X, bit 1 along Y, bit 2 along Z) of a grid
/// cell to fill in, if the cell has a diagonal-only touch.
fn fill_for(c: &[bool; 8]) -> Option<usize> {
    // The three faces through corner 0 (each face of the grid is the
    // first face of exactly one cell), as diagonal pairs.
    for [(a, d), (b, e)] in [[(0, 3), (1, 2)], [(0, 5), (1, 4)], [(0, 6), (2, 4)]] {
        if c[a] == c[d] && c[b] == c[e] && c[a] != c[b] {
            return Some(if c[a] { b } else { a });
        }
    }
    let count = c.iter().filter(|x| **x).count();
    for b in 0..4 {
        let opposite = 7 - b;
        if count == 2 && c[b] && c[opposite] {
            // Join them through the corner next to the first along X.
            return Some(b ^ 1);
        }
        if count == 6 && !c[b] && !c[opposite] {
            return Some(b);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closest_points_on_a_triangle() {
        let (a, b, c) = (Vec3::ZERO, Vec3::X, Vec3::Y);
        // Above the inside: straight down.
        let p = closest_on_triangle(Vec3::new(0.2, 0.2, 5.0), a, b, c);
        assert!(p.distance(Vec3::new(0.2, 0.2, 0.0)) < 1e-6);
        // Past a corner, past an edge.
        assert_eq!(closest_on_triangle(Vec3::new(-1.0, -1.0, 0.0), a, b, c), a);
        let p = closest_on_triangle(Vec3::new(0.5, -3.0, 1.0), a, b, c);
        assert!(p.distance(Vec3::new(0.5, 0.0, 0.0)) < 1e-6);
        let p = closest_on_triangle(Vec3::new(2.0, 2.0, 0.0), a, b, c);
        assert!(p.distance(Vec3::new(0.5, 0.5, 0.0)) < 1e-6);
    }

    #[test]
    fn a_flat_triangle_is_measured_along_its_edges() {
        assert!(
            (segment_distance(Vec3::new(0.5, 1.0, 0.0), Vec3::ZERO, Vec3::X) - 1.0).abs() < 1e-6
        );
        assert_eq!(segment_distance(Vec3::Y, Vec3::ZERO, Vec3::ZERO), 1.0);
    }

    #[test]
    fn diagonal_touches_are_filled_in() {
        // Two corners along a face's diagonal.
        let mut c = [false; 8];
        c[0] = true;
        c[3] = true;
        assert_eq!(fill_for(&c), Some(1));
        // Two corners along the cube's long diagonal.
        let mut c = [false; 8];
        c[0] = true;
        c[7] = true;
        assert_eq!(fill_for(&c), Some(1));
        // Six in, the two out opposite.
        let mut c = [true; 8];
        c[2] = false;
        c[5] = false;
        assert_eq!(fill_for(&c), Some(2));
        // Fine as they are.
        assert_eq!(fill_for(&[false; 8]), None);
        assert_eq!(fill_for(&[true; 8]), None);
        let mut c = [false; 8];
        c[0] = true;
        c[1] = true;
        assert_eq!(fill_for(&c), None);
    }
}
