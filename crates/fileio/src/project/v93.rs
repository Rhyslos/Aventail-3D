//! Version 93 of the project file ([decision 0251](../../../../docs/decisions/0251-high-to-low.md)):
//! version 92's, with the baker's cage painted, one more kind of recorded
//! edit (`v37::OperationV37::SetCagePush`, stored as [`CageEditV93`]).
//! Files before version 93 hold none.

use super::v1::Problem;
use crate::limits::Bounded;
use scene::{Operation, VertexId};
use serde::{Deserialize, Serialize};
use words::tr;

/// The cage painted, as stored: each point and its extra push.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct CageEditV93 {
    pub(crate) points: Bounded<(u32, f32)>,
}

impl CageEditV93 {
    /// The stored form of `op`, if it paints the cage.
    pub(crate) fn of(op: &Operation) -> Option<Self> {
        let Operation::SetCagePush { points } = op else {
            return None;
        };
        Some(Self {
            points: points.iter().map(|(v, p)| (v.get(), *p)).collect(),
        })
    }

    /// The edit read back; a push that isn't a number is refused.
    pub(crate) fn into_operation(self) -> Result<Operation, Problem> {
        let points: Vec<(VertexId, f32)> = self
            .points
            .into_iter()
            .map(|(v, p)| (VertexId::from_raw(v), p))
            .collect();
        if points.iter().any(|(_, p)| !p.is_finite()) {
            return Err(tr("a cage push that isn't a number").into());
        }
        Ok(Operation::SetCagePush { points })
    }
}
