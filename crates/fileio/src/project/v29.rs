//! Version 29 of the project file's contents ([decision 0098](../../../../docs/decisions/0098-seams.md)):
//! version 28's, with seams marked and cleared.
//!
//! One more kind of recorded edit: edges marked as seams or cleared, on
//! their own or mirrored. Everything else is stored as in version 28.

use super::v1::{FileOperation, Problem};
use super::v2::{AnchorsV2, anchors_from_file, anchors_to_file};
use super::v5::EntryWith;
use super::v28::OperationV28;
use crate::limits::Bounded;
use glam::Vec3;
use mesh::symmetry::Mirror;
use scene::{EdgeKey, Operation, VertexId};
use serde::{Deserialize, Serialize};

/// A recorded edit in version 29.
pub(crate) type EntryV29 = EntryWith<OperationV29>;

/// Version 29 of the file: version 27's, its edits stored as version 29's.
pub(crate) type ProjectV29 = super::v27::ProjectV27<EntryV29>;

/// The recorded operations version 29 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV29 {
    /// Everything version 28 could store, stored the same way.
    V28(OperationV28),
    /// Edges marked as seams, or cleared.
    SetSeams(SeamsV29),
    /// The same, made with symmetry on: the edit, the axes mirrored
    /// across, the point the planes go through and whether points are
    /// kept on their side.
    MirroredSetSeams {
        op: SeamsV29,
        axes: [bool; 3],
        centre: [f32; 3],
        clip: bool,
    },
}

/// Seams marked or cleared: the edges (each two point ids), mark or
/// clear, and anchors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct SeamsV29 {
    edges: Bounded<[u32; 2]>,
    seam: bool,
    anchors: Option<AnchorsV2>,
}

impl SeamsV29 {
    fn of(op: &Operation) -> Option<Self> {
        let Operation::SetSeams {
            edges,
            seam,
            anchors,
        } = op
        else {
            return None;
        };
        Some(Self {
            edges: edges
                .iter()
                .map(|e| {
                    let (a, b) = e.ends();
                    [a.get(), b.get()]
                })
                .collect(),
            seam: *seam,
            anchors: anchors.as_ref().map(anchors_to_file),
        })
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        let edges = self
            .edges
            .into_iter()
            .map(|[a, b]| {
                if a == b {
                    Err("one of its seams joins a point to itself".to_string())
                } else {
                    Ok(EdgeKey::new(VertexId::from_raw(a), VertexId::from_raw(b)))
                }
            })
            .collect::<Result<_, _>>()?;
        Ok(Operation::SetSeams {
            edges,
            seam: self.seam,
            anchors: self.anchors.map(anchors_from_file).transpose()?,
        })
    }
}

impl FileOperation for OperationV29 {
    fn to_file(op: &Operation) -> Option<Self> {
        if let Some(seams) = SeamsV29::of(op) {
            return Some(Self::SetSeams(seams));
        }
        if let Operation::Mirrored { op, mirror } = op
            && let Some(seams) = SeamsV29::of(op)
        {
            return Some(Self::MirroredSetSeams {
                op: seams,
                axes: mirror.axes,
                centre: mirror.centre.to_array(),
                clip: mirror.clip,
            });
        }
        Some(Self::V28(OperationV28::to_file(op)?))
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        match self {
            Self::V28(op) => op.into_operation(),
            Self::SetSeams(seams) => seams.into_operation(),
            Self::MirroredSetSeams {
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

    fn seams(seam: bool) -> Operation {
        Operation::SetSeams {
            edges: vec![
                EdgeKey::new(VertexId::from_raw(2), VertexId::from_raw(5)),
                EdgeKey::new(VertexId::from_raw(5), VertexId::from_raw(9)),
            ],
            seam,
            anchors: None,
        }
    }

    #[test]
    fn seams_survive_the_trip_on_their_own_and_mirrored() {
        for op in [
            seams(true),
            seams(false),
            Operation::Mirrored {
                op: Box::new(seams(true)),
                mirror: Mirror {
                    axes: [true, false, false],
                    centre: Vec3::ZERO,
                    clip: false,
                },
            },
        ] {
            let file = OperationV29::to_file(&op).unwrap();
            assert!(!matches!(file, OperationV29::V28(_)));
            assert_eq!(file.into_operation().unwrap(), op);
            assert!(OperationV28::to_file(&op).is_none(), "older versions can't");
        }
    }

    #[test]
    fn broken_seams_are_refused() {
        let bad_edge = OperationV29::SetSeams(SeamsV29 {
            edges: vec![[4, 4]].into_iter().collect(),
            seam: true,
            anchors: None,
        });
        assert!(bad_edge.into_operation().is_err());
        let no_axis = OperationV29::MirroredSetSeams {
            op: SeamsV29 {
                edges: Bounded::default(),
                seam: true,
                anchors: None,
            },
            axes: [false; 3],
            centre: [0.0; 3],
            clip: false,
        };
        assert!(no_axis.into_operation().is_err());
    }
}
