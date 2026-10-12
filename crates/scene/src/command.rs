//! Commands: every change a user can make to the scene, each one undoable.

use crate::object::{Object, ObjectId, ObjectKind, Transform};
use crate::stack::{EDITS_LAYER, Layer, Operation};
use crate::tree::{Edit, Place, Scene, SceneError};
use mesh::Primitive;
use std::sync::Arc;

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
    /// Takes everything out of the groups among these objects, putting it
    /// where each group was, and deletes the emptied groups. What was
    /// inside becomes the selection. Objects that aren't groups are left
    /// alone; with no group among them, nothing happens.
    Ungroup(Vec<ObjectId>),
    /// Gives primitives new settings (brought within limits first). Every
    /// object listed must be a primitive of the same kind as `primitive`:
    /// a cube's settings can't turn a sphere into a cube.
    SetPrimitive {
        /// The objects.
        ids: Vec<ObjectId>,
        /// Their new settings.
        primitive: Primitive,
    },
    /// Records an edit on top of an object's shape, in its top layer (an
    /// "Edits" layer is made if it has none). The base stays as it was, so
    /// a primitive's settings can still change and the edit replays on the
    /// new shape. Groups have no shape and are refused.
    Record {
        /// The object.
        id: ObjectId,
        /// The edit.
        operation: Operation,
    },
    /// Puts a changed version of a recorded edit in its place: an
    /// extrude's distance changed in the Attributes panel afterwards, say
    /// (decision 0038). Everything recorded above it replays on the new
    /// result.
    ReplaceOperation {
        /// The object.
        id: ObjectId,
        /// Which layer, from the bottom (0).
        layer: usize,
        /// Which operation in the layer, from the first (0).
        index: usize,
        /// The edit to put there.
        operation: Operation,
    },
    /// Moves, turns or scales objects: each gets a new transform, relative
    /// to its parent (decision 0030). A transform with a value that isn't
    /// a number is refused.
    SetTransforms(Vec<(ObjectId, Transform)>),
    /// Bakes each shape's base and recorded edits into a plain mesh, which
    /// becomes its new base (its settings are gone; its edits are part of
    /// the mesh). Groups, and meshes with nothing recorded, are left alone.
    MakeEditable(Vec<ObjectId>),
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
            Self::SetPrimitive { primitive, .. } => {
                format!("Change {} settings", primitive.name().to_lowercase())
            }
            Self::Record { operation, .. } => operation.describe(),
            Self::ReplaceOperation { operation, .. } => format!("Change {}", operation.describe()),
            Self::SetTransforms(changes) => {
                let ids: Vec<ObjectId> = changes.iter().map(|(id, _)| *id).collect();
                format!("Transform {}", objects(count(&ids)))
            }
            Self::MakeEditable(ids) => {
                let n = ids
                    .iter()
                    .filter(|id| scene.can_make_editable(**id))
                    .count();
                format!("Make {} editable", objects(n.max(1)))
            }
            Self::Ungroup(ids) => match groups_among(scene, ids).len() {
                1 => "Ungroup".into(),
                n => format!("Ungroup {n} groups"),
            },
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
                let mut object = Object::new(kind.clone());
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
            Command::SetPrimitive { ids, primitive } => {
                let after = ObjectKind::Primitive(primitive.clamped());
                for id in existing(self.scene, ids)? {
                    let before = self
                        .scene
                        .object(id)
                        .ok_or(SceneError::UnknownObject(id))?
                        .kind
                        .clone();
                    match &before {
                        ObjectKind::Primitive(p) if p.kind() == primitive.kind() => {}
                        _ => return Err(SceneError::WrongKind(primitive.name())),
                    }
                    if before != after {
                        self.edit(Edit::SetKind {
                            id,
                            before,
                            after: after.clone(),
                        })?;
                    }
                }
                Ok(None)
            }
            Command::Record { id, operation } => {
                let object = self
                    .scene
                    .object(*id)
                    .ok_or(SceneError::UnknownObject(*id))?;
                if !object.kind.has_shape() {
                    return Err(SceneError::NoShape);
                }
                let mut layers = object.stack.layers().len();
                if layers == 0 {
                    self.edit(Edit::InsertLayer {
                        id: *id,
                        index: 0,
                        layer: Layer::new(EDITS_LAYER),
                    })?;
                    layers = 1;
                }
                let layer = layers - 1;
                let index = self
                    .scene
                    .object(*id)
                    .map_or(0, |o| o.stack.layers()[layer].operations.len());
                self.edit(Edit::InsertOperation {
                    id: *id,
                    layer,
                    index,
                    operation: operation.clone(),
                })?;
                Ok(None)
            }
            Command::ReplaceOperation {
                id,
                layer,
                index,
                operation,
            } => {
                let object = self
                    .scene
                    .object(*id)
                    .ok_or(SceneError::UnknownObject(*id))?;
                let before = object
                    .stack
                    .layers()
                    .get(*layer)
                    .and_then(|l| l.operations.get(*index))
                    .cloned()
                    .ok_or(SceneError::History("that edit no longer exists"))?;
                if before == *operation {
                    return Ok(None);
                }
                self.edit(Edit::RemoveOperation {
                    id: *id,
                    layer: *layer,
                    index: *index,
                    operation: before,
                })?;
                self.edit(Edit::InsertOperation {
                    id: *id,
                    layer: *layer,
                    index: *index,
                    operation: operation.clone(),
                })?;
                Ok(None)
            }
            Command::SetTransforms(changes) => {
                for (id, after) in changes {
                    if !after.is_finite() {
                        return Err(SceneError::NotANumber);
                    }
                    let before = self
                        .scene
                        .object(*id)
                        .ok_or(SceneError::UnknownObject(*id))?
                        .transform;
                    if before != *after {
                        self.edit(Edit::SetTransform {
                            id: *id,
                            before,
                            after: *after,
                        })?;
                    }
                }
                Ok(None)
            }
            Command::MakeEditable(ids) => {
                for id in existing(self.scene, ids)? {
                    if !self.scene.can_make_editable(id) {
                        continue;
                    }
                    let Some(shape) = self.scene.evaluated(id) else {
                        continue;
                    };
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
                        after: ObjectKind::Mesh(Arc::clone(&shape.mesh)),
                    })?;
                }
                Ok(None)
            }
            Command::Ungroup(ids) => {
                existing(self.scene, ids)?;
                let groups = groups_among(self.scene, ids);
                if groups.is_empty() {
                    // Nothing to ungroup: never touch other objects.
                    return Ok(None);
                }
                let mut freed = Vec::new();
                for id in groups {
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

/// The groups among `ids`, in tree order, leaving out any inside another
/// group listed (ungrouping the outer one frees them first).
pub(crate) fn groups_among(scene: &Scene, ids: &[ObjectId]) -> Vec<ObjectId> {
    let groups: Vec<ObjectId> = ids
        .iter()
        .copied()
        .filter(|id| {
            scene
                .object(*id)
                .is_some_and(|o| o.kind == ObjectKind::Group)
        })
        .collect();
    top_level(scene, &groups)
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
