//! Version 17 of the project file's contents ([decision 0070](../../../../docs/decisions/0070-extrude-freedom.md)):
//! version 16's, with shaped extrudes.
//!
//! An extrude made the first way (each point along its own facing, one
//! row, the end as it was) is stored as before, inside
//! `OperationV17::V15`. Any other is stored with its whole shape: which
//! way it goes, its end's size, its steps and whether its end is flat; on
//! its own or mirrored. The file around the edits is version 16's.

use super::v1::{FileOperation, Problem};
use super::v2::{AnchorsV2, anchors_from_file, anchors_to_file};
use super::v5::EntryWith;
use super::v15::OperationV15;
use crate::limits::Bounded;
use glam::Vec3;
use mesh::FaceId;
use mesh::symmetry::Mirror;
use scene::{ExtrudeDirection, ExtrudeShape, Grouping, Operation};
use serde::{Deserialize, Serialize};

/// Version 17 of the file.
#[cfg(test)]
pub(crate) type ProjectV17 = super::v16::ProjectWithPlaces<EntryV17>;

/// A recorded edit in version 17.
pub(crate) type EntryV17 = EntryWith<OperationV17>;

/// The recorded operations version 17 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV17 {
    /// Everything version 15 could store, stored the same way.
    V15(OperationV15),
    /// Polygons extruded with a shape.
    Extrude(ExtrudeV17),
    /// The same, made with symmetry on: the edit, the axes mirrored
    /// across, the point the planes go through and whether points are
    /// kept on their side.
    MirroredExtrude {
        op: ExtrudeV17,
        axes: [bool; 3],
        centre: [f32; 3],
        clip: bool,
    },
}

/// A shaped extrude: its polygons, distance, whether each polygon goes on
/// its own, its shape and its anchors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ExtrudeV17 {
    faces: Bounded<u32>,
    distance: f32,
    separate: bool,
    direction: DirectionV17,
    end_size: f32,
    steps: u32,
    flat_end: bool,
    anchors: Option<AnchorsV2>,
}

/// Which way an extrude goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum DirectionV17 {
    Straight,
    AlongNormals,
    AlongX,
    AlongY,
    AlongZ,
}

impl ExtrudeV17 {
    /// The file's form of `op`, if it is an extrude not shaped the first
    /// way (those are stored as version 15 stores them).
    fn of(op: &Operation) -> Option<Self> {
        let Operation::ExtrudePolygons {
            faces,
            distance,
            grouping,
            shape,
            anchors,
            ..
        } = op
        else {
            return None;
        };
        if *shape == ExtrudeShape::EARLIER {
            return None;
        }
        Some(Self {
            faces: faces.iter().map(|f| f.get()).collect(),
            distance: *distance,
            separate: *grouping == Grouping::Separate,
            direction: match shape.direction {
                ExtrudeDirection::Straight => DirectionV17::Straight,
                ExtrudeDirection::AlongNormals => DirectionV17::AlongNormals,
                ExtrudeDirection::AlongX => DirectionV17::AlongX,
                ExtrudeDirection::AlongY => DirectionV17::AlongY,
                ExtrudeDirection::AlongZ => DirectionV17::AlongZ,
            },
            end_size: shape.end_size,
            steps: shape.steps,
            flat_end: shape.flat_end,
            anchors: anchors.as_ref().map(anchors_to_file),
        })
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        let Self {
            faces,
            distance,
            separate,
            direction,
            end_size,
            steps,
            flat_end,
            anchors,
        } = self;
        if !(distance.is_finite() && end_size.is_finite()) {
            return Err("its extrude holds a number that isn't real".into());
        }
        Ok(Operation::ExtrudePolygons {
            faces: faces.into_iter().map(FaceId::from_raw).collect(),
            distance,
            grouping: if separate {
                Grouping::Separate
            } else {
                Grouping::Together
            },
            shape: ExtrudeShape {
                direction: match direction {
                    DirectionV17::Straight => ExtrudeDirection::Straight,
                    DirectionV17::AlongNormals => ExtrudeDirection::AlongNormals,
                    DirectionV17::AlongX => ExtrudeDirection::AlongX,
                    DirectionV17::AlongY => ExtrudeDirection::AlongY,
                    DirectionV17::AlongZ => ExtrudeDirection::AlongZ,
                },
                end_size,
                steps,
                flat_end,
            }
            .clamped(),
            anchors: anchors.map(anchors_from_file).transpose()?,
            stretch: glam::Vec3::ONE,
        })
    }
}

impl FileOperation for OperationV17 {
    fn to_file(op: &Operation) -> Option<Self> {
        if let Some(extrude) = ExtrudeV17::of(op) {
            return Some(Self::Extrude(extrude));
        }
        if let Operation::Mirrored { op, mirror } = op
            && let Some(extrude) = ExtrudeV17::of(op)
        {
            return Some(Self::MirroredExtrude {
                op: extrude,
                axes: mirror.axes,
                centre: mirror.centre.to_array(),
                clip: mirror.clip,
            });
        }
        Some(Self::V15(OperationV15::to_file(op)?))
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        match self {
            Self::V15(op) => op.into_operation(),
            Self::Extrude(extrude) => extrude.into_operation(),
            Self::MirroredExtrude {
                op,
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

    fn shaped(end_size: f32, steps: u32) -> ExtrudeV17 {
        ExtrudeV17 {
            faces: vec![1].into_iter().collect(),
            distance: 2.0,
            separate: false,
            direction: DirectionV17::AlongZ,
            end_size,
            steps,
            flat_end: true,
            anchors: None,
        }
    }

    #[test]
    fn numbers_that_arent_real_are_refused_and_others_brought_in_range() {
        assert!(shaped(f32::NAN, 1).into_operation().is_err());
        let op = shaped(1e9, 100_000).into_operation().unwrap();
        let Operation::ExtrudePolygons { shape, .. } = op else {
            panic!("{op:?}");
        };
        assert_eq!(shape.end_size, mesh::edit::MAX_END_SIZE);
        assert_eq!(shape.steps, mesh::edit::MAX_EXTRUDE_STEPS);
        assert_eq!(shape.direction, ExtrudeDirection::AlongZ);
        assert!(shape.flat_end);
    }

    #[test]
    fn an_extrude_made_the_first_way_is_stored_as_before() {
        let op = Operation::ExtrudePolygons {
            faces: vec![FaceId::from_raw(3)],
            distance: 1.0,
            grouping: Grouping::Together,
            shape: ExtrudeShape::EARLIER,
            anchors: None,
            stretch: glam::Vec3::ONE,
        };
        assert!(matches!(
            OperationV17::to_file(&op),
            Some(OperationV17::V15(_))
        ));
        let new = Operation::ExtrudePolygons {
            faces: vec![FaceId::from_raw(3)],
            distance: 1.0,
            grouping: Grouping::Together,
            shape: ExtrudeShape::default(),
            anchors: None,
            stretch: glam::Vec3::ONE,
        };
        assert!(matches!(
            OperationV17::to_file(&new),
            Some(OperationV17::Extrude(_))
        ));
    }

    #[test]
    fn a_mirrored_extrude_needs_an_axis_and_a_real_middle() {
        let no_axis = OperationV17::MirroredExtrude {
            op: shaped(1.0, 1),
            axes: [false; 3],
            centre: [0.0; 3],
            clip: false,
        };
        assert!(no_axis.into_operation().is_err());
        let bad = OperationV17::MirroredExtrude {
            op: shaped(1.0, 1),
            axes: [true, false, false],
            centre: [f32::NAN, 0.0, 0.0],
            clip: false,
        };
        assert!(bad.into_operation().is_err());
    }
}
