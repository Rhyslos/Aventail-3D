//! Lattice cages ([decision 0144](../../../docs/decisions/0144-lattice-cage.md)):
//! a box of points round a shape; moving the points bends, squashes and
//! swells the shape inside smoothly, the way a sculptor's cage would.
//!
//! A lattice lives in its own space: a box of [`Lattice::size`] centred on
//! its middle, with [`Lattice::divisions`] points along each side (2 to
//! 16: 2 is a box with a point at each corner). Each point starts at its
//! *rest* place, evenly spread over the box, and can be moved anywhere.
//! A shape's point inside the box is moved by how far the cage's points
//! near it have moved from rest, weighed by how near they are
//! ([`Interpolation`]): **linear** follows the points straight between
//! neighbours (trilinear); **smooth** blends them as a cubic B-spline
//! (along a side with two points it is linear too). A cage whose points
//! are all at rest changes nothing. Points outside the box are moved as
//! the nearest place on its sides is (the box is the deformer's range,
//! [`crate::deform::BoxRange`]), so the shape carries on with no tear.
//!
//! The lattice only describes the cage; the scene keeps it as an object,
//! and the lattice modifier moves a shape's points through it with
//! [`crate::deform::deform`].

#[cfg(test)]
mod tests;
mod weights;

use crate::deform::{BoxRange, Deformer};
use crate::{Mesh, VertexId};
use glam::{Mat4, Vec3};
use weights::Weights;
use words::tr;

/// How a lattice blends its points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Interpolation {
    /// Straight between neighbouring points (trilinear): each point pulls
    /// only the cells it touches, with creases along the cage's lines.
    Linear,
    /// A smooth blend (cubic B-spline): no creases, each point pulls a
    /// little further round it. The standard.
    #[default]
    Smooth,
}

impl Interpolation {
    /// Both, in the order the Attributes panel lists them.
    pub const ALL: [Self; 2] = [Self::Smooth, Self::Linear];

    /// Its name, as the interface shows it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Linear => tr("Linear"),
            Self::Smooth => tr("Smooth"),
        }
    }
}

/// A change to a lattice's points or settings.
#[derive(Debug, Clone, PartialEq)]
pub enum LatticeEdit {
    /// Moves the points at these places in the list by `matrix` (in the
    /// lattice's space).
    Transform {
        /// The points' places in [`Lattice::points`].
        points: Vec<u32>,
        /// How they move.
        matrix: Mat4,
    },
    /// Gives each side this many points (each brought within 2 to 16),
    /// keeping the cage's shape as closely as the new points can.
    Divisions([u32; 3]),
    /// Blends the points another way.
    Interpolation(Interpolation),
    /// Puts every point back at rest.
    Reset,
}

/// Why a lattice change can't be made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatticeProblem {
    /// A point would go to a place that isn't a real number.
    NotANumber,
    /// A point would go further than [`Lattice::MAX_DISTANCE`] away.
    TooFar,
    /// The points don't match the divisions, or a setting is out of its
    /// limits (from a damaged file).
    Damaged,
}

impl std::fmt::Display for LatticeProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NotANumber => tr("a cage point would go to a place that isn't a number"),
            Self::TooFar => tr("a cage point would go more than 10 km away"),
            Self::Damaged => tr("the cage's points don't match its divisions or size"),
        })
    }
}

impl std::error::Error for LatticeProblem {}

/// A lattice cage: its box, its divisions, and where each point is.
#[derive(Debug, Clone, PartialEq)]
pub struct Lattice {
    divisions: [u32; 3],
    size: Vec3,
    points: Vec<Vec3>,
    interpolation: Interpolation,
}

impl Lattice {
    /// The fewest points along a side.
    pub const MIN_DIVISIONS: u32 = 2;
    /// The most points along a side.
    pub const MAX_DIVISIONS: u32 = 16;
    /// The standard points along each side.
    pub const STANDARD_DIVISIONS: [u32; 3] = [2, 2, 2];
    /// The smallest side of a box, in centimetres.
    pub const MIN_SIZE: f32 = 0.01;
    /// How far (centimetres) a point or side may be from the middle: 10 km.
    pub const MAX_DISTANCE: f32 = 1_000_000.0;

    /// A cage of `size` (each side brought within limits) with `divisions`
    /// points along each side (brought within 2 to 16), every point at
    /// rest.
    pub fn new(size: Vec3, divisions: [u32; 3], interpolation: Interpolation) -> Self {
        let divisions = divisions.map(|d| d.clamp(Self::MIN_DIVISIONS, Self::MAX_DIVISIONS));
        let size = Vec3::from_array(size.to_array().map(|s| {
            if s.is_finite() {
                s.abs().clamp(Self::MIN_SIZE, Self::MAX_DISTANCE)
            } else {
                Self::MIN_SIZE
            }
        }));
        let mut lattice = Self {
            divisions,
            size,
            points: Vec::new(),
            interpolation,
        };
        lattice.points = (0..lattice.point_count())
            .map(|i| lattice.rest(i))
            .collect();
        lattice
    }

    /// A cage fitted round a box of `size`: a side that is flat (a plane's
    /// thickness) is given a twentieth of the largest side, at least
    /// 0.1 cm, so the cage has room inside.
    pub fn fitted(size: Vec3, divisions: [u32; 3]) -> Self {
        let size = if size.is_finite() {
            size.abs()
        } else {
            Vec3::ONE
        };
        let least = (size.max_element() / 20.0).max(0.1);
        Self::new(
            size.max(Vec3::splat(least)),
            divisions,
            Interpolation::default(),
        )
    }

    /// A cage from saved parts, every value checked (for a project being
    /// opened): divisions 2 to 16, sides within limits, one real point
    /// within 10 km for each place.
    pub fn from_parts(
        divisions: [u32; 3],
        size: Vec3,
        points: Vec<Vec3>,
        interpolation: Interpolation,
    ) -> Result<Self, LatticeProblem> {
        let divisions_ok = divisions
            .iter()
            .all(|d| (Self::MIN_DIVISIONS..=Self::MAX_DIVISIONS).contains(d));
        let size_ok = size
            .to_array()
            .iter()
            .all(|s| (Self::MIN_SIZE..=Self::MAX_DISTANCE).contains(s));
        if !divisions_ok || !size_ok {
            return Err(LatticeProblem::Damaged);
        }
        let mut lattice = Self::new(size, divisions, interpolation);
        if points.len() != lattice.point_count() {
            return Err(LatticeProblem::Damaged);
        }
        check_points(&points)?;
        lattice.points = points;
        Ok(lattice)
    }

    /// The points along each side (X, Y, Z).
    pub fn divisions(&self) -> [u32; 3] {
        self.divisions
    }

    /// The box's size, centred on the lattice's middle.
    pub fn size(&self) -> Vec3 {
        self.size
    }

    /// How it blends its points.
    pub fn interpolation(&self) -> Interpolation {
        self.interpolation
    }

    /// Where each point is, in the lattice's space: X first, then Y,
    /// then Z ([`Self::index`]).
    pub fn points(&self) -> &[Vec3] {
        &self.points
    }

    /// How many points it has.
    pub fn point_count(&self) -> usize {
        self.divisions.iter().map(|d| *d as usize).product()
    }

    /// The place in [`Self::points`] of the point `x` along X, `y` along Y
    /// and `z` along Z.
    pub fn index(&self, x: usize, y: usize, z: usize) -> usize {
        let [nx, ny, _] = self.divisions.map(|d| d as usize);
        x + nx * (y + ny * z)
    }

    /// Where the point at place `i` is at rest: evenly spread over the box.
    pub fn rest(&self, i: usize) -> Vec3 {
        let [nx, ny, nz] = self.divisions.map(|d| d as usize);
        let at = [i % nx, (i / nx) % ny, i / (nx * ny)];
        let fraction = |a: usize, n: usize| a as f32 / (n - 1) as f32 - 0.5;
        Vec3::new(
            fraction(at[0], nx),
            fraction(at[1], ny),
            fraction(at[2], nz),
        ) * self.size
    }

    /// Whether every point is at rest (the cage changes nothing).
    pub fn is_at_rest(&self) -> bool {
        self.points
            .iter()
            .enumerate()
            .all(|(i, p)| p.distance_squared(self.rest(i)) < 1e-12)
    }

    /// Its box, as the deformer's range.
    pub fn range(&self) -> BoxRange {
        BoxRange::centred(self.size).unwrap_or(BoxRange::UNIT)
    }

    /// What moves a shape's points through the cage (the points' offsets
    /// from rest worked out once).
    pub fn deformer(&self) -> LatticeDeformer {
        LatticeDeformer {
            range: self.range(),
            divisions: self.divisions,
            interpolation: self.interpolation,
            offsets: self
                .points
                .iter()
                .enumerate()
                .map(|(i, p)| *p - self.rest(i))
                .collect(),
        }
    }

    /// The lattice after `edit`, or why it can't be made.
    pub fn edited(&self, edit: &LatticeEdit) -> Result<Self, LatticeProblem> {
        match edit {
            LatticeEdit::Transform { points, matrix } => {
                let mut out = self.clone();
                for &i in points {
                    if let Some(p) = out.points.get_mut(i as usize) {
                        *p = matrix.transform_point3(*p);
                    }
                }
                check_points(&out.points)?;
                Ok(out)
            }
            LatticeEdit::Divisions(divisions) => Ok(self.with_divisions(*divisions)),
            LatticeEdit::Interpolation(interpolation) => Ok(Self {
                interpolation: *interpolation,
                ..self.clone()
            }),
            LatticeEdit::Reset => Ok(Self::new(self.size, self.divisions, self.interpolation)),
        }
    }

    /// The same cage with `divisions` points along each side: each new
    /// point is placed where the old cage moves its rest place, so the
    /// cage keeps its shape as closely as the new points can.
    fn with_divisions(&self, divisions: [u32; 3]) -> Self {
        let mut out = Self::new(self.size, divisions, self.interpolation);
        if out.divisions == self.divisions {
            return self.clone();
        }
        let deformer = self.deformer();
        for p in &mut out.points {
            let rest = *p;
            let moved = deformer.moved(rest);
            *p = if moved.is_finite() { moved } else { rest };
        }
        out
    }

    /// The cage's points as a mesh of loose points, each named by its place
    /// in [`Self::points`]: for picking and selecting them in the Points
    /// mode.
    pub fn points_mesh(&self) -> Mesh {
        let slots = self.points.iter().map(|p| Some(*p)).collect();
        Mesh::from_slots(slots, Vec::new(), []).unwrap_or_default()
    }

    /// The point a [`Self::points_mesh`] point stands for.
    pub fn point_of(id: VertexId) -> u32 {
        id.get()
    }

    /// The cage's lines, as pairs of places in [`Self::points`]: each
    /// point to its next neighbour along X, Y and Z.
    pub fn lines(&self) -> Vec<(usize, usize)> {
        let [nx, ny, nz] = self.divisions.map(|d| d as usize);
        let mut out = Vec::new();
        for z in 0..nz {
            for y in 0..ny {
                for x in 0..nx {
                    let i = self.index(x, y, z);
                    if x + 1 < nx {
                        out.push((i, self.index(x + 1, y, z)));
                    }
                    if y + 1 < ny {
                        out.push((i, self.index(x, y + 1, z)));
                    }
                    if z + 1 < nz {
                        out.push((i, self.index(x, y, z + 1)));
                    }
                }
            }
        }
        out
    }
}

/// Checks every point is a real place within [`Lattice::MAX_DISTANCE`].
fn check_points(points: &[Vec3]) -> Result<(), LatticeProblem> {
    for p in points {
        if !p.is_finite() {
            return Err(LatticeProblem::NotANumber);
        }
        if p.abs().max_element() > Lattice::MAX_DISTANCE {
            return Err(LatticeProblem::TooFar);
        }
    }
    Ok(())
}

/// A lattice ready to move points: see [`Lattice::deformer`].
#[derive(Debug, Clone, PartialEq)]
pub struct LatticeDeformer {
    range: BoxRange,
    divisions: [u32; 3],
    interpolation: Interpolation,
    /// How far each point is from rest.
    offsets: Vec<Vec3>,
}

impl Deformer for LatticeDeformer {
    /// `p` moved by the cage: by how far the points round it have moved
    /// from rest, weighed by the interpolation, its place clamped into
    /// the box.
    fn moved(&self, p: Vec3) -> Vec3 {
        let f = self.range.fraction(p);
        let [nx, ny, nz] = self.divisions;
        let wx = Weights::of(f.x, nx, self.interpolation);
        let wy = Weights::of(f.y, ny, self.interpolation);
        let wz = Weights::of(f.z, nz, self.interpolation);
        let (sx, sy) = (nx as usize, (nx * ny) as usize);
        let mut offset = Vec3::ZERO;
        for (z, a) in wz.iter() {
            for (y, b) in wy.iter() {
                let row = z * sy + y * sx;
                let ab = a * b;
                for (x, c) in wx.iter() {
                    if let Some(o) = self.offsets.get(row + x) {
                        offset += *o * (ab * c);
                    }
                }
            }
        }
        p + offset
    }
}
