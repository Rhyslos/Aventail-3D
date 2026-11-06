//! Version 89 of the project file ([decision 0241](../../../../docs/decisions/0241-uv-sets.md)):
//! version 88's, with changes to shapes' UV sets, one more kind of
//! recorded edit (`v37::OperationV37::UvSet`, stored as [`UvSetEditV89`]).
//! The sets themselves live in the shapes' corner layers, saved as every
//! layer is. Files before version 89 hold none.

use super::v1::Problem;
use mesh::uv_sets::SetChange;
use scene::Operation;
use serde::{Deserialize, Serialize};
use words::tr;

/// A UV set change as stored: what (0 add, 1 copy, 2 rename, 3 delete, 4
/// make active), and its one or two names.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct UvSetEditV89 {
    pub(crate) kind: u8,
    pub(crate) name: String,
    pub(crate) other: String,
}

impl UvSetEditV89 {
    /// The stored form of `op`, if it changes UV sets.
    pub(crate) fn of(op: &Operation) -> Option<Self> {
        let Operation::UvSet { change } = op else {
            return None;
        };
        let (kind, name, other) = match change {
            SetChange::Add(n) => (0, n.clone(), String::new()),
            SetChange::Copy { from, to } => (1, from.clone(), to.clone()),
            SetChange::Rename { from, to } => (2, from.clone(), to.clone()),
            SetChange::Delete(n) => (3, n.clone(), String::new()),
            SetChange::MakeActive(n) => (4, n.clone(), String::new()),
        };
        Some(Self { kind, name, other })
    }

    /// The edit read back; an unknown kind or a name that can't name a
    /// set is refused.
    pub(crate) fn into_operation(self) -> Result<Operation, Problem> {
        let change = match self.kind {
            0 => SetChange::Add(self.name),
            1 => SetChange::Copy {
                from: self.name,
                to: self.other,
            },
            2 => SetChange::Rename {
                from: self.name,
                to: self.other,
            },
            3 => SetChange::Delete(self.name),
            4 => SetChange::MakeActive(self.name),
            _ => return Err(tr("a UV set change of an unknown kind").into()),
        };
        if !change.is_real() {
            return Err(tr("a UV set with a name that can't be one").into());
        }
        Ok(Operation::UvSet { change })
    }
}
