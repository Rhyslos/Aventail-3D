//! Version 111 of the project file ([decision 0293](../../../../docs/decisions/0293-blend-seams.md)):
//! version 110's, with Seam blend layers (`v64::LayerV64::seam_blend`,
//! stored as [`SeamBlendV111`]): the seams each blends (by object and
//! its two points' ids), how wide, and what its pixels were made from.
//! Its pixels are saved as for any paint layer, so it opens as it was
//! left without being made again.
//!
//! Files before version 111 hold none; an older app would keep the
//! pixels but forget the layer remakes itself, so the version goes up.

use super::v1::Problem;
use crate::limits::Bounded;
use paint::seam_blend::{MAX_SEAMS, MAX_WIDTH, MIN_WIDTH};
use paint::{SeamBlend, SeamId};
use serde::{Deserialize, Serialize};
use words::tr;

/// A stored Seam blend layer's settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct SeamBlendV111 {
    /// The seams it blends.
    pub(crate) seams: Bounded<SeamV111>,
    /// How far the blend reaches, in texels each side.
    pub(crate) width: f32,
    /// What its pixels were made from (a hash; 0 until made).
    pub(crate) made: u64,
}

/// One seam: the object, and its edge's two point ids.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct SeamV111 {
    /// The object's id.
    pub(crate) object: u64,
    /// The edge's two points' ids.
    pub(crate) points: [u32; 2],
}

/// `blend` as stored.
pub(crate) fn seam_blend_of(blend: &SeamBlend) -> SeamBlendV111 {
    SeamBlendV111 {
        seams: blend
            .seams
            .iter()
            .map(|s| SeamV111 {
                object: s.object,
                points: s.points,
            })
            .collect(),
        width: blend.width,
        made: blend.made,
    }
}

/// A stored Seam blend, checked: a width out of range or not a real
/// number, too many seams, or an edge from a point to itself is refused.
/// Seams out of order or twice are put in order, once each; a seam no
/// longer on the model is kept (the layer leaves it out).
pub(crate) fn seam_blend_from(stored: SeamBlendV111) -> Result<SeamBlend, Problem> {
    let damaged = || -> Problem { tr("its painted textures are damaged").into() };
    if !(MIN_WIDTH..=MAX_WIDTH).contains(&stored.width) || stored.seams.len() > MAX_SEAMS {
        return Err(damaged());
    }
    let mut seams = Vec::with_capacity(stored.seams.len());
    for seam in stored.seams.0 {
        let [a, b] = seam.points;
        if a == b {
            return Err(damaged());
        }
        seams.push(SeamId {
            object: seam.object,
            points: [a.min(b), a.max(b)],
        });
    }
    seams.sort_unstable();
    seams.dedup();
    let blend = SeamBlend {
        seams,
        width: stored.width,
        made: stored.made,
    };
    if blend.is_valid() {
        Ok(blend)
    } else {
        Err(damaged())
    }
}
