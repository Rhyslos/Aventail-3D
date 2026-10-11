//! Version 2 of the project file's contents (decision
//! [0031](../../../../docs/decisions/0031-project-format-version-2.md)):
//! version 1's structures, with one more kind of recorded operation,
//! turning or scaling points with a matrix (`TransformPoints`). Version 1
//! files are read with version 1's model and need no conversion: the
//! scene they describe is the same.

use super::v1::{FileOperation, Problem, ProjectFile, finite};
use glam::Mat4;
use mesh::VertexId;
use scene::Operation;
use serde::{Deserialize, Serialize};

/// Version 2 of the file.
pub(crate) type ProjectV2 = ProjectFile<OperationV2>;

/// The recorded operations version 2 stores. New kinds go at the end, in
/// a later version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV2 {
    /// As in version 1.
    MovePoints { points: Vec<u32>, offset: [f32; 3] },
    /// Points moved by an affine matrix, in the object's own space: its
    /// first three columns (each three numbers), then the move.
    TransformPoints { points: Vec<u32>, matrix: [f32; 12] },
}

impl FileOperation for OperationV2 {
    fn to_file(op: &Operation) -> Option<Self> {
        let ids = |points: &[VertexId]| points.iter().map(|v| v.get()).collect();
        Some(match op {
            Operation::MovePoints { points, offset } => Self::MovePoints {
                points: ids(points),
                offset: offset.to_array(),
            },
            Operation::TransformPoints { points, matrix } => {
                let mut numbers = [0.0; 12];
                for (column, chunk) in numbers.chunks_mut(3).enumerate() {
                    chunk.copy_from_slice(&matrix.col(column).truncate().to_array());
                }
                Self::TransformPoints {
                    points: ids(points),
                    matrix: numbers,
                }
            }
            // Operations added later need a later format version.
            _ => return None,
        })
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        let ids = |points: Vec<u32>| points.into_iter().map(VertexId::from_raw).collect();
        match self {
            Self::MovePoints { points, offset } => Ok(Operation::MovePoints {
                points: ids(points),
                offset: finite(offset, "recorded move")?,
            }),
            Self::TransformPoints { points, matrix } => {
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
                })
            }
        }
    }
}
