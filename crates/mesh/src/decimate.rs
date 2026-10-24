//! Decimation ([decision 0145](../../../docs/decisions/0145-decimate.md)):
//! fewer polygons with the shape kept, for game-ready meshes and levels of
//! detail.
//!
//! **The method** is quadric error collapse (Garland and Heckbert): the
//! shape is cut into triangles, and the cheapest edge is collapsed again
//! and again, one end point moving onto the other, until the triangles
//! left reach the target. An edge's cost is how far the moved point
//! strays from the planes of every triangle it once touched (a *quadric*,
//! summed as points merge), so flat areas go first and curves, corners
//! and the outline stay. Collapses are *half-edge* ones: the points left
//! are some of the shape's own points, never moved, which keeps the
//! result true to the shape, and levels of detail made one after another
//! nested (see [`Decimator`]).
//!
//! **Attribute-aware.** A UV coordinate moving with a collapse is
//! measured too, against the slope the texture had across the original
//! triangles, so collapses that would smear the texture cost more.
//!
//! **What is kept** ([`Keep`], each on as standard): UV seams (marked
//! seams, and edges where the UVs split), hard edges (those the object's
//! shading shows: marked, or sharper than its angle), material borders and
//! open borders stay as lines. A point on such a line only slides along
//! it (onto its neighbour on the line), weighed against planes standing on
//! the line so it stays straight where it was; a point where lines meet or
//! end never moves. Edges of three or more polygons are always kept.
//! No collapse turns a triangle over, makes a triangle twice, or joins a
//! surface to itself through a point.
//!
//! **Triangles out**, unless [`DecimateOptions::quads_where_flat`]: then
//! pairs of triangles lying flat together (within a degree and a half)
//! join into four-sided polygons again. Polygon values (materials) come
//! from the polygon each triangle was cut from; hard edges, seams and
//! creases stay on the edges that are left (carried along a kept line as
//! it shortens); UVs stay on their corners. Point values and other corner
//! values aren't carried. Loose points are left out.
//!
//! **Deterministic:** the same shape and settings give the same mesh,
//! point for point.

mod collapse;
mod output;
mod quadric;
mod setup;
#[cfg(test)]
mod tests;

use crate::{Hardness, Mesh};
use glam::{DVec3, Vec2, Vec3};
use quadric::{Quadric, UvQuadric};
use std::collections::{BinaryHeap, HashMap};
use words::tr;

/// How the target is given.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReduceBy {
    /// A share of the triangles the shape has.
    #[default]
    Ratio,
    /// A number of triangles.
    Triangles,
}

/// The features kept as lines while decimating (all on as standard).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Keep {
    /// UV seams: marked seams and edges where the UVs split.
    pub seams: bool,
    /// Hard edges: those the object's shading shows.
    pub hard_edges: bool,
    /// Borders between polygons of different materials.
    pub materials: bool,
    /// Open borders: edges with a polygon on one side only.
    pub borders: bool,
}

impl Default for Keep {
    fn default() -> Self {
        Self {
            seams: true,
            hard_edges: true,
            materials: true,
            borders: true,
        }
    }
}

/// The Decimate modifier's settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecimateOptions {
    /// Whether [`Self::ratio`] or [`Self::triangles`] sets the target.
    pub reduce_by: ReduceBy,
    /// The share of triangles kept (1 is all of them).
    pub ratio: f32,
    /// The triangles to end with.
    pub triangles: u32,
    /// What stays as lines.
    pub keep: Keep,
    /// Join flat pairs of triangles into four-sided polygons.
    pub quads_where_flat: bool,
}

impl DecimateOptions {
    /// The standard ratio: half the triangles.
    pub const STANDARD_RATIO: f32 = 0.5;
    /// The smallest ratio: a thousandth.
    pub const MIN_RATIO: f32 = 0.001;
    /// The standard target, in triangles.
    pub const STANDARD_TRIANGLES: u32 = 1_000;
    /// The smallest target.
    pub const MIN_TRIANGLES: u32 = 4;
    /// The largest target.
    pub const MAX_TRIANGLES: u32 = 10_000_000;

    /// The settings brought within limits (a ratio that isn't a number
    /// is the standard one).
    pub fn clamped(self) -> Self {
        let ratio = if self.ratio.is_finite() {
            self.ratio.clamp(Self::MIN_RATIO, 1.0)
        } else {
            Self::STANDARD_RATIO
        };
        Self {
            ratio,
            triangles: self
                .triangles
                .clamp(Self::MIN_TRIANGLES, Self::MAX_TRIANGLES),
            ..self
        }
    }

    /// How many triangles to end with, from `triangles` (at least one,
    /// when there are any).
    pub fn target(&self, triangles: usize) -> usize {
        let o = self.clamped();
        let target = match o.reduce_by {
            ReduceBy::Ratio => (triangles as f64 * f64::from(o.ratio)).round() as usize,
            ReduceBy::Triangles => o.triangles as usize,
        };
        target.clamp(triangles.min(1), triangles.max(1))
    }
}

impl Default for DecimateOptions {
    fn default() -> Self {
        Self {
            reduce_by: ReduceBy::Ratio,
            ratio: Self::STANDARD_RATIO,
            triangles: Self::STANDARD_TRIANGLES,
            keep: Keep::default(),
            quads_where_flat: false,
        }
    }
}

/// Why a shape can't be decimated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecimateProblem {
    /// A point of a polygon isn't at a real position.
    NotANumber,
}

impl std::fmt::Display for DecimateProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotANumber => f.write_str(tr("a point of the shape isn't at a real position")),
        }
    }
}

impl std::error::Error for DecimateProblem {}

/// How many triangles `mesh` has once cut into them (a polygon of `n`
/// corners is `n − 2`): what a game engine counts.
pub fn triangle_count(mesh: &Mesh) -> usize {
    mesh.faces().map(|(_, c)| c.len().saturating_sub(2)).sum()
}

/// `mesh` with fewer polygons, as `options` say (see the module notes).
/// `hardness` is how the object is shaded: the hard edges kept are the
/// ones it shows. A target at or above the triangles the shape has gives
/// the shape unchanged.
pub fn decimate(
    mesh: &Mesh,
    options: &DecimateOptions,
    hardness: Hardness,
) -> Result<Mesh, DecimateProblem> {
    let options = options.clamped();
    let triangles = triangle_count(mesh);
    let target = options.target(triangles);
    if target >= triangles {
        return Ok(mesh.clone());
    }
    let mut decimator = Decimator::new(mesh, options.keep, hardness)?;
    decimator.reduce_to(target);
    Ok(decimator.mesh(options.quads_where_flat))
}

/// Several levels of detail of `mesh` at once, one per target (in
/// triangles, largest first for nested levels), each continuing from the
/// one before: as quick as the smallest alone, and each level's points are
/// some of the level before's. A target above the triangles left gives the
/// level before again.
pub fn decimate_levels(
    mesh: &Mesh,
    keep: Keep,
    hardness: Hardness,
    targets: &[usize],
    quads_where_flat: bool,
) -> Result<Vec<Mesh>, DecimateProblem> {
    let mut decimator = Decimator::new(mesh, keep, hardness)?;
    Ok(targets
        .iter()
        .map(|target| {
            decimator.reduce_to(*target);
            decimator.mesh(quads_where_flat)
        })
        .collect())
}

/// How a point may move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    /// Onto any neighbour.
    Free,
    /// On a kept line: only along it.
    Line,
    /// Where lines meet or end, or where the surface is tangled: never.
    Locked,
}

/// The cheapest collapse found for a point.
#[derive(Debug, Clone, Copy)]
struct Best {
    target: u32,
    cost: f64,
}

/// A heap entry: a point's cheapest collapse, cheapest first, then by
/// point (so equal costs always go in the same order).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Entry {
    cost: f64,
    point: u32,
    version: u32,
}

impl Eq for Entry {}

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reversed: the heap gives the largest, and the cheapest is wanted.
        other
            .cost
            .total_cmp(&self.cost)
            .then(other.point.cmp(&self.point))
            .then(other.version.cmp(&self.version))
    }
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Marks an edge carries to the result.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Marks {
    hard: bool,
    seam: bool,
    /// Its crease, 0 for none.
    crease: f32,
}

impl Marks {
    fn is_empty(&self) -> bool {
        !self.hard && !self.seam && self.crease <= 0.0
    }

    /// What of `self` goes on along a collapse of an edge marked `along`.
    fn carried_by(self, along: Self) -> Self {
        Self {
            hard: self.hard && along.hard,
            seam: self.seam && along.seam,
            crease: if along.crease > 0.0 { self.crease } else { 0.0 },
        }
    }

    fn merge(&mut self, other: Self) {
        self.hard |= other.hard;
        self.seam |= other.seam;
        self.crease = self.crease.max(other.crease);
    }
}

/// A shape being decimated, step by step: the way to make several levels
/// of detail from one shape (the LOD generator's use, decision 0145).
///
/// [`Decimator::new`] does the setting up once (triangles, kept lines,
/// quadrics); [`Decimator::reduce_to`] collapses edges until no more than
/// a number of triangles is left, and can be called again with a smaller
/// number to go on from there; [`Decimator::mesh`] gives the shape as it
/// is now. So levels at 50%, 25% and 12% cost one decimation to 12%, and
/// each level's points are some of the one before's.
pub struct Decimator {
    /// The shape it started from (for polygon values).
    original: Mesh,
    /// Each point's place, as the shape has it.
    places: Vec<Vec3>,
    /// Each point's place moved and scaled into a box of size 1 (so costs
    /// don't depend on the shape's size).
    pos: Vec<DVec3>,
    /// Each UV corner group's (wedge's) point and UV.
    wedge_point: Vec<u32>,
    wedge_uv: Vec<Vec2>,
    /// Whether the shape has UVs.
    has_uvs: bool,
    /// Each triangle's corners (wedges) and the polygon slot it came from.
    tris: Vec<[u32; 3]>,
    tri_face: Vec<u32>,
    tri_alive: Vec<bool>,
    /// Triangles left.
    live: usize,
    /// Each point's triangles (some may be gone).
    tris_of: Vec<Vec<u32>>,
    point_alive: Vec<bool>,
    class: Vec<Class>,
    /// Kept lines: the edge's kinds (`setup::BORDER` and the rest).
    features: HashMap<u64, u8>,
    /// Marks carried to the result.
    marks: HashMap<u64, Marks>,
    quadrics: Vec<Quadric>,
    uv_quadrics: Vec<UvQuadric>,
    best: Vec<Option<Best>>,
    version: Vec<u32>,
    heap: BinaryHeap<Entry>,
    /// How much a collapse's length counts, to break ties between costs
    /// of nothing (flat areas) in favour of short edges: even triangles.
    tie: f64,
}

/// An edge's key: its two points, the smaller first.
fn key(a: u32, b: u32) -> u64 {
    let (lo, hi) = if a < b { (a, b) } else { (b, a) };
    (u64::from(lo) << 32) | u64::from(hi)
}

impl Decimator {
    /// How many triangles are left.
    pub fn triangle_count(&self) -> usize {
        self.live
    }
}
