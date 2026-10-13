//! Version 4 of the project file's contents ([decision 0040](../../../../docs/decisions/0040-bevel.md)):
//! version 3's structures, with recorded bevels of edges and points.
//! Earlier versions are read with their own models and need no
//! conversion.
//!
//! Version 3's operations are stored inside version 4's as they were
//! (`OperationV4::V3`), so their checks are shared; new kinds go at the
//! end, in a later version.

use super::v1::{FileOperation, Problem};
use super::v2::{AnchorsV2, anchors_from_file, anchors_to_file};
use super::v3::OperationV3;
use mesh::bevel::MAX_SEGMENTS;
use mesh::{EdgeKey, VertexId};
use scene::{BevelShape, Operation};
use serde::{Deserialize, Serialize};

/// Version 4 of the file.
#[cfg(test)]
pub(crate) type ProjectV4 = super::v1::ProjectFile<OperationV4>;

/// The recorded operations version 4 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV4 {
    /// Everything version 3 could store, stored the same way.
    V3(OperationV3),
    /// Edges bevelled, each by its two points.
    BevelEdges {
        edges: Vec<[u32; 2]>,
        width: f32,
        segments: u32,
        profile: f32,
        anchors: Option<AnchorsV2>,
    },
    /// Points bevelled.
    BevelPoints {
        points: Vec<u32>,
        width: f32,
        anchors: Option<AnchorsV2>,
    },
}

fn number(value: f32, what: &str) -> Result<f32, Problem> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!("its bevel {what} isn't a real number"))
    }
}

impl FileOperation for OperationV4 {
    fn to_file(op: &Operation) -> Option<Self> {
        let anchors = op.anchors().map(anchors_to_file);
        Some(match op {
            Operation::BevelEdges { edges, shape, .. } => Self::BevelEdges {
                edges: edges
                    .iter()
                    .map(|e| {
                        let (a, b) = e.ends();
                        [a.get(), b.get()]
                    })
                    .collect(),
                width: shape.width,
                segments: shape.segments,
                profile: shape.profile,
                anchors,
            },
            Operation::BevelPoints { points, width, .. } => Self::BevelPoints {
                points: points.iter().map(|v| v.get()).collect(),
                width: *width,
                anchors,
            },
            _ => Self::V3(OperationV3::to_file(op)?),
        })
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        let anchors = |a: Option<AnchorsV2>| a.map(anchors_from_file).transpose();
        match self {
            Self::V3(op) => op.into_operation(),
            Self::BevelEdges {
                edges,
                width,
                segments,
                profile,
                anchors: a,
            } => Ok(Operation::BevelEdges {
                edges: edges
                    .into_iter()
                    .map(|[a, b]| {
                        if a == b {
                            Err("one of its bevelled edges joins a point to itself".to_string())
                        } else {
                            Ok(EdgeKey::new(VertexId::from_raw(a), VertexId::from_raw(b)))
                        }
                    })
                    .collect::<Result<_, _>>()?,
                shape: BevelShape {
                    width: number(width, "width")?.max(0.0),
                    segments: segments.clamp(1, MAX_SEGMENTS),
                    profile: number(profile, "profile")?.clamp(-1.0, 1.0),
                },
                anchors: anchors(a)?,
            }),
            Self::BevelPoints {
                points,
                width,
                anchors: a,
            } => Ok(Operation::BevelPoints {
                points: points.into_iter().map(VertexId::from_raw).collect(),
                width: number(width, "width")?.max(0.0),
                anchors: anchors(a)?,
            }),
        }
    }
}
