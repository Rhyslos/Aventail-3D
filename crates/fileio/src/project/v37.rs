//! Version 37 of the project file's contents ([decision 0112](../../../../docs/decisions/0112-locked-pieces-and-piece-settings.md)):
//! version 36's, with locked pieces and pieces' own unwrap settings.
//!
//! Two more kinds of recorded edit: polygons' UVs locked or unlocked, and
//! polygons given their pieces' own unwrap settings (a way of laying flat,
//! straightening, a texel density; each or none). Everything else is
//! stored as in version 36.
//!
//! Version 44 ([decision 0143](../../../../docs/decisions/0143-subdivide-and-creases.md))
//! adds one more kind of recorded edit here, edges given a crease (or
//! theirs taken away): files before version 44 never hold one.
//!
//! Version 50 ([decision 0149](../../../../docs/decisions/0149-live-bevel.md))
//! adds edges given a bevel weight (or theirs taken away): files before
//! version 50 never hold one.
//!
//! Version 51 ([decision 0150](../../../../docs/decisions/0150-quadify.md))
//! adds the one-off Quadify: files before version 51 never hold one.
//!
//! Version 53 ([decision 0152](../../../../docs/decisions/0152-weld-by-distance.md))
//! adds the one-off Weld by distance: files before version 53 never hold
//! one.
//!
//! Version 77 ([decision 0205](../../../../docs/decisions/0205-vertex-colour-painting.md))
//! adds painted vertex colours: files before version 77 never hold one.

use super::v1::{FileOperation, Problem};
use super::v2::{AnchorsV2, anchors_from_file, anchors_to_file};
use super::v5::EntryWith;
use super::v34::{MethodV34, method_from_file, method_to_file};
use super::v36::OperationV36;
use crate::limits::Bounded;
use scene::{EdgeKey, FaceId, Operation, PieceSettings, VertexId};
use serde::{Deserialize, Serialize};
use words::tr;

/// A recorded edit in version 37.
pub(crate) type EntryV37 = EntryWith<OperationV37>;

/// The recorded operations version 37 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV37 {
    /// Everything version 36 could store, stored the same way.
    V36(OperationV36),
    /// Polygons' UVs locked or unlocked.
    SetUvLock {
        faces: Bounded<u32>,
        lock: bool,
        anchors: Option<AnchorsV2>,
    },
    /// Polygons given their pieces' own unwrap settings.
    SetPieceSettings {
        faces: Bounded<u32>,
        method: Option<MethodV34>,
        strips: Option<bool>,
        density: Option<f32>,
        anchors: Option<AnchorsV2>,
    },
    /// Edges given a crease, 0 to 1 (version 44, decision 0143).
    SetCreases {
        edges: Bounded<[u32; 2]>,
        crease: f32,
        anchors: Option<AnchorsV2>,
    },
    /// Edges given a bevel weight, 0 to 1 (version 50, decision 0149).
    SetBevelWeights {
        edges: Bounded<[u32; 2]>,
        weight: f32,
        anchors: Option<AnchorsV2>,
    },
    /// Triangles joined into quads (version 51, decision 0150).
    Quadify(super::v51::QuadifyEditV51),
    /// Close points welded into one (version 53, decision 0152).
    WeldByDistance(super::v53::WeldEditV53),
    /// Vertex colours painted (version 77, decision 0205).
    PaintColours(super::v77::ColoursEditV77),
}

/// Edges as stored: each as its two points.
fn edges_to_file(edges: &[EdgeKey]) -> Bounded<[u32; 2]> {
    edges
        .iter()
        .map(|e| {
            let (a, b) = e.ends();
            [a.get(), b.get()]
        })
        .collect()
}

/// Edges read back; one from a point to itself is refused with `why`.
fn edges_from_file(edges: Bounded<[u32; 2]>, why: &str) -> Result<Vec<EdgeKey>, Problem> {
    edges
        .into_iter()
        .map(|[a, b]| {
            if a == b {
                Err(why.to_string())
            } else {
                Ok(EdgeKey::new(VertexId::from_raw(a), VertexId::from_raw(b)))
            }
        })
        .collect::<Result<Vec<_>, _>>()
}

impl FileOperation for OperationV37 {
    fn to_file(op: &Operation) -> Option<Self> {
        match op {
            Operation::SetUvLock {
                faces,
                lock,
                anchors,
            } => Some(Self::SetUvLock {
                faces: faces.iter().map(|f| f.get()).collect(),
                lock: *lock,
                anchors: anchors.as_ref().map(anchors_to_file),
            }),
            Operation::SetPieceSettings {
                faces,
                settings,
                anchors,
            } => Some(Self::SetPieceSettings {
                faces: faces.iter().map(|f| f.get()).collect(),
                method: settings.method.map(method_to_file),
                strips: settings.strips,
                density: settings.density,
                anchors: anchors.as_ref().map(anchors_to_file),
            }),
            Operation::SetCreases {
                edges,
                crease,
                anchors,
            } => Some(Self::SetCreases {
                edges: edges_to_file(edges),
                crease: *crease,
                anchors: anchors.as_ref().map(anchors_to_file),
            }),
            Operation::SetBevelWeights {
                edges,
                weight,
                anchors,
            } => Some(Self::SetBevelWeights {
                edges: edges_to_file(edges),
                weight: *weight,
                anchors: anchors.as_ref().map(anchors_to_file),
            }),
            Operation::Quadify { .. } => super::v51::QuadifyEditV51::of(op).map(Self::Quadify),
            Operation::WeldByDistance { .. } => {
                super::v53::WeldEditV53::of(op).map(Self::WeldByDistance)
            }
            Operation::PaintColours { .. } => {
                super::v77::ColoursEditV77::of(op).map(Self::PaintColours)
            }
            _ => Some(Self::V36(OperationV36::to_file(op)?)),
        }
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        let operation = match self {
            Self::V36(op) => return op.into_operation(),
            Self::Quadify(edit) => return edit.into_operation(),
            Self::WeldByDistance(edit) => return edit.into_operation(),
            Self::PaintColours(edit) => return edit.into_operation(),
            Self::SetUvLock {
                faces,
                lock,
                anchors,
            } => Operation::SetUvLock {
                faces: faces.into_iter().map(FaceId::from_raw).collect(),
                lock,
                anchors: anchors.map(anchors_from_file).transpose()?,
            },
            Self::SetPieceSettings {
                faces,
                method,
                strips,
                density,
                anchors,
            } => Operation::SetPieceSettings {
                faces: faces.into_iter().map(FaceId::from_raw).collect(),
                settings: PieceSettings {
                    method: method.map(method_from_file),
                    strips,
                    density,
                },
                anchors: anchors.map(anchors_from_file).transpose()?,
            },
            Self::SetCreases {
                edges,
                crease,
                anchors,
            } => {
                let edges =
                    edges_from_file(edges, tr("one of its creases joins a point to itself"))?;
                let operation = Operation::SetCreases {
                    edges,
                    crease,
                    anchors: anchors.map(anchors_from_file).transpose()?,
                };
                if !operation.numbers_are_real() {
                    return Err(tr("a crease isn't a number from 0 to 1").into());
                }
                return Ok(operation);
            }
            Self::SetBevelWeights {
                edges,
                weight,
                anchors,
            } => {
                let edges = edges_from_file(
                    edges,
                    tr("one of its bevel weights joins a point to itself"),
                )?;
                let operation = Operation::SetBevelWeights {
                    edges,
                    weight,
                    anchors: anchors.map(anchors_from_file).transpose()?,
                };
                if !operation.numbers_are_real() {
                    return Err(tr("a bevel weight isn't a number from 0 to 1").into());
                }
                return Ok(operation);
            }
        };
        if !operation.numbers_are_real() {
            return Err(tr("a piece's texel density isn't a real number above 0").into());
        }
        Ok(operation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scene::UnwrapMethod;

    #[test]
    fn locks_and_piece_settings_survive_the_trip() {
        let faces = vec![FaceId::from_raw(1), FaceId::from_raw(4)];
        let ops = [
            Operation::SetUvLock {
                faces: faces.clone(),
                lock: true,
                anchors: None,
            },
            Operation::SetUvLock {
                faces: faces.clone(),
                lock: false,
                anchors: None,
            },
            Operation::SetPieceSettings {
                faces: faces.clone(),
                settings: PieceSettings {
                    method: Some(UnwrapMethod::AngleBased),
                    strips: Some(false),
                    density: Some(0.05),
                },
                anchors: None,
            },
            Operation::SetPieceSettings {
                faces,
                settings: PieceSettings::default(),
                anchors: None,
            },
        ];
        for op in ops {
            let file = OperationV37::to_file(&op).unwrap();
            assert!(!matches!(file, OperationV37::V36(_)), "{op:?}");
            assert_eq!(file.into_operation().unwrap(), op);
            assert!(OperationV36::to_file(&op).is_none(), "older versions can't");
        }
    }

    #[test]
    fn a_density_that_isnt_real_is_refused() {
        for density in [0.0, -1.0, f32::NAN] {
            let bad = OperationV37::SetPieceSettings {
                faces: Bounded::default(),
                method: None,
                strips: None,
                density: Some(density),
                anchors: None,
            };
            assert!(bad.into_operation().is_err(), "{density}");
        }
    }
}
