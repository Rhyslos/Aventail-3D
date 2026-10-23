//! Version 15 of the project file's contents ([decision 0062](../../../../docs/decisions/0062-spirals.md)):
//! version 14's, with bends that can spiral.
//!
//! A bent extrude is stored with its whole bend, including how much
//! further each turn rises; on its own or mirrored. Version 14's
//! operations are stored inside version 15's as they were
//! (`OperationV15::V14`). The file around them is version 12's, with its
//! instances. Version 16 keeps these operations as they are.

use super::v1::{FileOperation, Problem};
use super::v2::{AnchorsV2, anchors_from_file, anchors_to_file};
use super::v5::EntryWith;
use super::v14::OperationV14;
use crate::limits::Bounded;
use glam::Vec3;
use mesh::FaceId;
use mesh::symmetry::Mirror;
use scene::{Bend, Grouping, Operation};
use serde::{Deserialize, Serialize};
use words::tr;

/// Version 15 of the file.
#[cfg(test)]
pub(crate) type ProjectV15 = super::v12::ProjectWithInstances<EntryV15>;

/// A recorded edit in version 15.
pub(crate) type EntryV15 = EntryWith<OperationV15>;

/// The recorded operations version 15 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV15 {
    /// Everything version 14 could store, stored the same way.
    V14(OperationV14),
    /// Polygons extruded with a bend that may spiral.
    ExtrudeBent(ExtrudeBentV15),
    /// The same, made with symmetry on: the edit, the axes mirrored
    /// across, the point the planes go through and whether points are
    /// kept on their side.
    MirroredBent {
        op: ExtrudeBentV15,
        axes: [bool; 3],
        centre: [f32; 3],
        clip: bool,
    },
}

/// A bent extrude: its polygons, how far it goes on straight, whether
/// each polygon goes on its own, its bend and its anchors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ExtrudeBentV15 {
    faces: Bounded<u32>,
    distance: f32,
    separate: bool,
    bend: BendV15,
    anchors: Option<AnchorsV2>,
}

/// A bend: degrees, degrees, centimetres, rows (for each turn of a
/// spiral), centimetres risen for each turn, and whether it climbs the
/// mirrored way.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct BendV15 {
    angle: f32,
    direction: f32,
    radius: f32,
    segments: u32,
    rise: f32,
    mirrored: bool,
}

impl ExtrudeBentV15 {
    /// The file's form of `op`, if it is a bent extrude (not on a
    /// stretched object: version 21 adds the stretch).
    pub(super) fn of(op: &Operation) -> Option<Self> {
        let Operation::ExtrudeBent {
            faces,
            distance,
            grouping,
            bend,
            stretch,
            anchors,
        } = op
        else {
            return None;
        };
        if *stretch != Vec3::ONE {
            // A bend on a stretched object needs version 21.
            return None;
        }
        Some(Self {
            faces: faces.iter().map(|f| f.get()).collect(),
            distance: *distance,
            separate: *grouping == Grouping::Separate,
            bend: BendV15 {
                angle: bend.angle,
                direction: bend.direction,
                radius: bend.radius,
                segments: bend.segments,
                rise: bend.rise,
                mirrored: bend.mirrored,
            },
            anchors: anchors.as_ref().map(anchors_to_file),
        })
    }

    pub(super) fn into_operation(self) -> Result<Operation, Problem> {
        let Self {
            faces,
            distance,
            separate,
            bend,
            anchors,
        } = self;
        let real = [distance, bend.angle, bend.direction, bend.radius, bend.rise]
            .iter()
            .all(|x| x.is_finite());
        if !real {
            return Err(tr("its bent extrude holds a number that isn't real").into());
        }
        Ok(Operation::ExtrudeBent {
            stretch: Vec3::ONE,
            faces: faces.into_iter().map(FaceId::from_raw).collect(),
            distance,
            grouping: if separate {
                Grouping::Separate
            } else {
                Grouping::Together
            },
            bend: Bend {
                angle: bend.angle,
                direction: bend.direction,
                radius: bend.radius,
                segments: bend.segments,
                rise: bend.rise.min(mesh::primitives::MAX_SIZE),
                mirrored: bend.mirrored,
            }
            .clamped(),
            anchors: anchors.map(anchors_from_file).transpose()?,
        })
    }
}

impl FileOperation for OperationV15 {
    fn to_file(op: &Operation) -> Option<Self> {
        if let Some(bent) = ExtrudeBentV15::of(op) {
            return Some(Self::ExtrudeBent(bent));
        }
        if let Operation::Mirrored { op, mirror } = op
            && let Some(bent) = ExtrudeBentV15::of(op)
        {
            return Some(Self::MirroredBent {
                op: bent,
                axes: mirror.axes,
                centre: mirror.centre.to_array(),
                clip: mirror.clip,
            });
        }
        Some(Self::V14(OperationV14::to_file(op)?))
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        match self {
            Self::V14(op) => op.into_operation(),
            Self::ExtrudeBent(bent) => bent.into_operation(),
            Self::MirroredBent {
                op,
                axes,
                centre,
                clip,
            } => {
                let centre = Vec3::from_array(centre);
                if !centre.is_finite() {
                    return Err(tr("a mirror's middle isn't a real number").to_string());
                }
                if !axes.contains(&true) {
                    return Err(tr("a mirrored edit mirrors across no axis").to_string());
                }
                Ok(Operation::Mirrored {
                    op: Box::new(op.into_operation()?),
                    mirror: Mirror { axes, centre, clip },
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bent(rise: f32) -> ExtrudeBentV15 {
        ExtrudeBentV15 {
            faces: vec![1].into_iter().collect(),
            distance: 2.0,
            separate: false,
            bend: BendV15 {
                angle: 720.0,
                direction: 0.0,
                radius: 10.0,
                segments: 8,
                rise,
                mirrored: false,
            },
            anchors: None,
        }
    }

    #[test]
    fn a_rise_that_isnt_real_is_refused() {
        for rise in [f32::NAN, f32::INFINITY] {
            let error = bent(rise).into_operation().unwrap_err();
            assert!(error.contains("isn't real"), "{error}");
        }
    }

    #[test]
    fn a_rise_is_kept_in_range() {
        let op = bent(-5.0).into_operation().unwrap();
        let Operation::ExtrudeBent { bend, .. } = op else {
            panic!("{op:?}");
        };
        assert_eq!((bend.angle, bend.rise), (720.0, 0.0));
        let op = bent(1e30).into_operation().unwrap();
        let Operation::ExtrudeBent { bend, .. } = op else {
            panic!("{op:?}");
        };
        assert_eq!(bend.rise, mesh::primitives::MAX_SIZE);
    }

    #[test]
    fn a_mirrored_spiral_needs_an_axis_and_a_real_middle() {
        let no_axis = OperationV15::MirroredBent {
            op: bent(1.0),
            axes: [false; 3],
            centre: [0.0; 3],
            clip: false,
        };
        assert!(no_axis.into_operation().is_err());
        let bad_middle = OperationV15::MirroredBent {
            op: bent(1.0),
            axes: [true, false, false],
            centre: [f32::NAN, 0.0, 0.0],
            clip: false,
        };
        assert!(bad_middle.into_operation().is_err());
    }
}
