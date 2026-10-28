//! Version 80 of the project file ([decision 0210](../../../../docs/decisions/0210-material-layers.md)):
//! version 79's, with material layers (`v64::LayerV64::material`, stored
//! as [`MaterialFillV80`]): the kind, colours, scale, roughness, metal,
//! sharpness and pattern. Their pixels are saved as for any paint layer,
//! so an older app shows them as paint. Files before version 80 hold
//! none.

use super::v1::Problem;
use paint::{MaterialFill, MaterialKind};
use serde::{Deserialize, Serialize};
use words::tr;

/// A stored material layer's settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct MaterialFillV80 {
    pub(crate) kind: u8,
    pub(crate) colour: [u8; 4],
    pub(crate) second: [u8; 4],
    pub(crate) scale: f32,
    pub(crate) roughness: f32,
    pub(crate) metal: f32,
    pub(crate) sharpness: f32,
    pub(crate) seed: u32,
}

/// `fill` as stored.
pub(crate) fn material_of(fill: &MaterialFill) -> MaterialFillV80 {
    MaterialFillV80 {
        kind: fill.kind.index(),
        colour: fill.colour,
        second: fill.second,
        scale: fill.scale,
        roughness: fill.roughness,
        metal: fill.metal,
        sharpness: fill.sharpness,
        seed: fill.seed,
    }
}

/// A stored material layer's settings, checked: an unknown kind or
/// numbers out of range are refused.
pub(crate) fn material_from(stored: MaterialFillV80) -> Result<MaterialFill, Problem> {
    let damaged = || -> Problem { tr("its painted textures are damaged").into() };
    let fill = MaterialFill {
        kind: MaterialKind::from_index(stored.kind).ok_or_else(damaged)?,
        colour: stored.colour,
        second: stored.second,
        scale: stored.scale,
        roughness: stored.roughness,
        metal: stored.metal,
        sharpness: stored.sharpness,
        seed: stored.seed,
    };
    if fill.is_valid() {
        Ok(fill)
    } else {
        Err(damaged())
    }
}
