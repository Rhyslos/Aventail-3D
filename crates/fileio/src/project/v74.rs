//! Version 74 of the project file ([decision 0196](../../../../docs/decisions/0196-light-and-shade.md)):
//! version 73's, with light-and-shade layers (`v64::LayerV64::lighting`):
//! the sun's direction, the light and shade colours, softness and
//! shadows. Their pixels are saved as for any paint layer, so an older
//! app shows them as paint.

use super::v1::Problem;
use paint::Lighting;
use serde::{Deserialize, Serialize};
use words::tr;

/// A stored light-and-shade layer's settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct LightingV74 {
    pub(crate) around: f32,
    pub(crate) height: f32,
    pub(crate) light: [u8; 4],
    pub(crate) shade: [u8; 4],
    pub(crate) softness: f32,
    pub(crate) shadows: bool,
}

/// `lighting` as stored.
pub(crate) fn lighting_of(lighting: &Lighting) -> LightingV74 {
    LightingV74 {
        around: lighting.around,
        height: lighting.height,
        light: lighting.light,
        shade: lighting.shade,
        softness: lighting.softness,
        shadows: lighting.shadows,
    }
}

/// A stored light-and-shade layer, checked: numbers out of range are
/// refused.
pub(crate) fn lighting_from(stored: LightingV74) -> Result<Lighting, Problem> {
    let lighting = Lighting {
        around: stored.around,
        height: stored.height,
        light: stored.light,
        shade: stored.shade,
        softness: stored.softness,
        shadows: stored.shadows,
    };
    if !lighting.is_valid() {
        return Err(tr("its painted textures are damaged").into());
    }
    Ok(lighting)
}
