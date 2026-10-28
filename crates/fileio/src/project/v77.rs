//! Version 77 of the project file ([decision 0205](../../../../docs/decisions/0205-vertex-colour-painting.md)):
//! version 76's, with vertex colour painting, one more kind of recorded
//! edit (`v37::OperationV37::PaintColours`, stored as [`ColoursEditV77`]).
//! The colours themselves live in the shapes' corner layers, saved as
//! every layer is. Files before version 77 hold none.

use super::v1::Problem;
use crate::limits::Bounded;
use scene::{FaceId, Operation, VertexId};
use serde::{Deserialize, Serialize};

/// Painted vertex colours as stored: each corner's polygon, point and
/// colour.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ColoursEditV77 {
    pub(crate) corners: Bounded<(u32, u32, [u8; 4])>,
}

impl ColoursEditV77 {
    /// The stored form of `op`, if it paints vertex colours.
    pub(crate) fn of(op: &Operation) -> Option<Self> {
        let Operation::PaintColours { corners } = op else {
            return None;
        };
        Some(Self {
            corners: corners
                .iter()
                .map(|(f, v, c)| (f.get(), v.get(), *c))
                .collect(),
        })
    }

    /// The edit read back (any colour is a colour; corners no longer in
    /// the shape are left out when it replays).
    pub(crate) fn into_operation(self) -> Result<Operation, Problem> {
        Ok(Operation::PaintColours {
            corners: self
                .corners
                .into_iter()
                .map(|(f, v, c)| (FaceId::from_raw(f), VertexId::from_raw(v), c))
                .collect(),
        })
    }
}
