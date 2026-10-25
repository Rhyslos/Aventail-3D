//! Version 53 of the project file ([decision 0152](../../../../docs/decisions/0152-weld-by-distance.md)):
//! version 52's, with the Weld by distance modifier
//! (`v43::KindV43::WeldByDistance`, its settings stored as [`WeldV53`])
//! and the one-off Weld by distance now, one more kind of recorded edit
//! (`v37::OperationV37::WeldByDistance`, stored as [`WeldEditV53`]).
//! Files before version 53 hold neither.

use super::v1::Problem;
use super::v2::{AnchorsV2, anchors_from_file, anchors_to_file};
use crate::limits::Bounded;
use scene::{EditSettings, Operation, VertexId, WeldOptions};
use serde::{Deserialize, Serialize};
use words::tr;

/// Weld by distance's settings as stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct WeldV53 {
    /// Distance, in centimetres.
    pub(crate) distance: f32,
    /// Only at open borders.
    pub(crate) only_borders: bool,
    /// Keep UV seams.
    pub(crate) keep_seams: bool,
}

impl WeldV53 {
    /// The stored form of `options`.
    pub(crate) fn of(options: &WeldOptions) -> Self {
        Self {
            distance: options.distance,
            only_borders: options.only_borders,
            keep_seams: options.keep_seams,
        }
    }

    /// The settings read back (brought within their limits by whoever
    /// uses them).
    pub(crate) fn options(&self) -> WeldOptions {
        WeldOptions {
            distance: self.distance,
            only_borders: self.only_borders,
            keep_seams: self.keep_seams,
        }
    }
}

/// The one-off Weld by distance as stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct WeldEditV53 {
    /// The points; none for the whole shape.
    pub(crate) points: Bounded<u32>,
    /// Its settings.
    pub(crate) options: WeldV53,
    /// Where the points were.
    pub(crate) anchors: Option<AnchorsV2>,
}

impl WeldEditV53 {
    /// The stored form of `op`, if it is a Weld by distance.
    pub(crate) fn of(op: &Operation) -> Option<Self> {
        let Operation::WeldByDistance {
            points,
            options,
            anchors,
        } = op
        else {
            return None;
        };
        Some(Self {
            points: points.iter().map(|v| v.get()).collect(),
            options: WeldV53::of(options),
            anchors: anchors.as_ref().map(anchors_to_file),
        })
    }

    /// The edit read back. A distance out of range is brought within
    /// limits; one that isn't a number is refused as damaged.
    pub(crate) fn into_operation(self) -> Result<Operation, Problem> {
        let options = self.options.options();
        let operation = Operation::WeldByDistance {
            points: self.points.into_iter().map(VertexId::from_raw).collect(),
            options,
            anchors: self.anchors.map(anchors_from_file).transpose()?,
        };
        if !operation.numbers_are_real() {
            return Err(tr("a Weld by distance's Distance isn't a real number").into());
        }
        // Brought within limits.
        Ok(operation.with(EditSettings {
            weld: Some(options),
            ..EditSettings::default()
        }))
    }
}
