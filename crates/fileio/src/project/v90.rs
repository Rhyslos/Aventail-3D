//! Version 90 of the project file ([decision 0242](../../../../docs/decisions/0242-lightmap-uvs.md)):
//! version 89's, with lightmap UVs made, one more kind of recorded edit
//! (`v37::OperationV37::MakeLightmap`, stored as [`LightmapEditV90`]).
//! Files before version 90 hold none.

use super::v1::Problem;
use scene::Operation;
use serde::{Deserialize, Serialize};
use words::tr;

/// Lightmap UVs made, as stored: the size, the padding, and the hard
/// edges' angle (`None`: the edges marked hard).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct LightmapEditV90 {
    pub(crate) size: u32,
    pub(crate) padding: u32,
    pub(crate) angle: Option<f32>,
}

impl LightmapEditV90 {
    /// The stored form of `op`, if it makes lightmap UVs.
    pub(crate) fn of(op: &Operation) -> Option<Self> {
        let Operation::MakeLightmap {
            size,
            padding,
            hardness,
        } = op
        else {
            return None;
        };
        Some(Self {
            size: *size,
            padding: *padding,
            angle: match hardness {
                mesh::Hardness::Marked => None,
                mesh::Hardness::ByAngle(a) => Some(*a),
            },
        })
    }

    /// The edit read back; numbers out of range are refused.
    pub(crate) fn into_operation(self) -> Result<Operation, Problem> {
        let op = Operation::MakeLightmap {
            size: self.size,
            padding: self.padding,
            hardness: self.angle.map_or(mesh::Hardness::Marked, mesh::Hardness::ByAngle),
        };
        if !op.numbers_are_real() {
            return Err(tr("lightmap settings out of range").into());
        }
        Ok(op)
    }
}
