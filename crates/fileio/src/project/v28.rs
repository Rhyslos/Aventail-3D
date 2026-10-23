//! Version 28 of the project file's contents ([decision 0097](../../../../docs/decisions/0097-editing-uvs-by-hand.md)):
//! version 27's, with UVs moved by hand.
//!
//! One more kind of recorded edit: moving, turning or scaling the UVs of
//! some polygons or points. Everything else is stored as in version 27
//! (its edits as version 26's).

use super::v1::{FileOperation, Problem};
use super::v2::{AnchorsV2, anchors_from_file, anchors_to_file};
use super::v5::EntryWith;
use super::v26::OperationV26;
use crate::limits::Bounded;
use scene::{FaceId, Operation, VertexId};
use serde::{Deserialize, Serialize};
use words::tr;

/// A recorded edit in version 28.
pub(crate) type EntryV28 = EntryWith<OperationV28>;

/// Version 28 of the file: version 27's, its edits stored as version 28's.
#[cfg(test)]
pub(crate) type ProjectV28 = super::v27::ProjectV27<EntryV28>;

/// The recorded operations version 28 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV28 {
    /// Everything version 26 could store, stored the same way.
    V26(OperationV26),
    /// UVs moved by hand: the polygons and points, the change (a 2D
    /// affine transform, its six numbers column by column) and anchors.
    TransformUvs {
        faces: Bounded<u32>,
        points: Bounded<u32>,
        transform: [f32; 6],
        anchors: Option<AnchorsV2>,
    },
}

impl FileOperation for OperationV28 {
    fn to_file(op: &Operation) -> Option<Self> {
        if let Operation::TransformUvs {
            faces,
            points,
            transform,
            anchors,
        } = op
        {
            return Some(Self::TransformUvs {
                faces: faces.iter().map(|f| f.get()).collect(),
                points: points.iter().map(|v| v.get()).collect(),
                transform: transform.to_cols_array(),
                anchors: anchors.as_ref().map(anchors_to_file),
            });
        }
        Some(Self::V26(OperationV26::to_file(op)?))
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        match self {
            Self::V26(op) => op.into_operation(),
            Self::TransformUvs {
                faces,
                points,
                transform,
                anchors,
            } => {
                let operation = Operation::TransformUvs {
                    faces: faces.into_iter().map(FaceId::from_raw).collect(),
                    points: points.into_iter().map(VertexId::from_raw).collect(),
                    transform: glam::Affine2::from_cols_array(&transform),
                    anchors: anchors.map(anchors_from_file).transpose()?,
                };
                if !operation.numbers_are_real() {
                    return Err(tr("UVs are moved by numbers that aren't real").into());
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
    fn moved_uvs_survive_the_trip_and_broken_ones_are_refused() {
        let op = Operation::TransformUvs {
            faces: vec![FaceId::from_raw(3), FaceId::from_raw(7)],
            points: vec![VertexId::from_raw(2)],
            transform: glam::Affine2::from_scale_angle_translation(
                glam::Vec2::new(0.5, 2.0),
                0.3,
                glam::Vec2::new(0.1, -0.2),
            ),
            anchors: None,
        };
        let file = OperationV28::to_file(&op).unwrap();
        assert!(matches!(file, OperationV28::TransformUvs { .. }));
        assert_eq!(file.into_operation().unwrap(), op);
        assert!(OperationV26::to_file(&op).is_none(), "older versions can't");
        let broken = OperationV28::TransformUvs {
            faces: Bounded::default(),
            points: Bounded::default(),
            transform: [f32::NAN; 6],
            anchors: None,
        };
        assert!(broken.into_operation().is_err());
    }
}
