//! Version 87 of the project file ([decision 0236](../../../../docs/decisions/0236-editing-pieces.md)):
//! version 86's, with UVs set corner by corner (stitching, cutting,
//! aligning, orienting and warping pieces), one more kind of recorded edit
//! (`v37::OperationV37::SetUvs`, stored as [`UvsEditV87`]). Files before
//! version 87 hold none.

use super::v1::Problem;
use crate::limits::Bounded;
use scene::{FaceId, Operation, VertexId};
use serde::{Deserialize, Serialize};
use words::tr;

/// UVs set as stored: each corner's polygon, point and UV.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct UvsEditV87 {
    pub(crate) corners: Bounded<(u32, u32, [f32; 2])>,
}

impl UvsEditV87 {
    /// The stored form of `op`, if it sets UVs.
    pub(crate) fn of(op: &Operation) -> Option<Self> {
        let Operation::SetUvs { corners } = op else {
            return None;
        };
        Some(Self {
            corners: corners
                .iter()
                .map(|(f, v, uv)| (f.get(), v.get(), uv.to_array()))
                .collect(),
        })
    }

    /// The edit read back; a UV that isn't a number is refused.
    pub(crate) fn into_operation(self) -> Result<Operation, Problem> {
        let corners: Vec<(FaceId, VertexId, glam::Vec2)> = self
            .corners
            .into_iter()
            .map(|(f, v, uv)| (FaceId::from_raw(f), VertexId::from_raw(v), glam::Vec2::from_array(uv)))
            .collect();
        if corners.iter().any(|(_, _, uv)| !uv.is_finite()) {
            return Err(tr("a UV that isn't a number").into());
        }
        Ok(Operation::SetUvs { corners })
    }
}
