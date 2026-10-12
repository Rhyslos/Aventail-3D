//! A bounding volume hierarchy: boxes inside boxes around a mesh's
//! triangles, points or edges, so finding what is under the mouse tests a
//! handful of them instead of all of them
//! ([decision 0035](../../../docs/decisions/0035-heavy-meshes.md)).
//!
//! The items are sorted along a space-filling curve (Morton order), so
//! items near each other in space sit near each other in the list; then
//! every [`LEAF`] items get a box, every [`BRANCH`] boxes a box around
//! them, and so on up to a few boxes at the top. Building costs one sort.
//! When the items only move a little (points dragged), the same order is
//! kept and only the boxes are worked out again ([`Bvh::refit`]), which
//! is much quicker than building afresh.

use crate::camera::Ray;
use glam::Vec3;
use mesh::parallel::in_parallel;
use std::sync::Arc;

/// Items per box at the bottom.
const LEAF: usize = 8;
/// Boxes per box above that.
const BRANCH: usize = 4;

/// A box with sides along the axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Aabb {
    pub(crate) min: Vec3,
    pub(crate) max: Vec3,
}

impl Aabb {
    /// A box around nothing: growing it by anything gives that thing.
    pub(crate) const EMPTY: Self = Self {
        min: Vec3::splat(f32::INFINITY),
        max: Vec3::splat(f32::NEG_INFINITY),
    };

    /// The box around these points.
    pub(crate) fn around(points: impl IntoIterator<Item = Vec3>) -> Self {
        let mut b = Self::EMPTY;
        for p in points {
            b.min = b.min.min(p);
            b.max = b.max.max(p);
        }
        b
    }

    fn merge(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    fn is_empty(&self) -> bool {
        self.min.x > self.max.x
    }

    fn centre(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Its eight corners.
    pub(crate) fn corners(&self) -> [Vec3; 8] {
        let (a, b) = (self.min, self.max);
        [
            Vec3::new(a.x, a.y, a.z),
            Vec3::new(b.x, a.y, a.z),
            Vec3::new(a.x, b.y, a.z),
            Vec3::new(b.x, b.y, a.z),
            Vec3::new(a.x, a.y, b.z),
            Vec3::new(b.x, a.y, b.z),
            Vec3::new(a.x, b.y, b.z),
            Vec3::new(b.x, b.y, b.z),
        ]
    }

    /// How far along `ray` it enters the box (0 if it starts inside), or
    /// `None` if it misses. The ray's direction needn't be unit length:
    /// distances are in units of it.
    pub(crate) fn entry(&self, ray: &Ray) -> Option<f32> {
        if self.is_empty() {
            return None;
        }
        let mut near = 0.0f32;
        let mut far = f32::INFINITY;
        for axis in 0..3 {
            let (o, d) = (ray.origin[axis], ray.direction[axis]);
            let (lo, hi) = (self.min[axis], self.max[axis]);
            if d.abs() < 1e-20 {
                if o < lo || o > hi {
                    return None;
                }
                continue;
            }
            let (mut t0, mut t1) = ((lo - o) / d, (hi - o) / d);
            if t0 > t1 {
                std::mem::swap(&mut t0, &mut t1);
            }
            near = near.max(t0);
            far = far.min(t1);
            if near > far {
                return None;
            }
        }
        Some(near)
    }
}

/// Spreads the lowest 10 bits of `x` out to every third bit.
fn spread(x: u32) -> u32 {
    let mut x = x & 0x3ff;
    x = (x | (x << 16)) & 0x0300_00ff;
    x = (x | (x << 8)) & 0x0300_f00f;
    x = (x | (x << 4)) & 0x030c_30c3;
    (x | (x << 2)) & 0x0924_9249
}

/// Boxes inside boxes around a list of items (numbered from 0).
#[derive(Debug, Clone, Default)]
pub(crate) struct Bvh {
    /// The items, in the order the boxes cover them.
    order: Arc<Vec<u32>>,
    /// Where each item is in `order`.
    slot: Arc<Vec<u32>>,
    /// The boxes, bottom level first: level 0 has a box per [`LEAF`]
    /// items, each level above a box per [`BRANCH`] boxes below.
    levels: Vec<Vec<Aabb>>,
}

impl Bvh {
    /// Builds the boxes around `count` items, `bounds(i)` being item `i`'s
    /// box.
    pub(crate) fn build(count: usize, bounds: impl Fn(u32) -> Aabb + Sync) -> Self {
        let boxes: Vec<Aabb> = in_parallel(count, |range| {
            range.map(|i| bounds(i as u32)).collect::<Vec<_>>()
        })
        .into_iter()
        .flatten()
        .collect();
        let whole = boxes
            .iter()
            .filter(|b| !b.is_empty())
            .fold(Aabb::EMPTY, |a, b| a.merge(*b));
        let size = (whole.max - whole.min).max(Vec3::splat(1e-12));
        let mut keyed: Vec<u64> = boxes
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let code = if b.is_empty() {
                    u32::MAX
                } else {
                    let q = ((b.centre() - whole.min) / size * 1023.0)
                        .clamp(Vec3::ZERO, Vec3::splat(1023.0));
                    spread(q.x as u32) | (spread(q.y as u32) << 1) | (spread(q.z as u32) << 2)
                };
                (u64::from(code) << 32) | i as u64
            })
            .collect();
        keyed.sort_unstable();
        let order: Vec<u32> = keyed.into_iter().map(|k| k as u32).collect();
        let mut slot = vec![0u32; order.len()];
        for (at, item) in order.iter().enumerate() {
            slot[*item as usize] = at as u32;
        }
        let mut bvh = Self {
            order: Arc::new(order),
            slot: Arc::new(slot),
            levels: Vec::new(),
        };
        bvh.fit(|i| boxes[i as usize]);
        bvh
    }

    /// The same items in the same order, with their boxes worked out
    /// again: for when they have moved but not changed.
    pub(crate) fn refit(&self, bounds: impl Fn(u32) -> Aabb + Sync) -> Self {
        let mut bvh = Self {
            order: Arc::clone(&self.order),
            slot: Arc::clone(&self.slot),
            levels: Vec::new(),
        };
        bvh.fit(bounds);
        bvh
    }

    /// The same, when only `items` moved: only their boxes, and the boxes
    /// around those, are worked out again.
    pub(crate) fn refit_items(&self, items: &[u32], bounds: impl Fn(u32) -> Aabb) -> Self {
        let mut levels = self.levels.clone();
        let mut dirty: Vec<usize> = items
            .iter()
            .filter_map(|i| self.slot.get(*i as usize))
            .map(|at| *at as usize / LEAF)
            .collect();
        dirty.sort_unstable();
        dirty.dedup();
        if let Some(leaves) = levels.first_mut() {
            for leaf in &dirty {
                let end = ((leaf + 1) * LEAF).min(self.order.len());
                leaves[*leaf] = self.order[leaf * LEAF..end]
                    .iter()
                    .map(|i| bounds(*i))
                    .fold(Aabb::EMPTY, Aabb::merge);
            }
        }
        for level in 1..levels.len() {
            for node in dirty.iter_mut() {
                *node /= BRANCH;
            }
            dirty.dedup();
            let (below, above) = levels.split_at_mut(level);
            let below = &below[level - 1];
            for node in &dirty {
                let end = ((node + 1) * BRANCH).min(below.len());
                above[0][*node] = below[node * BRANCH..end]
                    .iter()
                    .copied()
                    .fold(Aabb::EMPTY, Aabb::merge);
            }
        }
        Self {
            order: Arc::clone(&self.order),
            slot: Arc::clone(&self.slot),
            levels,
        }
    }

    fn fit(&mut self, bounds: impl Fn(u32) -> Aabb + Sync) {
        let order = &self.order;
        let leaf_count = order.len().div_ceil(LEAF);
        let leaves: Vec<Aabb> = in_parallel(leaf_count, |range| {
            range
                .map(|leaf| {
                    let end = ((leaf + 1) * LEAF).min(order.len());
                    order[leaf * LEAF..end]
                        .iter()
                        .map(|i| bounds(*i))
                        .fold(Aabb::EMPTY, Aabb::merge)
                })
                .collect::<Vec<_>>()
        })
        .into_iter()
        .flatten()
        .collect();
        self.levels = vec![leaves];
        while self.levels.last().is_some_and(|l| l.len() > BRANCH) {
            let below = self.levels.last().map(Vec::as_slice).unwrap_or_default();
            let above = below
                .chunks(BRANCH)
                .map(|c| c.iter().copied().fold(Aabb::EMPTY, Aabb::merge))
                .collect();
            self.levels.push(above);
        }
    }

    /// How many items it covers.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.order.len()
    }

    /// Calls `item` for every item inside a box that `keep` accepts (and
    /// whose enclosing boxes it accepted too).
    pub(crate) fn visit(&self, mut keep: impl FnMut(&Aabb) -> bool, mut item: impl FnMut(u32)) {
        let Some(top) = self.levels.len().checked_sub(1) else {
            return;
        };
        let mut stack: Vec<(usize, usize)> = (0..self.levels[top].len())
            .rev()
            .map(|i| (top, i))
            .collect();
        while let Some((level, index)) = stack.pop() {
            if !keep(&self.levels[level][index]) {
                continue;
            }
            if level == 0 {
                let end = ((index + 1) * LEAF).min(self.order.len());
                for i in &self.order[index * LEAF..end] {
                    item(*i);
                }
            } else {
                let end = ((index + 1) * BRANCH).min(self.levels[level - 1].len());
                for child in (index * BRANCH..end).rev() {
                    stack.push((level - 1, child));
                }
            }
        }
    }

    /// The item `hit` says the ray reaches first (with how far along the
    /// ray), skipping boxes further away than the best so far.
    pub(crate) fn nearest(
        &self,
        ray: &Ray,
        mut hit: impl FnMut(u32) -> Option<f32>,
    ) -> Option<(f32, u32)> {
        let mut best: Option<(f32, u32)> = None;
        let top = self.levels.len().checked_sub(1)?;
        let mut stack: Vec<(usize, usize)> =
            (0..self.levels[top].len()).map(|i| (top, i)).collect();
        while let Some((level, index)) = stack.pop() {
            let Some(entry) = self.levels[level][index].entry(ray) else {
                continue;
            };
            if best.is_some_and(|(d, _)| entry > d) {
                continue;
            }
            if level == 0 {
                let end = ((index + 1) * LEAF).min(self.order.len());
                for i in &self.order[index * LEAF..end] {
                    if let Some(d) = hit(*i)
                        && best.is_none_or(|(b, _)| d < b)
                    {
                        best = Some((d, *i));
                    }
                }
            } else {
                let end = ((index + 1) * BRANCH).min(self.levels[level - 1].len());
                for child in index * BRANCH..end {
                    stack.push((level - 1, child));
                }
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn points(n: usize) -> Vec<Vec3> {
        (0..n)
            .map(|i| {
                let f = i as f32;
                Vec3::new((f * 1.7).sin() * 50.0, (f * 0.9).cos() * 50.0, f * 0.1)
            })
            .collect()
    }

    #[test]
    fn visiting_everything_finds_every_item_once() {
        let pts = points(1000);
        let bvh = Bvh::build(pts.len(), |i| Aabb::around([pts[i as usize]]));
        let mut seen = vec![0; pts.len()];
        bvh.visit(|_| true, |i| seen[i as usize] += 1);
        assert!(seen.iter().all(|n| *n == 1));
        assert_eq!(bvh.len(), 1000);
    }

    #[test]
    fn visiting_a_region_finds_exactly_what_a_full_scan_finds() {
        let pts = points(5000);
        let bvh = Bvh::build(pts.len(), |i| Aabb::around([pts[i as usize]]));
        let region = Aabb {
            min: Vec3::new(-10.0, -20.0, 0.0),
            max: Vec3::new(30.0, 25.0, 300.0),
        };
        let inside = |p: Vec3| p.cmpge(region.min).all() && p.cmple(region.max).all();
        let mut found = Vec::new();
        bvh.visit(
            |b| b.min.cmple(region.max).all() && b.max.cmpge(region.min).all(),
            |i| {
                if inside(pts[i as usize]) {
                    found.push(i);
                }
            },
        );
        found.sort_unstable();
        let expected: Vec<u32> = (0..pts.len() as u32)
            .filter(|i| inside(pts[*i as usize]))
            .collect();
        assert_eq!(found, expected);
        assert!(!found.is_empty());
    }

    #[test]
    fn the_nearest_hit_matches_a_full_scan_and_refitting_follows_moves() {
        let pts = points(3000);
        let ray = Ray {
            origin: Vec3::new(0.0, 0.0, -100.0),
            direction: Vec3::Z,
        };
        // Items are small boxes; a hit is where the ray passes through.
        let hit = |p: &[Vec3], i: u32| {
            let b = Aabb {
                min: p[i as usize] - Vec3::splat(5.0),
                max: p[i as usize] + Vec3::splat(5.0),
            };
            b.entry(&ray)
        };
        let bvh = Bvh::build(pts.len(), |i| Aabb {
            min: pts[i as usize] - Vec3::splat(5.0),
            max: pts[i as usize] + Vec3::splat(5.0),
        });
        let scan = (0..pts.len() as u32)
            .filter_map(|i| hit(&pts, i).map(|d| (d, i)))
            .min_by(|a, b| a.0.total_cmp(&b.0));
        assert_eq!(bvh.nearest(&ray, |i| hit(&pts, i)), scan);
        // Move everything; refit; the answer still matches a scan.
        let moved: Vec<Vec3> = pts.iter().map(|p| *p + Vec3::new(7.0, -3.0, 2.0)).collect();
        let refit = bvh.refit(|i| Aabb {
            min: moved[i as usize] - Vec3::splat(5.0),
            max: moved[i as usize] + Vec3::splat(5.0),
        });
        let scan = (0..moved.len() as u32)
            .filter_map(|i| hit(&moved, i).map(|d| (d, i)))
            .min_by(|a, b| a.0.total_cmp(&b.0));
        assert_eq!(refit.nearest(&ray, |i| hit(&moved, i)), scan);
    }

    #[test]
    fn refitting_a_few_items_matches_refitting_all() {
        let pts = points(4000);
        let boxed = |p: &[Vec3], i: u32| Aabb::around([p[i as usize]]);
        let bvh = Bvh::build(pts.len(), |i| boxed(&pts, i));
        let mut moved = pts.clone();
        let items = [3u32, 1500, 3999];
        for i in items {
            moved[i as usize] += Vec3::new(400.0, -300.0, 50.0);
        }
        let partial = bvh.refit_items(&items, |i| boxed(&moved, i));
        let full = bvh.refit(|i| boxed(&moved, i));
        assert_eq!(partial.levels, full.levels);
    }

    #[test]
    fn many_items_are_built_on_several_threads_the_same() {
        let pts = points(250_000);
        let bvh = Bvh::build(pts.len(), |i| Aabb::around([pts[i as usize]]));
        let mut seen = vec![false; pts.len()];
        bvh.visit(|_| true, |i| seen[i as usize] = true);
        assert!(seen.iter().all(|s| *s));
        let parts = in_parallel(250_000, |r| r.len());
        assert_eq!(parts.iter().sum::<usize>(), 250_000);
    }

    #[test]
    fn nothing_to_cover_finds_nothing() {
        let bvh = Bvh::build(0, |_| Aabb::EMPTY);
        let ray = Ray {
            origin: Vec3::ZERO,
            direction: Vec3::X,
        };
        assert_eq!(bvh.nearest(&ray, |_| Some(1.0)), None);
        let mut any = false;
        bvh.visit(|_| true, |_| any = true);
        assert!(!any);
        assert_eq!(Bvh::default().nearest(&ray, |_| Some(1.0)), None);
    }

    #[test]
    fn rays_enter_boxes_where_expected() {
        let b = Aabb {
            min: Vec3::splat(-1.0),
            max: Vec3::splat(1.0),
        };
        let ray = |o: Vec3, d: Vec3| Ray {
            origin: o,
            direction: d,
        };
        assert_eq!(b.entry(&ray(Vec3::new(-5.0, 0.0, 0.0), Vec3::X)), Some(4.0));
        assert_eq!(b.entry(&ray(Vec3::ZERO, Vec3::X)), Some(0.0), "inside");
        assert_eq!(b.entry(&ray(Vec3::new(-5.0, 3.0, 0.0), Vec3::X)), None);
        assert_eq!(
            b.entry(&ray(Vec3::new(5.0, 0.0, 0.0), Vec3::X)),
            None,
            "behind"
        );
        assert_eq!(Aabb::EMPTY.entry(&ray(Vec3::ZERO, Vec3::X)), None);
        assert_eq!(b.corners().len(), 8);
    }
}
