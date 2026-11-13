//! Version 103 of the project file ([decision 0277](../../../../docs/decisions/0277-animated-boxes.md)):
//! version 102's, with animated materials. Each graph (`v95::GraphV95`)
//! gains the moment of its animation that exports and the painter bake
//! (`moment`, in seconds), and each box (`v95::BoxV95`) the keys of its
//! settings marked "change in the game" (`in_game`), kept for the engine
//! translators. Files before version 103 hold neither: their graphs are
//! baked at the start and nothing is marked, which is how they worked; an
//! older app would drop both, so the version goes up.
//!
//! The moment read is brought within 0 to `matgraph::MAX_MOMENT` (not a
//! number is the start), and marks on settings a box doesn't have, or
//! that can't change in the game, are dropped, by
//! `matgraph::Graph::from_parts`.

use crate::limits::Bounded;

/// A graph's moment as stored: none at the start.
pub(crate) fn moment_of(moment: f32) -> Option<f32> {
    (moment != 0.0).then_some(moment)
}

/// The moment stored as `stored`.
pub(crate) fn moment_from(stored: Option<f32>) -> f32 {
    stored.unwrap_or(0.0)
}

/// Whether a box has no settings marked (none are stored then).
pub(crate) fn no_keys(keys: &Bounded<String>) -> bool {
    keys.is_empty()
}
