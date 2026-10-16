//! Version 36 of the project file's contents ([decision 0111](../../../../docs/decisions/0111-seam-preferences-and-stretch-budget.md)):
//! version 35's, with seam preferences and stretch budgets.
//!
//! Three more kinds of recorded edit: edges given a seam preference
//! (prefer, never, or cleared), polygons given a stretch budget (or theirs
//! cleared), and an unwrap with a stretch budget for the whole shape.
//! Edits older versions can store are stored as they stored them.

use super::v1::{FileOperation, Problem};
use super::v2::{AnchorsV2, anchors_from_file, anchors_to_file};
use super::v5::EntryWith;
use super::v34::{
    AxisV34, MethodV34, axis_from_file, axis_to_file, method_from_file, method_to_file,
};
use super::v35::{HideV35, OperationV35, hide_from_file, hide_to_file};
use crate::limits::Bounded;
use scene::{FaceId, Operation, SeamPreference, UnwrapOptions};
use serde::{Deserialize, Serialize};

/// A recorded edit in version 36.
pub(crate) type EntryV36 = EntryWith<OperationV36>;

/// A seam preference, as stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum PreferenceV36 {
    Free,
    Prefer,
    Never,
}

/// The recorded operations version 36 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV36 {
    /// Everything version 35 could store, stored the same way.
    V35(OperationV35),
    /// Edges given a seam preference.
    SetSeamPreference {
        edges: Bounded<[u32; 2]>,
        preference: PreferenceV36,
        anchors: Option<AnchorsV2>,
    },
    /// Polygons given a stretch budget (or theirs cleared).
    SetStretchBudget {
        faces: Bounded<u32>,
        budget: Option<f32>,
        anchors: Option<AnchorsV2>,
    },
    /// Pieces unwrapped with a stretch budget.
    Unwrap {
        faces: Bounded<u32>,
        strips: bool,
        strip_angle: f32,
        method: MethodV34,
        only: Option<AxisV34>,
        hide: HideV35,
        budget: f32,
        anchors: Option<AnchorsV2>,
    },
}

fn preference_to_file(preference: SeamPreference) -> PreferenceV36 {
    match preference {
        SeamPreference::Free => PreferenceV36::Free,
        SeamPreference::Prefer => PreferenceV36::Prefer,
        SeamPreference::Never => PreferenceV36::Never,
    }
}

fn preference_from_file(preference: PreferenceV36) -> SeamPreference {
    match preference {
        PreferenceV36::Free => SeamPreference::Free,
        PreferenceV36::Prefer => SeamPreference::Prefer,
        PreferenceV36::Never => SeamPreference::Never,
    }
}

impl FileOperation for OperationV36 {
    fn to_file(op: &Operation) -> Option<Self> {
        if let Some(older) = OperationV35::to_file(op) {
            return Some(Self::V35(older));
        }
        match op {
            Operation::SetSeamPreference {
                edges,
                preference,
                anchors,
            } => Some(Self::SetSeamPreference {
                edges: edges
                    .iter()
                    .map(|e| {
                        let (a, b) = e.ends();
                        [a.get(), b.get()]
                    })
                    .collect(),
                preference: preference_to_file(*preference),
                anchors: anchors.as_ref().map(anchors_to_file),
            }),
            Operation::SetStretchBudget {
                faces,
                budget,
                anchors,
            } => Some(Self::SetStretchBudget {
                faces: faces.iter().map(|f| f.get()).collect(),
                budget: *budget,
                anchors: anchors.as_ref().map(anchors_to_file),
            }),
            Operation::Unwrap {
                faces,
                options,
                anchors,
            } => Some(Self::Unwrap {
                faces: faces.iter().map(|f| f.get()).collect(),
                strips: options.strips,
                strip_angle: options.strip_angle,
                method: method_to_file(options.method),
                only: options.only.map(axis_to_file),
                hide: hide_to_file(options.hide),
                budget: options.budget?,
                anchors: anchors.as_ref().map(anchors_to_file),
            }),
            _ => None,
        }
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        let operation = match self {
            Self::V35(op) => return op.into_operation(),
            Self::SetSeamPreference {
                edges,
                preference,
                anchors,
            } => Operation::SetSeamPreference {
                edges: edges
                    .into_iter()
                    .map(|[a, b]| {
                        if a == b {
                            Err("one of its seam preferences joins a point to itself".to_string())
                        } else {
                            Ok(scene::EdgeKey::new(
                                scene::VertexId::from_raw(a),
                                scene::VertexId::from_raw(b),
                            ))
                        }
                    })
                    .collect::<Result<_, _>>()?,
                preference: preference_from_file(preference),
                anchors: anchors.map(anchors_from_file).transpose()?,
            },
            Self::SetStretchBudget {
                faces,
                budget,
                anchors,
            } => Operation::SetStretchBudget {
                faces: faces.into_iter().map(FaceId::from_raw).collect(),
                budget,
                anchors: anchors.map(anchors_from_file).transpose()?,
            },
            Self::Unwrap {
                faces,
                strips,
                strip_angle,
                method,
                only,
                hide,
                budget,
                anchors,
            } => Operation::Unwrap {
                faces: faces.into_iter().map(FaceId::from_raw).collect(),
                options: UnwrapOptions {
                    strips,
                    strip_angle,
                    method: method_from_file(method),
                    only: only.map(axis_from_file),
                    hide: hide_from_file(hide),
                    budget: Some(budget),
                },
                anchors: anchors.map(anchors_from_file).transpose()?,
            },
        };
        if !operation.numbers_are_real() {
            return Err("a stretch budget or unwrap setting is out of range".into());
        }
        Ok(operation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scene::{EdgeKey, HideSeams, UnwrapMethod, VertexId};

    #[test]
    fn preferences_budgets_and_budgeted_unwraps_survive_the_trip() {
        let edge = EdgeKey::new(VertexId::from_raw(1), VertexId::from_raw(2));
        let faces = vec![FaceId::from_raw(1), FaceId::from_raw(4)];
        let mut ops = Vec::new();
        for preference in [
            SeamPreference::Free,
            SeamPreference::Prefer,
            SeamPreference::Never,
        ] {
            ops.push(Operation::SetSeamPreference {
                edges: vec![edge],
                preference,
                anchors: None,
            });
        }
        for budget in [Some(1.5), None] {
            ops.push(Operation::SetStretchBudget {
                faces: faces.clone(),
                budget,
                anchors: None,
            });
        }
        ops.push(Operation::Unwrap {
            faces: faces.clone(),
            options: UnwrapOptions {
                method: UnwrapMethod::SizeKeeping,
                hide: HideSeams::Below,
                budget: Some(2.0),
                ..UnwrapOptions::plain(true)
            },
            anchors: None,
        });
        for op in ops {
            let file = OperationV36::to_file(&op).unwrap();
            assert!(!matches!(file, OperationV36::V35(_)), "{op:?}");
            assert_eq!(file.into_operation().unwrap(), op);
            assert!(OperationV35::to_file(&op).is_none(), "older versions can't");
        }
    }

    #[test]
    fn an_edge_from_a_point_to_itself_is_refused() {
        let bad = OperationV36::SetSeamPreference {
            edges: vec![[3, 3]].into_iter().collect(),
            preference: PreferenceV36::Never,
            anchors: None,
        };
        assert!(bad.into_operation().is_err());
    }

    #[test]
    fn budgets_out_of_range_are_refused() {
        for budget in [0.5, 50.0, f32::NAN] {
            let bad = OperationV36::SetStretchBudget {
                faces: Bounded::default(),
                budget: Some(budget),
                anchors: None,
            };
            assert!(bad.into_operation().is_err(), "{budget}");
        }
    }
}
