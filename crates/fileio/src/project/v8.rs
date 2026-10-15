//! Version 8 of the project file's contents ([decision 0050](../../../../docs/decisions/0050-pipe-bends.md)):
//! version 7's, with extrudes that bend.
//!
//! Version 7's operations are stored inside version 8's as they were
//! (`OperationV8::V7`), so their checks are shared; the entry around each
//! operation is version 5's.

use super::v1::{FileOperation, Problem};
use super::v2::{AnchorsV2, anchors_from_file, anchors_to_file};
use super::v5::EntryWith;
use super::v7::OperationV7;
use crate::limits::Bounded;
use mesh::FaceId;
use scene::{Bend, Grouping, Operation};
use serde::{Deserialize, Serialize};

/// Version 8 of the file.
#[cfg(test)]
pub(crate) type ProjectV8 = super::v5::ProjectWith<EntryV8>;

/// A recorded edit in version 8.
pub(crate) type EntryV8 = EntryWith<OperationV8>;

/// The recorded operations version 8 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV8 {
    /// Everything version 7 could store, stored the same way.
    V7(OperationV7),
    /// Polygons extruded with a bend first.
    ExtrudeBent {
        faces: Bounded<u32>,
        distance: f32,
        /// Each polygon on its own (`true`) or neighbours together.
        separate: bool,
        bend: BendV8,
        anchors: Option<AnchorsV2>,
    },
}

/// A bend: degrees, degrees, centimetres and rows.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct BendV8 {
    pub(crate) angle: f32,
    pub(crate) direction: f32,
    pub(crate) radius: f32,
    pub(crate) segments: u32,
}

impl FileOperation for OperationV8 {
    fn to_file(op: &Operation) -> Option<Self> {
        Some(match op {
            // (A bend on a stretched object needs version 21.)
            Operation::ExtrudeBent {
                faces,
                distance,
                grouping,
                bend,
                stretch,
                anchors,
            } if *stretch == glam::Vec3::ONE => Self::ExtrudeBent {
                faces: faces.iter().map(|f| f.get()).collect(),
                distance: *distance,
                separate: *grouping == Grouping::Separate,
                bend: BendV8 {
                    angle: bend.angle,
                    direction: bend.direction,
                    radius: bend.radius,
                    segments: bend.segments,
                },
                anchors: anchors.as_ref().map(anchors_to_file),
            },
            _ => Self::V7(OperationV7::to_file(op)?),
        })
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        match self {
            Self::V7(op) => op.into_operation(),
            Self::ExtrudeBent {
                faces,
                distance,
                separate,
                bend,
                anchors,
            } => {
                let real = [distance, bend.angle, bend.direction, bend.radius]
                    .iter()
                    .all(|x| x.is_finite());
                if !real {
                    return Err("its bent extrude holds a number that isn't real".into());
                }
                Ok(Operation::ExtrudeBent {
                    stretch: glam::Vec3::ONE,
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
                        ..Bend::default()
                    }
                    .clamped(),
                    anchors: anchors.map(anchors_from_file).transpose()?,
                })
            }
        }
    }
}
