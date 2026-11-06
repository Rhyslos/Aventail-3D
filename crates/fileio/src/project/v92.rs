//! Version 92 of the project file ([decision 0248](../../../../docs/decisions/0248-uv-intents.md)):
//! version 91's, with UV intents set on pieces, one more kind of recorded
//! edit (`v37::OperationV37::SetUvIntent`, stored as [`IntentEditV92`]).
//! The intents themselves live in the shapes' polygon layers, saved as
//! every layer is. Files before version 92 hold none.

use super::v1::Problem;
use super::v2::{AnchorsV2, anchors_from_file, anchors_to_file};
use crate::limits::Bounded;
use mesh::uv_intents::UvIntent;
use scene::{FaceId, Operation};
use serde::{Deserialize, Serialize};
use words::tr;

/// An intent set, as stored: the polygons, which intent (0 none, 1 strip,
/// 2 density, 3 trim row, 4 upright), its two values, and the polygons'
/// anchors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct IntentEditV92 {
    pub(crate) faces: Bounded<u32>,
    pub(crate) kind: u8,
    pub(crate) a: f32,
    pub(crate) b: f32,
    pub(crate) anchors: Option<AnchorsV2>,
}

impl IntentEditV92 {
    /// The stored form of `op`, if it sets an intent.
    pub(crate) fn of(op: &Operation) -> Option<Self> {
        let Operation::SetUvIntent {
            faces,
            intent,
            anchors,
        } = op
        else {
            return None;
        };
        let (kind, a, b) = match *intent {
            UvIntent::None => (0, 0.0, 0.0),
            UvIntent::Strip => (1, 0.0, 0.0),
            UvIntent::Density(d) => (2, d, 0.0),
            UvIntent::TrimRow { bottom, top } => (3, bottom, top),
            UvIntent::Upright => (4, 0.0, 0.0),
        };
        Some(Self {
            faces: faces.iter().map(|f| f.get()).collect(),
            kind,
            a,
            b,
            anchors: anchors.as_ref().map(anchors_to_file),
        })
    }

    /// The edit read back; an unknown intent, or values it can't have,
    /// are refused.
    pub(crate) fn into_operation(self) -> Result<Operation, Problem> {
        let intent = match self.kind {
            0 => UvIntent::None,
            1 => UvIntent::Strip,
            2 => UvIntent::Density(self.a),
            3 => UvIntent::TrimRow {
                bottom: self.a,
                top: self.b,
            },
            4 => UvIntent::Upright,
            _ => return Err(tr("an unknown kind of UV intent").into()),
        };
        if !intent.is_real() {
            return Err(tr("a UV intent out of range").into());
        }
        Ok(Operation::SetUvIntent {
            faces: self.faces.into_iter().map(FaceId::from_raw).collect(),
            intent,
            anchors: self.anchors.map(anchors_from_file).transpose()?,
        })
    }
}
