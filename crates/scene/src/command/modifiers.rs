//! The commands that add, change, move, remove and bake modifiers
//! ([decision 0142](../../../../docs/decisions/0142-modifier-stack-and-remesh.md)).
//! Every one changes an object's list through one edit,
//! [`Edit::SetModifiers`], so undo puts the whole list back.

use super::Run;
use crate::modifier::{Modifier, ModifierKind, ModifierRefusal, Purpose};
use crate::object::ObjectId;
use crate::tree::{Edit, SceneError};
use words::{tr, trf};

/// The undo label for a modifier command.
pub(super) fn label(scene: &crate::Scene, command: &super::Command) -> Option<String> {
    use super::Command;
    let name_at = |id: &ObjectId, index: &usize| {
        scene
            .modifiers(*id)
            .get(*index)
            .map_or_else(|| tr("modifier"), |m| m.kind.name())
    };
    Some(match command {
        Command::AddModifier { ids, kind } => match ids.len() {
            0 | 1 => trf!("Add {}", kind.name()),
            n => trf!("Add {} to {} objects", kind.name(), n),
        },
        Command::ChangeModifier {
            id,
            index,
            modifier,
        } => {
            let before = scene.modifiers(*id).get(*index);
            match before {
                Some(b) if b.on != modifier.on && modifier.on => {
                    trf!("Switch {} on", modifier.kind.name())
                }
                Some(b) if b.on != modifier.on => trf!("Switch {} off", modifier.kind.name()),
                Some(b) if b.in_viewport != modifier.in_viewport && modifier.in_viewport => {
                    trf!("Show {} in the viewport", modifier.kind.name())
                }
                Some(b) if b.in_viewport != modifier.in_viewport => {
                    trf!("Hide {} in the viewport", modifier.kind.name())
                }
                _ => trf!("Change {}", modifier.kind.name()),
            }
        }
        Command::RemoveModifier { id, index } => trf!("Remove {}", name_at(id, index)),
        Command::DuplicateModifier { id, index } => trf!("Duplicate {}", name_at(id, index)),
        Command::BakeModifiersUpTo { id, index } => {
            trf!("Bake into shape up to {}", name_at(id, index))
        }
        Command::MoveModifier { id, from, to } => {
            if to < from {
                trf!("Move {} up", name_at(id, from))
            } else {
                trf!("Move {} down", name_at(id, from))
            }
        }
        _ => return None,
    })
}

impl Run<'_> {
    /// Gives `id` these modifiers, if they make sense
    /// ([`crate::Scene::check_modifiers`]).
    fn set_modifiers(&mut self, id: ObjectId, after: Vec<Modifier>) -> Result<(), SceneError> {
        let object = self.scene.object(id).ok_or(SceneError::UnknownObject(id))?;
        if object.modifiers == after {
            return Ok(());
        }
        self.scene
            .check_modifiers(id, &after)
            .map_err(SceneError::Modifier)?;
        let before = object.modifiers.clone();
        self.replace_modifiers(id, before, after)
    }

    /// Gives object `id` the modifiers `after` in place of `before`, every
    /// one named ([`crate::modifier::with_ids`]), and takes out of its
    /// levels of detail (and its instances') the changes to modifiers no
    /// longer there (decision 0165), in the same step. Every change to a
    /// list of modifiers goes through here.
    pub(super) fn replace_modifiers(
        &mut self,
        id: ObjectId,
        before: Vec<Modifier>,
        after: Vec<Modifier>,
    ) -> Result<(), SceneError> {
        let after = crate::modifier::with_ids(after);
        self.edit(Edit::SetModifiers { id, before, after })?;
        self.forget_gone_overrides(id)
    }

    /// Adds a modifier of `kind` (its settings brought within limits) at
    /// the end of each object's list, or before the first it should come
    /// before ([`ModifierKind::goes_before`]). Refused, changing nothing,
    /// if any object can't take it.
    pub(super) fn add_modifier(
        &mut self,
        ids: &[ObjectId],
        kind: &ModifierKind,
    ) -> Result<(), SceneError> {
        if ids.is_empty() {
            return Err(SceneError::Modifier(ModifierRefusal::NotAShape));
        }
        for id in super::existing(self.scene, ids)? {
            let mut after = self.scene.modifiers(id).to_vec();
            let at = after
                .iter()
                .position(|m| kind.goes_before(&m.kind))
                .unwrap_or(after.len());
            after.insert(at, Modifier::new(kind.clone()));
            self.set_modifiers(id, after)?;
        }
        Ok(())
    }

    /// Puts `modifier` (settings brought within limits) in place of the
    /// one at `index`, which must be of the same kind.
    pub(super) fn change_modifier(
        &mut self,
        id: ObjectId,
        index: usize,
        modifier: &Modifier,
    ) -> Result<(), SceneError> {
        let mut after = self.scene.modifiers(id).to_vec();
        let slot = after
            .get_mut(index)
            .ok_or(SceneError::Modifier(ModifierRefusal::NoSuchModifier))?;
        if !slot.kind.same_kind(&modifier.kind) {
            return Err(SceneError::Modifier(ModifierRefusal::OtherKind));
        }
        *slot = Modifier {
            id: slot.id,
            kind: modifier.kind.clamped(),
            ..modifier.clone()
        };
        self.set_modifiers(id, after)
    }

    /// Takes the modifier at `index` away.
    pub(super) fn remove_modifier(&mut self, id: ObjectId, index: usize) -> Result<(), SceneError> {
        let mut after = self.scene.modifiers(id).to_vec();
        if index >= after.len() {
            return Err(SceneError::Modifier(ModifierRefusal::NoSuchModifier));
        }
        after.remove(index);
        self.set_modifiers(id, after)
    }

    /// Moves the modifier at `from` to `to` (both places in the list as
    /// it is now).
    pub(super) fn move_modifier(
        &mut self,
        id: ObjectId,
        from: usize,
        to: usize,
    ) -> Result<(), SceneError> {
        let mut after = self.scene.modifiers(id).to_vec();
        if from >= after.len() || to >= after.len() {
            return Err(SceneError::Modifier(ModifierRefusal::NoSuchModifier));
        }
        let modifier = after.remove(from);
        after.insert(to, modifier);
        self.set_modifiers(id, after)
    }

    /// Puts a copy of the modifier at `index` just after it, under a new
    /// id (given by [`crate::modifier::with_ids`]).
    pub(super) fn duplicate_modifier(
        &mut self,
        id: ObjectId,
        index: usize,
    ) -> Result<(), SceneError> {
        let mut after = self.scene.modifiers(id).to_vec();
        let copy = Modifier {
            id: crate::ModifierId(0),
            ..after
                .get(index)
                .ok_or(SceneError::Modifier(ModifierRefusal::NoSuchModifier))?
                .clone()
        };
        after.insert(index + 1, copy);
        self.set_modifiers(id, after)
    }

    /// Bakes the modifiers up to the one at `index` into the shape: the
    /// list is cut short for the bake (without forgetting the levels of
    /// detail's changes to those after it), the shape baked, and the list
    /// given back without those that are now part of the shape.
    pub(super) fn bake_modifiers_up_to(
        &mut self,
        id: ObjectId,
        index: usize,
    ) -> Result<(), SceneError> {
        let all = self.scene.modifiers(id).to_vec();
        if index >= all.len() {
            return Err(SceneError::Modifier(ModifierRefusal::NoSuchModifier));
        }
        if !self.scene.can_make_editable(id) {
            return Err(SceneError::Modifier(ModifierRefusal::NotAShape));
        }
        let (head, tail) = all.split_at(index + 1);
        self.edit(Edit::SetModifiers {
            id,
            before: all.clone(),
            after: head.to_vec(),
        })?;
        let problems = if self.bakes_modifiers(id) {
            self.bake_shape_with_modifiers(id)?.unwrap_or_default()
        } else {
            Vec::new()
        };
        let kept: Vec<Modifier> = head
            .iter()
            .enumerate()
            .filter(|(i, m)| !m.on || problems.iter().any(|(p, _)| p == i))
            .map(|(_, m)| m.clone())
            .chain(tail.iter().cloned())
            .collect();
        self.replace_modifiers(id, head.to_vec(), kept)
    }

    /// Takes out of every other object's modifiers each modifier that
    /// reads one of `going` (objects about to be deleted).
    pub(super) fn forget_modifier_sources(&mut self, going: &[ObjectId]) -> Result<(), SceneError> {
        let users: Vec<ObjectId> = self
            .scene
            .walk()
            .into_iter()
            .map(|(id, _)| id)
            .filter(|id| !going.contains(id))
            .filter(|id| {
                self.scene
                    .modifiers(*id)
                    .iter()
                    .any(|m| m.kind.sources().iter().any(|s| going.contains(s)))
            })
            .collect();
        for id in users {
            let before = self.scene.modifiers(id).to_vec();
            // A scatter loses the shapes going, and goes with the last; a
            // lattice goes with its cage.
            let after: Vec<Modifier> = before
                .iter()
                .filter_map(|m| {
                    Some(Modifier {
                        kind: m.kind.without_sources(going)?,
                        ..m.clone()
                    })
                })
                .collect();
            self.replace_modifiers(id, before, after)?;
        }
        Ok(())
    }

    /// Takes out of every other object's modifiers each one that reads
    /// `changed` (which has just become another kind) and can't read its
    /// new kind, as [`Self::forget_modifier_sources`] does for a deleted
    /// object (Bend along curve reads only a curve); and, if `changed` no
    /// longer has a shape, takes it out of every boolean as a cutter.
    pub(super) fn forget_unreadable_sources(
        &mut self,
        changed: ObjectId,
    ) -> Result<(), SceneError> {
        let Some(kind) = self.scene.object(changed).map(|o| o.kind.clone()) else {
            return Ok(());
        };
        let users: Vec<ObjectId> = self
            .scene
            .walk()
            .into_iter()
            .map(|(id, _)| id)
            .filter(|id| *id != changed)
            .filter(|id| {
                self.scene
                    .modifiers(*id)
                    .iter()
                    .any(|m| m.kind.sources().contains(&changed) && !m.kind.takes_source(&kind))
            })
            .collect();
        for id in users {
            let before = self.scene.modifiers(id).to_vec();
            let after: Vec<Modifier> = before
                .iter()
                .filter_map(|m| {
                    if !m.kind.sources().contains(&changed) || m.kind.takes_source(&kind) {
                        return Some(m.clone());
                    }
                    Some(Modifier {
                        kind: m.kind.without_sources(&[changed])?,
                        ..m.clone()
                    })
                })
                .collect();
            self.replace_modifiers(id, before, after)?;
        }
        if !kind.has_shape() {
            self.forget_cutters(&[changed])?;
        }
        Ok(())
    }

    /// Takes away `id`'s modifiers if they don't make sense here (a group
    /// or an instance, a source that isn't there or would make a loop):
    /// for an object put in from elsewhere.
    pub(super) fn keep_usable_modifiers(&mut self, id: ObjectId) -> Result<(), SceneError> {
        let before = self.scene.modifiers(id).to_vec();
        if before.is_empty() || self.scene.check_modifiers(id, &before).is_ok() {
            return Ok(());
        }
        let mut after: Vec<Modifier> = before
            .iter()
            .filter(|m| {
                self.scene
                    .check_modifiers(id, std::slice::from_ref(*m))
                    .is_ok()
            })
            .cloned()
            .collect();
        if self.scene.check_modifiers(id, &after).is_err() {
            after.clear();
        }
        self.replace_modifiers(id, before, after)
    }

    /// Points the copies' modifiers at the copies of their sources, where
    /// those were copied too (`copied` pairs each original with its copy).
    pub(super) fn follow_copied_sources(
        &mut self,
        copied: &[(ObjectId, ObjectId)],
    ) -> Result<(), SceneError> {
        let copy_of = |id: ObjectId| {
            copied
                .iter()
                .find(|(original, _)| *original == id)
                .map_or(id, |(_, copy)| *copy)
        };
        for (_, copy) in copied {
            let before = self.scene.modifiers(*copy).to_vec();
            let after: Vec<Modifier> = before
                .iter()
                .map(|m| Modifier {
                    kind: m.kind.with_sources_renamed(copy_of),
                    ..m.clone()
                })
                .collect();
            if after != before {
                self.replace_modifiers(*copy, before, after)?;
            }
        }
        Ok(())
    }

    /// Whether baking `id` takes in its modifiers: any of them (an
    /// instance's original's) is on.
    pub(super) fn bakes_modifiers(&self, id: ObjectId) -> bool {
        self.scene.has_modifiers_for(id, Purpose::Export)
    }

    /// Ends `id`'s modifiers once their result is part of its shape: those
    /// that were on and worked go; those switched off, or that couldn't do
    /// their work (`problems`, by place), stay.
    pub(super) fn end_modifiers(
        &mut self,
        id: ObjectId,
        problems: &[(usize, String)],
    ) -> Result<(), SceneError> {
        let before = self.scene.modifiers(id).to_vec();
        let after: Vec<Modifier> = before
            .iter()
            .enumerate()
            .filter(|(i, m)| !m.on || problems.iter().any(|(p, _)| p == i))
            .map(|(_, m)| m.clone())
            .collect();
        if after != before {
            self.replace_modifiers(id, before, after)?;
        }
        Ok(())
    }
}
