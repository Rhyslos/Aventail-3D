//! Version 61 of the project file ([decision 0160](../../../../docs/decisions/0160-kit-helpers.md)):
//! version 60's, with the project's kit grid (`v43::ProjectV43::kit_grid`,
//! stored as [`KitGridV61`]): its step and whether new shapes are made
//! kit-sized. It is stored only when it isn't the standard (1 m, shapes
//! made at 1 m); files before version 61 never hold it and open with the
//! standard.
//!
//! An older app would drop the kit grid without a word on saving, so it
//! gets a version of its own: the older app says it needs updating
//! instead.

use super::v1::Problem;
use scene::{Document, KitGrid};
use serde::{Deserialize, Serialize};
use words::tr;

/// The kit grid as stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct KitGridV61 {
    /// The step, in centimetres.
    pub(crate) step: f32,
    /// Whether new shapes are made kit-sized.
    #[serde(default)]
    pub(crate) sized_shapes: bool,
}

/// The kit grid to store for `document`: `None` for the standard.
pub(crate) fn kit_grid_of(document: &Document) -> Option<KitGridV61> {
    let grid = document.scene().kit_grid();
    (grid != KitGrid::default()).then_some(KitGridV61 {
        step: grid.step,
        sized_shapes: grid.sized_shapes,
    })
}

/// `document` with the stored kit grid, checked.
pub(crate) fn with_kit_grid(
    document: Document,
    stored: Option<KitGridV61>,
) -> Result<Document, Problem> {
    let Some(stored) = stored else {
        return Ok(document);
    };
    let grid = KitGrid {
        step: stored.step,
        sized_shapes: stored.sized_shapes,
    };
    if !grid.is_valid() {
        return Err(tr("its kit grid isn't a size a kit can use").into());
    }
    Ok(document.with_kit_grid(grid))
}
