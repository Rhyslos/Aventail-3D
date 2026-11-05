//! Version 86 of the project file ([decision 0235](../../../../docs/decisions/0235-projections.md)):
//! version 85's, with projected and world-aligned UVs, one more kind of
//! recorded edit (`v37::OperationV37::ProjectUvs`, stored as
//! [`ProjectEditV86`]). Files before version 86 hold none.

use super::v1::Problem;
use super::v2::{AnchorsV2, anchors_from_file, anchors_to_file};
use crate::limits::Bounded;
use scene::{FaceId, Operation};
use serde::{Deserialize, Serialize};
use scene::{ProjectOptions, Projection};
use words::tr;

/// A projection as stored: the polygons, the shape (0 flat, 1 box, 2
/// cylinder, 3 ball), the placement's 16 numbers (column by column), the
/// world scale if world-aligned, and the polygons' anchors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProjectEditV86 {
    pub(crate) faces: Bounded<u32>,
    pub(crate) kind: u8,
    pub(crate) placement: [f32; 16],
    pub(crate) world: Option<f32>,
    pub(crate) anchors: Option<AnchorsV2>,
}

fn kind_to_file(kind: Projection) -> u8 {
    match kind {
        Projection::Planar => 0,
        Projection::Box => 1,
        Projection::Cylinder => 2,
        Projection::Sphere => 3,
    }
}

impl ProjectEditV86 {
    /// The stored form of `op`, if it projects UVs.
    pub(crate) fn of(op: &Operation) -> Option<Self> {
        let Operation::ProjectUvs {
            faces,
            options,
            anchors,
        } = op
        else {
            return None;
        };
        Some(Self {
            faces: faces.iter().map(|f| f.get()).collect(),
            kind: kind_to_file(options.kind),
            placement: options.placement.to_cols_array(),
            world: options.world,
            anchors: anchors.as_ref().map(anchors_to_file),
        })
    }

    /// The edit read back; an unknown shape, or a placement that can't be
    /// undone or isn't made of numbers, is refused.
    pub(crate) fn into_operation(self) -> Result<Operation, Problem> {
        let kind = match self.kind {
            0 => Projection::Planar,
            1 => Projection::Box,
            2 => Projection::Cylinder,
            3 => Projection::Sphere,
            _ => return Err(tr("a projection of an unknown kind").into()),
        };
        let options = ProjectOptions {
            kind,
            placement: glam::Mat4::from_cols_array(&self.placement),
            world: self.world,
        };
        if !options.is_real() {
            return Err(tr("a projection placed impossibly").into());
        }
        Ok(Operation::ProjectUvs {
            faces: self.faces.into_iter().map(FaceId::from_raw).collect(),
            options,
            anchors: self.anchors.map(anchors_from_file).transpose()?,
        })
    }
}
