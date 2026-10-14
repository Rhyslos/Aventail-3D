//! Version 10 of the project file's contents ([decision 0052](../../../../docs/decisions/0052-clean-up.md)):
//! version 9's, with the clean-up edits: delete, dissolve, fill holes,
//! flip polygons and fix facing.
//!
//! Version 9's operations are stored inside version 10's as they were
//! (`OperationV10::V9`); the entry around each operation is version 5's.

use super::v1::{FileOperation, Problem};
use super::v2::{AnchorsV2, anchors_from_file, anchors_to_file};
use super::v5::{EntryWith, ProjectWith};
use super::v9::OperationV9;
use crate::limits::Bounded;
use mesh::{EdgeKey, FaceId, VertexId};
use scene::Operation;
use serde::{Deserialize, Serialize};

/// Version 10 of the file.
pub(crate) type ProjectV10 = ProjectWith<EntryV10>;

/// A recorded edit in version 10.
pub(crate) type EntryV10 = EntryWith<OperationV10>;

/// Points, edges and polygons named in a clean-up edit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ElementsV10 {
    points: Bounded<u32>,
    edges: Bounded<[u32; 2]>,
    faces: Bounded<u32>,
}

/// The recorded operations version 10 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV10 {
    /// Everything version 9 could store, stored the same way.
    V9(OperationV9),
    DeleteElements {
        elements: ElementsV10,
        anchors: Option<AnchorsV2>,
    },
    Dissolve {
        elements: ElementsV10,
        anchors: Option<AnchorsV2>,
    },
    FillHoles {
        points: Bounded<u32>,
        anchors: Option<AnchorsV2>,
    },
    FlipPolygons {
        faces: Bounded<u32>,
        anchors: Option<AnchorsV2>,
    },
    FixFacing,
}

type Elements = (Vec<VertexId>, Vec<EdgeKey>, Vec<FaceId>);

fn to_file(points: &[VertexId], edges: &[EdgeKey], faces: &[FaceId]) -> ElementsV10 {
    ElementsV10 {
        points: points.iter().map(|v| v.get()).collect(),
        edges: edges
            .iter()
            .map(|e| {
                let (a, b) = e.ends();
                [a.get(), b.get()]
            })
            .collect(),
        faces: faces.iter().map(|f| f.get()).collect(),
    }
}

fn from_file(e: ElementsV10) -> Result<Elements, Problem> {
    let edges = e
        .edges
        .into_iter()
        .map(|[a, b]| {
            if a == b {
                Err("one of its edges joins a point to itself".to_string())
            } else {
                Ok(EdgeKey::new(VertexId::from_raw(a), VertexId::from_raw(b)))
            }
        })
        .collect::<Result<_, _>>()?;
    Ok((
        e.points.into_iter().map(VertexId::from_raw).collect(),
        edges,
        e.faces.into_iter().map(FaceId::from_raw).collect(),
    ))
}

impl FileOperation for OperationV10 {
    fn to_file(op: &Operation) -> Option<Self> {
        let anchors = op.anchors().map(anchors_to_file);
        Some(match op {
            Operation::DeleteElements {
                points,
                edges,
                faces,
                ..
            } => Self::DeleteElements {
                elements: to_file(points, edges, faces),
                anchors,
            },
            Operation::Dissolve {
                points,
                edges,
                faces,
                ..
            } => Self::Dissolve {
                elements: to_file(points, edges, faces),
                anchors,
            },
            Operation::FillHoles { points, .. } => Self::FillHoles {
                points: points.iter().map(|v| v.get()).collect(),
                anchors,
            },
            Operation::FlipPolygons { faces, .. } => Self::FlipPolygons {
                faces: faces.iter().map(|f| f.get()).collect(),
                anchors,
            },
            Operation::FixFacing => Self::FixFacing,
            _ => Self::V9(OperationV9::to_file(op)?),
        })
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        let anchors = |a: Option<AnchorsV2>| a.map(anchors_from_file).transpose();
        match self {
            Self::V9(op) => op.into_operation(),
            Self::DeleteElements {
                elements,
                anchors: a,
            } => {
                let (points, edges, faces) = from_file(elements)?;
                Ok(Operation::DeleteElements {
                    points,
                    edges,
                    faces,
                    anchors: anchors(a)?,
                })
            }
            Self::Dissolve {
                elements,
                anchors: a,
            } => {
                let (points, edges, faces) = from_file(elements)?;
                Ok(Operation::Dissolve {
                    points,
                    edges,
                    faces,
                    anchors: anchors(a)?,
                })
            }
            Self::FillHoles { points, anchors: a } => Ok(Operation::FillHoles {
                points: points.into_iter().map(VertexId::from_raw).collect(),
                anchors: anchors(a)?,
            }),
            Self::FlipPolygons { faces, anchors: a } => Ok(Operation::FlipPolygons {
                faces: faces.into_iter().map(FaceId::from_raw).collect(),
                anchors: anchors(a)?,
            }),
            Self::FixFacing => Ok(Operation::FixFacing),
        }
    }
}
