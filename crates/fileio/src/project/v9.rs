//! Version 9 of the project file's contents ([decision 0051](../../../../docs/decisions/0051-connect-bridge-weld.md)):
//! version 8's, with Connect, Bridge and Weld.
//!
//! Version 8's operations are stored inside version 9's as they were
//! (`OperationV9::V8`), so their checks are shared; the entry around each
//! operation is version 5's.

use super::v1::{FileOperation, Problem};
use super::v2::{AnchorsV2, anchors_from_file, anchors_to_file};
use super::v5::{EntryWith, ProjectWith};
use super::v8::OperationV8;
use crate::limits::Bounded;
use mesh::join::{MAX_BRIDGE_SEGMENTS, MAX_CONNECT_CUTS};
use mesh::{EdgeKey, FaceId, VertexId};
use scene::Operation;
use serde::{Deserialize, Serialize};

/// Version 9 of the file.
pub(crate) type ProjectV9 = ProjectWith<EntryV9>;

/// A recorded edit in version 9.
pub(crate) type EntryV9 = EntryWith<OperationV9>;

/// The recorded operations version 9 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV9 {
    /// Everything version 8 could store, stored the same way.
    V8(OperationV8),
    /// Points (or new points along edges) joined across their polygons.
    Connect {
        points: Bounded<u32>,
        edges: Bounded<[u32; 2]>,
        cuts: u32,
        anchors: Option<AnchorsV2>,
    },
    /// A band between two borders or two polygons.
    Bridge {
        edges: Bounded<[u32; 2]>,
        faces: Bounded<u32>,
        segments: u32,
        twist: i32,
        anchors: Option<AnchorsV2>,
    },
    /// Points melted into one.
    Weld {
        points: Bounded<u32>,
        target: Option<u32>,
        anchors: Option<AnchorsV2>,
    },
}

fn edges_to_file(edges: &[EdgeKey]) -> Bounded<[u32; 2]> {
    edges
        .iter()
        .map(|e| {
            let (a, b) = e.ends();
            [a.get(), b.get()]
        })
        .collect()
}

fn edges_from_file(edges: Bounded<[u32; 2]>, what: &str) -> Result<Vec<EdgeKey>, Problem> {
    edges
        .into_iter()
        .map(|[a, b]| {
            if a == b {
                Err(format!("one of its {what} edges joins a point to itself"))
            } else {
                Ok(EdgeKey::new(VertexId::from_raw(a), VertexId::from_raw(b)))
            }
        })
        .collect()
}

impl FileOperation for OperationV9 {
    fn to_file(op: &Operation) -> Option<Self> {
        let anchors = op.anchors().map(anchors_to_file);
        Some(match op {
            Operation::Connect {
                points,
                edges,
                cuts,
                ..
            } => Self::Connect {
                points: points.iter().map(|v| v.get()).collect(),
                edges: edges_to_file(edges),
                cuts: *cuts,
                anchors,
            },
            Operation::Bridge {
                edges,
                faces,
                segments,
                twist,
                ..
            } => Self::Bridge {
                edges: edges_to_file(edges),
                faces: faces.iter().map(|f| f.get()).collect(),
                segments: *segments,
                twist: *twist,
                anchors,
            },
            Operation::Weld { points, target, .. } => Self::Weld {
                points: points.iter().map(|v| v.get()).collect(),
                target: target.map(|t| t.get()),
                anchors,
            },
            _ => Self::V8(OperationV8::to_file(op)?),
        })
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        let anchors = |a: Option<AnchorsV2>| a.map(anchors_from_file).transpose();
        match self {
            Self::V8(op) => op.into_operation(),
            Self::Connect {
                points,
                edges,
                cuts,
                anchors: a,
            } => Ok(Operation::Connect {
                points: points.into_iter().map(VertexId::from_raw).collect(),
                edges: edges_from_file(edges, "connected")?,
                cuts: cuts.clamp(1, MAX_CONNECT_CUTS),
                anchors: anchors(a)?,
            }),
            Self::Bridge {
                edges,
                faces,
                segments,
                twist,
                anchors: a,
            } => Ok(Operation::Bridge {
                edges: edges_from_file(edges, "bridged")?,
                faces: faces.into_iter().map(FaceId::from_raw).collect(),
                segments: segments.clamp(1, MAX_BRIDGE_SEGMENTS),
                twist: twist.clamp(-1_000_000, 1_000_000),
                anchors: anchors(a)?,
            }),
            Self::Weld {
                points,
                target,
                anchors: a,
            } => Ok(Operation::Weld {
                points: points.into_iter().map(VertexId::from_raw).collect(),
                target: target.map(VertexId::from_raw),
                anchors: anchors(a)?,
            }),
        }
    }
}
