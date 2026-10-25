//! Convex parts ([decision 0151](../../../docs/decisions/0151-collision-hulls.md)):
//! a closed shape split into a few convex hulls that together follow its
//! dents, for a game engine's collision.
//!
//! One hull round a chair fills in the space under its seat; a few hulls,
//! one per leg and one for the seat and back, let things go under it while
//! staying cheap to collide with. This is an *approximate convex
//! decomposition*, in the manner of V-HACD, kept simple and the same every
//! time:
//!
//! 1. **Inside and outside.** The shape is sampled on a grid of little
//!    cubes (voxels), about 32 to 96 across its longest side as **Detail**
//!    goes from 0 to 1, each inside or outside by the winding number
//!    (Remesh's test, so overlapping pieces are one solid).
//! 2. **Waste.** A part is the shape inside a box. Its hull is worked out
//!    exactly, from the shape's triangles cut to the box and the box's
//!    corners that are inside the shape; its *waste* is the hull's volume
//!    less the part's own (its voxels'): the empty space the hull fills
//!    in.
//! 3. **Splitting.** While there are fewer parts than **Max parts**, the
//!    part with the most waste is cut in two by a plane square to an axis,
//!    if its waste is more than a share of the whole volume (smaller as
//!    Detail grows). Up to eight places along each axis are tried, each
//!    scored by the two halves' waste as their voxels show it (quick), and
//!    the best is taken.
//! 4. **Hulls.** Each part's hull, with at most [`MAX_HULL_CORNERS`]
//!    corners (as physics engines prefer, and V-HACD's standard). A hull
//!    under the limit is exact, and parts next to each other meet on the
//!    plane between them with no gap. A hull that needs more corners keeps,
//!    on each side where it was cut, the same few corners of the section
//!    as its neighbour does (up to 12, spread evenly round it), so the two
//!    still meet on the plane with no gap, a little inside the shape's own
//!    section there; its other corners are the farthest of its other
//!    points, so it sits a little inside the shape elsewhere too. (A part
//!    cut on more than four sides is limited as a whole, and may leave a
//!    small gap.)
//!
//! The shape must be closed: an open one has no inside to split.

use crate::hull::{Hull, HullProblem, convex_hull, convex_hull_limited};
use crate::remesh::{Grid, RemeshProblem, Soup, parity};
use crate::{Bounds, Mesh};
use glam::Vec3;
use split::{MEASURE_CORNERS, Part, Splitter};
use words::tr;

mod split;

/// The most corners each part's hull keeps.
pub const MAX_HULL_CORNERS: usize = 64;

/// How a shape is split into convex parts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConvexPartsOptions {
    /// The most parts (1 to [`Self::MOST_PARTS`]).
    pub max_parts: u32,
    /// How closely the parts follow the shape's dents, 0 to 1: higher
    /// finds smaller dents, takes longer and may use more parts.
    pub detail: f32,
}

impl ConvexPartsOptions {
    /// The standard Max parts.
    pub const STANDARD_MAX_PARTS: u32 = 8;
    /// The largest Max parts.
    pub const MOST_PARTS: u32 = 32;
    /// The standard Detail.
    pub const STANDARD_DETAIL: f32 = 0.5;

    /// The settings within their limits; a Detail that isn't a number
    /// becomes the standard one.
    pub fn clamped(self) -> Self {
        Self {
            max_parts: self.max_parts.clamp(1, Self::MOST_PARTS),
            detail: if self.detail.is_nan() {
                Self::STANDARD_DETAIL
            } else {
                self.detail.clamp(0.0, 1.0)
            },
        }
    }

    /// The voxels across the shape's longest side.
    fn resolution(self) -> usize {
        32 + (64.0 * self.detail).round() as usize
    }

    /// The share of the whole volume a part may waste and not be split.
    fn tolerance(self) -> f64 {
        let loose = f64::from(1.0 - self.detail);
        0.2 * loose * loose + 0.005
    }
}

impl Default for ConvexPartsOptions {
    fn default() -> Self {
        Self {
            max_parts: Self::STANDARD_MAX_PARTS,
            detail: Self::STANDARD_DETAIL,
        }
    }
}

/// Why a shape can't be split into convex parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartsProblem {
    /// It has no polygons.
    Empty,
    /// A point isn't at a real place.
    NotANumber,
    /// It isn't closed, so it has no inside.
    NotClosed,
    /// It is flat.
    Flat,
}

impl std::fmt::Display for PartsProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => f.write_str(tr("the shape has no polygons")),
            Self::NotANumber => f.write_str(tr("a point of the shape isn't at a real position")),
            Self::NotClosed => f.write_str(tr(
                "the shape isn't closed: it has holes in its surface, or edges shared by more \
                 than two polygons, so it has no inside to split",
            )),
            Self::Flat => HullProblem::Flat.fmt(f),
        }
    }
}

impl std::error::Error for PartsProblem {}

impl From<HullProblem> for PartsProblem {
    fn from(problem: HullProblem) -> Self {
        match problem {
            HullProblem::Empty => Self::Empty,
            HullProblem::Flat => Self::Flat,
        }
    }
}

/// The hull round every point of `mesh`'s polygons, with at most
/// `max_corners` corners (`usize::MAX` for all).
pub fn wrapped(mesh: &Mesh, max_corners: usize) -> Result<Hull, HullProblem> {
    let points: Vec<Vec3> = mesh
        .faces()
        .flat_map(|(_, corners)| corners.iter().filter_map(|v| mesh.position(*v)))
        .collect();
    convex_hull_limited(&points, max_corners)
}

/// `mesh` split into convex hulls (see the module notes): at most
/// `options.max_parts`, each with at most [`MAX_HULL_CORNERS`] corners.
/// A shape too thin for the grid to see inside is one hull.
pub fn convex_parts(mesh: &Mesh, options: &ConvexPartsOptions) -> Result<Vec<Hull>, PartsProblem> {
    let options = options.clamped();
    let soup = Soup::of(mesh).map_err(|p| match p {
        RemeshProblem::NotANumber => PartsProblem::NotANumber,
        _ => PartsProblem::Empty,
    })?;
    if !crate::remesh::is_closed(mesh) {
        return Err(PartsProblem::NotClosed);
    }
    let whole = || -> Result<Vec<Hull>, PartsProblem> {
        let points: Vec<Vec3> = soup.triangles.iter().flatten().copied().collect();
        Ok(vec![convex_hull_limited(&points, MAX_HULL_CORNERS)?])
    };
    let Some(bounds) = soup.bounds() else {
        return Err(PartsProblem::Empty);
    };
    if options.max_parts == 1 {
        return whole();
    }
    let Some(grid) = grid_round(bounds, options.resolution()) else {
        return whole();
    };
    let inside = parity(&grid, &soup);
    let splitter = Splitter::new(&grid, &inside, &soup);
    let Some(first) = splitter.part([0; 3], grid.size, [[None; 2]; 3], None) else {
        // Thinner than the grid sees: one hull.
        return whole();
    };
    if first.voxels < 8 {
        return whole();
    }
    let tolerance = first.voxels as f64 * splitter.voxel_volume() * options.tolerance();
    let mut parts = vec![first];
    while parts.len() < options.max_parts as usize {
        let Some(worst) = (0..parts.len())
            .filter(|i| !parts[*i].done)
            .max_by(|a, b| parts[*a].waste.total_cmp(&parts[*b].waste).then(b.cmp(a)))
        else {
            break;
        };
        if parts[worst].waste <= tolerance {
            break;
        }
        match splitter.split(&parts[worst]) {
            Some((low, high)) => {
                parts[worst] = low;
                parts.insert(worst + 1, high);
            }
            None => parts[worst].done = true,
        }
    }
    merge_parts(&mut parts, tolerance, splitter.voxel_volume());
    let hulls: Vec<Hull> = parts
        .iter()
        .filter_map(|p| part_hull(&p.points, &p.sides))
        .collect();
    if hulls.is_empty() {
        return whole();
    }
    Ok(hulls)
}

/// The most corners a part's hull keeps on each side where it was cut:
/// the same few on both sides of the cut, so neighbours meet there.
const CORNERS_PER_CUT: usize = 12;

/// The most corners a part's hull keeps on all its cuts together; with
/// more cuts than fit, its hull is limited as a whole instead.
const CORNERS_ON_CUTS: usize = 48;

/// The hull of a part made from `points` inside `sides`, with at most
/// [`MAX_HULL_CORNERS`] corners. A hull that fits under the limit is
/// exact. Otherwise, on each side where the part was cut it keeps the
/// same corners its neighbour keeps there (up to [`CORNERS_PER_CUT`] of
/// the section's outline, picked evenly round it from the outline alone),
/// so the two meet on the plane between them with no gap, and the rest of
/// its corners are the farthest of its other points.
fn part_hull(points: &[Vec3], sides: &split::Sides) -> Option<Hull> {
    let limited = convex_hull_limited(points, MAX_HULL_CORNERS).ok()?;
    let cuts: Vec<(usize, f32)> = sides
        .iter()
        .enumerate()
        .flat_map(|(axis, ends)| ends.iter().flatten().map(move |at| (axis, *at)))
        .collect();
    if cuts.is_empty() {
        return Some(limited);
    }
    // Holding every point, it is exact: the limit wasn't reached.
    let size =
        crate::Bounds::around(points.iter().copied()).map_or(0.0, |b| b.size().max_element());
    if points.iter().all(|p| limited.contains(*p, size * 1e-5)) {
        return Some(limited);
    }
    let on_a_cut = |p: &Vec3| cuts.iter().any(|(axis, at)| p[*axis] == *at);
    let mut kept: Vec<Vec3> = Vec::new();
    for (axis, at) in &cuts {
        let section: Vec<Vec3> = points.iter().filter(|p| p[*axis] == *at).copied().collect();
        kept.extend(evenly(&outline(&section, *axis), CORNERS_PER_CUT));
    }
    if kept.len() > CORNERS_ON_CUTS {
        return Some(limited);
    }
    let others: Vec<Vec3> = points.iter().filter(|p| !on_a_cut(p)).copied().collect();
    if let Ok(rest) = convex_hull_limited(&others, MAX_HULL_CORNERS - kept.len()) {
        kept.extend(rest.points);
    }
    convex_hull(&kept).ok().or(Some(limited))
}

/// The corners of `section`'s outline (points on a plane square to
/// `axis`), counter-clockwise from the lowest, each once; points on a
/// straight stretch of it are left out.
fn outline(section: &[Vec3], axis: usize) -> Vec<Vec3> {
    let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
    let mut points: Vec<Vec3> = section.to_vec();
    points.sort_by(|a, b| a[u].total_cmp(&b[u]).then(a[v].total_cmp(&b[v])));
    points.dedup();
    if points.len() < 3 {
        return points;
    }
    let turn = |o: Vec3, a: Vec3, b: Vec3| {
        f64::from(a[u] - o[u]) * f64::from(b[v] - o[v])
            - f64::from(a[v] - o[v]) * f64::from(b[u] - o[u])
    };
    // Andrew's monotone chain: the lower outline, then the upper.
    let mut hull: Vec<Vec3> = Vec::with_capacity(points.len() + 1);
    for pass in 0..2 {
        let start = hull.len();
        let order: Box<dyn Iterator<Item = &Vec3>> = if pass == 0 {
            Box::new(points.iter())
        } else {
            Box::new(points.iter().rev())
        };
        for p in order {
            while hull.len() >= start + 2
                && turn(hull[hull.len() - 2], hull[hull.len() - 1], *p) <= 0.0
            {
                hull.pop();
            }
            hull.push(*p);
        }
        hull.pop();
    }
    hull
}

/// At most `count` of `corners`, spread evenly round them from the first.
fn evenly(corners: &[Vec3], count: usize) -> Vec<Vec3> {
    if corners.len() <= count {
        return corners.to_vec();
    }
    (0..count)
        .map(|i| corners[i * corners.len() / count])
        .collect()
}

/// Joins parts whose joint hull wastes no more than `tolerance`, the
/// least wasteful pair first, until no pair can join: splitting one part
/// at a time can leave a part cut in two that is convex as a whole (the
/// base of a U, cut once beside each post).
fn merge_parts(parts: &mut Vec<Part>, tolerance: f64, voxel_volume: f64) {
    loop {
        let mut best: Option<(f64, usize, usize, Vec<Vec3>)> = None;
        for i in 0..parts.len() {
            for j in i + 1..parts.len() {
                if !parts[i].touches(&parts[j]) {
                    continue;
                }
                let points: Vec<Vec3> = parts[i]
                    .points
                    .iter()
                    .chain(&parts[j].points)
                    .copied()
                    .collect();
                let own = (parts[i].voxels + parts[j].voxels) as f64 * voxel_volume;
                let Ok(hull) = convex_hull_limited(&points, MEASURE_CORNERS) else {
                    continue;
                };
                let waste = (hull.volume() - own).max(0.0);
                if waste <= tolerance && best.as_ref().is_none_or(|b| waste < b.0) {
                    best = Some((waste, i, j, points));
                }
            }
        }
        let Some((waste, i, j, points)) = best else {
            return;
        };
        let gone = parts.remove(j);
        let kept = &mut parts[i];
        for axis in 0..3 {
            kept.low[axis] = kept.low[axis].min(gone.low[axis]);
            kept.high[axis] = kept.high[axis].max(gone.high[axis]);
        }
        kept.voxels += gone.voxels;
        kept.points = points;
        kept.waste = waste;
    }
}

/// The grid of voxel middles over `bounds`, `resolution` across its
/// longest side; `None` for a shape with no size.
fn grid_round(bounds: Bounds, resolution: usize) -> Option<Grid> {
    let size = bounds.size();
    let step = size.max_element() / resolution as f32;
    if !(step.is_finite() && step > 0.0) {
        return None;
    }
    let counts = [size.x, size.y, size.z].map(|s| ((s / step).ceil() as usize).max(1));
    let half = Vec3::new(
        (counts[0] - 1) as f32,
        (counts[1] - 1) as f32,
        (counts[2] - 1) as f32,
    ) * step
        * 0.5;
    Some(Grid {
        origin: bounds.centre() - half,
        step,
        size: counts,
    })
}

#[cfg(test)]
mod tests;
