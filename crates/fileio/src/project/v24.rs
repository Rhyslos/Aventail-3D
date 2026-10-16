//! Version 24 of the project file's contents ([decision 0086](../../../../docs/decisions/0086-groove.md)):
//! version 23's, with grooves.
//!
//! One more kind of recorded edit: a groove (or ridge) along edges, with
//! its width, depth and sides, the stretch of the object it was made on,
//! and its anchors; on its own or mirrored. Everything else is stored as
//! in version 23.

use super::v1::{FileOperation, Problem};
use super::v2::{AnchorsV2, anchors_from_file, anchors_to_file};
use super::v5::EntryWith;

use super::v23::OperationV23;
use glam::Vec3;
use mesh::edit::{GrooveShape, GrooveSides};
use mesh::symmetry::Mirror;
use mesh::{EdgeKey, VertexId};
use scene::Operation;
use serde::{Deserialize, Serialize};

/// Version 24 of the file: version 20's (to 23's), its edits stored as
/// version 24's.
#[cfg(test)]
pub(crate) type ProjectV24 = super::v20::ProjectV20<EntryV24>;

/// A recorded edit in version 24.
pub(crate) type EntryV24 = EntryWith<OperationV24>;

/// The recorded operations version 24 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV24 {
    /// Everything version 23 could store, stored the same way.
    V23(OperationV23),
    /// A groove along edges.
    Groove(GrooveV24),
    /// The same, made with symmetry on: the edit, the axes mirrored
    /// across, the point the planes go through and whether points are
    /// kept on their side.
    MirroredGroove {
        op: GrooveV24,
        axes: [bool; 3],
        centre: [f32; 3],
        clip: bool,
    },
}

/// A groove: its edges (each two point ids), width, depth, whether its
/// sides slope, the object's stretch, and its anchors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct GrooveV24 {
    edges: crate::limits::Bounded<[u32; 2]>,
    width: f32,
    depth: f32,
    sloped: bool,
    stretch: [f32; 3],
    anchors: Option<AnchorsV2>,
}

impl GrooveV24 {
    fn of(op: &Operation) -> Option<Self> {
        let Operation::Groove {
            edges,
            shape,
            stretch,
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
            width: shape.width,
            depth: shape.depth,
            sloped: shape.sides == GrooveSides::Sloped,
            stretch: stretch.to_array(),
            anchors: anchors.as_ref().map(anchors_to_file),
        })
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        if !(self.width.is_finite() && self.depth.is_finite()) {
            return Err("a groove's width or depth isn't a real number".into());
        }
        let stretch = Vec3::from_array(self.stretch);
        if mesh::stretch::usable(stretch) != stretch {
            return Err("a groove's stretch isn't a real number, or is out of range".into());
        }
        let edges = self
            .edges
            .into_iter()
            .map(|[a, b]| {
                if a == b {
                    Err("one of a groove's edges joins a point to itself".to_string())
                } else {
                    Ok(EdgeKey::new(VertexId::from_raw(a), VertexId::from_raw(b)))
                }
            })
            .collect::<Result<_, _>>()?;
        let shape = GrooveShape {
            width: self.width,
            depth: self.depth,
            sides: if self.sloped {
                GrooveSides::Sloped
            } else {
                GrooveSides::Straight
            },
        }
        .clamped()
        .ok_or_else(|| "a groove's width or depth isn't a real number".to_string())?;
        Ok(Operation::Groove {
            edges,
            shape,
            stretch,
            anchors: self.anchors.map(anchors_from_file).transpose()?,
        })
    }
}

impl FileOperation for OperationV24 {
    fn to_file(op: &Operation) -> Option<Self> {
        if let Some(groove) = GrooveV24::of(op) {
            return Some(Self::Groove(groove));
        }
        if let Operation::Mirrored { op, mirror } = op
            && let Some(groove) = GrooveV24::of(op)
        {
            return Some(Self::MirroredGroove {
                op: groove,
                axes: mirror.axes,
                centre: mirror.centre.to_array(),
                clip: mirror.clip,
            });
        }
        Some(Self::V23(OperationV23::to_file(op)?))
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        match self {
            Self::V23(op) => op.into_operation(),
            Self::Groove(groove) => groove.into_operation(),
            Self::MirroredGroove {
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

    fn groove() -> Operation {
        Operation::Groove {
            edges: vec![EdgeKey::new(VertexId::from_raw(2), VertexId::from_raw(5))],
            shape: GrooveShape {
                width: 1.5,
                depth: 0.75,
                sides: GrooveSides::Sloped,
            },
            stretch: Vec3::new(0.8, 1.2, 1.0),
            anchors: None,
        }
    }

    #[test]
    fn a_groove_survives_the_trip_on_its_own_and_mirrored() {
        for op in [
            groove(),
            Operation::Mirrored {
                op: Box::new(groove()),
                mirror: Mirror {
                    axes: [false, true, false],
                    centre: Vec3::ZERO,
                    clip: true,
                },
            },
        ] {
            let file = OperationV24::to_file(&op).unwrap();
            assert!(!matches!(file, OperationV24::V23(_)));
            assert_eq!(file.into_operation().unwrap(), op);
        }
        // Older versions can't store one.
        assert!(OperationV23::to_file(&groove()).is_none());
    }

    #[test]
    fn broken_grooves_are_refused() {
        let Some(good) = GrooveV24::of(&groove()) else {
            panic!("a groove");
        };
        let cases = [
            GrooveV24 {
                width: f32::NAN,
                ..good.clone()
            },
            GrooveV24 {
                depth: f32::INFINITY,
                ..good.clone()
            },
            GrooveV24 {
                stretch: [0.0, 1.0, 1.0],
                ..good.clone()
            },
            GrooveV24 {
                edges: vec![[3, 3]].into_iter().collect(),
                ..good.clone()
            },
        ];
        for case in cases {
            assert!(case.into_operation().is_err());
        }
        let wide = GrooveV24 { width: 1e9, ..good };
        let Operation::Groove { shape, .. } = wide.into_operation().unwrap() else {
            panic!("a groove");
        };
        assert_eq!(shape.width, mesh::groove::MAX_GROOVE_WIDTH);
    }
}
