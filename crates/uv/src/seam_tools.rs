//! Quicker seam work ([decision 0234](../../../docs/decisions/0234-quicker-seam-work.md)):
//! the **Fewer seams ↔ less stretch** slider's settings, seams along
//! edge loops and rings, and copying seams to the other side of a
//! symmetrical shape.
//!
//! - [`balanced`] turns one number from 0 to 1 into Suggest seams' corner
//!   angle and stretch budget together. At 0, only what a piece needs to
//!   lie flat is cut (no corners, no budget): the fewest seams, the most
//!   stretch. At the middle ([`STANDARD_BALANCE`]) it gives the standard
//!   settings. Towards 1, sharper and sharper corners are cut and pieces
//!   are held to a tighter and tighter budget: more seams, less stretch.
//! - [`loops`] and [`rings`] widen a set of edges to the loops or rings
//!   they are on, the way a double-click selects them.
//! - [`mirrored`] finds, for each seam, its mirror across a plane of the
//!   shape: the seams to add so both sides match.

use mesh::symmetry::{Mirror, Partners};
use mesh::{EdgeKey, Mesh};
use std::collections::BTreeSet;

/// The slider's place that gives the standard settings (corner angle
/// [`crate::seams::CORNER_ANGLE`], no budget).
pub const STANDARD_BALANCE: f32 = 0.5;

/// The sharpest corner angle the slider asks for (degrees), at 1.
pub const TIGHTEST_CORNER: f32 = 30.0;

/// The loosest budget the slider sets, just past the middle, and the
/// tightest, at 1 (as factors: how many times off a piece may be).
pub const LOOSEST_BUDGET: f32 = 4.0;
/// See [`LOOSEST_BUDGET`].
pub const TIGHTEST_BUDGET: f32 = 1.15;

/// Suggest seams' corner angle (degrees) and stretch budget (`None`: no
/// budget) for the slider at `t` (0: fewest seams, 1: least stretch;
/// clamped, and a broken number counts as the middle).
///
/// Below the middle the corner angle rises from the standard to 180°
/// (no corners cut) and there is no budget. Above it the corner angle
/// falls to [`TIGHTEST_CORNER`] and the budget tightens from
/// [`LOOSEST_BUDGET`] to [`TIGHTEST_BUDGET`], evenly on a log scale (each
/// step asks the same share less stretch).
pub fn balanced(t: f32) -> (f32, Option<f32>) {
    let t = if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        STANDARD_BALANCE
    };
    let standard = crate::seams::CORNER_ANGLE;
    if t <= STANDARD_BALANCE {
        let k = t / STANDARD_BALANCE;
        return (180.0 + (standard - 180.0) * k, None);
    }
    let k = (t - STANDARD_BALANCE) / (1.0 - STANDARD_BALANCE);
    let corner = standard + (TIGHTEST_CORNER - standard) * k;
    let budget = (LOOSEST_BUDGET.ln() + (TIGHTEST_BUDGET.ln() - LOOSEST_BUDGET.ln()) * k).exp();
    (corner, Some(budget))
}

/// `edges` and every edge on the same loops (edges of `mesh` only; an edge
/// the shape doesn't have is left out).
pub fn loops(mesh: &Mesh, edges: &[EdgeKey]) -> BTreeSet<EdgeKey> {
    let topology = mesh.topology();
    edges.iter().flat_map(|e| topology.edge_loop(*e)).collect()
}

/// `edges` and every edge on the same rings (across quads; edges of
/// `mesh` only).
pub fn rings(mesh: &Mesh, edges: &[EdgeKey]) -> BTreeSet<EdgeKey> {
    let topology = mesh.topology();
    edges.iter().flat_map(|e| topology.edge_ring(*e)).collect()
}

/// The mirror across the shape's own `axis` (0 X, 1 Y, 2 Z) through its
/// origin, as [`mirrored`] uses.
pub fn mirror_across(axis: usize) -> Mirror {
    let mut axes = [false; 3];
    axes[axis.min(2)] = true;
    Mirror::through_origin(axes)
}

/// The mirrors of `seams` across `axis` (0 X, 1 Y, 2 Z, through the
/// shape's origin) that aren't among them: the edges to mark so both
/// sides have the same seams. A seam without a mirror (where the shape
/// isn't symmetrical) is skipped. Sorted, each once.
pub fn mirrored(mesh: &Mesh, seams: &[EdgeKey], axis: usize) -> Vec<EdgeKey> {
    let mirror = mirror_across(axis);
    let mut reflect = [false; 3];
    reflect[axis.min(2)] = true;
    let partners = Partners::new(mesh, mirror, reflect);
    let have: BTreeSet<EdgeKey> = seams.iter().copied().collect();
    let found: BTreeSet<EdgeKey> = seams
        .iter()
        .filter_map(|e| partners.edge(*e))
        .filter(|e| !have.contains(e))
        .collect();
    found.into_iter().collect()
}

#[cfg(test)]
mod tests;
