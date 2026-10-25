//! Version 56 of the project file ([decision 0155](../../../../docs/decisions/0155-levels-of-detail.md)):
//! version 55's, with each object's levels of detail, stored with its
//! development settings (`v39::ObjectV39::lods`, as [`LodsV56`]). Files
//! before version 56 hold none.

use crate::limits::Bounded;
use scene::{LodLevel, Lods};
use serde::{Deserialize, Serialize};

/// An object's levels of detail as stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct LodsV56 {
    /// Whether it has them.
    pub(crate) on: bool,
    /// Each level: its ratio (a share, 1 is all) and the distance it is
    /// shown from (centimetres). More than `Lods::MAX_LEVELS` are cut
    /// short on reading.
    pub(crate) levels: Bounded<(f32, f32)>,
    /// The level the viewport shows: 0 for the full shape.
    pub(crate) preview: u32,
}

impl LodsV56 {
    /// The stored form of an object's levels of detail: only when on, or
    /// set to something other than the standard.
    pub(crate) fn of(object: &scene::Object) -> Option<Self> {
        let lods = &object.lods;
        (*lods != Lods::default()).then(|| Self {
            on: lods.on,
            levels: lods.levels.iter().map(|l| (l.ratio, l.distance)).collect(),
            preview: u32::try_from(lods.preview).unwrap_or(u32::MAX),
        })
    }

    /// The settings read back (clamped by whoever uses them).
    pub(crate) fn lods(&self) -> Lods {
        Lods {
            on: self.on,
            levels: self
                .levels
                .iter()
                .take(Lods::MAX_LEVELS)
                .map(|(ratio, distance)| LodLevel {
                    ratio: *ratio,
                    distance: *distance,
                })
                .collect(),
            preview: usize::try_from(self.preview).unwrap_or(usize::MAX),
        }
    }
}
