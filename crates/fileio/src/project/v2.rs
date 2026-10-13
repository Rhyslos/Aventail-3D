//! Version 2 of the project file's contents (decisions
//! [0031](../../../../docs/decisions/0031-project-format-version-2.md) and
//! [0032](../../../../docs/decisions/0032-edits-find-their-points-again.md)):
//! version 1's structures, with recorded operations that can turn and
//! scale points with a matrix (`TransformPoints`) and that keep where
//! their points were (anchors), to find them again if the shape's points
//! are renumbered. Version 1 files are read with version 1's model and
//! need no conversion: the scene they describe is the same.

#[cfg(test)]
use super::v1::ProjectFile;
use super::v1::{FileOperation, Problem, finite};
use glam::{Mat4, Vec3};
use mesh::{AnchorShape, Anchors, Bounds, VertexId};
use scene::Operation;
use serde::{Deserialize, Serialize};

/// Version 2 of the file (written by tests; the app reads version 2 files
/// through `decode`).
#[cfg(test)]
pub(crate) type ProjectV2 = ProjectFile<OperationV2>;

/// The recorded operations version 2 stores. New kinds go at the end, in
/// a later version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV2 {
    /// Points moved by an offset.
    MovePoints {
        points: Vec<u32>,
        offset: [f32; 3],
        anchors: Option<AnchorsV2>,
    },
    /// Points moved by an affine matrix, in the object's own space: its
    /// first three columns (each three numbers), then the move.
    TransformPoints {
        points: Vec<u32>,
        matrix: [f32; 12],
        anchors: Option<AnchorsV2>,
    },
}

/// Where an operation's elements were (see `mesh::anchors`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct AnchorsV2 {
    /// The shape's box then: its smallest and largest corners.
    pub(crate) bounds: [[f32; 3]; 2],
    /// Each point named, and where it was.
    pub(crate) points: Vec<(u32, [f32; 3])>,
    pub(crate) shape: AnchorShapeV2,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum AnchorShapeV2 {
    Points(Vec<[f32; 3]>),
    Edges(Vec<[[f32; 3]; 2]>),
    Polygons(Vec<Vec<[f32; 3]>>),
}

pub(crate) fn anchors_to_file(a: &Anchors) -> AnchorsV2 {
    AnchorsV2 {
        bounds: [a.bounds.min.to_array(), a.bounds.max.to_array()],
        points: a
            .points
            .iter()
            .map(|(v, p)| (v.get(), p.to_array()))
            .collect(),
        shape: match &a.shape {
            AnchorShape::Points(p) => {
                AnchorShapeV2::Points(p.iter().map(|v| v.to_array()).collect())
            }
            AnchorShape::Edges(e) => AnchorShapeV2::Edges(
                e.iter()
                    .map(|[a, b]| [a.to_array(), b.to_array()])
                    .collect(),
            ),
            AnchorShape::Polygons(p) => AnchorShapeV2::Polygons(
                p.iter()
                    .map(|c| c.iter().map(|v| v.to_array()).collect())
                    .collect(),
            ),
        },
    }
}

pub(crate) fn anchors_from_file(a: AnchorsV2) -> Result<Anchors, Problem> {
    let place = |p: [f32; 3]| finite(p, "recorded place");
    let bounds = Bounds::new(place(a.bounds[0])?, place(a.bounds[1])?)
        .ok_or_else(|| "its recorded box isn't a real box".to_string())?;
    let points = a
        .points
        .into_iter()
        .map(|(v, p)| Ok((VertexId::from_raw(v), place(p)?)))
        .collect::<Result<Vec<_>, Problem>>()?;
    let shape = match a.shape {
        AnchorShapeV2::Points(p) => {
            AnchorShape::Points(p.into_iter().map(place).collect::<Result<_, _>>()?)
        }
        AnchorShapeV2::Edges(e) => AnchorShape::Edges(
            e.into_iter()
                .map(|[a, b]| Ok([place(a)?, place(b)?]))
                .collect::<Result<_, Problem>>()?,
        ),
        AnchorShapeV2::Polygons(p) => AnchorShape::Polygons(
            p.into_iter()
                .map(|c| c.into_iter().map(place).collect::<Result<Vec<Vec3>, _>>())
                .collect::<Result<_, _>>()?,
        ),
    };
    Ok(Anchors {
        bounds,
        points,
        shape,
        // Kept from format version 5 on.
        structure: None,
        reach: Vec::new(),
    })
}

impl FileOperation for OperationV2 {
    fn to_file(op: &Operation) -> Option<Self> {
        let ids = |points: &[VertexId]| points.iter().map(|v| v.get()).collect();
        let anchors = |a: &Option<Anchors>| a.as_ref().map(anchors_to_file);
        Some(match op {
            Operation::MovePoints {
                points,
                offset,
                anchors: a,
            } => Self::MovePoints {
                points: ids(points),
                offset: offset.to_array(),
                anchors: anchors(a),
            },
            Operation::TransformPoints {
                points,
                matrix,
                anchors: a,
            } => {
                let mut numbers = [0.0; 12];
                for (column, chunk) in numbers.chunks_mut(3).enumerate() {
                    chunk.copy_from_slice(&matrix.col(column).truncate().to_array());
                }
                Self::TransformPoints {
                    points: ids(points),
                    matrix: numbers,
                    anchors: anchors(a),
                }
            }
            // Operations added later need a later format version.
            _ => return None,
        })
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        let ids = |points: Vec<u32>| points.into_iter().map(VertexId::from_raw).collect();
        let anchors = |a: Option<AnchorsV2>| a.map(anchors_from_file).transpose();
        match self {
            Self::MovePoints {
                points,
                offset,
                anchors: a,
            } => Ok(Operation::MovePoints {
                points: ids(points),
                offset: finite(offset, "recorded move")?,
                anchors: anchors(a)?,
            }),
            Self::TransformPoints {
                points,
                matrix,
                anchors: a,
            } => {
                let column = |i: usize| {
                    finite(
                        [matrix[i * 3], matrix[i * 3 + 1], matrix[i * 3 + 2]],
                        "recorded turn or scale",
                    )
                };
                let built = Mat4::from_cols(
                    column(0)?.extend(0.0),
                    column(1)?.extend(0.0),
                    column(2)?.extend(0.0),
                    column(3)?.extend(1.0),
                );
                Ok(Operation::TransformPoints {
                    points: ids(points),
                    matrix: built,
                    anchors: anchors(a)?,
                })
            }
        }
    }
}
