//! The commands for levels of detail ([decision 0155](../../../../docs/decisions/0155-levels-of-detail.md),
//! [decision 0165](../../../../docs/decisions/0165-levels-panel.md)):
//! changing an object's levels, making a level by hand and starting it
//! again from the model, and keeping levels tidy when what they name
//! changes (a modifier removed, a hand-made level's shape deleted or
//! copied).

use super::{Command, Run};
use crate::lod::{HandLevel, Lods, level_name};
use crate::object::{Object, ObjectId, ObjectKind};
use crate::tree::{Edit, Place, SceneError};
use words::text;

/// Why a level can't be made by hand, or started again.
const NO_LEVELS: &str = text!("it has no levels of detail on");
const NO_SUCH_LEVEL: &str = text!("it has no such level");
const NOT_BY_HAND: &str = text!("that level isn't made by hand");
const NOTHING_MADE: &str = text!("the level couldn't be made, so there's nothing to copy");

impl Run<'_> {
    /// Gives object `id` the levels of detail `after` in place of what it
    /// has, unless they are the same.
    fn replace_lods(&mut self, id: ObjectId, after: Lods) -> Result<(), SceneError> {
        let object = self.scene.object(id).ok_or(SceneError::UnknownObject(id))?;
        if object.lods == after {
            return Ok(());
        }
        let before = Box::new(object.lods.clone());
        self.edit(Edit::SetLods {
            id,
            before,
            after: Box::new(after),
        })
    }

    /// [`Command::SetLods`]: each object gets `lods` (clamped, its changes
    /// to the modifiers kept to its own), keeping its own levels made by
    /// hand where those levels stay; a level made by hand that goes takes
    /// its shape with it. With more than one object, each keeps its own
    /// changes to the modifiers (their ids are each object's own).
    pub(super) fn set_lods(&mut self, ids: &[ObjectId], lods: &Lods) -> Result<(), SceneError> {
        let lods = lods.clamped();
        let mut gone: Vec<ObjectId> = Vec::new();
        for id in ids {
            let object = self
                .scene
                .object(*id)
                .ok_or(SceneError::UnknownObject(*id))?;
            if !object.kind.has_shape() {
                continue;
            }
            let own = object.lods.clone();
            let mut after = lods.clone();
            for (k, level) in after.levels.iter_mut().enumerate() {
                let mine = own.levels.get(k);
                level.hand = mine.and_then(|l| l.hand);
                if ids.len() > 1 {
                    level.overrides = mine.map(|l| l.overrides.clone()).unwrap_or_default();
                }
            }
            gone.extend(
                own.levels
                    .iter()
                    .skip(after.levels.len())
                    .filter_map(|l| l.hand.map(|h| h.shape))
                    .filter(|shape| self.scene.contains(*shape)),
            );
            let modifiers = self
                .scene
                .modifiers(self.scene.modifier_owner(*id))
                .to_vec();
            self.replace_lods(*id, after.tidied(&modifiers))?;
        }
        if !gone.is_empty() {
            self.command(&Command::Delete(gone))?;
        }
        Ok(())
    }

    /// Takes out of the levels of `owner` and its instances the changes to
    /// modifiers `owner` no longer has (decision 0165: removing a modifier
    /// removes its changes).
    pub(super) fn forget_gone_overrides(&mut self, owner: ObjectId) -> Result<(), SceneError> {
        let modifiers = self.scene.modifiers(owner).to_vec();
        let users: Vec<ObjectId> = self
            .scene
            .walk()
            .into_iter()
            .map(|(id, _)| id)
            .filter(|id| *id == owner || self.scene.modifier_owner(*id) == owner)
            .collect();
        for id in users {
            let Some(object) = self.scene.object(id) else {
                continue;
            };
            let mut after = object.lods.clone();
            for level in &mut after.levels {
                level.overrides = crate::lod::tidied_overrides(&level.overrides, &modifiers);
            }
            self.replace_lods(id, after)?;
        }
        Ok(())
    }

    /// [`Command::MakeLevelByHand`]: a copy of what level `k` of `id` shows,
    /// as a shape of its own inside `id`, named after it, smoothed and
    /// coloured as it is, its pivot on `id`'s. Returns the new shape (to
    /// select, so it is edited at once).
    pub(super) fn make_level_by_hand(
        &mut self,
        id: ObjectId,
        k: usize,
    ) -> Result<ObjectId, SceneError> {
        let object = self
            .scene
            .object(id)
            .ok_or(SceneError::UnknownObject(id))?
            .clone();
        if !object.lods.on || !object.kind.has_shape() {
            return Err(SceneError::Level(NO_LEVELS));
        }
        if k >= object.lods.clamped().levels.len() {
            return Err(SceneError::Level(NO_SUCH_LEVEL));
        }
        if let Some((shape, _)) = self.scene.level_shape(id, k) {
            return Ok(shape);
        }
        let mesh = self
            .scene
            .automatic_level(id, k)
            .ok_or(SceneError::Level(NOTHING_MADE))?;
        let made_from = self
            .scene
            .model_fingerprint(id)
            .ok_or(SceneError::NoShape)?;
        let mut shape = Object::new(ObjectKind::Mesh(mesh));
        shape.name = self.scene.unused_name(&level_name(&object.name, k + 1));
        // Its pivot on the object's, so it sits exactly on the model.
        shape.offset = object.offset;
        shape.hardness = self
            .scene
            .object(self.scene.modifier_owner(id))
            .map_or(object.hardness, |o| o.hardness);
        shape.materials = self.scene.material_list(id);
        if shape.materials == vec![crate::Material::standard()] {
            shape.materials.clear();
        }
        let place = Place {
            parent: Some(id),
            index: self.scene.children(Some(id)).len(),
        };
        let new = self.insert(shape, place)?;
        let mut lods = self
            .scene
            .object(id)
            .ok_or(SceneError::UnknownObject(id))?
            .lods
            .clone();
        if let Some(level) = lods.levels.get_mut(k) {
            level.hand = Some(HandLevel {
                shape: new,
                made_from,
            });
        }
        self.replace_lods(id, lods)?;
        Ok(new)
    }

    /// [`Command::RestartLevel`]: level `k` of `id`, made by hand, made
    /// again from the model as it is now: its shape replaced by what the
    /// level would show (its edits and placing gone), and no longer marked
    /// as made from an older model.
    pub(super) fn restart_level(&mut self, id: ObjectId, k: usize) -> Result<(), SceneError> {
        let Some((shape, _)) = self.scene.level_shape(id, k) else {
            return Err(SceneError::Level(NOT_BY_HAND));
        };
        let mesh = self
            .scene
            .automatic_level(id, k)
            .ok_or(SceneError::Level(NOTHING_MADE))?;
        let made_from = self
            .scene
            .model_fingerprint(id)
            .ok_or(SceneError::NoShape)?;
        let held = self
            .scene
            .object(shape)
            .ok_or(SceneError::UnknownObject(shape))?
            .clone();
        // Its edits go, last layer first, then the shape is replaced.
        for (index, layer) in held.stack.layers().iter().enumerate().rev() {
            self.edit(Edit::RemoveLayer {
                id: shape,
                index,
                layer: layer.clone(),
            })?;
        }
        self.edit(Edit::SetKind {
            id: shape,
            before: held.kind.clone(),
            after: ObjectKind::Mesh(mesh),
        })?;
        let offset = self
            .scene
            .object(id)
            .ok_or(SceneError::UnknownObject(id))?
            .offset;
        self.edit(Edit::SetTransform {
            id: shape,
            before: held.transform,
            after: crate::Transform::default(),
        })?;
        self.edit(Edit::SetOffset {
            id: shape,
            before: held.offset,
            after: offset,
        })?;
        let mut lods = self
            .scene
            .object(id)
            .ok_or(SceneError::UnknownObject(id))?
            .lods
            .clone();
        if let Some(hand) = lods.levels.get_mut(k).and_then(|l| l.hand.as_mut()) {
            hand.made_from = made_from;
        }
        self.replace_lods(id, lods)
    }

    /// Takes the levels whose shapes made by hand are among `going`
    /// (objects about to be deleted) back to being made from the model,
    /// for the objects that stay.
    pub(super) fn forget_level_shapes(&mut self, going: &[ObjectId]) -> Result<(), SceneError> {
        let owners: Vec<ObjectId> = self
            .scene
            .walk()
            .into_iter()
            .map(|(id, _)| id)
            .filter(|id| !going.contains(id))
            .collect();
        for id in owners {
            let Some(object) = self.scene.object(id) else {
                continue;
            };
            let mut after = object.lods.clone();
            for level in &mut after.levels {
                if level.hand.is_some_and(|h| going.contains(&h.shape)) {
                    level.hand = None;
                }
            }
            self.replace_lods(id, after)?;
        }
        Ok(())
    }

    /// Takes away the levels made by hand of an object put in from
    /// elsewhere: the shapes they name aren't its (copies point theirs at
    /// the copied shapes afterwards, [`Self::follow_copied_levels`]).
    pub(super) fn keep_usable_levels(&mut self, id: ObjectId) -> Result<(), SceneError> {
        let Some(object) = self.scene.object(id) else {
            return Ok(());
        };
        let mut after = object.lods.clone();
        for level in &mut after.levels {
            level.hand = None;
        }
        self.replace_lods(id, after)
    }

    /// Points the copies' levels made by hand at the copies of their
    /// shapes (`copied` pairs each original with its copy).
    pub(super) fn follow_copied_levels(
        &mut self,
        copied: &[(ObjectId, ObjectId)],
    ) -> Result<(), SceneError> {
        for (original, copy) in copied {
            let Some(from) = self.scene.object(*original) else {
                continue;
            };
            let hands: Vec<Option<HandLevel>> = from.lods.levels.iter().map(|l| l.hand).collect();
            let Some(object) = self.scene.object(*copy) else {
                continue;
            };
            let mut after = object.lods.clone();
            for (level, hand) in after.levels.iter_mut().zip(hands) {
                level.hand = hand.and_then(|h| {
                    copied
                        .iter()
                        .find(|(o, _)| *o == h.shape)
                        .map(|(_, c)| HandLevel {
                            shape: *c,
                            made_from: h.made_from,
                        })
                });
            }
            self.replace_lods(*copy, after)?;
        }
        Ok(())
    }
}
