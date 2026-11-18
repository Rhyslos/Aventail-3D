//! What the remesh grid's values are made from: which points are inside
//! the shape (from where each row of points crosses its surface), how far
//! a point is from a triangle, and which grid cells need a corner filled
//! in so the new surface stays a clean solid.

use super::{Grid, Soup};
use glam::{Vec2, Vec3};

/// How far from the surface (in grid steps) distances are measured; past
/// it, a point only needs to know which side it is on. One step is
/// enough: the new surface's points are worked out only along grid edges
/// it crosses, whose ends are at most a step from it.
pub(super) const BAND: f32 = 1.0;

/// A point outside made inside to clean up a corner-only touch sits this
/// far (in grid steps) inside the new surface.
pub(super) const FILLED: f32 = 0.01;

/// No triangle is nearer: a point far from the surface.
pub(super) const NO_TRIANGLE: u32 = u32::MAX;

/// The grid index along each axis of the point nearest `p`, rounded with
/// `round` and kept within the grid.
pub(super) fn cell_index(grid: &Grid, p: Vec3, round: fn(f32) -> f32) -> [usize; 3] {
    let at = (p - grid.origin) / grid.step;
    let mut out = [0; 3];
    for (axis, slot) in out.iter_mut().enumerate() {
        let max = grid.size[axis].saturating_sub(1) as f32;
        *slot = round(at[axis]).clamp(0.0, max) as usize;
    }
    out
}

/// The distance from `p` to the segment from `a` to `b`.
pub(super) fn segment_distance(p: Vec3, a: Vec3, b: Vec3) -> f32 {
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

/// Where a row of grid points (along X) crosses the surface.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Hit {
    /// The row: `j + ny * k`.
    row: u64,
    /// Where along X.
    x: f32,
    /// Going in (+1) or out (-1).
    way: i8,
}

/// Every place the rows of grid points along X cross a shape's surface,
/// row by row and along each row, so whether any point is inside can be
/// told from its row alone. Each row's line is moved a tiny, odd fraction
/// of a step off the row, so it never runs exactly through a triangle's
/// edge or corner.
pub(crate) struct Rows {
    hits: Vec<Hit>,
}

impl Rows {
    /// The crossings of `grid`'s rows with `soup`'s triangles, worked out
    /// on every core.
    pub(crate) fn new(grid: &Grid, soup: &Soup) -> Self {
        let parts = crate::parallel::in_parallel(soup.triangles.len(), |range| {
            let mut hits = Vec::new();
            for t in &soup.triangles[range] {
                crossings(grid, t, &mut hits);
            }
            hits
        });
        let mut hits: Vec<Hit> = parts.into_iter().flatten().collect();
        hits.sort_unstable_by(|p, q| p.row.cmp(&q.row).then(p.x.total_cmp(&q.x)));
        Self { hits }
    }

    /// Row (`j`, `k`)'s crossings.
    pub(crate) fn row(&self, grid: &Grid, j: usize, k: usize) -> &[Hit] {
        let row = j as u64 + grid.size[1] as u64 * k as u64;
        let from = self.hits.partition_point(|h| h.row < row);
        let to = from + self.hits[from..].partition_point(|h| h.row == row);
        &self.hits[from..to]
    }

    /// Rows (`j0 + n`, `k`) for `n` in `0..count` (as many as the grid
    /// has), each with its crossings, finding the first only by search.
    pub(crate) fn rows_along(
        &self,
        grid: &Grid,
        j0: usize,
        k: usize,
        count: usize,
        mut f: impl FnMut(usize, &[Hit]),
    ) {
        let count = count.min(grid.size[1].saturating_sub(j0));
        if count == 0 || k >= grid.size[2] {
            return;
        }
        let first = j0 as u64 + grid.size[1] as u64 * k as u64;
        let mut at = self.hits.partition_point(|h| h.row < first);
        for n in 0..count {
            let row = first + n as u64;
            while at < self.hits.len() && self.hits[at].row < row {
                at += 1;
            }
            let from = at;
            while at < self.hits.len() && self.hits[at].row == row {
                at += 1;
            }
            f(n, &self.hits[from..at]);
        }
    }

    /// Whether points `i0..i0 + out.len()` of a row with crossings `hits`
    /// are inside: the line goes into the surface (through a triangle
    /// facing back along X) more often than out of it before the point
    /// (the *winding number* isn't zero), so shapes that overlap count as
    /// one solid, their overlap inside too. A row that doesn't come out as
    /// often as it goes in (only by rounding, on a closed shape) would be
    /// inside out to the grid's edge: it is counted by crossings instead,
    /// its last one dropped if odd.
    pub(crate) fn inside(grid: &Grid, hits: &[Hit], i0: usize, out: &mut [bool]) {
        let balanced = hits.iter().map(|h| i32::from(h.way)).sum::<i32>() == 0;
        let count = if balanced {
            hits.len()
        } else {
            hits.len() - hits.len() % 2
        };
        let xs = &hits[..count];
        let first = grid.origin.x + i0 as f32 * grid.step;
        let mut crossed = xs.partition_point(|h| h.x < first);
        let mut winding: i32 = xs[..crossed].iter().map(|h| i32::from(h.way)).sum();
        for (n, slot) in out.iter_mut().enumerate() {
            let x = grid.origin.x + (i0 + n) as f32 * grid.step;
            while crossed < xs.len() && xs[crossed].x < x {
                winding += i32::from(xs[crossed].way);
                crossed += 1;
            }
            *slot = if balanced {
                winding != 0
            } else {
                crossed % 2 == 1
            };
        }
    }
}

/// Adds where the rows of `grid` cross triangle `[a, b, c]` to `hits`.
fn crossings(grid: &Grid, [a, b, c]: &[Vec3; 3], hits: &mut Vec<Hit>) {
    let [_, ny, nz] = grid.size;
    let nudge = Vec2::new(0.001_234_5, 0.000_765_4) * grid.step;
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
        return;
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
                hits.push(Hit {
                    row: j as u64 + ny as u64 * k as u64,
                    x,
                    way,
                });
            }
        }
    }
}

/// Which points of a whole `grid` are inside the shape `soup` (see
/// [`Rows::inside`]), in the grid's index order.
pub(crate) fn parity(grid: &Grid, soup: &Soup) -> Vec<bool> {
    let [nx, ny, nz] = grid.size;
    let rows = Rows::new(grid, soup);
    let mut inside = vec![false; grid.len()];
    for k in 0..nz {
        for j in 0..ny {
            let at = grid.index(0, j, k);
            Rows::inside(grid, rows.row(grid, j, k), 0, &mut inside[at..at + nx]);
        }
    }
    inside
}

/// The corner (bit 0 along X, bit 1 along Y, bit 2 along Z) of a grid
/// cell to fill in, if the cell has a diagonal-only touch.
pub(super) fn fill_for(c: &[bool; 8]) -> Option<usize> {
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
    use crate::bvh::closest_on_triangle;

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
