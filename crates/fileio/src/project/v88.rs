//! Version 88 of the project file ([decision 0240](../../../../docs/decisions/0240-packing-control.md)):
//! version 87's, with pieces' packing values (a packing group, a grain, a
//! texture scale), one more kind of recorded edit
//! (`v37::OperationV37::SetPackValues`, stored as [`PackEditV88`]). The
//! values themselves live in the shapes' polygon layers, saved as every
//! layer is. Files before version 88 hold none.

use super::v1::Problem;
use super::v2::{AnchorsV2, anchors_from_file, anchors_to_file};
use crate::limits::Bounded;
use mesh::uvs::{PackParts, PackValues};
use scene::{FaceId, Operation};
use serde::{Deserialize, Serialize};
use words::tr;

/// Packing values set, as stored: the polygons, the values, which of
/// them are set (group, grain, scale), and the polygons' anchors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct PackEditV88 {
    pub(crate) faces: Bounded<u32>,
    pub(crate) group: i32,
    pub(crate) grain: i32,
    pub(crate) scale: f32,
    pub(crate) which: [bool; 3],
    pub(crate) anchors: Option<AnchorsV2>,
}

impl PackEditV88 {
    /// The stored form of `op`, if it sets packing values.
    pub(crate) fn of(op: &Operation) -> Option<Self> {
        let Operation::SetPackValues {
            faces,
            values,
            which,
            anchors,
        } = op
        else {
            return None;
        };
        Some(Self {
            faces: faces.iter().map(|f| f.get()).collect(),
            group: values.group,
            grain: values.grain,
            scale: values.scale,
            which: [which.group, which.grain, which.scale],
            anchors: anchors.as_ref().map(anchors_to_file),
        })
    }

    /// The edit read back; values a piece can't have are refused.
    pub(crate) fn into_operation(self) -> Result<Operation, Problem> {
        let values = PackValues {
            group: self.group,
            grain: self.grain,
            scale: self.scale,
        };
        if !values.is_real() {
            return Err(tr("packing values out of range").into());
        }
        Ok(Operation::SetPackValues {
            faces: self.faces.into_iter().map(FaceId::from_raw).collect(),
            values,
            which: PackParts {
                group: self.which[0],
                grain: self.which[1],
                scale: self.which[2],
            },
            anchors: self.anchors.map(anchors_from_file).transpose()?,
        })
    }
}
