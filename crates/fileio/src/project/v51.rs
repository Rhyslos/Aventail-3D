//! Version 51 of the project file ([decision 0150](../../../../docs/decisions/0150-quadify.md)):
//! version 50's, with the Quadify modifier (`v43::KindV43::Quadify`, its
//! settings stored as [`QuadifyV51`]) and the one-off Quadify, one more
//! kind of recorded edit (`v37::OperationV37::Quadify`, stored as
//! [`QuadifyEditV51`]). Files before version 51 hold neither.

use super::v1::Problem;
use super::v2::{AnchorsV2, anchors_from_file, anchors_to_file};
use crate::limits::Bounded;
use scene::{EditSettings, FaceId, Hardness, Operation, QuadifyOptions};
use serde::{Deserialize, Serialize};
use words::tr;

/// Quadify's settings as stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct QuadifyV51 {
    /// Max angle, in degrees.
    pub(crate) max_angle: f32,
    /// Max shape distortion, in degrees.
    pub(crate) max_distortion: f32,
    /// Keep UV seams.
    pub(crate) keep_seams: bool,
    /// Keep hard edges.
    pub(crate) keep_hard_edges: bool,
    /// Keep material borders.
    pub(crate) keep_materials: bool,
}

impl QuadifyV51 {
    /// The stored form of `options`.
    pub(crate) fn of(options: &QuadifyOptions) -> Self {
        Self {
            max_angle: options.max_angle,
            max_distortion: options.max_distortion,
            keep_seams: options.keep_seams,
            keep_hard_edges: options.keep_hard_edges,
            keep_materials: options.keep_materials,
        }
    }

    /// The settings read back (brought within their limits by whoever
    /// uses them).
    pub(crate) fn options(&self) -> QuadifyOptions {
        QuadifyOptions {
            max_angle: self.max_angle,
            max_distortion: self.max_distortion,
            keep_seams: self.keep_seams,
            keep_hard_edges: self.keep_hard_edges,
            keep_materials: self.keep_materials,
        }
    }
}

/// The one-off Quadify as stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct QuadifyEditV51 {
    /// The polygons; none for the whole shape.
    pub(crate) faces: Bounded<u32>,
    /// Its settings.
    pub(crate) options: QuadifyV51,
    /// How the object was shaded when it was made: the angle when by
    /// angle, none when by its marked hard edges.
    pub(crate) smoothing_angle: Option<f32>,
    /// Where the polygons were.
    pub(crate) anchors: Option<AnchorsV2>,
}

impl QuadifyEditV51 {
    /// The stored form of `op`, if it is a Quadify.
    pub(crate) fn of(op: &Operation) -> Option<Self> {
        let Operation::Quadify {
            faces,
            options,
            hardness,
            anchors,
        } = op
        else {
            return None;
        };
        Some(Self {
            faces: faces.iter().map(|f| f.get()).collect(),
            options: QuadifyV51::of(options),
            smoothing_angle: match hardness {
                Hardness::Marked => None,
                Hardness::ByAngle(angle) => Some(*angle),
            },
            anchors: anchors.as_ref().map(anchors_to_file),
        })
    }

    /// The edit read back. Settings out of range are brought within
    /// limits; ones that aren't numbers, or a smoothing angle that isn't
    /// one from 0 to 180°, are refused as damaged.
    pub(crate) fn into_operation(self) -> Result<Operation, Problem> {
        let options = self.options.options();
        let operation = Operation::Quadify {
            faces: self.faces.into_iter().map(FaceId::from_raw).collect(),
            options,
            hardness: match self.smoothing_angle {
                None => Hardness::Marked,
                Some(angle) => Hardness::ByAngle(angle),
            },
            anchors: self.anchors.map(anchors_from_file).transpose()?,
        };
        if !operation.numbers_are_real() {
            return Err(tr("a Quadify's settings aren't real angles").into());
        }
        // Brought within limits.
        Ok(operation.with(EditSettings {
            quadify: Some(options),
            ..EditSettings::default()
        }))
    }
}
