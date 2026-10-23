//! Version 37 of the project file's contents ([decision 0112](../../../../docs/decisions/0112-locked-pieces-and-piece-settings.md)):
//! version 36's, with locked pieces and pieces' own unwrap settings.
//!
//! Two more kinds of recorded edit: polygons' UVs locked or unlocked, and
//! polygons given their pieces' own unwrap settings (a way of laying flat,
//! straightening, a texel density; each or none). Everything else is
//! stored as in version 36.

use super::v1::{FileOperation, Problem};
use super::v2::{AnchorsV2, anchors_from_file, anchors_to_file};
use super::v5::EntryWith;
use super::v34::{MethodV34, method_from_file, method_to_file};
use super::v36::OperationV36;
use crate::limits::Bounded;
use scene::{FaceId, Operation, PieceSettings};
use serde::{Deserialize, Serialize};
use words::tr;

/// A recorded edit in version 37.
pub(crate) type EntryV37 = EntryWith<OperationV37>;

/// The recorded operations version 37 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV37 {
    /// Everything version 36 could store, stored the same way.
    V36(OperationV36),
    /// Polygons' UVs locked or unlocked.
    SetUvLock {
        faces: Bounded<u32>,
        lock: bool,
        anchors: Option<AnchorsV2>,
    },
    /// Polygons given their pieces' own unwrap settings.
    SetPieceSettings {
        faces: Bounded<u32>,
        method: Option<MethodV34>,
        strips: Option<bool>,
        density: Option<f32>,
        anchors: Option<AnchorsV2>,
    },
}

impl FileOperation for OperationV37 {
    fn to_file(op: &Operation) -> Option<Self> {
        match op {
            Operation::SetUvLock {
                faces,
                lock,
                anchors,
            } => Some(Self::SetUvLock {
                faces: faces.iter().map(|f| f.get()).collect(),
                lock: *lock,
                anchors: anchors.as_ref().map(anchors_to_file),
            }),
            Operation::SetPieceSettings {
                faces,
                settings,
                anchors,
            } => Some(Self::SetPieceSettings {
                faces: faces.iter().map(|f| f.get()).collect(),
                method: settings.method.map(method_to_file),
                strips: settings.strips,
                density: settings.density,
                anchors: anchors.as_ref().map(anchors_to_file),
            }),
            _ => Some(Self::V36(OperationV36::to_file(op)?)),
        }
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        let operation = match self {
            Self::V36(op) => return op.into_operation(),
            Self::SetUvLock {
                faces,
                lock,
                anchors,
            } => Operation::SetUvLock {
                faces: faces.into_iter().map(FaceId::from_raw).collect(),
                lock,
                anchors: anchors.map(anchors_from_file).transpose()?,
            },
            Self::SetPieceSettings {
                faces,
                method,
                strips,
                density,
                anchors,
            } => Operation::SetPieceSettings {
                faces: faces.into_iter().map(FaceId::from_raw).collect(),
                settings: PieceSettings {
                    method: method.map(method_from_file),
                    strips,
                    density,
                },
                anchors: anchors.map(anchors_from_file).transpose()?,
            },
        };
        if !operation.numbers_are_real() {
            return Err(tr("a piece's texel density isn't a real number above 0").into());
        }
        Ok(operation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scene::UnwrapMethod;

    #[test]
    fn locks_and_piece_settings_survive_the_trip() {
        let faces = vec![FaceId::from_raw(1), FaceId::from_raw(4)];
        let ops = [
            Operation::SetUvLock {
                faces: faces.clone(),
                lock: true,
                anchors: None,
            },
            Operation::SetUvLock {
                faces: faces.clone(),
                lock: false,
                anchors: None,
            },
            Operation::SetPieceSettings {
                faces: faces.clone(),
                settings: PieceSettings {
                    method: Some(UnwrapMethod::AngleBased),
                    strips: Some(false),
                    density: Some(0.05),
                },
                anchors: None,
            },
            Operation::SetPieceSettings {
                faces,
                settings: PieceSettings::default(),
                anchors: None,
            },
        ];
        for op in ops {
            let file = OperationV37::to_file(&op).unwrap();
            assert!(!matches!(file, OperationV37::V36(_)), "{op:?}");
            assert_eq!(file.into_operation().unwrap(), op);
            assert!(OperationV36::to_file(&op).is_none(), "older versions can't");
        }
    }

    #[test]
    fn a_density_that_isnt_real_is_refused() {
        for density in [0.0, -1.0, f32::NAN] {
            let bad = OperationV37::SetPieceSettings {
                faces: Bounded::default(),
                method: None,
                strips: None,
                density: Some(density),
                anchors: None,
            };
            assert!(bad.into_operation().is_err(), "{density}");
        }
    }
}
