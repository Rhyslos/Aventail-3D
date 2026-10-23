//! Booleans between closed shapes: union, subtract and intersect
//! ([decision 0135](../../../docs/decisions/0135-live-booleans.md)).
//!
//! Layer 1 (core). Knows nothing about objects, files, the GPU or UI. See
//! this crate's README for its purpose and limits.
//!
//! [`apply`] takes a shape to keep and one or more cutters (each placed
//! into the kept shape's space by a matrix), and gives the result as an
//! ordinary polygon mesh:
//!
//! 1. Every shape is put into one mesh with [`mesh::parts::append`], so the
//!    cutters' UVs, materials and hard edges come along, and a mirrored
//!    cutter is turned the right way round.
//! 2. Each shape is cut into triangles and handed to Manifold (the
//!    `manifold-rust` library), each triangle tagged with the polygon it
//!    came from. Manifold needs closed solids: a shape with holes in its
//!    surface, or edges shared by more than two polygons, is refused with
//!    a reason, never a crash.
//! 3. Manifold's triangles are put back together into polygons
//!    (`rebuild`): the triangles that came from the same polygon, if they
//!    still form one piece without holes, become one polygon again (a
//!    quad cut by a cylinder becomes an n-gon round the cut), with the
//!    original's material and its corners' UVs worked out at the new
//!    corners. Edges along an original edge keep its values (hard,
//!    seam); the new edges along the cut are seams, and hard where the
//!    surfaces meet more sharply than [`Options::seam_angle`].
//!
//! The same input always gives the same result, with the same element
//! ids: Manifold is deterministic, and the result is built in a fixed
//! order.

mod rebuild;
mod solid;

use glam::Mat4;
use mesh::Mesh;

/// What a cutter does to the shape it cuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    /// Adds the cutter: the result is everything inside either.
    Union,
    /// Takes the cutter away: the result is what is inside the shape but
    /// not inside the cutter.
    Subtract,
    /// Keeps only where they overlap: what is inside both.
    Intersect,
}

impl Kind {
    /// Every kind, in the order menus list them.
    pub const ALL: [Self; 3] = [Self::Union, Self::Subtract, Self::Intersect];

    /// Its name, as menus show it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Union => "Union",
            Self::Subtract => "Subtract",
            Self::Intersect => "Intersect",
        }
    }
}

/// One cutter: its shape, what it does, and where it is.
#[derive(Debug, Clone, Copy)]
pub struct Cutter<'a> {
    /// What it does.
    pub kind: Kind,
    /// Its shape, in its own space.
    pub mesh: &'a Mesh,
    /// Where it is, in the kept shape's own space.
    pub matrix: Mat4,
}

/// How the result is built.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Options {
    /// New edges along the cut are hard where the surfaces on either side
    /// meet more sharply than this, in degrees.
    pub seam_angle: f32,
    /// The most polygons the result may have; a bigger one is refused.
    pub max_polygons: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            seam_angle: 30.0,
            max_polygons: 4_000_000,
        }
    }
}

/// Why a boolean couldn't be worked out, in plain words. The shape is
/// then left as it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// The kept shape isn't a closed solid.
    KeptNotClosed,
    /// A cutter (by its place in the list, from 0) isn't a closed solid.
    CutterNotClosed(usize),
    /// A cutter is squashed flat (scaled to nothing along some direction).
    CutterFlat(usize),
    /// A shape has a point that isn't a number.
    NotANumber,
    /// The result would have more polygons than allowed.
    TooBig,
    /// The library couldn't work it out, for the reason given.
    Failed(String),
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KeptNotClosed => f.write_str(
                "the shape isn't a closed solid: it has holes in its surface, or edges shared \
                 by more than two polygons",
            ),
            Self::CutterNotClosed(_) => f.write_str(
                "a cutter isn't a closed solid: it has holes in its surface, or edges shared by \
                 more than two polygons",
            ),
            Self::CutterFlat(_) => f.write_str("a cutter is squashed flat (scaled to nothing)"),
            Self::NotANumber => f.write_str("a point's position isn't a number"),
            Self::TooBig => f.write_str("the result would have too many polygons"),
            Self::Failed(why) => write!(f, "it couldn't be worked out ({why})"),
        }
    }
}

impl std::error::Error for Problem {}

/// The kept shape `kept` with each of `cutters` applied in turn. The
/// result is a new mesh; nothing is changed in place. Refused, with a
/// reason, if any shape isn't a closed solid. Never panics: should the
/// library itself fail unexpectedly, that is a [`Problem::Failed`] too.
pub fn apply(kept: &Mesh, cutters: &[Cutter], options: &Options) -> Result<Mesh, Problem> {
    for (i, cutter) in cutters.iter().enumerate() {
        if !cutter.matrix.is_finite() {
            return Err(Problem::NotANumber);
        }
        if cutter.matrix.determinant().abs() < 1e-9 {
            return Err(Problem::CutterFlat(i));
        }
    }
    let joined = solid::Joined::new(kept, cutters);
    let solids = joined.solids()?;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        solid::combine(&solids, cutters)
    }))
    .map_err(|_| Problem::Failed("the boolean library stopped".to_string()))??;
    rebuild::rebuild(&joined, &result, options)
}

#[cfg(test)]
mod tests;
