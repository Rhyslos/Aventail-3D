//! Version 34 of the project file's contents ([decision 0109](../../../../docs/decisions/0109-pins-and-unwrap-methods.md)):
//! version 33's, with pinned UVs and unwraps that choose their way.
//!
//! Two more kinds of recorded edit: UVs pinned or unpinned (polygons and
//! points, and their anchors), and an unwrap that says how it lays pieces
//! flat (keep shapes, angle-based, keep sizes or automatic) and whether it
//! unfolds one way only. Plain unwraps are still stored as version 30
//! stored them. Everything else is stored as in version 33.

use super::v1::{FileOperation, Problem};
use super::v2::{AnchorsV2, anchors_from_file, anchors_to_file};
use super::v5::EntryWith;
use super::v33::OperationV33;
use crate::limits::Bounded;
use scene::{FaceId, Operation, UnfoldAxis, UnwrapMethod, VertexId};
use serde::{Deserialize, Serialize};

/// A recorded edit in version 34.
pub(crate) type EntryV34 = EntryWith<OperationV34>;

/// The ways of laying flat, as stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum MethodV34 {
    Conformal,
    AngleBased,
    SizeKeeping,
    Automatic,
}

/// Unfolding one way, as stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum AxisV34 {
    Across,
    Up,
}

/// The recorded operations version 34 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV34 {
    /// Everything version 33 could store, stored the same way.
    V33(OperationV33),
    /// UVs pinned or unpinned.
    SetUvPins {
        faces: Bounded<u32>,
        points: Bounded<u32>,
        pin: bool,
        anchors: Option<AnchorsV2>,
    },
    /// Pieces unwrapped a chosen way.
    Unwrap {
        faces: Bounded<u32>,
        strips: bool,
        method: MethodV34,
        only: Option<AxisV34>,
        anchors: Option<AnchorsV2>,
    },
}

fn method_to_file(method: UnwrapMethod) -> MethodV34 {
    match method {
        UnwrapMethod::Conformal => MethodV34::Conformal,
        UnwrapMethod::AngleBased => MethodV34::AngleBased,
        UnwrapMethod::SizeKeeping => MethodV34::SizeKeeping,
        UnwrapMethod::Automatic => MethodV34::Automatic,
    }
}

fn method_from_file(method: MethodV34) -> UnwrapMethod {
    match method {
        MethodV34::Conformal => UnwrapMethod::Conformal,
        MethodV34::AngleBased => UnwrapMethod::AngleBased,
        MethodV34::SizeKeeping => UnwrapMethod::SizeKeeping,
        MethodV34::Automatic => UnwrapMethod::Automatic,
    }
}

fn axis_to_file(axis: UnfoldAxis) -> AxisV34 {
    match axis {
        UnfoldAxis::Across => AxisV34::Across,
        UnfoldAxis::Up => AxisV34::Up,
    }
}

fn axis_from_file(axis: AxisV34) -> UnfoldAxis {
    match axis {
        AxisV34::Across => UnfoldAxis::Across,
        AxisV34::Up => UnfoldAxis::Up,
    }
}

impl FileOperation for OperationV34 {
    fn to_file(op: &Operation) -> Option<Self> {
        match op {
            Operation::SetUvPins {
                faces,
                points,
                pin,
                anchors,
            } => Some(Self::SetUvPins {
                faces: faces.iter().map(|f| f.get()).collect(),
                points: points.iter().map(|v| v.get()).collect(),
                pin: *pin,
                anchors: anchors.as_ref().map(anchors_to_file),
            }),
            Operation::Unwrap {
                faces,
                strips,
                method,
                only,
                anchors,
            } if *method != UnwrapMethod::Conformal || only.is_some() => Some(Self::Unwrap {
                faces: faces.iter().map(|f| f.get()).collect(),
                strips: *strips,
                method: method_to_file(*method),
                only: only.map(axis_to_file),
                anchors: anchors.as_ref().map(anchors_to_file),
            }),
            _ => Some(Self::V33(OperationV33::to_file(op)?)),
        }
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        Ok(match self {
            Self::V33(op) => op.into_operation()?,
            Self::SetUvPins {
                faces,
                points,
                pin,
                anchors,
            } => Operation::SetUvPins {
                faces: faces.into_iter().map(FaceId::from_raw).collect(),
                points: points.into_iter().map(VertexId::from_raw).collect(),
                pin,
                anchors: anchors.map(anchors_from_file).transpose()?,
            },
            Self::Unwrap {
                faces,
                strips,
                method,
                only,
                anchors,
            } => Operation::Unwrap {
                faces: faces.into_iter().map(FaceId::from_raw).collect(),
                strips,
                method: method_from_file(method),
                only: only.map(axis_from_file),
                anchors: anchors.map(anchors_from_file).transpose()?,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pins_and_chosen_unwraps_survive_the_trip() {
        let some = vec![FaceId::from_raw(1), FaceId::from_raw(4)];
        let points = vec![VertexId::from_raw(2)];
        let mut ops = vec![
            Operation::SetUvPins {
                faces: some.clone(),
                points: Vec::new(),
                pin: true,
                anchors: None,
            },
            Operation::SetUvPins {
                faces: Vec::new(),
                points,
                pin: false,
                anchors: None,
            },
        ];
        for method in UnwrapMethod::ALL {
            for only in [None, Some(UnfoldAxis::Across), Some(UnfoldAxis::Up)] {
                if method == UnwrapMethod::Conformal && only.is_none() {
                    continue;
                }
                ops.push(Operation::Unwrap {
                    faces: some.clone(),
                    strips: true,
                    method,
                    only,
                    anchors: None,
                });
            }
        }
        for op in ops {
            let file = OperationV34::to_file(&op).unwrap();
            assert!(!matches!(file, OperationV34::V33(_)), "{op:?}");
            assert_eq!(file.into_operation().unwrap(), op);
            assert!(OperationV33::to_file(&op).is_none(), "older versions can't");
        }
    }

    #[test]
    fn a_plain_unwrap_is_stored_the_old_way() {
        let op = Operation::Unwrap {
            faces: vec![FaceId::from_raw(1)],
            strips: false,
            method: UnwrapMethod::Conformal,
            only: None,
            anchors: None,
        };
        let file = OperationV34::to_file(&op).unwrap();
        assert!(matches!(file, OperationV34::V33(_)));
        assert_eq!(file.into_operation().unwrap(), op);
    }
}
