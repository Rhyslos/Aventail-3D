//! The Live bevel modifier's work ([decision 0149](../../../docs/decisions/0149-live-bevel.md)):
//! choosing which edges to bevel, then bevelling them with the improved
//! bevel ([`crate::bevel`], decision 0141).
//!
//! The Bevel tool bevels the edges you select, once, as a recorded edit.
//! The modifier bevels the edges that match a rule, every time the shape
//! is worked out, so the bevel follows the shape as you model:
//!
//! - **By angle** (the standard): every edge whose two polygons meet more
//!   sharply than [`LiveBevelOptions::angle`] (30°): a box's edges, not
//!   the gentle steps round a cylinder.
//! - **Hard edges**: the edges the object's shading shows hard (its
//!   Smoothing: marked hard edges, or those sharper than its angle).
//! - **Marked edges**: edges with a bevel weight ([`crate::bevel_weight`]),
//!   each that share of the width.
//!
//! The bevel itself is the improved one, with its rounded corner patches
//! and *Stop at neighbours* narrowing only the points that need it; even
//! width and welding are on, as for a new Bevel tool bevel. Only edges
//! with a polygon on each side can be bevelled; others are skipped.

use crate::bevel::{BevelOptions, BevelShape, MAX_SEGMENTS, WELD_DISTANCE, bevel_edges_weighted};
use crate::{EdgeKey, Hardness, Mesh};
use words::tr;

/// The widest a live bevel can be, in the mesh's units (100 m).
pub const MAX_WIDTH: f32 = 10_000.0;

/// Which edges a live bevel bevels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BevelEdges {
    /// Edges sharper than an angle (the standard).
    #[default]
    ByAngle,
    /// The edges the object's shading shows hard.
    Hard,
    /// Edges with a bevel weight, each that share of the width.
    Marked,
}

/// The Live bevel modifier's settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiveBevelOptions {
    /// Which edges are bevelled.
    pub edges: BevelEdges,
    /// With [`BevelEdges::ByAngle`]: edges whose polygons meet more sharply
    /// than this many degrees (0 flat, up to 180). Standard 30°.
    pub angle: f32,
    /// How far the bevel reaches into each polygon, in the mesh's units
    /// (on a right-angled edge). Standard 2 cm.
    pub width: f32,
    /// How many rows each bevel has (1: a flat chamfer). Standard 2.
    pub segments: u32,
    /// Round (1, the standard), flat (0) or hollow (-1).
    pub profile: f32,
    /// *Stop at neighbours*: each point narrowed only as much as it needs
    /// so bevels never cross over each other (on, the standard).
    pub clamp: bool,
}

impl Default for LiveBevelOptions {
    fn default() -> Self {
        Self {
            edges: BevelEdges::ByAngle,
            angle: 30.0,
            width: 2.0,
            segments: 2,
            profile: 1.0,
            clamp: true,
        }
    }
}

impl LiveBevelOptions {
    /// The settings brought within their limits: angle 0 to 180°, width 0
    /// to [`MAX_WIDTH`], 1 to [`MAX_SEGMENTS`] rows, profile -1 to 1.
    /// Values that aren't real numbers become the standard ones.
    #[must_use]
    pub fn clamped(self) -> Self {
        let standard = Self::default();
        let real = |x: f32, or: f32| if x.is_finite() { x } else { or };
        Self {
            angle: real(self.angle, standard.angle).clamp(0.0, 180.0),
            width: real(self.width, standard.width).clamp(0.0, MAX_WIDTH),
            segments: self.segments.clamp(1, MAX_SEGMENTS),
            profile: real(self.profile, standard.profile).clamp(-1.0, 1.0),
            ..self
        }
    }

    /// The bevel these settings make: improved, even, welding new points
    /// closer than a millimetre (less for a very fine bevel, so its rows
    /// aren't welded together).
    pub fn shape(self) -> BevelShape {
        let o = self.clamped();
        let row = o.width / o.segments as f32;
        BevelShape {
            width: o.width,
            segments: o.segments,
            profile: o.profile,
            options: BevelOptions {
                clamp: o.clamp,
                weld_distance: WELD_DISTANCE.min(row * 0.25),
                ..BevelOptions::default()
            },
        }
    }
}

/// Why a live bevel has nothing to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveBevelProblem {
    /// By angle: no edge is sharper than the angle.
    NothingSharp,
    /// Hard edges: the shading shows none.
    NothingHard,
    /// Marked edges: no edge has a bevel weight.
    NothingMarked,
}

impl std::fmt::Display for LiveBevelProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NothingSharp => {
                tr("no edge is sharper than the angle: lower the Angle to bevel more edges")
            }
            Self::NothingHard => tr(
                "the shape has no hard edges: mark some in the Edges mode, or bevel by angle \
                 instead",
            ),
            Self::NothingMarked => tr(
                "no edge has a bevel weight: select edges in the Edges mode (press 2) and set \
                 their Bevel weight in the Attributes panel",
            ),
        })
    }
}

impl std::error::Error for LiveBevelProblem {}

/// The edges `options` bevels on `mesh` and each one's share of the width
/// (1 but for marked edges), in edge order. `hardness` is the object's
/// Smoothing, for [`BevelEdges::Hard`]. Edges the bevel can't take (open
/// borders, three or more polygons) may be listed: it skips them.
pub fn chosen_edges(
    mesh: &Mesh,
    options: &LiveBevelOptions,
    hardness: Hardness,
) -> Vec<(EdgeKey, f32)> {
    let options = options.clamped();
    let full = |edges: Vec<EdgeKey>| edges.into_iter().map(|e| (e, 1.0)).collect();
    match options.edges {
        BevelEdges::ByAngle => full(crate::hardness::sharp_edges(mesh, options.angle)),
        BevelEdges::Hard => {
            // Custom normals' breaks aren't hard edges a person chose.
            if crate::custom_normals::has_custom_normals(mesh) {
                let mut plain = mesh.clone();
                crate::custom_normals::forget_custom_normals(&mut plain);
                full(crate::hardness::shading_hard_edges(&plain, hardness))
            } else {
                full(crate::hardness::shading_hard_edges(mesh, hardness))
            }
        }
        BevelEdges::Marked => {
            let topology = mesh.topology();
            crate::bevel_weight::bevel_weights(mesh)
                .into_iter()
                .filter(|(e, _)| topology.has_edge(*e))
                .collect()
        }
    }
}

/// `mesh` with the edges `options` chooses bevelled, or why there are
/// none. A width of 0 gives the shape as it is. Never panics; settings
/// out of range are brought within them.
pub fn live_bevel(
    mesh: &Mesh,
    options: &LiveBevelOptions,
    hardness: Hardness,
) -> Result<Mesh, LiveBevelProblem> {
    let options = options.clamped();
    let edges = chosen_edges(mesh, &options, hardness);
    if edges.is_empty() {
        return Err(match options.edges {
            BevelEdges::ByAngle => LiveBevelProblem::NothingSharp,
            BevelEdges::Hard => LiveBevelProblem::NothingHard,
            BevelEdges::Marked => LiveBevelProblem::NothingMarked,
        });
    }
    let mut out = mesh.clone();
    if options.width > 0.0 {
        bevel_edges_weighted(&mut out, &edges, options.shape());
    }
    Ok(out)
}

/// Roughly how many edges `options` bevels on `mesh`, for estimating the
/// work (the object's Smoothing isn't known here: hard edges are counted
/// as the marked ones or those sharper than 30°, whichever are more).
pub fn edge_estimate(mesh: &Mesh, options: &LiveBevelOptions) -> usize {
    let options = options.clamped();
    match options.edges {
        BevelEdges::ByAngle => crate::hardness::sharp_edges(mesh, options.angle).len(),
        BevelEdges::Hard => mesh
            .hard_edges()
            .count()
            .max(crate::hardness::sharp_edges(mesh, 30.0).len()),
        BevelEdges::Marked => crate::bevel_weight::bevel_weights(mesh).len(),
    }
}

#[cfg(test)]
mod tests;
