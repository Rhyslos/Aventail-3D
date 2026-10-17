//! Version 30 of the project file's contents ([decision 0099](../../../../docs/decisions/0099-unwrap-relax-straighten.md)):
//! version 29's, with UVs unwrapped, straightened and relaxed.
//!
//! Three more kinds of recorded edit, each naming polygons (and their
//! anchors): unwrapping (with whether strips come out straight),
//! straightening and relaxing. Everything else is stored as in version 29.

use super::v1::{FileOperation, Problem};
use super::v2::{AnchorsV2, anchors_from_file, anchors_to_file};
use super::v5::EntryWith;
use super::v29::OperationV29;
use crate::limits::Bounded;
use scene::{FaceId, Operation};
use serde::{Deserialize, Serialize};

/// A recorded edit in version 30.
pub(crate) type EntryV30 = EntryWith<OperationV30>;

/// Version 30 of the file: version 27's, its edits stored as version 30's.
#[cfg(test)]
pub(crate) type ProjectV30 = super::v27::ProjectV27<EntryV30>;

/// The recorded operations version 30 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV30 {
    /// Everything version 29 could store, stored the same way.
    V29(OperationV29),
    /// Pieces unwrapped.
    Unwrap {
        faces: Bounded<u32>,
        strips: bool,
        anchors: Option<AnchorsV2>,
    },
    /// Quad pieces straightened.
    StraightenUvs {
        faces: Bounded<u32>,
        anchors: Option<AnchorsV2>,
    },
    /// UVs relaxed.
    RelaxUvs {
        faces: Bounded<u32>,
        anchors: Option<AnchorsV2>,
    },
}

fn ids(faces: &[FaceId]) -> Bounded<u32> {
    faces.iter().map(|f| f.get()).collect()
}

fn faces(ids: Bounded<u32>) -> Vec<FaceId> {
    ids.into_iter().map(FaceId::from_raw).collect()
}

impl FileOperation for OperationV30 {
    fn to_file(op: &Operation) -> Option<Self> {
        Some(match op {
            Operation::Unwrap {
                faces,
                strips,
                anchors,
            } => Self::Unwrap {
                faces: ids(faces),
                strips: *strips,
                anchors: anchors.as_ref().map(anchors_to_file),
            },
            Operation::StraightenUvs { faces, anchors } => Self::StraightenUvs {
                faces: ids(faces),
                anchors: anchors.as_ref().map(anchors_to_file),
            },
            Operation::RelaxUvs { faces, anchors } => Self::RelaxUvs {
                faces: ids(faces),
                anchors: anchors.as_ref().map(anchors_to_file),
            },
            _ => Self::V29(OperationV29::to_file(op)?),
        })
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        Ok(match self {
            Self::V29(op) => op.into_operation()?,
            Self::Unwrap {
                faces: list,
                strips,
                anchors,
            } => Operation::Unwrap {
                faces: faces(list),
                strips,
                anchors: anchors.map(anchors_from_file).transpose()?,
            },
            Self::StraightenUvs {
                faces: list,
                anchors,
            } => Operation::StraightenUvs {
                faces: faces(list),
                anchors: anchors.map(anchors_from_file).transpose()?,
            },
            Self::RelaxUvs {
                faces: list,
                anchors,
            } => Operation::RelaxUvs {
                faces: faces(list),
                anchors: anchors.map(anchors_from_file).transpose()?,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unwrapping_straightening_and_relaxing_survive_the_trip() {
        let some = vec![FaceId::from_raw(1), FaceId::from_raw(4)];
        for op in [
            Operation::Unwrap {
                faces: some.clone(),
                strips: true,
                anchors: None,
            },
            Operation::Unwrap {
                faces: some.clone(),
                strips: false,
                anchors: None,
            },
            Operation::StraightenUvs {
                faces: some.clone(),
                anchors: None,
            },
            Operation::RelaxUvs {
                faces: some.clone(),
                anchors: None,
            },
        ] {
            let file = OperationV30::to_file(&op).unwrap();
            assert!(!matches!(file, OperationV30::V29(_)));
            assert_eq!(file.into_operation().unwrap(), op);
            assert!(OperationV29::to_file(&op).is_none(), "older versions can't");
        }
    }
}
