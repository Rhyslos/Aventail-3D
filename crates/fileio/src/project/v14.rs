//! Version 14 of the project file's contents ([decision 0059](../../../../docs/decisions/0059-symmetry-for-every-edit.md)):
//! version 13's, with edits made with symmetry on.
//!
//! Version 13's operations are stored inside version 14's as they were
//! (`OperationV14::V13`); a mirrored edit holds a version 13 operation, so
//! mirrored edits never nest. The file around them is version 12's, with
//! its instances.

use super::v1::{FileOperation, Problem};
use super::v5::EntryWith;
use super::v12::ProjectWithInstances;
use super::v13::OperationV13;
use glam::Vec3;
use mesh::symmetry::Mirror;
use scene::Operation;
use serde::{Deserialize, Serialize};

/// Version 14 of the file.
pub(crate) type ProjectV14 = ProjectWithInstances<EntryV14>;

/// A recorded edit in version 14.
pub(crate) type EntryV14 = EntryWith<OperationV14>;

/// The recorded operations version 14 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV14 {
    /// Everything version 13 could store, stored the same way.
    V13(OperationV13),
    /// An edit made with symmetry on: the edit, the axes mirrored across,
    /// the point the planes go through, the weld distance (0: off) and
    /// whether points are kept on their side.
    Mirrored {
        op: OperationV13,
        axes: [bool; 3],
        centre: [f32; 3],
        weld: f32,
        clip: bool,
    },
}

impl FileOperation for OperationV14 {
    fn to_file(op: &Operation) -> Option<Self> {
        Some(match op {
            Operation::Mirrored { op, mirror } => Self::Mirrored {
                op: OperationV13::to_file(op)?,
                axes: mirror.axes,
                centre: mirror.centre.to_array(),
                weld: mirror.weld,
                clip: mirror.clip,
            },
            _ => Self::V13(OperationV13::to_file(op)?),
        })
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        match self {
            Self::V13(op) => op.into_operation(),
            Self::Mirrored {
                op,
                axes,
                centre,
                weld,
                clip,
            } => {
                let centre = Vec3::from_array(centre);
                if !centre.is_finite() {
                    return Err("a mirror's middle isn't a real number".to_string());
                }
                if !weld.is_finite() || weld < 0.0 {
                    return Err("a mirror's weld distance isn't a real distance".to_string());
                }
                if !axes.contains(&true) {
                    return Err("a mirrored edit mirrors across no axis".to_string());
                }
                let op = op.into_operation()?;
                if !scene::mirroring::mirrors(&op) {
                    return Err("an edit that can't be mirrored is marked mirrored".to_string());
                }
                Ok(Operation::Mirrored {
                    op: Box::new(op),
                    mirror: Mirror {
                        axes,
                        centre,
                        weld: weld.min(mesh::primitives::MAX_SIZE),
                        clip,
                    },
                })
            }
        }
    }
}
