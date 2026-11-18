//! The sculpting commands ([decision 0304](../../../../docs/decisions/0304-sculpt-mode-and-layers.md)):
//! a stroke's sculpt put in the object's top Sculpt layer, or in a new
//! Sculpt layer on top; and Remesh for sculpting ([decision 0312](../../../../docs/decisions/0312-remesh-for-sculpting.md)),
//! a new shape baked under a fresh Sculpt layer.

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
    /// `joined` go. Selects `id`.
    pub(super) fn remesh_for_sculpting(
        &mut self,
        id: ObjectId,
        shape: &Arc<mesh::Mesh>,
        materials: Option<&[crate::Material]>,
        joined: &[ObjectId],
    ) -> Result<Option<Vec<ObjectId>>, SceneError> {
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
                sculpt: sculpt::Sculpt::new(shape),
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
