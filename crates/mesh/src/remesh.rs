//! Voxel remesh ([decision 0142](../../../docs/decisions/0142-modifier-stack-and-remesh.md)):
//! a closed shape rebuilt as an even surface of quads, whatever its
//! polygons were before.
//!
//! The shape is sampled on a grid of points [`RemeshOptions::voxel_size`]
//! apart: each point is inside or outside the shape (found by counting
//! how often a line along X goes into and out of the surface, so shapes
//! that overlap count as one solid), and knows how far it is from the
//! surface near it. A new surface is then made between the
//! inside and outside points with *surface nets*: one point in each grid
//! cell the surface passes through (the middle of where it crosses the
//! cell's edges), and one quad across each grid edge it crosses. Before
//! that, places where the inside and outside points touch only at a
//! corner are filled in, so the new surface is always a clean closed
//! solid (every edge shared by exactly two quads). Optional relax passes
//! ([`RemeshOptions::smooth`]) even it out without shrinking it.
//!
//! The new surface keeps each polygon's material from the nearest
//! original polygon. Its UVs can't follow from the old ones, so it gets a
//! simple box projection (each polygon seen from the side it faces most),
//! with seams where the projection changes side.
//!
//! Only closed shapes can be remeshed: a shape with holes in its surface
//! has no inside. Points at the same place count as one, so separate
//! pieces that meet still close up.

mod field;
mod nets;
#[cfg(test)]
mod tests;

use crate::{Bounds, Mesh};
pub(crate) use field::parity;
use glam::Vec3;
use std::collections::HashMap;
use words::{tr, trf};

/// A remesh's settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RemeshOptions {
    /// How far apart the grid's points are, in centimetres: the size of
    /// the new polygons, roughly.
    pub voxel_size: f32,
    /// Relax passes evening out the new surface (0 for none).
    pub smooth: u32,
}

impl RemeshOptions {
    /// The standard voxel size: 2 cm.
    pub const STANDARD_VOXEL: f32 = 2.0;
    /// The smallest voxel size: 0.1 mm.
    pub const MIN_VOXEL: f32 = 0.01;
    /// The largest voxel size: 100 m.
    pub const MAX_VOXEL: f32 = 10_000.0;
    /// The standard number of relax passes.
    pub const STANDARD_SMOOTH: u32 = 2;
    /// The most relax passes.
    pub const MAX_SMOOTH: u32 = 20;
    /// The most grid points a remesh may use: past this it is refused,
    /// with the smallest voxel size that would fit.
    pub const MAX_VOXELS: u64 = 2_000_000;

    /// The settings brought within limits: a voxel size that isn't a real
    /// number becomes the standard one.
    pub fn clamped(self) -> Self {
        let voxel_size = if self.voxel_size.is_finite() {
            self.voxel_size.clamp(Self::MIN_VOXEL, Self::MAX_VOXEL)
        } else {
            Self::STANDARD_VOXEL
        };
        Self {
            voxel_size,
            smooth: self.smooth.min(Self::MAX_SMOOTH),
        }
    }
}

impl Default for RemeshOptions {
    fn default() -> Self {
        Self {
            voxel_size: Self::STANDARD_VOXEL,
            smooth: Self::STANDARD_SMOOTH,
        }
    }
}

/// Why a shape can't be remeshed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RemeshProblem {
    /// The shape has no polygons.
    Empty,
    /// A point isn't at a real position.
    NotANumber,
    /// The shape isn't closed: it has holes in its surface (or edges
    /// shared by three polygons), so it has no inside.
    NotClosed,
    /// The grid would need `voxels` points, more than
    /// [`RemeshOptions::MAX_VOXELS`]; `smallest` is the smallest voxel
    /// size that fits.
    TooFine {
        /// The grid points needed.
        voxels: u64,
        /// The smallest voxel size that fits, in centimetres.
        smallest: f32,
    },
    /// Nothing was left: the shape is thinner than a voxel everywhere.
    NothingLeft,
    /// The new surface would have more polygons than a mesh can hold.
    TooBig,
}

impl std::fmt::Display for RemeshProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => f.write_str(tr("the shape has no polygons")),
            Self::NotANumber => f.write_str(tr("a point of the shape isn't at a real position")),
            Self::NotClosed => f.write_str(tr(
                "the shape isn't closed: it has holes in its surface, or edges shared by more \
                 than two polygons, so it has no inside",
            )),
            Self::TooFine { voxels, smallest } => f.write_str(&trf!(
                "the voxels are too small for this shape: it would take {} million of them, and \
                 the most is 2 million. Make Voxel size {:.2} cm or more",
                format!("{:.1}", *voxels as f64 / 1e6),
                smallest
            )),
            Self::NothingLeft => f.write_str(tr(
                "nothing is left: the shape is thinner than a voxel. Make Voxel size smaller",
            )),
            Self::TooBig => f.write_str(tr("the new surface would have too many polygons")),
        }
    }
}

impl std::error::Error for RemeshProblem {}

/// The shape's triangles, each with the material slot of the polygon it
/// came from.
pub(crate) struct Soup {
    pub(crate) triangles: Vec<[Vec3; 3]>,
    pub(crate) slots: Vec<u32>,
    /// Whether any polygon has a material other than the first.
    pub(crate) has_materials: bool,
}

impl Soup {
    pub(crate) fn of(mesh: &Mesh) -> Result<Self, RemeshProblem> {
        let mut triangles = Vec::with_capacity(mesh.face_count() * 2);
        let mut slots = Vec::with_capacity(mesh.face_count() * 2);
        let mut has_materials = false;
        for (face, corners) in mesh.faces() {
            let slot = crate::material::slot_of(mesh, face);
            has_materials |= slot != 0;
            let mut points = Vec::with_capacity(corners.len());
            for v in corners {
                let p = mesh.position(*v).ok_or(RemeshProblem::NotANumber)?;
                if !p.is_finite() {
                    return Err(RemeshProblem::NotANumber);
                }
                points.push(p);
            }
            crate::face_triangles(mesh, corners, |[a, b, c]| {
                triangles.push([points[a], points[b], points[c]]);
                slots.push(slot);
            });
        }
        if triangles.is_empty() {
            return Err(RemeshProblem::Empty);
        }
        Ok(Self {
            triangles,
            slots,
            has_materials,
        })
    }

    pub(crate) fn bounds(&self) -> Option<Bounds> {
        Bounds::around(self.triangles.iter().flatten().copied())
    }
}

/// Points closer than this (in centimetres) are one point when telling
/// whether a shape is closed.
const SAME_SPOT: f32 = 1e-3;

/// Whether `mesh` is closed: every edge, with points at the same place
/// counted as one, is used by an even number of polygons (two, usually).
pub fn is_closed(mesh: &Mesh) -> bool {
    let key = |p: Vec3| {
        let q = (p / SAME_SPOT).round();
        [q.x as i64, q.y as i64, q.z as i64]
    };
    let mut uses: HashMap<([i64; 3], [i64; 3]), u32> = HashMap::new();
    for (_, corners) in mesh.faces() {
        let n = corners.len();
        for i in 0..n {
            let (Some(a), Some(b)) = (
                mesh.position(corners[i]),
                mesh.position(corners[(i + 1) % n]),
            ) else {
                return false;
            };
            let (a, b) = (key(a), key(b));
            if a == b {
                // No length: not really an edge.
                continue;
            }
            *uses.entry(if a < b { (a, b) } else { (b, a) }).or_insert(0) += 1;
        }
    }
    !uses.is_empty() && uses.values().all(|n| n % 2 == 0)
}

/// The grid of sample points round a shape.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Grid {
    /// Where point (0, 0, 0) is.
    pub(crate) origin: Vec3,
    /// How far apart the points are.
    pub(crate) step: f32,
    /// How many points along X, Y and Z.
    pub(crate) size: [usize; 3],
}

/// Empty grid points kept round the shape on every side, so the surface
/// never reaches the grid's edge.
const PADDING: usize = 2;

impl Grid {
    /// How many points along each axis a grid of `step` round `bounds`
    /// has, as real numbers (so a huge count can't overflow).
    fn counts(bounds: Bounds, step: f32) -> [f64; 3] {
        let size = bounds.size();
        [size.x, size.y, size.z]
            .map(|s| (f64::from(s) / f64::from(step)).ceil() + 1.0 + 2.0 * PADDING as f64)
    }

    fn total(bounds: Bounds, step: f32) -> f64 {
        Self::counts(bounds, step).iter().product()
    }

    /// The grid round `bounds`, centred on it, or why it would be too
    /// big.
    fn around(bounds: Bounds, step: f32) -> Result<Self, RemeshProblem> {
        let total = Self::total(bounds, step);
        if total > RemeshOptions::MAX_VOXELS as f64 {
            return Err(RemeshProblem::TooFine {
                voxels: total as u64,
                smallest: smallest_step(bounds, step),
            });
        }
        let counts = Self::counts(bounds, step);
        let size = counts.map(|c| c as usize);
        let half = Vec3::new(
            (size[0] - 1) as f32,
            (size[1] - 1) as f32,
            (size[2] - 1) as f32,
        ) * step
            * 0.5;
        Ok(Self {
            origin: bounds.centre() - half,
            step,
            size,
        })
    }

    /// The number of points.
    pub(crate) fn len(&self) -> usize {
        self.size[0] * self.size[1] * self.size[2]
    }

    /// The index of point (i, j, k).
    pub(crate) fn index(&self, i: usize, j: usize, k: usize) -> usize {
        i + self.size[0] * (j + self.size[1] * k)
    }

    /// Where point (i, j, k) is.
    pub(crate) fn point(&self, i: usize, j: usize, k: usize) -> Vec3 {
        self.origin + Vec3::new(i as f32, j as f32, k as f32) * self.step
    }
}

/// The smallest voxel size (rounded up to 0.01 cm) that keeps a grid round
/// `bounds` within [`RemeshOptions::MAX_VOXELS`], from `step` up.
fn smallest_step(bounds: Bounds, step: f32) -> f32 {
    let fits = |s: f32| Grid::total(bounds, s) <= RemeshOptions::MAX_VOXELS as f64;
    let mut s = step.max(RemeshOptions::MIN_VOXEL);
    // Bounded: each pass grows it by 2%, and 2 000 passes is far past any
    // real shape (1.02^2000 is about 10^17).
    for _ in 0..2_000 {
        if fits(s) {
            break;
        }
        s *= 1.02;
    }
    let mut rounded = (s * 100.0).ceil() / 100.0;
    for _ in 0..100 {
        if fits(rounded) {
            break;
        }
        rounded += 0.01;
    }
    rounded
}

/// How many grid points remeshing `mesh` at `voxel_size` would use, or
/// `None` for a shape with no polygons (or not at real positions).
pub fn voxel_count(mesh: &Mesh, voxel_size: f32) -> Option<u64> {
    let options = RemeshOptions {
        voxel_size,
        smooth: 0,
    }
    .clamped();
    let bounds = Soup::of(mesh).ok()?.bounds()?;
    Some(Grid::total(bounds, options.voxel_size) as u64)
}

/// `mesh` rebuilt as an even surface of quads `options.voxel_size` apart
/// (see the module notes), or why it can't be.
pub fn remesh(mesh: &Mesh, options: &RemeshOptions) -> Result<Mesh, RemeshProblem> {
    let options = options.clamped();
    let soup = Soup::of(mesh)?;
    if !is_closed(mesh) {
        return Err(RemeshProblem::NotClosed);
    }
    let bounds = soup.bounds().ok_or(RemeshProblem::NotANumber)?;
    let grid = Grid::around(bounds, options.voxel_size)?;
    let field = field::Field::new(&grid, &soup);
    let mut surface = nets::Surface::extract(&grid, &field);
    if surface.quads.is_empty() {
        return Err(RemeshProblem::NothingLeft);
    }
    surface.relax(options.smooth);
    surface.into_mesh(&soup)
}
