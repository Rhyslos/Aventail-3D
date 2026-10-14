//! Version 7 of the project file's contents ([decision 0049](../../../../docs/decisions/0049-slit.md)):
//! version 6's, with recorded slits.
//!
//! Version 6's operations are stored inside version 7's as they were
//! (`OperationV7::V6`), so their checks are shared; the entry around each
//! operation is version 5's.

use super::v1::{FileOperation, Problem};
use super::v5::{EntryWith, ProjectWith};
use super::v6::OperationV6;
use crate::limits::Bounded;
use glam::Vec3;
use mesh::slit::MAX_LINE_POINTS;
use scene::{Eye, Operation, SlitLine};
use serde::{Deserialize, Serialize};

/// Version 7 of the file.
pub(crate) type ProjectV7 = ProjectWith<EntryV7>;

/// A recorded edit in version 7.
pub(crate) type EntryV7 = EntryWith<OperationV7>;

/// The recorded operations version 7 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV7 {
    /// Everything version 6 could store, stored the same way.
    V6(OperationV6),
    /// A slit: its line's points, where it is seen from, and whether it
    /// cuts through.
    Slit {
        points: Bounded<[f32; 3]>,
        eye: EyeV7,
        through: bool,
    },
}

/// Where a slit is seen from.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) enum EyeV7 {
    At([f32; 3]),
    Along([f32; 3]),
}

impl FileOperation for OperationV7 {
    fn to_file(op: &Operation) -> Option<Self> {
        Some(match op {
            Operation::Slit { line } => Self::Slit {
                points: line.points.iter().map(|p| p.to_array()).collect(),
                eye: match line.eye {
                    Eye::At(e) => EyeV7::At(e.to_array()),
                    Eye::Along(d) => EyeV7::Along(d.to_array()),
                },
                through: line.through,
            },
            _ => Self::V6(OperationV6::to_file(op)?),
        })
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        match self {
            Self::V6(op) => op.into_operation(),
            Self::Slit {
                points,
                eye,
                through,
            } => {
                if points.len() < 2 || points.len() > MAX_LINE_POINTS {
                    return Err(format!(
                        "its slit has {} points (2 to {MAX_LINE_POINTS} can be stored)",
                        points.len()
                    ));
                }
                let real = |p: [f32; 3]| p.iter().all(|x| x.is_finite());
                let eye = match eye {
                    EyeV7::At(e) if real(e) => Eye::At(Vec3::from_array(e)),
                    EyeV7::Along(d) if real(d) && Vec3::from_array(d).length() > 1e-6 => {
                        Eye::Along(Vec3::from_array(d))
                    }
                    _ => return Err("its slit's view isn't real numbers".into()),
                };
                if !points.iter().all(|p| real(*p)) {
                    return Err("one of its slit's points isn't real numbers".into());
                }
                Ok(Operation::Slit {
                    line: SlitLine {
                        points: points.into_iter().map(Vec3::from_array).collect(),
                        eye,
                        through,
                    },
                })
            }
        }
    }
}
