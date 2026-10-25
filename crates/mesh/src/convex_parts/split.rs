//! Cutting a shape's parts in two, and measuring them (see the parent
//! module's notes, steps 2 to 4).

use crate::hull::{convex_hull, convex_hull_limited};
use crate::remesh::{Grid, Soup};
use glam::Vec3;

/// Evenly spaced places tried along each axis when cutting a part.
const EVEN_TRIES: usize = 6;
/// Places where its slices change most, tried along each axis too.
const CHANGE_TRIES: usize = 3;

/// How much uneven halves count against a cut: the difference in their
/// volumes times this.
const BALANCE_WEIGHT: f64 = 0.1;
/// How much the size of a cut counts against it: its area times this
/// share of the part's longest side, as a volume.
const CUT_WEIGHT: f64 = 0.05;
/// The most corners of a hull worked out only to measure a part's waste:
/// enough to measure it closely, few enough to be quick on dense shapes.
pub(super) const MEASURE_CORNERS: usize = 256;

/// The `count` places between `slices` where they change most, the
/// biggest changes first (as places from the first slice: 1 is between
/// the first and second).
fn changes(slices: &[usize], count: usize) -> Vec<usize> {
    let mut steps: Vec<(usize, usize)> = slices
        .windows(2)
        .enumerate()
        .map(|(n, w)| (w[1].abs_diff(w[0]), n + 1))
        .filter(|(change, _)| *change > 0)
        .collect();
    steps.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    steps.into_iter().take(count).map(|(_, at)| at).collect()
}

/// A box of voxels: its first voxel along each axis, and one past its
/// last.
type VoxelBox = ([usize; 3], [usize; 3]);

/// Where a part's box ends on each axis, in the shape's space: its low and
/// high side, `None` where it goes on for ever (no cut there yet).
pub(super) type Sides = [[Option<f32>; 2]; 3];

/// One part of the shape: the voxels in a box of the grid, and the shape
/// inside its sides.
pub(super) struct Part {
    /// Its first voxel along each axis, and one past its last.
    pub(super) low: [usize; 3],
    pub(super) high: [usize; 3],
    pub(super) sides: Sides,
    /// The triangles that reach into it.
    pub(super) triangles: Vec<u32>,
    /// How many of its voxels are inside the shape.
    pub(super) voxels: usize,
    /// The points its hull is made from.
    pub(super) points: Vec<Vec3>,
    /// Its hull's volume less its own.
    pub(super) waste: f64,
    /// Whether it can't be split.
    pub(super) done: bool,
}

impl Part {
    /// Whether its voxel box and `other`'s touch or overlap.
    pub(super) fn touches(&self, other: &Part) -> bool {
        (0..3).all(|a| self.low[a] <= other.high[a] && other.low[a] <= self.high[a])
    }
}

/// Splits a shape into parts.
pub(super) struct Splitter<'a> {
    grid: &'a Grid,
    inside: &'a [bool],
    soup: &'a Soup,
    /// The shape's points' places along each axis, in order, each once.
    places: [Vec<f32>; 3],
}

impl<'a> Splitter<'a> {
    pub(super) fn new(grid: &'a Grid, inside: &'a [bool], soup: &'a Soup) -> Self {
        let places = [0, 1, 2].map(|axis| {
            let mut list: Vec<f32> = soup.triangles.iter().flatten().map(|p| p[axis]).collect();
            list.sort_by(f32::total_cmp);
            list.dedup();
            list
        });
        Self {
            grid,
            inside,
            soup,
            places,
        }
    }

    pub(super) fn voxel_volume(&self) -> f64 {
        f64::from(self.grid.step).powi(3)
    }

    /// Where the cut between voxels `index - 1` and `index` along `axis`
    /// is: at a point of the shape between their middles if there is one
    /// (the nearest to halfway), so a cut runs along a box's side rather
    /// than through it; otherwise halfway.
    fn cut_at(&self, axis: usize, index: usize) -> f32 {
        let middle = |i: usize| self.grid.origin[axis] + i as f32 * self.grid.step;
        let (low, high) = (middle(index.saturating_sub(1)), middle(index));
        let halfway = (low + high) * 0.5;
        let places = &self.places[axis];
        let from = places.partition_point(|x| *x <= low);
        let to = places.partition_point(|x| *x <= high);
        places[from..to]
            .iter()
            .copied()
            .min_by(|a, b| (a - halfway).abs().total_cmp(&(b - halfway).abs()))
            .unwrap_or(halfway)
    }

    /// The part of the voxels from `low` to `high` and the shape inside
    /// `sides` (its triangles among `from`, or all), its box shrunk to its
    /// inside voxels; `None` when it has none.
    pub(super) fn part(
        &self,
        low: [usize; 3],
        high: [usize; 3],
        sides: Sides,
        from: Option<&[u32]>,
    ) -> Option<Part> {
        let (voxels, tight) = self.count(low, high);
        let (low, high) = tight?;
        let reaches = |t: &u32| {
            let corners = &self.soup.triangles[*t as usize];
            (0..3).all(|axis| {
                let lowest = corners.iter().map(|p| p[axis]).fold(f32::MAX, f32::min);
                let highest = corners.iter().map(|p| p[axis]).fold(f32::MIN, f32::max);
                sides[axis][0].is_none_or(|s| highest >= s)
                    && sides[axis][1].is_none_or(|s| lowest <= s)
            })
        };
        let triangles: Vec<u32> = match from {
            Some(list) => list.iter().copied().filter(reaches).collect(),
            None => (0..self.soup.triangles.len() as u32).collect(),
        };
        let points = self.exact_points(&triangles, &sides);
        let waste = convex_hull_limited(&points, MEASURE_CORNERS).map_or(0.0, |hull| {
            (hull.volume() - voxels as f64 * self.voxel_volume()).max(0.0)
        });
        Some(Part {
            low,
            high,
            sides,
            triangles,
            voxels,
            points,
            waste,
            done: false,
        })
    }

    /// How many voxels from `low` to `high` are inside, and the box round
    /// them.
    fn count(&self, low: [usize; 3], high: [usize; 3]) -> (usize, Option<VoxelBox>) {
        let mut count = 0;
        let mut tight: Option<VoxelBox> = None;
        for k in low[2]..high[2] {
            for j in low[1]..high[1] {
                for i in low[0]..high[0] {
                    if self.inside[self.grid.index(i, j, k)] {
                        count += 1;
                        let at = [i, j, k];
                        let (lo, hi) = tight.get_or_insert((at, [i + 1, j + 1, k + 1]));
                        for a in 0..3 {
                            lo[a] = lo[a].min(at[a]);
                            hi[a] = hi[a].max(at[a] + 1);
                        }
                    }
                }
            }
        }
        (count, tight)
    }

    /// The points the hull of the shape inside `sides` is made from: its
    /// `triangles` cut to the sides, and the box's corners inside the
    /// shape.
    fn exact_points(&self, triangles: &[u32], sides: &Sides) -> Vec<Vec3> {
        let mut points = Vec::new();
        for t in triangles {
            let [a, b, c] = self.soup.triangles[*t as usize];
            let facing = (b - a).cross(c - a);
            let mut polygon = vec![a, b, c];
            for (axis, ends) in sides.iter().enumerate() {
                if let Some(s) = ends[0] {
                    polygon = clip(&polygon, facing, axis, s, true);
                }
                if let Some(s) = ends[1] {
                    polygon = clip(&polygon, facing, axis, s, false);
                }
            }
            points.extend(polygon);
        }
        // The box's corners: one side's place on each axis.
        let choices: Vec<Vec<f32>> = sides
            .iter()
            .map(|ends| ends.iter().flatten().copied().collect())
            .collect();
        for x in &choices[0] {
            for y in &choices[1] {
                for z in &choices[2] {
                    let corner = Vec3::new(*x, *y, *z);
                    if self.is_inside(corner) {
                        points.push(corner);
                    }
                }
            }
        }
        points
    }

    /// Whether `p` is inside the shape, by the winding number along a
    /// line in X (nudged as Remesh's is).
    fn is_inside(&self, p: Vec3) -> bool {
        let nudge = glam::Vec2::new(0.001_234_5, 0.000_765_4) * self.grid.step;
        let q = glam::Vec2::new(p.y, p.z) + nudge;
        let cross = |a: glam::Vec2, b: glam::Vec2, c: glam::Vec2| (b - a).perp_dot(c - a);
        let mut winding = 0;
        for [a, b, c] in &self.soup.triangles {
            let (pa, pb, pc) = (
                glam::Vec2::new(a.y, a.z),
                glam::Vec2::new(b.y, b.z),
                glam::Vec2::new(c.y, c.z),
            );
            let area = cross(pa, pb, pc);
            if area.abs() <= f32::MIN_POSITIVE {
                continue;
            }
            let (wa, wb, wc) = (cross(pb, pc, q), cross(pc, pa, q), cross(pa, pb, q));
            let all_in = if area > 0.0 {
                wa > 0.0 && wb > 0.0 && wc > 0.0
            } else {
                wa < 0.0 && wb < 0.0 && wc < 0.0
            };
            if all_in && (wa * a.x + wb * b.x + wc * c.x) / area > p.x {
                winding += if area < 0.0 { 1 } else { -1 };
            }
        }
        winding != 0
    }

    /// The best cut of `part` in two, or `None` if it can't be cut.
    ///
    /// Tried along each axis: evenly spaced places, and the places where
    /// the part's slices change most (a box standing on another changes
    /// there). Then places near the best are tried, closer and closer.
    /// Each is scored by the two halves' waste, plus a little for how
    /// uneven the halves are and for the size of the cut, as V-HACD's
    /// balance term does: of cuts that save about as much, one near the
    /// middle, through a thin place, goes first (a ring is cut across its
    /// band, not along it, and not by slicing a sliver off its edge).
    pub(super) fn split(&self, part: &Part) -> Option<(Part, Part)> {
        let slices: Vec<Vec<usize>> = (0..3).map(|axis| self.slices(part, axis)).collect();
        let longest = (0..3)
            .map(|a| part.high[a] - part.low[a])
            .max()
            .unwrap_or(0);
        let per_area = CUT_WEIGHT * longest as f64 * self.voxel_volume();
        let per_voxel = BALANCE_WEIGHT * self.voxel_volume();
        let mut best: Option<(f64, usize, usize)> = None;
        let mut tried = std::collections::HashSet::new();
        let mut try_at = |axis: usize, at: usize, best: &mut Option<(f64, usize, usize)>| {
            if at <= part.low[axis] || at >= part.high[axis] || !tried.insert((axis, at)) {
                return;
            }
            let (mut low_high, mut high_low) = (part.high, part.low);
            low_high[axis] = at;
            high_low[axis] = at;
            let (Some(a), Some(b)) = (
                self.voxel_waste(part.low, low_high),
                self.voxel_waste(high_low, part.high),
            ) else {
                return;
            };
            let n = at - part.low[axis];
            let area = (slices[axis][n - 1] + slices[axis][n]) as f64 * 0.5;
            let below: usize = slices[axis][..n].iter().sum();
            let balance = below.abs_diff(part.voxels - below.min(part.voxels)) as f64;
            let score = a + b + area * per_area + balance * per_voxel;
            if best.is_none_or(|(s, _, _)| score < s) {
                *best = Some((score, axis, at));
            }
        };
        for (axis, slices) in slices.iter().enumerate() {
            let length = part.high[axis] - part.low[axis];
            if length < 2 {
                continue;
            }
            for t in 1..=EVEN_TRIES {
                try_at(
                    axis,
                    part.low[axis] + (length * t).div_ceil(EVEN_TRIES + 1),
                    &mut best,
                );
            }
            for at in changes(slices, CHANGE_TRIES) {
                try_at(axis, part.low[axis] + at, &mut best);
            }
        }
        let (_, axis, _) = best?;
        let mut reach = (part.high[axis] - part.low[axis]).div_ceil(EVEN_TRIES + 1);
        while reach > 1 {
            reach = reach.div_ceil(2);
            let (_, _, at) = best?;
            try_at(axis, at.saturating_sub(reach), &mut best);
            try_at(axis, at + reach, &mut best);
        }
        let (_, axis, at) = best?;
        let cut = self.cut_at(axis, at);
        let (mut low_high, mut high_low) = (part.high, part.low);
        low_high[axis] = at;
        high_low[axis] = at;
        let (mut low_sides, mut high_sides) = (part.sides, part.sides);
        low_sides[axis][1] = Some(cut);
        high_sides[axis][0] = Some(cut);
        Some((
            self.part(part.low, low_high, low_sides, Some(&part.triangles))?,
            self.part(high_low, part.high, high_sides, Some(&part.triangles))?,
        ))
    }

    /// How many of `part`'s voxels are inside, slice by slice along
    /// `axis`.
    fn slices(&self, part: &Part, axis: usize) -> Vec<usize> {
        let mut slices = vec![0; part.high[axis] - part.low[axis]];
        for k in part.low[2]..part.high[2] {
            for j in part.low[1]..part.high[1] {
                for i in part.low[0]..part.high[0] {
                    if self.inside[self.grid.index(i, j, k)] {
                        slices[[i, j, k][axis] - part.low[axis]] += 1;
                    }
                }
            }
        }
        slices
    }

    /// The waste of the voxels from `low` to `high` as they show it (their
    /// hull's volume less theirs, both of whole voxels), or `None` with no
    /// voxels inside. The hull is made from the voxels at the ends of each
    /// row along X, which are all it needs.
    fn voxel_waste(&self, low: [usize; 3], high: [usize; 3]) -> Option<f64> {
        let g = self.grid;
        let mut points = Vec::new();
        let mut count = 0;
        let corner = |i: f32, j: f32, k: f32| g.origin + Vec3::new(i, j, k) * g.step;
        for k in low[2]..high[2] {
            for j in low[1]..high[1] {
                let mut ends: Option<(usize, usize)> = None;
                for i in low[0]..high[0] {
                    if self.inside[g.index(i, j, k)] {
                        count += 1;
                        ends = Some(ends.map_or((i, i), |(first, _)| (first, i)));
                    }
                }
                let Some((first, last)) = ends else {
                    continue;
                };
                let (j, k) = (j as f32, k as f32);
                for (dj, dk) in [(-0.5, -0.5), (0.5, -0.5), (-0.5, 0.5), (0.5, 0.5)] {
                    points.push(corner(first as f32 - 0.5, j + dj, k + dk));
                    points.push(corner(last as f32 + 0.5, j + dj, k + dk));
                }
            }
        }
        if count == 0 {
            return None;
        }
        let volume = convex_hull(&points).map_or(0.0, |h| h.volume());
        Some((volume - count as f64 * self.voxel_volume()).max(0.0))
    }
}

/// `polygon` (part of a triangle facing `facing`) cut by the plane where
/// `axis` is `at`, keeping the side above it (`keep_above`) or below. New
/// corners lie exactly on the plane, so parts either side of it meet with
/// no gap. What only touches the plane (an edge or a corner on it) is
/// dropped: it has no area on the kept side. A polygon lying in the plane
/// is kept only by the side the shape is on (it faces away from it).
fn clip(polygon: &[Vec3], facing: Vec3, axis: usize, at: f32, keep_above: bool) -> Vec<Vec3> {
    let side = |p: Vec3| {
        if keep_above {
            p[axis] - at
        } else {
            at - p[axis]
        }
    };
    let farthest = polygon.iter().map(|p| side(*p)).fold(f32::MIN, f32::max);
    if farthest <= 0.0 {
        let lies_in = polygon.iter().all(|p| side(*p) == 0.0);
        let shape_this_side = if keep_above {
            facing[axis] < 0.0
        } else {
            facing[axis] > 0.0
        };
        return if lies_in && shape_this_side {
            polygon.to_vec()
        } else {
            Vec::new()
        };
    }
    let mut out = Vec::with_capacity(polygon.len() + 2);
    for (i, p) in polygon.iter().enumerate() {
        let q = polygon[(i + 1) % polygon.len()];
        let (dp, dq) = (side(*p), side(q));
        if dp >= 0.0 {
            out.push(*p);
        }
        if (dp >= 0.0) != (dq >= 0.0) {
            let mut x = *p + (q - *p) * (dp / (dp - dq));
            x[axis] = at;
            out.push(x);
        }
    }
    out
}
