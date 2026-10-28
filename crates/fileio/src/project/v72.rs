//! Version 72 of the project file ([decision 0194](../../../../docs/decisions/0194-gradient-fills.md)):
//! version 71's, with gradient fill layers (`v64::LayerV64::gradient`):
//! the colours, the axis and where it starts and ends. Their pixels are
//! saved as for any paint layer, so an older app shows them as paint.

use super::v1::Problem;
use paint::{Gradient, GradientAxis};
use serde::{Deserialize, Serialize};
use words::tr;

/// A stored gradient fill.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct GradientV72 {
    pub(crate) from: [u8; 4],
    pub(crate) to: [u8; 4],
    /// 0 X, 1 Y, 2 Z.
    pub(crate) axis: u8,
    pub(crate) start: f32,
    pub(crate) end: f32,
}

/// `gradient` as stored.
pub(crate) fn gradient_of(gradient: &Gradient) -> GradientV72 {
    GradientV72 {
        from: gradient.from,
        to: gradient.to,
        axis: gradient.axis.index() as u8,
        start: gradient.start,
        end: gradient.end,
    }
}

/// A stored gradient, checked: an unknown axis or numbers out of range
/// are refused.
pub(crate) fn gradient_from(stored: GradientV72) -> Result<Gradient, Problem> {
    let axis = GradientAxis::from_index(stored.axis as usize)
        .ok_or_else(|| -> Problem { tr("its painted textures are damaged").into() })?;
    let gradient = Gradient {
        from: stored.from,
        to: stored.to,
        axis,
        start: stored.start,
        end: stored.end,
    };
    if !gradient.is_valid() {
        return Err(tr("its painted textures are damaged").into());
    }
    Ok(gradient)
}
