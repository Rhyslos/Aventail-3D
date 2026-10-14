//! Version 13 of the project file's contents ([decision 0055](../../../../docs/decisions/0055-basics.md)):
//! version 12's, with slid points.
//!
//! Version 12's operations are stored inside version 13's as they were
//! (`OperationV13::V12`); the file around them is version 12's, with its
//! instances.

use super::v1::{FileOperation, Problem};
use super::v2::{AnchorsV2, anchors_from_file, anchors_to_file};
use super::v5::EntryWith;
use super::v12::OperationV12;
use crate::limits::Bounded;
use mesh::VertexId;
use scene::{Operation, Slide};
use serde::{Deserialize, Serialize};

/// Version 13 of the file.
#[cfg(test)]
pub(crate) type ProjectV13 = super::v12::ProjectWithInstances<EntryV13>;

/// A recorded edit in version 13.
pub(crate) type EntryV13 = EntryWith<OperationV13>;

/// The recorded operations version 13 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV13 {
    /// Everything version 12 could store, stored the same way.
    V12(OperationV12),
    /// Each point, its neighbour and how far along the edge.
    SlidePoints {
        slides: Bounded<(u32, u32, f32)>,
        anchors: Option<AnchorsV2>,
    },
}

impl FileOperation for OperationV13 {
    fn to_file(op: &Operation) -> Option<Self> {
        Some(match op {
            Operation::SlidePoints { slides, .. } => Self::SlidePoints {
                slides: slides
                    .iter()
                    .map(|s| (s.point.get(), s.towards.get(), s.amount))
                    .collect(),
                anchors: op.anchors().map(anchors_to_file),
            },
            _ => Self::V12(OperationV12::to_file(op)?),
        })
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        match self {
            Self::V12(op) => op.into_operation(),
            Self::SlidePoints { slides, anchors } => {
                let slides = slides
                    .into_iter()
                    .map(|(point, towards, amount)| {
                        if !amount.is_finite() {
                            return Err("a slide's amount isn't a real number".to_string());
                        }
                        if point == towards {
                            return Err("a point slides towards itself".to_string());
                        }
                        Ok(Slide {
                            point: VertexId::from_raw(point),
                            towards: VertexId::from_raw(towards),
                            amount: amount.clamp(0.0, 1.0),
                        })
                    })
                    .collect::<Result<_, _>>()?;
                Ok(Operation::SlidePoints {
                    slides,
                    anchors: anchors.map(anchors_from_file).transpose()?,
                })
            }
        }
    }
}
