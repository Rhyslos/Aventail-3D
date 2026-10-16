//! Version 35 of the project file's contents ([decision 0110](../../../../docs/decisions/0110-controllable-automation.md)):
//! version 34's, with every choice of the automatic UV tools kept.
//!
//! Beside version 33's model (as version 34 stores it): how the project's
//! edits mend the UVs they break (on or off, and the two limits). Older
//! files open with the standard mending. Two more kinds of recorded edit:
//! an unwrap with every choice it makes (strips and how square they must
//! be, the way, one-way unfolding, where cuts hide), and relaxing with its
//! number of steps. Edits that older versions can store are stored as they
//! stored them.

use super::v1::{FileOperation, Problem};
use super::v2::{AnchorsV2, anchors_from_file, anchors_to_file};
use super::v5::EntryWith;
use super::v33::ProjectV33;
use super::v34::{
    AxisV34, MethodV34, OperationV34, axis_from_file, axis_to_file, method_from_file,
    method_to_file,
};
use crate::limits::Bounded;
use scene::{Document, FaceId, HideSeams, MendOptions, Operation, UnwrapOptions};
use serde::{Deserialize, Serialize};

/// A recorded edit in version 35.
pub(crate) type EntryV35 = EntryWith<OperationV35>;

/// Version 35 of the file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProjectV35<E> {
    /// Version 33's file (as version 34 stores it), its edits stored as
    /// `E`.
    pub(crate) project: ProjectV33<E>,
    /// How the project's edits mend UVs.
    pub(crate) mending: MendingV35,
}

/// How the project's edits mend UVs, as stored.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct MendingV35 {
    pub(crate) on: bool,
    pub(crate) uneven: f32,
    pub(crate) size_off: f32,
}

impl<E: FileOperation> ProjectV35<E> {
    /// The file model of a document.
    pub(crate) fn from_document(document: &Document) -> Self {
        let mending = document.scene().mending();
        Self {
            project: ProjectV33::from_document(document),
            mending: MendingV35 {
                on: mending.on,
                uneven: mending.uneven,
                size_off: mending.size_off,
            },
        }
    }

    /// The document this file model describes, after checking every value.
    pub(crate) fn into_document(self) -> Result<Document, Problem> {
        let mending = MendOptions {
            on: self.mending.on,
            uneven: self.mending.uneven,
            size_off: self.mending.size_off,
        };
        if !mending.is_real() {
            return Err("its mending limits are out of range".into());
        }
        Ok(self.project.into_document()?.with_mending(mending))
    }
}

/// Where cuts hide, as stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum HideV35 {
    BelowAndBehind,
    Below,
    Behind,
    Shortest,
}

fn hide_to_file(hide: HideSeams) -> HideV35 {
    match hide {
        HideSeams::BelowAndBehind => HideV35::BelowAndBehind,
        HideSeams::Below => HideV35::Below,
        HideSeams::Behind => HideV35::Behind,
        HideSeams::Shortest => HideV35::Shortest,
    }
}

fn hide_from_file(hide: HideV35) -> HideSeams {
    match hide {
        HideV35::BelowAndBehind => HideSeams::BelowAndBehind,
        HideV35::Below => HideSeams::Below,
        HideV35::Behind => HideSeams::Behind,
        HideV35::Shortest => HideSeams::Shortest,
    }
}

/// The recorded operations version 35 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV35 {
    /// Everything version 34 could store, stored the same way.
    V34(OperationV34),
    /// Pieces unwrapped with every choice given.
    Unwrap {
        faces: Bounded<u32>,
        strips: bool,
        strip_angle: f32,
        method: MethodV34,
        only: Option<AxisV34>,
        hide: HideV35,
        anchors: Option<AnchorsV2>,
    },
    /// UVs relaxed with a number of steps.
    RelaxUvs {
        faces: Bounded<u32>,
        steps: u32,
        anchors: Option<AnchorsV2>,
    },
}

impl FileOperation for OperationV35 {
    fn to_file(op: &Operation) -> Option<Self> {
        if let Some(older) = OperationV34::to_file(op) {
            return Some(Self::V34(older));
        }
        match op {
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
                anchors: anchors.as_ref().map(anchors_to_file),
            }),
            Operation::RelaxUvs {
                faces,
                steps,
                anchors,
            } => Some(Self::RelaxUvs {
                faces: faces.iter().map(|f| f.get()).collect(),
                steps: *steps,
                anchors: anchors.as_ref().map(anchors_to_file),
            }),
            _ => None,
        }
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        let operation = match self {
            Self::V34(op) => return op.into_operation(),
            Self::Unwrap {
                faces,
                strips,
                strip_angle,
                method,
                only,
                hide,
                anchors,
            } => Operation::Unwrap {
                faces: faces.into_iter().map(FaceId::from_raw).collect(),
                options: UnwrapOptions {
                    strips,
                    strip_angle,
                    method: method_from_file(method),
                    only: only.map(axis_from_file),
                    hide: hide_from_file(hide),
                },
                anchors: anchors.map(anchors_from_file).transpose()?,
            },
            Self::RelaxUvs {
                faces,
                steps,
                anchors,
            } => Operation::RelaxUvs {
                faces: faces.into_iter().map(FaceId::from_raw).collect(),
                steps,
                anchors: anchors.map(anchors_from_file).transpose()?,
            },
        };
        if !operation.numbers_are_real() {
            return Err("an unwrap's or relax's settings are out of range".into());
        }
        Ok(operation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scene::{UnfoldAxis, UnwrapMethod};

    #[test]
    fn every_unwrap_choice_and_relax_steps_survive_the_trip() {
        let some = vec![FaceId::from_raw(1), FaceId::from_raw(4)];
        let mut ops = Vec::new();
        for hide in HideSeams::ALL {
            ops.push(Operation::Unwrap {
                faces: some.clone(),
                options: UnwrapOptions {
                    strips: true,
                    strip_angle: 40.0,
                    method: UnwrapMethod::Automatic,
                    only: Some(UnfoldAxis::Up),
                    hide,
                },
                anchors: None,
            });
        }
        ops.push(Operation::Unwrap {
            faces: some.clone(),
            options: UnwrapOptions {
                hide: HideSeams::Shortest,
                ..UnwrapOptions::plain(false)
            },
            anchors: None,
        });
        ops.push(Operation::RelaxUvs {
            faces: some.clone(),
            steps: 200,
            anchors: None,
        });
        for op in ops {
            let file = OperationV35::to_file(&op).unwrap();
            assert!(!matches!(file, OperationV35::V34(_)), "{op:?}");
            assert_eq!(file.into_operation().unwrap(), op);
            assert!(OperationV34::to_file(&op).is_none(), "older versions can't");
        }
        // What older versions can store is stored their way.
        for op in [
            Operation::RelaxUvs {
                faces: some.clone(),
                steps: scene::RELAX_STEPS as u32,
                anchors: None,
            },
            Operation::Unwrap {
                faces: some.clone(),
                options: UnwrapOptions {
                    method: UnwrapMethod::SizeKeeping,
                    ..UnwrapOptions::plain(true)
                },
                anchors: None,
            },
        ] {
            assert!(matches!(
                OperationV35::to_file(&op),
                Some(OperationV35::V34(_))
            ));
        }
    }

    #[test]
    fn nonsense_settings_are_refused() {
        let bad = [
            OperationV35::Unwrap {
                faces: Bounded::default(),
                strips: true,
                strip_angle: f32::NAN,
                method: MethodV34::Conformal,
                only: None,
                hide: HideV35::Below,
                anchors: None,
            },
            OperationV35::Unwrap {
                faces: Bounded::default(),
                strips: true,
                strip_angle: 95.0,
                method: MethodV34::Conformal,
                only: None,
                hide: HideV35::Below,
                anchors: None,
            },
            OperationV35::RelaxUvs {
                faces: Bounded::default(),
                steps: u32::MAX,
                anchors: None,
            },
        ];
        for op in bad {
            assert!(op.into_operation().is_err());
        }
    }

    #[test]
    fn mending_comes_back_and_bad_limits_are_refused() {
        let mending = MendOptions {
            on: false,
            uneven: 5.0,
            size_off: 2.0,
        };
        let doc = Document::new().with_mending(mending);
        let file = ProjectV35::<EntryV35>::from_document(&doc);
        let back = file.clone().into_document().unwrap();
        assert_eq!(back.scene().mending(), mending);
        for (uneven, size_off) in [(0.5, 3.0), (3.0, f32::INFINITY), (f32::NAN, 3.0)] {
            let mut bad = file.clone();
            bad.mending.uneven = uneven;
            bad.mending.size_off = size_off;
            assert!(bad.into_document().is_err(), "{uneven} {size_off}");
        }
    }
}
