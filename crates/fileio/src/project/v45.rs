//! Version 45 of the project file's contents: version 44 (version 43's
//! model) with lattice cages and the Lattice modifier
//! ([decision 0144](../../../../docs/decisions/0144-lattice-cage.md)).
//!
//! Version 41's model (and so 43's) holds a cage object as an empty group,
//! in its place in the tree with its name, transform and settings; this
//! version adds each cage's points ([`LatticeV45`], in
//! [`super::v43::ProjectV43::lattices`]) and the modifier that names one
//! (`KindV43::Lattice`). An older app would open cages as empty groups
//! and the shapes unbent, and lose both on saving, so cages get a version
//! of their own: the older app says it needs updating instead.

use super::v1::Problem;
use crate::limits::Bounded;
use glam::Vec3;
use scene::{Document, Interpolation, Lattice, ObjectId, ObjectKind};
use serde::{Deserialize, Serialize};
use words::{tr, trf};

/// One lattice cage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct LatticeV45 {
    /// The object (an empty group in version 41's model).
    pub(crate) id: u64,
    /// Points along each side (X, Y, Z), 2 to 16.
    pub(crate) divisions: [u32; 3],
    /// The box's size, centimetres.
    pub(crate) size: [f32; 3],
    /// Where each point is, X first, then Y, then Z.
    pub(crate) points: Bounded<[f32; 3]>,
    /// Blended smoothly (a B-spline) rather than straight between points.
    #[serde(default = "smooth")]
    pub(crate) smooth: bool,
}

fn smooth() -> bool {
    true
}

impl LatticeV45 {
    fn of(id: ObjectId, lattice: &Lattice) -> Self {
        Self {
            id: id.get(),
            divisions: lattice.divisions(),
            size: lattice.size().to_array(),
            points: lattice.points().iter().map(|p| p.to_array()).collect(),
            smooth: lattice.interpolation() == Interpolation::Smooth,
        }
    }

    /// The cage, every value checked.
    fn lattice(self) -> Result<Lattice, Problem> {
        let interpolation = if self.smooth {
            Interpolation::Smooth
        } else {
            Interpolation::Linear
        };
        Lattice::from_parts(
            self.divisions,
            Vec3::from_array(self.size),
            self.points.0.into_iter().map(Vec3::from_array).collect(),
            interpolation,
        )
        .map_err(|why| Problem::from(trf!("a lattice can't be used: {}", why)))
    }
}

/// Every cage in `document`, in tree order.
pub(crate) fn lattices_of(document: &Document) -> Bounded<LatticeV45> {
    let scene = document.scene();
    scene
        .walk()
        .iter()
        .filter_map(|(id, _)| match &scene.object(*id)?.kind {
            ObjectKind::Lattice(l) => Some(LatticeV45::of(*id, l)),
            _ => None,
        })
        .collect()
}

/// `document` with each of `lattices` given back to its object (stored as
/// an empty group).
pub(crate) fn with_lattices(
    mut document: Document,
    lattices: Bounded<LatticeV45>,
) -> Result<Document, Problem> {
    let mut seen = std::collections::BTreeSet::new();
    for entry in lattices {
        if !seen.insert(entry.id) {
            return Err(tr("a lattice is listed twice").into());
        }
        let id = ObjectId::from_raw(entry.id);
        let lattice = entry.lattice()?;
        document = document.with_lattice(id, lattice).map_err(Problem::from)?;
    }
    Ok(document)
}
