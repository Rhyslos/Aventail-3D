//! Version 11 of the project file's contents ([decision 0053](../../../../docs/decisions/0053-hard-and-soft-edges.md)):
//! version 10's, with hard and soft edges: edges marked hard or soft, and
//! hard edges by angle.
//!
//! Version 10's operations are stored inside version 11's as they were
//! (`OperationV11::V10`); the entry around each operation is version 5's.

use super::v1::{FileOperation, Problem};
use super::v2::{AnchorsV2, anchors_from_file, anchors_to_file};
use super::v5::{EntryWith, ProjectWith};
use super::v10::OperationV10;
use crate::limits::Bounded;
use mesh::{EdgeKey, VertexId};
use scene::Operation;
use serde::{Deserialize, Serialize};

/// Version 11 of the file.
pub(crate) type ProjectV11 = ProjectWith<EntryV11>;

/// A recorded edit in version 11.
pub(crate) type EntryV11 = EntryWith<OperationV11>;

/// The recorded operations version 11 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV11 {
    /// Everything version 10 could store, stored the same way.
    V10(OperationV10),
    SetHardEdges {
        edges: Bounded<[u32; 2]>,
        hard: bool,
        anchors: Option<AnchorsV2>,
    },
    HardEdgesByAngle {
        angle: f32,
    },
}

impl FileOperation for OperationV11 {
    fn to_file(op: &Operation) -> Option<Self> {
        Some(match op {
            Operation::SetHardEdges { edges, hard, .. } => Self::SetHardEdges {
                edges: edges
                    .iter()
                    .map(|e| {
                        let (a, b) = e.ends();
                        [a.get(), b.get()]
                    })
                    .collect(),
                hard: *hard,
                anchors: op.anchors().map(anchors_to_file),
            },
            Operation::HardEdgesByAngle { angle } => Self::HardEdgesByAngle { angle: *angle },
            _ => Self::V10(OperationV10::to_file(op)?),
        })
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        match self {
            Self::V10(op) => op.into_operation(),
            Self::SetHardEdges {
                edges,
                hard,
                anchors,
            } => {
                let edges = edges
                    .into_iter()
                    .map(|[a, b]| {
                        if a == b {
                            Err("one of its edges joins a point to itself".to_string())
                        } else {
                            Ok(EdgeKey::new(VertexId::from_raw(a), VertexId::from_raw(b)))
                        }
                    })
                    .collect::<Result<_, _>>()?;
                Ok(Operation::SetHardEdges {
                    edges,
                    hard,
                    anchors: anchors.map(anchors_from_file).transpose()?,
                })
            }
            Self::HardEdgesByAngle { angle } => {
                if !angle.is_finite() {
                    return Err("its angle isn't a real number".to_string());
                }
                Ok(Operation::HardEdgesByAngle {
                    angle: angle.clamp(0.0, mesh::hardness::MAX_ANGLE),
                })
            }
        }
    }
}
