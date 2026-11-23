//! The sculpting commands ([decision 0304](../../../../docs/decisions/0304-sculpt-mode-and-layers.md)):
//! a stroke's sculpt put in the object's top Sculpt layer, or in a new
//! Sculpt layer on top; Remesh for sculpting ([decision 0312](../../../../docs/decisions/0312-remesh-for-sculpting.md)),
//! a new shape baked under a fresh Sculpt layer; and a sculpt put on a
//! new shape of its own ([decision 0315](../../../../docs/decisions/0315-detail-levels.md):
//! Delete lower levels and Rebuild lower levels).

use super::Run;
use crate::object::{ObjectId, ObjectKind};
use crate::stack::{Entry, Layer, Operation};
use crate::tree::{Edit, SceneError};
use std::sync::Arc;

impl Run<'_> {
    /// Puts `sculpt` on object `id` (see `Command::Sculpt`).
    pub(super) fn sculpt(
        &mut self,
        id: ObjectId,
        sculpt: &sculpt::Sculpt,
    ) -> Result<(), SceneError> {
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

impl Run<'_> {
    /// Remesh for sculpting (see `Command::RemeshForSculpting`): `shape`
    /// becomes object `id`'s base, under a fresh Sculpt layer; the objects
    /// `joined` go. Selects `id`. With `sculpt` (see
    /// `Command::RebaseSculpt`), the Sculpt layer holds it instead of
    /// starting empty.
    pub(super) fn remesh_for_sculpting(
        &mut self,
        id: ObjectId,
        shape: &Arc<mesh::Mesh>,
        materials: Option<&[crate::Material]>,
        joined: &[ObjectId],
        sculpt: Option<&sculpt::Sculpt>,
    ) -> Result<Option<Vec<ObjectId>>, SceneError> {
        if sculpt.is_some_and(|s| !s.is_valid()) {
            return Err(SceneError::NotANumber);
        }
        if shape.face_count() == 0 {
            return Err(SceneError::NoShape);
        }
        let object = self.scene.object(id).ok_or(SceneError::UnknownObject(id))?;
        match object.kind {
            ObjectKind::Mesh(_) | ObjectKind::Primitive(_) => {}
            ObjectKind::Instance(_) => return Err(SceneError::Instance),
            ObjectKind::Curve(_) => {
                return Err(SceneError::Curve(crate::curve::RECORD_ON_CURVE));
            }
            _ => return Err(SceneError::NoShape),
        }
        if joined.contains(&id) {
            return Err(SceneError::NestedInItself);
        }
        if let Some(after) = materials {
            let before = object.materials.clone();
            if before != after {
                self.edit(Edit::SetMaterials {
                    id,
                    before,
                    after: after.to_vec(),
                })?;
            }
        }
        let object = self.scene.object(id).ok_or(SceneError::UnknownObject(id))?;
        let before = object.kind.clone();
        let layers = object.stack.layers().to_vec();
        // Top layer first, so every removal is of the top one.
        for (index, layer) in layers.into_iter().enumerate().rev() {
            self.edit(Edit::RemoveLayer { id, index, layer })?;
        }
        self.edit(Edit::SetKind {
            id,
            before,
            after: ObjectKind::Mesh(Arc::clone(shape)),
        })?;
        let stack = &self
            .scene
            .object(id)
            .ok_or(SceneError::UnknownObject(id))?
            .stack;
        let mut layer = Layer::new(stack.next_sculpt_name());
        layer.entries.push(Entry::new(
            stack.next_edit_id(),
            Operation::Sculpt {
                // A sculpt given for another shape is fitted to this one.
                sculpt: match sculpt {
                    Some(sculpt) => sculpt.fitted_to(shape).0,
                    None => sculpt::Sculpt::new(shape),
                },
            },
        ));
        self.edit(Edit::InsertLayer {
            id,
            index: 0,
            layer,
        })?;
        self.delete_joined(joined)?;
        Ok(Some(vec![id]))
    }
}
