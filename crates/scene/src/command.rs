//! Commands: every change a user can make to the scene, each one undoable.

use crate::object::{Object, ObjectId, ObjectKind};
use crate::tree::{Edit, Place, Scene, SceneError};

/// A change to the scene. Every change goes through one, so it can be
/// undone and redone.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Adds a new object of `kind` at the top of the Object Manager's
    /// list, named after its kind (with a number if that name is taken).
    /// It becomes the selection.
    Create(ObjectKind),
    /// Adds a ready-made object at `place`. It becomes the selection.
    Insert {
        /// The object.
        object: Object,
        /// Where it goes.
        place: Place,
    },
    /// Renames an object. Spaces around the name are trimmed; an empty
    /// name is refused.
    Rename {
        /// The object.
        id: ObjectId,
        /// Its new name.
        name: String,
    },
    /// Shows or hides objects (and so everything inside them).
    SetVisible {
        /// The objects.
        ids: Vec<ObjectId>,
        /// Shown (`true`) or hidden.
        visible: bool,
    },
    /// Moves objects, with everything inside them, under `parent` (`None`
    /// for the top of the tree), before the child now at `index` (or at
    /// the end). Moved objects keep their order. An object can't move into
    /// itself or anything inside it.
    Move {
        /// The objects. Any inside another one listed move with it.
        ids: Vec<ObjectId>,
        /// The new parent.
        parent: Option<ObjectId>,
        /// The position among the parent's children, counted before the
        /// move.
        index: usize,
    },
    /// Deletes objects and everything inside them.
    Delete(Vec<ObjectId>),
    /// Puts objects into a new group, which takes the place of the first
    /// of them. The group becomes the selection.
    Group(Vec<ObjectId>),
    /// Takes everything out of these groups (or other objects), putting it
    /// where the group was, and deletes the emptied objects. What was
    /// inside becomes the selection.
    Ungroup(Vec<ObjectId>),
}

impl Command {
    /// A short description for the Edit menu, such as "Delete 3 objects".
    pub fn describe(&self, scene: &Scene) -> String {
        let count = |ids: &[ObjectId]| top_level(scene, ids).len();
        let objects = |n: usize| {
            if n == 1 {
                "object".to_string()
            } else {
                format!("{n} objects")
            }
        };
        match self {
            Self::Create(kind) => format!("Create {}", kind.name().to_lowercase()),
            Self::Insert { object, .. } => format!("Add {}", object.name),
            Self::Rename { .. } => "Rename".into(),
            Self::SetVisible { ids, visible } => {
                let verb = if *visible { "Show" } else { "Hide" };
                format!("{verb} {}", objects(count(ids)))
            }
            Self::Move { ids, .. } => format!("Move {}", objects(count(ids))),
            Self::Delete(ids) => format!("Delete {}", objects(count(ids))),
            Self::Group(ids) => format!("Group {}", objects(count(ids))),
            Self::Ungroup(ids) => {
                if count(ids) == 1 {
                    "Ungroup".into()
                } else {
                    format!("Ungroup {}", count(ids))
                }
            }
        }
    }
}

/// What carrying out a command did.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Applied {
    /// The edits, in the order they were applied.
    pub(crate) edits: Vec<Edit>,
    /// What should be selected afterwards, if the command picks it.
    pub(crate) select: Option<Vec<ObjectId>>,
}

/// Carries out `command` on `scene`. On failure the scene is left exactly
/// as it was.
pub(crate) fn apply(scene: &mut Scene, command: &Command) -> Result<Applied, SceneError> {
    let mut run = Run {
        scene,
        edits: Vec::new(),
    };
    match run.command(command) {
        Ok(select) => Ok(Applied {
            edits: run.edits,
            select,
        }),
        Err(error) => {
            run.roll_back();
            Err(error)
        }
    }
}

/// A command being carried out: the scene, and the edits made so far.
struct Run<'a> {
    scene: &'a mut Scene,
    edits: Vec<Edit>,
}

impl Run<'_> {
    fn edit(&mut self, edit: Edit) -> Result<(), SceneError> {
        self.scene.apply(&edit)?;
        self.edits.push(edit);
        Ok(())
    }

    /// Undoes the edits made so far.
    fn roll_back(&mut self) {
        while let Some(edit) = self.edits.pop() {
            // Undoing edits that were just applied cannot fail.
            let _ = self.scene.apply(&edit.inverse());
        }
    }

    fn place_of(&self, id: ObjectId) -> Result<Place, SceneError> {
        self.scene.place(id).ok_or(SceneError::UnknownObject(id))
    }

    /// Carries out the command; returns what to select, if it decides.
    fn command(&mut self, command: &Command) -> Result<Option<Vec<ObjectId>>, SceneError> {
        match command {
            Command::Create(kind) => {
                let mut object = Object::new(*kind);
                object.name = self.scene.unused_name(kind.name());
                let id = self.insert(
                    object,
                    Place {
                        parent: None,
                        index: 0,
                    },
                )?;
                Ok(Some(vec![id]))
            }
            Command::Insert { object, place } => {
                if object.name.trim().is_empty() {
                    return Err(SceneError::EmptyName);
                }
                let id = self.insert(object.clone(), *place)?;
                Ok(Some(vec![id]))
            }
            Command::Rename { id, name } => {
                let name = name.trim();
                if name.is_empty() {
                    return Err(SceneError::EmptyName);
                }
                let before = self
                    .scene
                    .object(*id)
                    .ok_or(SceneError::UnknownObject(*id))?
                    .name
                    .clone();
                if before != name {
                    self.edit(Edit::Rename {
                        id: *id,
                        before,
                        after: name.to_string(),
                    })?;
                }
                Ok(None)
            }
            Command::SetVisible { ids, visible } => {
                for id in existing(self.scene, ids)? {
                    let before = self.scene.object(id).is_some_and(|o| o.visible);
                    if before != *visible {
                        self.edit(Edit::SetVisible {
                            id,
                            before,
                            after: *visible,
                        })?;
                    }
                }
                Ok(None)
            }
            Command::Move { ids, parent, index } => {
                let ids = top_level(self.scene, &existing(self.scene, ids)?);
                self.move_into(&ids, *parent, *index)?;
                Ok(None)
            }
            Command::Delete(ids) => {
                for id in top_level(self.scene, &existing(self.scene, ids)?) {
                    // Empty the object first, last child first, so every
                    // removal is of an object with nothing inside.
                    for inner in self.scene.subtree(id).into_iter().rev() {
                        let object = self
                            .scene
                            .object(inner)
                            .ok_or(SceneError::UnknownObject(inner))?
                            .clone();
                        let place = self.place_of(inner)?;
                        self.edit(Edit::Remove {
                            id: inner,
                            object,
                            place,
                        })?;
                    }
                }
                Ok(Some(Vec::new()))
            }
            Command::Group(ids) => {
                let ids = top_level(self.scene, &existing(self.scene, ids)?);
                let Some(first) = ids.first() else {
                    return Ok(None);
                };
                let place = self.place_of(*first)?;
                let mut group = Object::new(ObjectKind::Group);
                group.name = self.scene.unused_name(ObjectKind::Group.name());
                let group_id = self.insert(group, place)?;
                self.move_into(&ids, Some(group_id), 0)?;
                Ok(Some(vec![group_id]))
            }
            Command::Ungroup(ids) => {
                let mut freed = Vec::new();
                for id in top_level(self.scene, &existing(self.scene, ids)?) {
                    let place = self.place_of(id)?;
                    let children = self.scene.children(Some(id)).to_vec();
                    // Into the group's place, before the group itself.
                    self.move_into(&children, place.parent, place.index)?;
                    let object = self
                        .scene
                        .object(id)
                        .ok_or(SceneError::UnknownObject(id))?
                        .clone();
                    let place = self.place_of(id)?;
                    self.edit(Edit::Remove { id, object, place })?;
                    freed.extend(children);
                }
                Ok(Some(freed))
            }
        }
    }

    fn insert(&mut self, object: Object, place: Place) -> Result<ObjectId, SceneError> {
        let id = self.scene.next_id();
        self.edit(Edit::Insert { id, object, place })?;
        Ok(id)
    }

    /// Moves `ids` (top-level among themselves, in order) under `parent`,
    /// before the child now at `index`.
    fn move_into(
        &mut self,
        ids: &[ObjectId],
        parent: Option<ObjectId>,
        index: usize,
    ) -> Result<(), SceneError> {
        if let Some(p) = parent {
            if !self.scene.contains(p) {
                return Err(SceneError::UnknownObject(p));
            }
            if ids.iter().any(|id| self.scene.is_inside(p, *id)) {
                return Err(SceneError::NestedInItself);
            }
        }
        // The first child at or after `index` that isn't moving: everything
        // moved goes just before it, in order (or at the end, if none).
        let anchor = self
            .scene
            .children(parent)
            .iter()
            .skip(index)
            .find(|c| !ids.contains(c))
            .copied();
        for id in ids {
            let from = self.place_of(*id)?;
            let mut after_removal = self.scene.children(parent).to_vec();
            after_removal.retain(|c| c != id);
            let to_index = anchor
                .and_then(|a| after_removal.iter().position(|c| *c == a))
                .unwrap_or(after_removal.len());
            let to = Place {
                parent,
                index: to_index,
            };
            if to != from {
                self.edit(Edit::Relocate { id: *id, from, to })?;
            }
        }
        Ok(())
    }
}

/// `ids` without repeats, in tree order, failing if any doesn't exist.
fn existing(scene: &Scene, ids: &[ObjectId]) -> Result<Vec<ObjectId>, SceneError> {
    if let Some(missing) = ids.iter().find(|id| !scene.contains(**id)) {
        return Err(SceneError::UnknownObject(*missing));
    }
    Ok(scene
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .filter(|id| ids.contains(id))
        .collect())
}

/// `ids` in tree order, leaving out any that sit inside another one listed
/// (those go along with it anyway). Unknown ids are left out.
pub(crate) fn top_level(scene: &Scene, ids: &[ObjectId]) -> Vec<ObjectId> {
    scene
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .filter(|id| ids.contains(id))
        .filter(|id| {
            !ids.iter()
                .any(|other| other != id && scene.is_inside(*id, *other))
        })
        .collect()
}
