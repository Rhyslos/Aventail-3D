//! Version 21 of the project file's contents ([decision 0081](../../../../docs/decisions/0081-bends-keep-their-shape-on-stretched-objects.md)):
//! version 20's, with bends that keep their shape on stretched objects.
//!
//! Among the recorded edits, one more kind: a bent extrude made on an
//! object stretched unevenly, with how much the object was stretched
//! along each of its axes, on its own or mirrored. Every other edit (a
//! bend on an object scaled evenly among them) is stored as in version
//! 20, and older files open with no stretch on their bends.

use super::v1::{FileOperation, Problem};
use super::v5::EntryWith;
use super::v15::ExtrudeBentV15;
use super::v18::OperationV18;
use super::v20::ProjectV20;
use glam::Vec3;
use mesh::symmetry::Mirror;
use scene::Operation;
use serde::{Deserialize, Serialize};

/// Version 21 of the file: version 20's, its edits stored as version
/// 21's.
pub(crate) type ProjectV21 = ProjectV20<EntryV21>;

/// A recorded edit in version 21.
pub(crate) type EntryV21 = EntryWith<OperationV21>;

/// The recorded operations version 21 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV21 {
    /// Everything version 18 to 20 could store, stored the same way.
    V18(OperationV18),
    /// A bent extrude on a stretched object, and the stretch along the
    /// object's X, Y and Z.
    StretchedBent {
        op: ExtrudeBentV15,
        stretch: [f32; 3],
    },
    /// The same, made with symmetry on: the edit, the axes mirrored
    /// across, the point the planes go through and whether points are
    /// kept on their side.
    MirroredStretchedBent {
        op: ExtrudeBentV15,
        stretch: [f32; 3],
        axes: [bool; 3],
        centre: [f32; 3],
        clip: bool,
    },
}

/// The file's form of `op` and its stretch, if it is a bent extrude on a
/// stretched object.
fn stretched_bent(op: &Operation) -> Option<(ExtrudeBentV15, [f32; 3])> {
    let Operation::ExtrudeBent { stretch, .. } = op else {
        return None;
    };
    if *stretch == Vec3::ONE {
        return None;
    }
    let mut plain = op.clone();
    if let Operation::ExtrudeBent { stretch, .. } = &mut plain {
        *stretch = Vec3::ONE;
    }
    Some((ExtrudeBentV15::of(&plain)?, stretch.to_array()))
}

/// The bent extrude `op` with `stretch`, after checking it.
fn with_stretch(op: ExtrudeBentV15, stretch: [f32; 3]) -> Result<Operation, Problem> {
    let stretch = Vec3::from_array(stretch);
    if mesh::extrude::usable_stretch(stretch) != stretch {
        return Err(
            "a bend's stretch isn't a real number, or is out of range (above 0, at most 1000)"
                .into(),
        );
    }
    let mut operation = op.into_operation()?;
    if let Operation::ExtrudeBent { stretch: s, .. } = &mut operation {
        *s = stretch;
    }
    Ok(operation)
}

impl FileOperation for OperationV21 {
    fn to_file(op: &Operation) -> Option<Self> {
        if let Some((op, stretch)) = stretched_bent(op) {
            return Some(Self::StretchedBent { op, stretch });
        }
        if let Operation::Mirrored { op, mirror } = op
            && let Some((op, stretch)) = stretched_bent(op)
        {
            return Some(Self::MirroredStretchedBent {
                op,
                stretch,
                axes: mirror.axes,
                centre: mirror.centre.to_array(),
                clip: mirror.clip,
            });
        }
        Some(Self::V18(OperationV18::to_file(op)?))
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        match self {
            Self::V18(op) => op.into_operation(),
            Self::StretchedBent { op, stretch } => with_stretch(op, stretch),
            Self::MirroredStretchedBent {
                op,
                stretch,
                axes,
                centre,
                clip,
            } => {
                let centre = Vec3::from_array(centre);
                if !centre.is_finite() {
                    return Err("a mirror's middle isn't a real number".to_string());
                }
                if !axes.contains(&true) {
                    return Err("a mirrored edit mirrors across no axis".to_string());
                }
                Ok(Operation::Mirrored {
                    op: Box::new(with_stretch(op, stretch)?),
                    mirror: Mirror { axes, centre, clip },
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scene::{Bend, Grouping};

    fn bent(stretch: Vec3) -> Operation {
        Operation::ExtrudeBent {
            faces: vec![mesh::FaceId::from_raw(3)],
            distance: 5.0,
            grouping: Grouping::Together,
            bend: Bend {
                angle: 90.0,
                radius: 10.0,
                ..Bend::default()
            },
            stretch,
            anchors: None,
        }
    }

    #[test]
    fn a_stretched_bend_survives_the_trip() {
        let op = bent(Vec3::new(0.75, 0.75, 1.5));
        let file = OperationV21::to_file(&op).unwrap();
        assert!(matches!(file, OperationV21::StretchedBent { .. }));
        assert_eq!(file.into_operation().unwrap(), op);
        let mirrored = Operation::Mirrored {
            op: Box::new(op),
            mirror: Mirror {
                axes: [true, false, false],
                centre: Vec3::ZERO,
                clip: true,
            },
        };
        let file = OperationV21::to_file(&mirrored).unwrap();
        assert!(matches!(file, OperationV21::MirroredStretchedBent { .. }));
        assert_eq!(file.into_operation().unwrap(), mirrored);
    }

    #[test]
    fn an_even_bend_is_stored_as_before() {
        let op = bent(Vec3::ONE);
        let file = OperationV21::to_file(&op).unwrap();
        assert!(matches!(file, OperationV21::V18(_)));
        assert_eq!(file.into_operation().unwrap(), op);
    }

    #[test]
    fn a_broken_stretch_is_refused() {
        let Some((op, _)) = stretched_bent(&bent(Vec3::new(0.5, 1.0, 1.5))) else {
            panic!("not stored as stretched");
        };
        for stretch in [
            [f32::NAN, 1.0, 1.0],
            [0.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0],
            [1.0, f32::INFINITY, 1.0],
            [1.0, 1.0, 5000.0],
        ] {
            let error = OperationV21::StretchedBent {
                op: op.clone(),
                stretch,
            }
            .into_operation()
            .unwrap_err();
            assert!(error.contains("stretch"), "{error}");
        }
        let error = OperationV21::MirroredStretchedBent {
            op,
            stretch: [0.5, 1.0, 1.5],
            axes: [false; 3],
            centre: [0.0; 3],
            clip: false,
        }
        .into_operation()
        .unwrap_err();
        assert!(error.contains("no axis"), "{error}");
    }
}
