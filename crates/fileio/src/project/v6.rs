//! Version 6 of the project file's contents ([decision 0047](../../../../docs/decisions/0047-loop-cut.md)):
//! version 5's structures, with recorded loop cuts.
//!
//! Version 4's operations are stored inside version 6's as they were
//! (`OperationV6::V4`), so their checks are shared; the entry around each
//! operation (its id, switches and structure) is version 5's.

use super::v1::{FileOperation, Problem};
use super::v2::{AnchorsV2, anchors_from_file, anchors_to_file};
use super::v4::OperationV4;
use super::v5::EntryWith;
use mesh::loopcut::MAX_CUTS;
use mesh::{EdgeKey, VertexId};
use scene::Operation;
use serde::{Deserialize, Serialize};
use words::tr;

/// Version 6 of the file.
#[cfg(test)]
pub(crate) type ProjectV6 = super::v5::ProjectWith<EntryV6>;

/// A recorded edit in version 6.
pub(crate) type EntryV6 = EntryWith<OperationV6>;

/// The recorded operations version 6 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV6 {
    /// Everything version 4 (and 5) could store, stored the same way.
    V4(OperationV4),
    /// A loop cut across an edge, named by its two points.
    LoopCut {
        edge: [u32; 2],
        cuts: u32,
        position: f32,
        anchors: Option<AnchorsV2>,
    },
}

impl FileOperation for OperationV6 {
    fn to_file(op: &Operation) -> Option<Self> {
        Some(match op {
            Operation::LoopCut {
                edge,
                cuts,
                position,
                anchors,
            } => {
                let (a, b) = edge.ends();
                Self::LoopCut {
                    edge: [a.get(), b.get()],
                    cuts: *cuts,
                    position: *position,
                    anchors: anchors.as_ref().map(anchors_to_file),
                }
            }
            _ => Self::V4(OperationV4::to_file(op)?),
        })
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        match self {
            Self::V4(op) => op.into_operation(),
            Self::LoopCut {
                edge: [a, b],
                cuts,
                position,
                anchors,
            } => {
                if a == b {
                    return Err(
                        tr("its loop cut is across an edge joining a point to itself").into(),
                    );
                }
                if !position.is_finite() {
                    return Err(tr("its loop cut's position isn't a real number").into());
                }
                Ok(Operation::LoopCut {
                    edge: EdgeKey::new(VertexId::from_raw(a), VertexId::from_raw(b)),
                    cuts: cuts.clamp(1, MAX_CUTS),
                    position: position.clamp(0.0, 1.0),
                    anchors: anchors.map(anchors_from_file).transpose()?,
                })
            }
        }
    }
}
