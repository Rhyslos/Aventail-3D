//! Version 26 of the project file's contents ([decision 0089](../../../../docs/decisions/0089-apply-transforms.md)):
//! version 25's, with applied placements.
//!
//! One more kind of recorded edit: placing the whole shape by a matrix,
//! which applying an object's position, rotation or scale records.
//! Everything else is stored as in version 25 (its edits as version 24's).

use super::v1::{FileOperation, Problem};
use super::v5::EntryWith;
use super::v24::OperationV24;

use glam::Mat4;
use scene::Operation;
use serde::{Deserialize, Serialize};

/// Version 26 of the file: version 25's, its edits stored as version
/// 26's.
#[cfg(test)]
pub(crate) type ProjectV26 = super::v25::ProjectV25<EntryV26>;

/// A recorded edit in version 26.
pub(crate) type EntryV26 = EntryWith<OperationV26>;

/// The recorded operations version 26 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV26 {
    /// Everything version 24 could store, stored the same way.
    V24(OperationV24),
    /// The whole shape placed by a matrix (its 16 numbers, column by
    /// column).
    PlaceShape { matrix: [f32; 16] },
}

impl FileOperation for OperationV26 {
    fn to_file(op: &Operation) -> Option<Self> {
        if let Operation::PlaceShape { matrix } = op {
            return Some(Self::PlaceShape {
                matrix: matrix.to_cols_array(),
            });
        }
        Some(Self::V24(OperationV24::to_file(op)?))
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        match self {
            Self::V24(op) => op.into_operation(),
            Self::PlaceShape { matrix } => {
                let operation = Operation::PlaceShape {
                    matrix: Mat4::from_cols_array(&matrix),
                };
                if !operation.numbers_are_real() {
                    return Err(
                        "an applied placement isn't made of real numbers, or flattens the shape"
                            .into(),
                    );
                }
                Ok(operation)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_applied_placement_survives_the_trip_and_broken_ones_are_refused() {
        let op = Operation::PlaceShape {
            matrix: Mat4::from_scale_rotation_translation(
                glam::Vec3::new(2.0, -1.0, 0.5),
                glam::Quat::from_rotation_z(0.4),
                glam::Vec3::new(1.0, 2.0, 3.0),
            ),
        };
        let file = OperationV26::to_file(&op).unwrap();
        assert!(matches!(file, OperationV26::PlaceShape { .. }));
        assert_eq!(file.into_operation().unwrap(), op);
        assert!(OperationV24::to_file(&op).is_none(), "older versions can't");
        for broken in [[f32::NAN; 16], [0.0; 16]] {
            assert!(
                OperationV26::PlaceShape { matrix: broken }
                    .into_operation()
                    .is_err()
            );
        }
    }
}
