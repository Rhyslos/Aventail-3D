//! Version 3 of the project file's contents ([decision 0038](../../../../docs/decisions/0038-extrude-and-inset.md)):
//! version 2's structures, with recorded operations that extrude and
//! inset polygons and extrude edges. Versions 1 and 2 are read with their
//! own models and need no conversion: the scene they describe is the
//! same.
//!
//! Version 2's operations are stored inside version 3's as they were
//! (`OperationV3::V2`), so their checks are shared; new kinds go at the
//! end, in a later version.

#[cfg(test)]
use super::v1::ProjectFile;
use super::v1::{FileOperation, Problem};
use super::v2::{AnchorsV2, OperationV2, anchors_from_file, anchors_to_file};
use crate::limits::Bounded;
use mesh::{EdgeKey, FaceId, VertexId};
use scene::{Grouping, Operation};
use serde::{Deserialize, Serialize};

/// Version 3 of the file (written by tests; the app reads version 3
/// files through `decode`).
#[cfg(test)]
pub(crate) type ProjectV3 = ProjectFile<OperationV3>;

/// The recorded operations version 3 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV3 {
    /// Everything version 2 could store, stored the same way.
    V2(OperationV2),
    /// Polygons extruded.
    ExtrudePolygons {
        faces: Bounded<u32>,
        distance: f32,
        /// Each polygon on its own (`true`) or neighbours together.
        separate: bool,
        anchors: Option<AnchorsV2>,
    },
    /// Polygons inset.
    InsetPolygons {
        faces: Bounded<u32>,
        distance: f32,
        separate: bool,
        anchors: Option<AnchorsV2>,
    },
    /// Border edges extruded, each by its two points.
    ExtrudeEdges {
        edges: Bounded<[u32; 2]>,
        distance: f32,
        separate: bool,
        anchors: Option<AnchorsV2>,
    },
}

fn separate(grouping: Grouping) -> bool {
    grouping == Grouping::Separate
}

fn grouping(separate: bool) -> Grouping {
    if separate {
        Grouping::Separate
    } else {
        Grouping::Together
    }
}

fn distance(value: f32, what: &str) -> Result<f32, Problem> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!("its {what} distance isn't a real number"))
    }
}

impl FileOperation for OperationV3 {
    fn to_file(op: &Operation) -> Option<Self> {
        let faces = |faces: &[FaceId]| faces.iter().map(|f| f.get()).collect();
        let anchors = |a: Option<&mesh::Anchors>| a.map(anchors_to_file);
        Some(match op {
            Operation::ExtrudePolygons {
                faces: f,
                distance,
                grouping,
                ..
            } => Self::ExtrudePolygons {
                faces: faces(f),
                distance: *distance,
                separate: separate(*grouping),
                anchors: anchors(op.anchors()),
            },
            Operation::InsetPolygons {
                faces: f,
                distance,
                grouping,
                ..
            } => Self::InsetPolygons {
                faces: faces(f),
                distance: *distance,
                separate: separate(*grouping),
                anchors: anchors(op.anchors()),
            },
            Operation::ExtrudeEdges {
                edges,
                distance,
                grouping,
                ..
            } => Self::ExtrudeEdges {
                edges: edges
                    .iter()
                    .map(|e| {
                        let (a, b) = e.ends();
                        [a.get(), b.get()]
                    })
                    .collect(),
                distance: *distance,
                separate: separate(*grouping),
                anchors: anchors(op.anchors()),
            },
            _ => Self::V2(OperationV2::to_file(op)?),
        })
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        let faces = |faces: Bounded<u32>| faces.into_iter().map(FaceId::from_raw).collect();
        let anchors = |a: Option<AnchorsV2>| a.map(anchors_from_file).transpose();
        match self {
            Self::V2(op) => op.into_operation(),
            Self::ExtrudePolygons {
                faces: f,
                distance: d,
                separate,
                anchors: a,
            } => Ok(Operation::ExtrudePolygons {
                faces: faces(f),
                distance: distance(d, "extrude")?,
                grouping: grouping(separate),
                anchors: anchors(a)?,
            }),
            Self::InsetPolygons {
                faces: f,
                distance: d,
                separate,
                anchors: a,
            } => Ok(Operation::InsetPolygons {
                faces: faces(f),
                distance: distance(d, "inset")?,
                grouping: grouping(separate),
                anchors: anchors(a)?,
            }),
            Self::ExtrudeEdges {
                edges,
                distance: d,
                separate,
                anchors: a,
            } => Ok(Operation::ExtrudeEdges {
                edges: edges
                    .into_iter()
                    .map(|[a, b]| {
                        if a == b {
                            Err("one of its extruded edges joins a point to itself".to_string())
                        } else {
                            Ok(EdgeKey::new(VertexId::from_raw(a), VertexId::from_raw(b)))
                        }
                    })
                    .collect::<Result<_, _>>()?,
                distance: distance(d, "extrude")?,
                grouping: grouping(separate),
                anchors: anchors(a)?,
            }),
        }
    }
}
