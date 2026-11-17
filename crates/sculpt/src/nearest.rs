//! Finding, for each point of a changed shape, the nearest point of the
//! shape a sculpt was made for: a grid of cells round the old points,
//! searched ring by ring outwards from each new point's cell.

use glam::{IVec3, Vec3};
use std::collections::HashMap;

/// About how many old points share a cell.
const PER_CELL: f32 = 4.0;

/// The old points in a grid.
pub(crate) struct Grid {
    cell: f32,
    min: Vec3,
    max: Vec3,
    cells: HashMap<IVec3, Vec<u32>>,
}

impl Grid {
    /// The grid round `points` (those that aren't real numbers are left
    /// out). `None` when none are.
    pub(crate) fn new(points: &[Vec3]) -> Option<Self> {
        let real: Vec<(u32, Vec3)> = points
            .iter()
            .enumerate()
            .filter(|(_, p)| p.is_finite())
            .map(|(i, p)| (i as u32, *p))
            .collect();
        let first = real.first()?.1;
        let (mut min, mut max) = (first, first);
        for (_, p) in &real {
            min = min.min(*p);
            max = max.max(*p);
        }
        let size = (max - min).max(Vec3::splat(1e-6));
        // A sculpt's points lie on a surface: cells sized so about
        // `PER_CELL` points share one, spread over about half the box's
        // outside (a ball's or a flat square's points both fit this).
        let area = size.x * size.y + size.y * size.z + size.z * size.x;
        let cell = (area * PER_CELL / real.len() as f32)
            .sqrt()
            .max(size.max_element() / 4096.0)
            .max(1e-6);
        let mut grid = Self {
            cell,
            min,
            max,
            cells: HashMap::new(),
        };
        for (i, p) in real {
            grid.cells.entry(grid.key(p)).or_default().push(i);
        }
        Some(grid)
    }

    fn key(&self, p: Vec3) -> IVec3 {
        ((p - self.min) / self.cell).floor().as_ivec3()
    }

    /// The old point nearest `p` (by its place in `points`, the list the
    /// grid was made from).
    pub(crate) fn nearest(&self, points: &[Vec3], p: Vec3) -> Option<u32> {
        if !p.is_finite() {
            return None;
        }
        // Searched from the nearest place in the grid's box: every point in
        // the box is at least as far from `p` as that place is, plus how
        // far it is from that place (the box is convex).
        let inside = p.clamp(self.min, self.max);
        let outside = p.distance_squared(inside);
        let at = self.key(inside);
        let last = self.key(self.max);
        let reach = (at.max(last - at)).max_element().max(0) + 1;
        let mut best: Option<(f32, u32)> = None;
        for ring in 0..=reach {
            // Every point in this ring or farther is at least this far.
            if let Some((d, _)) = best {
                let near = (ring - 1).max(0) as f32 * self.cell;
                if near * near + outside > d {
                    break;
                }
            }
            for key in shell(at, ring) {
                let Some(list) = self.cells.get(&key) else {
                    continue;
                };
                for &i in list {
                    let d = points[i as usize].distance_squared(p);
                    if best.is_none_or(|(b, j)| d < b || (d == b && i < j)) {
                        best = Some((d, i));
                    }
                }
            }
        }
        best.map(|(_, i)| i)
    }
}

/// The cells exactly `ring` steps from `at` (the surface of a cube).
fn shell(at: IVec3, ring: i32) -> impl Iterator<Item = IVec3> {
    let r = ring;
    (-r..=r).flat_map(move |x| {
        (-r..=r).flat_map(move |y| {
            let edge = x.abs() == r || y.abs() == r;
            let zs: Vec<i32> = if edge {
                (-r..=r).collect()
            } else if r == 0 {
                vec![0]
            } else {
                vec![-r, r]
            };
            zs.into_iter().map(move |z| at + IVec3::new(x, y, z))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nearest_is_found_as_by_looking_at_every_point() {
        let mut state = 7u64;
        let mut next = move || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((state >> 33) as f32 / (1u64 << 31) as f32) * 10.0 - 5.0
        };
        let points: Vec<Vec3> = (0..500).map(|_| Vec3::new(next(), next(), next())).collect();
        let grid = Grid::new(&points).unwrap();
        for _ in 0..200 {
            let q = Vec3::new(next(), next(), next()) * 1.5;
            let found = grid.nearest(&points, q).unwrap();
            let best = points
                .iter()
                .map(|p| p.distance_squared(q))
                .fold(f32::INFINITY, f32::min);
            assert!((points[found as usize].distance_squared(q) - best).abs() < 1e-4);
        }
    }

    #[test]
    fn points_that_arent_numbers_are_left_out() {
        assert!(Grid::new(&[Vec3::NAN]).is_none());
        let points = [Vec3::NAN, Vec3::ONE];
        let grid = Grid::new(&points).unwrap();
        assert_eq!(grid.nearest(&points, Vec3::ZERO), Some(1));
        assert_eq!(grid.nearest(&points, Vec3::NAN), None);
    }
}
