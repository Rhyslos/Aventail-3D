//! The sculpting command ([decision 0304](../../../../docs/decisions/0304-sculpt-mode-and-layers.md)):
//! a stroke's sculpt put in the object's top Sculpt layer, or in a new
//! Sculpt layer on top.

use super::Run;
use crate::object::{ObjectId, ObjectKind};
use crate::stack::{Entry, Layer, Operation};
use crate::tree::{Edit, SceneError};

impl Run<'_> {
    /// Puts `sculpt` on object `id` (see `Command::Sculpt`).
    pub(super) fn sculpt(&mut self, id: ObjectId, sculpt: &sculpt::Sculpt) -> Result<(), SceneError> {
        if !sculpt.is_valid() {
            return Err(SceneError::NotANumber);
        }
        let object = self.scene.object(id).ok_or(SceneError::UnknownObject(id))?;
        if !object.kind.has_shape() {
            return Err(SceneError::NoShape);
        }
        match object.kind {
            ObjectKind::Instance(_) => return Err(SceneError::Instance),
            ObjectKind::Curve(_) => {
                return Err(SceneError::Curve(crate::curve::RECORD_ON_CURVE));
            }
            _ => {}
        }
        let operation = Operation::Sculpt {
            sculpt: sculpt.clone(),
        };
        let stack = &object.stack;
        match stack.sculpt_layer() {
            Some(layer) => {
                let before = stack.layers()[layer].entries[0].clone();
                if before.operation == operation {
                    return Ok(());
                }
                let after = Entry {
                    operation,
                    ..before.clone()
                };
                self.swap_entry(id, layer, 0, before, after)
            }
            None => {
                let index = stack.layers().len();
                let mut layer = Layer::new(stack.next_sculpt_name());
                layer
                    .entries
                    .push(Entry::new(stack.next_edit_id(), operation));
                self.edit(Edit::InsertLayer { id, index, layer })
            }
        }
    }
}
