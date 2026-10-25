//! The Displace modifier ([decision 0157](../../../docs/decisions/0157-displace.md)):
//! a shape's points pushed in and out by a hand-written noise, along
//! their normals or an axis, everywhere or only round chosen points.
//!
//! The work is in [`mesh::displace`]. It reads no other object, so it has
//! no sources; the chosen points are point ids of the shape as modelled
//! (the Points mode's), set with *Use selected* in the Attributes panel,
//! as Shrinkwrap's are (decision 0147).

use crate::modifier::ModifierContext;
use mesh::displace::{DisplaceOptions, displace};
use mesh::{Mesh, VertexId};
use std::collections::BTreeSet;
use words::tr;

/// The Displace modifier's settings: the noise, and which points it
/// pushes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DisplaceSettings {
    /// The noise and how far it pushes.
    pub options: DisplaceOptions,
    /// The points it pushes (fading out round them over the falloff), by
    /// id from the shape as modelled, each once, in order; empty for
    /// every point.
    pub points: Vec<VertexId>,
}

impl DisplaceSettings {
    /// The settings within limits: points listed once each, in order.
    pub fn clamped(&self) -> Self {
        let points: BTreeSet<VertexId> = self.points.iter().copied().collect();
        Self {
            options: self.options.clamped(),
            points: points.into_iter().collect(),
        }
    }

    /// Roughly how much work it is on `input`, in polygons of a boolean's
    /// worth.
    pub(crate) fn work(&self, input: &Mesh) -> usize {
        mesh::displace::work(input, &self.options, !self.points.is_empty())
    }
}

/// `input` (in the object's space) displaced by `settings`.
pub(crate) fn apply_displace(
    input: &Mesh,
    settings: &DisplaceSettings,
    _context: &ModifierContext,
) -> Result<Mesh, String> {
    if settings.points.is_empty() {
        return Ok(displace(input, None, &settings.options));
    }
    let only: BTreeSet<VertexId> = settings
        .points
        .iter()
        .copied()
        .filter(|v| input.position(*v).is_some())
        .collect();
    if only.is_empty() {
        return Err(
            tr("none of its chosen points are left: choose them again, or use all points")
                .to_string(),
        );
    }
    Ok(displace(input, Some(&only), &settings.options))
}
