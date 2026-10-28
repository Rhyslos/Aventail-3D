//! Version 73 of the project file ([decision 0195](../../../../docs/decisions/0195-smart-masks.md)):
//! version 72's, with smart masks (`v67::MaskV67::smart`): what the mask
//! picks out and its settings. Their pixels are saved as for any mask,
//! so an older app shows them as painted masks.

use super::v1::Problem;
use paint::{SmartKind, SmartMask};
use serde::{Deserialize, Serialize};
use words::tr;

/// A stored smart mask.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct SmartMaskV73 {
    /// 0 edges, 1 cavities, 2 facing up, 3 height.
    pub(crate) kind: u8,
    pub(crate) amount: f32,
    pub(crate) softness: f32,
    pub(crate) noise: f32,
    pub(crate) noise_size: f32,
    pub(crate) seed: u32,
}

/// `mask` as stored.
pub(crate) fn smart_of(mask: &SmartMask) -> SmartMaskV73 {
    SmartMaskV73 {
        kind: mask.kind.index(),
        amount: mask.amount,
        softness: mask.softness,
        noise: mask.noise,
        noise_size: mask.noise_size,
        seed: mask.seed,
    }
}

/// A stored smart mask, checked: an unknown kind or numbers out of range
/// are refused.
pub(crate) fn smart_from(stored: SmartMaskV73) -> Result<SmartMask, Problem> {
    let damaged = || -> Problem { tr("its painted textures are damaged").into() };
    let mask = SmartMask {
        kind: SmartKind::from_index(stored.kind).ok_or_else(damaged)?,
        amount: stored.amount,
        softness: stored.softness,
        noise: stored.noise,
        noise_size: stored.noise_size,
        seed: stored.seed,
    };
    if !mask.is_valid() {
        return Err(damaged());
    }
    Ok(mask)
}
