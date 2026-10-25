//! Version 23 of the project file's contents ([decision 0085](../../../../docs/decisions/0085-bevel-control.md)):
//! version 22's, with bevels' further settings.
//!
//! A bevel of edges made with any of skew, flip side, even width, stop at
//! overlaps or weld (on or off, and how near) is stored as version 22
//! stores a bevel without them, and those settings beside it. Every other
//! edit, and bevels with none of them, is stored as in version 22. Older
//! files open with bevels as they always were (none of the settings).
//!
//! Version 42 ([decision 0141](../../../../docs/decisions/0141-improved-bevel.md))
//! adds one more kind of operation here, an improved bevel, stored the
//! same way: files before version 42 never hold one, and their bevels
//! open as they were made.

use super::v1::{FileOperation, Problem};
use super::v5::EntryWith;
use super::v22::OperationV22;
use mesh::edit::BevelOptions;
use scene::{EditSettings, Operation};
use serde::{Deserialize, Serialize};
use words::{tr, trf};

/// Version 23 of the file: version 20's (to 22's), its edits stored as
/// version 23's.
#[cfg(test)]
pub(crate) type ProjectV23 = super::v20::ProjectV20<EntryV23>;

/// A recorded edit in version 23.
pub(crate) type EntryV23 = EntryWith<OperationV23>;

/// The recorded operations version 23 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV23 {
    /// Everything version 22 could store, stored the same way.
    V22(OperationV22),
    /// A bevel of edges (on its own, mirrored or on a stretched object)
    /// as version 22 stores it without its further settings, and those
    /// settings.
    Bevel {
        op: OperationV22,
        options: BevelOptionsV23,
    },
    /// An improved bevel of edges (version 42, decision 0141: rounded
    /// corners and stopping at each point's neighbours), stored as
    /// [`Self::Bevel`] is.
    Improved {
        op: OperationV22,
        options: BevelOptionsV23,
    },
}

/// A bevel's further settings: skew (-1 to 1), flip side, even width,
/// stop at overlaps, weld, and the weld distance (0 to 10 cm).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct BevelOptionsV23 {
    skew: f32,
    flip_side: bool,
    even: bool,
    clamp: bool,
    weld: bool,
    weld_distance: f32,
}

/// A bevel of edges' settings, if `op` is one (mirrored or not).
fn options_of(op: &Operation) -> Option<BevelOptions> {
    match op.inner() {
        Operation::BevelEdges { shape, .. } => Some(shape.options),
        _ => None,
    }
}

/// `op` with a bevel's settings set to `options`.
fn with_options(op: &Operation, options: BevelOptions) -> Operation {
    op.with(EditSettings {
        bevel_options: Some(options),
        ..EditSettings::default()
    })
}

impl FileOperation for OperationV23 {
    fn to_file(op: &Operation) -> Option<Self> {
        match options_of(op) {
            Some(options) if options != BevelOptions::EARLIER => {
                let plain = with_options(op, BevelOptions::EARLIER);
                let improved = options.improved;
                let op = OperationV22::to_file(&plain)?;
                let options = BevelOptionsV23 {
                    skew: options.skew,
                    flip_side: options.flip_side,
                    even: options.even,
                    clamp: options.clamp,
                    weld: options.weld,
                    weld_distance: options.weld_distance,
                };
                Some(if improved {
                    Self::Improved { op, options }
                } else {
                    Self::Bevel { op, options }
                })
            }
            _ => Some(Self::V22(OperationV22::to_file(op)?)),
        }
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        let improved = matches!(self, Self::Improved { .. });
        match self {
            Self::V22(op) => op.into_operation(),
            Self::Bevel { op, options } | Self::Improved { op, options } => {
                let options = BevelOptions {
                    skew: options.skew,
                    flip_side: options.flip_side,
                    even: options.even,
                    clamp: options.clamp,
                    weld: options.weld,
                    weld_distance: options.weld_distance,
                    improved,
                };
                // NaN never equals itself, so this refuses those too.
                if options.clamped() != options {
                    return Err(tr(
                        "a bevel's skew or weld distance isn't a real number, or is out \
                                of range",
                    )
                    .into());
                }
                let operation = op.into_operation()?;
                match options_of(&operation) {
                    Some(BevelOptions::EARLIER) => Ok(with_options(&operation, options)),
                    Some(_) => Err(tr("a bevel is stored with its settings twice").into()),
                    None => Err(trf!(
                        "{} is stored with a bevel's settings, which it can't have",
                        operation.describe()
                    )),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mesh::edit::BevelShape;
    use mesh::symmetry::Mirror;
    use mesh::{EdgeKey, VertexId};

    fn bevel(options: BevelOptions) -> Operation {
        Operation::BevelEdges {
            edges: vec![EdgeKey::new(VertexId::from_raw(0), VertexId::from_raw(1))],
            shape: BevelShape {
                width: 3.0,
                segments: 2,
                profile: 0.5,
                options,
            },
            stretch: glam::Vec3::new(0.5, 1.0, 1.5),
            anchors: None,
        }
    }

    #[test]
    fn a_bevel_with_settings_survives_the_trip() {
        let options = BevelOptions {
            skew: -0.4,
            flip_side: true,
            even: true,
            clamp: false,
            weld: true,
            weld_distance: 0.25,
            improved: false,
        };
        for op in [
            bevel(options),
            bevel(BevelOptions::default()),
            bevel(BevelOptions {
                improved: true,
                ..BevelOptions::EARLIER
            }),
            Operation::Mirrored {
                op: Box::new(bevel(options)),
                mirror: Mirror {
                    axes: [true, false, false],
                    centre: glam::Vec3::ZERO,
                    clip: false,
                },
            },
        ] {
            let file = OperationV23::to_file(&op).unwrap();
            let improved =
                matches!(op.inner(), Operation::BevelEdges { shape, .. } if shape.options.improved);
            if improved {
                assert!(matches!(file, OperationV23::Improved { .. }), "{op:?}");
            } else {
                assert!(matches!(file, OperationV23::Bevel { .. }), "{op:?}");
            }
            assert_eq!(file.into_operation().unwrap(), op);
        }
    }

    #[test]
    fn an_earlier_bevel_is_stored_as_before() {
        let op = bevel(BevelOptions::EARLIER);
        let file = OperationV23::to_file(&op).unwrap();
        assert!(matches!(file, OperationV23::V22(_)));
        assert_eq!(file.into_operation().unwrap(), op);
    }

    #[test]
    fn broken_settings_are_refused() {
        let earlier = BevelOptions {
            improved: false,
            ..BevelOptions::default()
        };
        let OperationV23::Bevel { op, options } = OperationV23::to_file(&bevel(earlier)).unwrap()
        else {
            panic!("stored with settings");
        };
        for (skew, distance) in [(f32::NAN, 0.1), (2.0, 0.1), (0.0, -1.0), (0.0, 50.0)] {
            let error = OperationV23::Bevel {
                op: op.clone(),
                options: BevelOptionsV23 {
                    skew,
                    weld_distance: distance,
                    ..options
                },
            }
            .into_operation()
            .unwrap_err();
            assert!(error.contains("skew or weld"), "{error}");
        }
        // Settings on an edit that isn't a bevel of edges.
        let error = OperationV23::Bevel {
            op: OperationV22::to_file(&Operation::FixFacing).unwrap(),
            options,
        }
        .into_operation()
        .unwrap_err();
        assert!(error.contains("can't have"), "{error}");
    }
}
