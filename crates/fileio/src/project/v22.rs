//! Version 22 of the project file's contents ([decision 0082](../../../../docs/decisions/0082-edits-keep-their-shape-on-stretched-objects.md)):
//! version 21's, with every edit that keeps a stretch.
//!
//! Extrudes, insets and bevels made on an object stretched unevenly
//! remember the stretch (how much the object was stretched along each of
//! its axes), so they keep their shape as seen. One more kind of stored
//! edit carries it: any version 21 edit, stored as version 21 stores it
//! without a stretch, and the stretch beside it. Edits with no stretch
//! are stored exactly as in version 21, so a file with none is version
//! 21's file with a new number. Older files open with no stretch, except
//! version 21's stretched bends, which keep theirs.

use super::v1::{FileOperation, Problem};
use super::v5::EntryWith;
use super::v21::OperationV21;
use glam::Vec3;
use scene::Operation;
use serde::{Deserialize, Serialize};
use words::{tr, trf};

/// Version 22 of the file: version 20's (and 21's), its edits stored as
/// version 22's.
#[cfg(test)]
pub(crate) type ProjectV22 = super::v20::ProjectV20<EntryV22>;

/// A recorded edit in version 22.
pub(crate) type EntryV22 = EntryWith<OperationV22>;

/// The recorded operations version 22 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV22 {
    /// Everything version 21 could store, stored the same way.
    V21(OperationV21),
    /// An edit made on a stretched object (on its own or mirrored), as
    /// version 21 stores it without the stretch, and the stretch along
    /// the object's X, Y and Z.
    Stretched { op: OperationV21, stretch: [f32; 3] },
}

impl FileOperation for OperationV22 {
    fn to_file(op: &Operation) -> Option<Self> {
        let stretch = op.stretch();
        if stretch == Vec3::ONE {
            return Some(Self::V21(OperationV21::to_file(op)?));
        }
        Some(Self::Stretched {
            op: OperationV21::to_file(&op.with_stretch(Vec3::ONE))?,
            stretch: stretch.to_array(),
        })
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        match self {
            Self::V21(op) => op.into_operation(),
            Self::Stretched { op, stretch } => {
                let stretch = Vec3::from_array(stretch);
                if mesh::stretch::usable(stretch) != stretch {
                    return Err(
                        tr("an edit's stretch isn't a real number, or is out of range \
                                (above 0, under 1000)")
                        .into(),
                    );
                }
                let operation = op.into_operation()?;
                if !operation.keeps_stretch() {
                    return Err(trf!(
                        "{} is stored with a stretch, which it can't have",
                        operation.describe()
                    ));
                }
                if operation.stretch() != Vec3::ONE {
                    return Err(tr("an edit is stored with two stretches").into());
                }
                Ok(operation.with_stretch(stretch))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mesh::edit::BevelShape;
    use mesh::symmetry::Mirror;
    use mesh::{EdgeKey, FaceId, VertexId};
    use scene::{Bend, ExtrudeShape, Grouping};

    fn stretched_edits(stretch: Vec3) -> Vec<Operation> {
        let faces = vec![FaceId::from_raw(2)];
        let edges = vec![EdgeKey::new(VertexId::from_raw(0), VertexId::from_raw(1))];
        let plain = [
            Operation::ExtrudePolygons {
                faces: faces.clone(),
                distance: 4.0,
                grouping: Grouping::Together,
                shape: ExtrudeShape::default(),
                stretch,
                anchors: None,
            },
            Operation::ExtrudeBent {
                faces: faces.clone(),
                distance: 4.0,
                grouping: Grouping::Separate,
                bend: Bend {
                    angle: 50.0,
                    ..Bend::default()
                },
                stretch,
                anchors: None,
            },
            Operation::InsetPolygons {
                faces,
                distance: 1.5,
                grouping: Grouping::Together,
                stretch,
                anchors: None,
            },
            Operation::ExtrudeEdges {
                edges: edges.clone(),
                distance: 3.0,
                grouping: Grouping::Together,
                stretch,
                anchors: None,
            },
            Operation::BevelEdges {
                edges,
                shape: BevelShape {
                    width: 2.0,
                    segments: 3,
                    profile: 0.5,
                    options: mesh::edit::BevelOptions::EARLIER,
                },
                stretch,
                anchors: None,
            },
            Operation::BevelPoints {
                points: vec![VertexId::from_raw(4)],
                width: 1.0,
                stretch,
                anchors: None,
            },
        ];
        let mirrored = plain.iter().map(|op| Operation::Mirrored {
            op: Box::new(op.clone()),
            mirror: Mirror {
                axes: [true, false, false],
                centre: Vec3::ZERO,
                clip: true,
            },
        });
        plain.iter().cloned().chain(mirrored).collect()
    }

    #[test]
    fn every_stretched_edit_survives_the_trip() {
        for op in stretched_edits(Vec3::new(0.6, 1.2, 1.2)) {
            let file = OperationV22::to_file(&op).unwrap();
            assert!(matches!(file, OperationV22::Stretched { .. }), "{op:?}");
            assert_eq!(file.into_operation().unwrap(), op);
        }
    }

    #[test]
    fn edits_without_a_stretch_are_stored_as_before() {
        for op in stretched_edits(Vec3::ONE) {
            let file = OperationV22::to_file(&op).unwrap();
            assert!(matches!(file, OperationV22::V21(_)), "{op:?}");
            assert_eq!(file.into_operation().unwrap(), op);
        }
    }

    #[test]
    fn broken_stretches_are_refused() {
        let ops = stretched_edits(Vec3::new(0.5, 1.0, 1.5));
        let OperationV22::Stretched { op, .. } = OperationV22::to_file(&ops[0]).unwrap() else {
            panic!("stored as stretched");
        };
        for stretch in [
            [f32::NAN, 1.0, 1.0],
            [0.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0],
            [1.0, f32::INFINITY, 1.0],
            [1.0, 1.0, 5000.0],
        ] {
            let error = OperationV22::Stretched {
                op: op.clone(),
                stretch,
            }
            .into_operation()
            .unwrap_err();
            assert!(error.contains("stretch"), "{error}");
        }
        // An edit that can't have one.
        let fill = OperationV21::to_file(&Operation::FixFacing).unwrap();
        let error = OperationV22::Stretched {
            op: fill,
            stretch: [0.5, 1.0, 1.5],
        }
        .into_operation()
        .unwrap_err();
        assert!(error.contains("can't have"), "{error}");
        // A version 21 stretched bend given a second stretch.
        let OperationV22::Stretched { .. } = OperationV22::to_file(&ops[1]).unwrap() else {
            panic!("stored as stretched");
        };
        let twice = OperationV21::to_file(&ops[1]).unwrap();
        let error = OperationV22::Stretched {
            op: twice,
            stretch: [0.5, 1.0, 1.5],
        }
        .into_operation()
        .unwrap_err();
        assert!(error.contains("two stretches"), "{error}");
    }
}
