//! Version 67 of the project file ([decision 0182](../../../../docs/decisions/0182-masks-and-polygon-fill.md)):
//! version 66's, with layer masks (`v64::LayerV64::mask`, stored as
//! [`MaskV67`]: the painted tiles, white or black, inverted, and the
//! strokes painted on it) and what the polygon fill tool filled in each
//! stroke (`v65::StrokeV65::fills`). Files before version 67 hold neither.

use super::v1::Problem;
use super::v64::{TileV64, image_from, tiles_of};
use crate::limits::Bounded;
use paint::Mask;
use serde::{Deserialize, Serialize};

/// A layer's mask as stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct MaskV67 {
    /// Its painted tiles.
    pub(crate) tiles: Bounded<TileV64>,
    /// Whether the unpainted mask shows the layer.
    pub(crate) white: bool,
    /// Whether it works the other way round.
    pub(crate) inverted: bool,
    /// The strokes painted on it.
    pub(crate) strokes: Bounded<super::v65::StrokeV65>,
    /// A smart mask's settings (version 73).
    #[serde(default)]
    pub(crate) smart: Option<super::v73::SmartMaskV73>,
}

/// The mask to store.
pub(crate) fn mask_of(mask: &Mask) -> MaskV67 {
    MaskV67 {
        tiles: tiles_of(&mask.image),
        white: mask.white,
        inverted: mask.inverted,
        strokes: super::v65::strokes_of(&mask.strokes),
        smart: mask.smart.as_ref().map(super::v73::smart_of),
    }
}

/// The stored mask of a texture set `size` pixels across, checked.
pub(crate) fn mask_from(stored: MaskV67, size: u32) -> Result<Mask, Problem> {
    Ok(Mask {
        image: image_from(stored.tiles, size)?,
        white: stored.white,
        inverted: stored.inverted,
        strokes: super::v65::strokes_from(stored.strokes)?,
        smart: stored.smart.map(super::v73::smart_from).transpose()?,
    })
}
