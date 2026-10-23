//! Version 32 of the project file's contents ([decision 0101](../../../../docs/decisions/0101-packing.md)):
//! version 31's, with packed UVs.
//!
//! One more kind of recorded edit: islands moved to where packing put
//! them, each its polygons and its move. Everything else is stored as in
//! version 31.

use super::v1::{FileOperation, Problem};
use super::v5::EntryWith;
use super::v31::OperationV31;
use crate::limits::Bounded;
use scene::{FaceId, Operation};
use serde::{Deserialize, Serialize};
use words::tr;

/// A recorded edit in version 32.
pub(crate) type EntryV32 = EntryWith<OperationV32>;

/// The recorded operations version 32 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV32 {
    /// Everything version 31 could store, stored the same way.
    V31(OperationV31),
    /// Islands packed: each its polygons and its move (a 2D affine
    /// transform's six numbers, column by column).
    PackUvs { islands: Bounded<IslandV32> },
}

/// One packed island.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct IslandV32 {
    faces: Bounded<u32>,
    transform: [f32; 6],
}

impl FileOperation for OperationV32 {
    fn to_file(op: &Operation) -> Option<Self> {
        if let Operation::PackUvs { islands } = op {
            return Some(Self::PackUvs {
                islands: islands
                    .iter()
                    .map(|(faces, t)| IslandV32 {
                        faces: faces.iter().map(|f| f.get()).collect(),
                        transform: t.to_cols_array(),
                    })
                    .collect(),
            });
        }
        Some(Self::V31(OperationV31::to_file(op)?))
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        match self {
            Self::V31(op) => op.into_operation(),
            Self::PackUvs { islands } => {
                let operation = Operation::PackUvs {
                    islands: islands
                        .into_iter()
                        .map(|i| {
                            (
                                i.faces.into_iter().map(FaceId::from_raw).collect(),
                                glam::Affine2::from_cols_array(&i.transform),
                            )
                        })
                        .collect(),
                };
                if !operation.numbers_are_real() {
                    return Err(tr("an island is moved by numbers that aren't real").into());
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
    fn packed_islands_survive_the_trip_and_broken_ones_are_refused() {
        let op = Operation::PackUvs {
            islands: vec![
                (
                    vec![FaceId::from_raw(1), FaceId::from_raw(2)],
                    glam::Affine2::from_scale_angle_translation(
                        glam::Vec2::splat(0.3),
                        std::f32::consts::FRAC_PI_2,
                        glam::Vec2::new(0.2, 0.4),
                    ),
                ),
                (vec![FaceId::from_raw(5)], glam::Affine2::IDENTITY),
            ],
        };
        let file = OperationV32::to_file(&op).unwrap();
        assert!(matches!(file, OperationV32::PackUvs { .. }));
        assert_eq!(file.into_operation().unwrap(), op);
        assert!(OperationV31::to_file(&op).is_none(), "older versions can't");
        let broken = OperationV32::PackUvs {
            islands: vec![IslandV32 {
                faces: Bounded::default(),
                transform: [f32::INFINITY; 6],
            }]
            .into_iter()
            .collect(),
        };
        assert!(broken.into_operation().is_err());
    }
}
