//! The commands that set up, change and bake booleans
//! ([decision 0135](../../../../docs/decisions/0135-live-booleans.md)).

use super::Run;
use crate::boolean::{Boolean, BooleanKind, BooleanRefusal};
use crate::object::ObjectId;
use crate::tree::{Edit, SceneError};
use mesh::Mesh;
use std::sync::Arc;

impl Run<'_> {
    /// Cuts `kept` with `cutters` (`kind`): added to its last boolean if
    /// that is the same kind, else as a new one. Selects `kept`.
    pub(super) fn add_boolean(
        &mut self,
        kept: ObjectId,
        cutters: &[ObjectId],
        kind: BooleanKind,
    ) -> Result<Option<Vec<ObjectId>>, SceneError> {
        let object = self
            .scene
            .object(kept)
            .ok_or(SceneError::UnknownObject(kept))?;
        let mut list: Vec<ObjectId> = Vec::new();
        for cutter in cutters {
            if !self.scene.contains(*cutter) {
                return Err(SceneError::UnknownObject(*cutter));
            }
            if !list.contains(cutter) {
                list.push(*cutter);
            }
        }
        if list.is_empty() {
            return Err(SceneError::Boolean(BooleanRefusal::NoCutter));
        }
        let before = object.booleans.clone();
        let mut after = before.clone();
        match after.last_mut() {
            Some(last) if last.kind == kind => {
                for cutter in list {
                    if !last.cutters.contains(&cutter) {
                        last.cutters.push(cutter);
                    }
                }
            }
            _ => after.push(Boolean {
                kind,
                cutters: list,
            }),
        }
        self.set_booleans(kept, after)?;
        Ok(Some(vec![kept]))
    }

    /// Gives `id` these booleans (empty ones left out), if they make
    /// sense ([`crate::Scene::check_booleans`]).
    pub(super) fn set_booleans(
        &mut self,
        id: ObjectId,
        booleans: Vec<Boolean>,
    ) -> Result<(), SceneError> {
        let object = self.scene.object(id).ok_or(SceneError::UnknownObject(id))?;
        let after: Vec<Boolean> = booleans
            .into_iter()
            .filter(|b| !b.cutters.is_empty())
            .collect();
        if object.booleans == after {
            return Ok(());
        }
        self.scene
            .check_booleans(id, &after)
            .map_err(SceneError::Boolean)?;
        let before = object.booleans.clone();
        self.edit(Edit::SetBooleans { id, before, after })
    }

    /// Takes `going` (objects about to be deleted) out of every other
    /// object's booleans.
    pub(super) fn forget_cutters(&mut self, going: &[ObjectId]) -> Result<(), SceneError> {
        let users: Vec<ObjectId> = self
            .scene
            .walk()
            .into_iter()
            .map(|(id, _)| id)
            .filter(|id| !going.contains(id))
            .filter(|id| {
                self.scene
                    .booleans(*id)
                    .iter()
                    .any(|b| b.cutters.iter().any(|c| going.contains(c)))
            })
            .collect();
        for id in users {
            let mut after = self.scene.booleans(id).to_vec();
            for boolean in &mut after {
                boolean.cutters.retain(|c| !going.contains(c));
            }
            after.retain(|b| !b.cutters.is_empty());
            let before = self.scene.booleans(id).to_vec();
            self.edit(Edit::SetBooleans { id, before, after })?;
        }
        Ok(())
    }

    /// Takes out of `id`'s booleans every cutter that isn't a shape in the
    /// scene or would make a loop (and every boolean of a group or an
    /// instance): for an object put in from elsewhere.
    pub(super) fn keep_usable_cutters(&mut self, id: ObjectId) -> Result<(), SceneError> {
        let before = self.scene.booleans(id).to_vec();
        if before.is_empty() {
            return Ok(());
        }
        let mut after = before.clone();
        if self.scene.check_booleans(id, &after).is_err() {
            for boolean in &mut after {
                boolean.cutters.retain(|c| {
                    let one = [Boolean {
                        kind: boolean.kind,
                        cutters: vec![*c],
                    }];
                    self.scene.check_booleans(id, &one).is_ok()
                });
            }
            after.retain(|b| !b.cutters.is_empty());
            if self.scene.check_booleans(id, &after).is_err() {
                after.clear();
            }
        }
        if after != before {
            self.edit(Edit::SetBooleans { id, before, after })?;
        }
        Ok(())
    }

    /// The object's cut shape, if it has booleans and they work.
    pub(super) fn cut_result(&self, id: ObjectId) -> Option<Arc<Mesh>> {
        if self.scene.booleans(id).is_empty() {
            return None;
        }
        self.scene.cut_shape(id).and_then(Result::ok)
    }

    /// Ends `id`'s booleans once their result is part of its shape: they
    /// go, and each cutter that no longer cuts anything is hidden (shown,
    /// it would cover the result as a solid shape again). The cutters are
    /// kept, so they can be used again.
    pub(super) fn end_booleans(&mut self, id: ObjectId) -> Result<(), SceneError> {
        let before = self.scene.booleans(id).to_vec();
        if before.is_empty() {
            return Ok(());
        }
        self.edit(Edit::SetBooleans {
            id,
            before: before.clone(),
            after: Vec::new(),
        })?;
        let mut cutters: Vec<ObjectId> = before.into_iter().flat_map(|b| b.cutters).collect();
        cutters.dedup();
        for cutter in cutters {
            let shown = self.scene.object(cutter).is_some_and(|o| o.visible);
            if shown && !self.scene.is_cutter(cutter) {
                self.edit(Edit::SetVisible {
                    id: cutter,
                    before: true,
                    after: false,
                })?;
            }
        }
        Ok(())
    }

    /// Points the copies' booleans at the copies of their cutters, where
    /// those were copied too (`copied` pairs each original with its copy).
    pub(super) fn follow_copied_cutters(
        &mut self,
        copied: &[(ObjectId, ObjectId)],
    ) -> Result<(), SceneError> {
        let copy_of = |id: ObjectId| {
            copied
                .iter()
                .find(|(original, _)| *original == id)
                .map(|(_, copy)| *copy)
        };
        for (_, copy) in copied {
            let before = self.scene.booleans(*copy).to_vec();
            if before.is_empty() {
                continue;
            }
            let mut after = before.clone();
            for boolean in &mut after {
                for cutter in &mut boolean.cutters {
                    if let Some(new) = copy_of(*cutter) {
                        *cutter = new;
                    }
                }
            }
            if after != before {
                self.edit(Edit::SetBooleans {
                    id: *copy,
                    before,
                    after,
                })?;
            }
        }
        Ok(())
    }
}
