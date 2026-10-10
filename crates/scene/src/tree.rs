//! The object tree, and the small reversible edits every change to it is
//! made of.

use crate::object::{Object, ObjectId};
use glam::Mat4;
use mesh::Bounds;
use std::collections::BTreeMap;

/// Something that went wrong changing the scene, in plain words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneError {
    /// No object has this id (it may have been deleted).
    UnknownObject(ObjectId),
    /// An object can't go inside itself or inside one of its own children.
    NestedInItself,
    /// Names can't be empty.
    EmptyName,
    /// Nothing to undo or redo, or the history no longer matches the scene.
    History(&'static str),
}

impl std::fmt::Display for SceneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownObject(id) => write!(f, "there is no object {id}"),
            Self::NestedInItself => {
                f.write_str("an object can't go inside itself or one of its children")
            }
            Self::EmptyName => f.write_str("a name can't be empty"),
            Self::History(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for SceneError {}

/// Where an object sits in the tree: under which parent (`None` for the top
/// of the tree) and at which position among its brothers and sisters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Place {
    /// The parent, or `None` for the top level.
    pub parent: Option<ObjectId>,
    /// Position among the parent's children, from 0.
    pub index: usize,
}

/// One small, reversible change to the tree. Every command is made of
/// these, recorded as they are applied, so undo can apply their opposites
/// in reverse order and land exactly where it started.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Edit {
    /// Puts a new object (with no children) at `place`.
    Insert {
        id: ObjectId,
        object: Object,
        place: Place,
    },
    /// Takes out an object that has no children, from `place`.
    Remove {
        id: ObjectId,
        object: Object,
        place: Place,
    },
    /// Moves an object (with its children) from one place to another.
    /// `to.index` counts positions with the object already taken out.
    Relocate {
        id: ObjectId,
        from: Place,
        to: Place,
    },
    /// Changes a name.
    Rename {
        id: ObjectId,
        before: String,
        after: String,
    },
    /// Shows or hides an object.
    SetVisible {
        id: ObjectId,
        before: bool,
        after: bool,
    },
}

impl Edit {
    /// The edit that undoes this one.
    pub(crate) fn inverse(&self) -> Self {
        match self.clone() {
            Self::Insert { id, object, place } => Self::Remove { id, object, place },
            Self::Remove { id, object, place } => Self::Insert { id, object, place },
            Self::Relocate { id, from, to } => Self::Relocate {
                id,
                from: to,
                to: from,
            },
            Self::Rename { id, before, after } => Self::Rename {
                id,
                before: after,
                after: before,
            },
            Self::SetVisible { id, before, after } => Self::SetVisible {
                id,
                before: after,
                after: before,
            },
        }
    }
}

/// An object with its place in the tree.
#[derive(Debug, Clone, PartialEq)]
struct Node {
    object: Object,
    parent: Option<ObjectId>,
    children: Vec<ObjectId>,
}

/// Every object in the scene, arranged in a tree: objects can sit inside
/// other objects, and the order among brothers and sisters matters (it is
/// the order the Object Manager lists them in).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Scene {
    nodes: BTreeMap<ObjectId, Node>,
    /// The objects at the top of the tree, in order.
    roots: Vec<ObjectId>,
    /// The next new object's id.
    next_id: u64,
}

impl Scene {
    /// An empty scene.
    pub fn new() -> Self {
        Self::default()
    }

    /// How many objects there are, at every level.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether there are no objects at all.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// The object with this id, if it exists.
    pub fn object(&self, id: ObjectId) -> Option<&Object> {
        self.nodes.get(&id).map(|n| &n.object)
    }

    /// Whether an object with this id exists.
    pub fn contains(&self, id: ObjectId) -> bool {
        self.nodes.contains_key(&id)
    }

    /// The object's parent, or `None` at the top of the tree (or if it
    /// doesn't exist).
    pub fn parent(&self, id: ObjectId) -> Option<ObjectId> {
        self.nodes.get(&id).and_then(|n| n.parent)
    }

    /// The objects directly inside `parent` (the top of the tree for
    /// `None`), in order.
    pub fn children(&self, parent: Option<ObjectId>) -> &[ObjectId] {
        match parent {
            None => &self.roots,
            Some(id) => self.nodes.get(&id).map_or(&[], |n| n.children.as_slice()),
        }
    }

    /// Where the object sits.
    pub fn place(&self, id: ObjectId) -> Option<Place> {
        let parent = self.nodes.get(&id)?.parent;
        let index = self.children(parent).iter().position(|c| *c == id)?;
        Some(Place { parent, index })
    }

    /// Every object, top to bottom as the Object Manager lists them (each
    /// object followed by everything inside it), with how deep it is (0 at
    /// the top of the tree).
    pub fn walk(&self) -> Vec<(ObjectId, usize)> {
        let mut out = Vec::with_capacity(self.nodes.len());
        let mut stack: Vec<(ObjectId, usize)> =
            self.roots.iter().rev().map(|id| (*id, 0)).collect();
        while let Some((id, depth)) = stack.pop() {
            out.push((id, depth));
            stack.extend(
                self.children(Some(id))
                    .iter()
                    .rev()
                    .map(|c| (*c, depth + 1)),
            );
        }
        out
    }

    /// Whether `id` is `ancestor` or somewhere inside it.
    pub fn is_inside(&self, id: ObjectId, ancestor: ObjectId) -> bool {
        let mut at = Some(id);
        // Bounded by the number of objects, so a damaged tree can't loop.
        for _ in 0..=self.nodes.len() {
            match at {
                Some(current) if current == ancestor => return true,
                Some(current) => at = self.parent(current),
                None => return false,
            }
        }
        false
    }

    /// Whether the object is seen in the viewport: it and all its parents
    /// are shown.
    pub fn is_shown(&self, id: ObjectId) -> bool {
        let mut at = Some(id);
        for _ in 0..=self.nodes.len() {
            match at.and_then(|i| self.nodes.get(&i)) {
                Some(node) if !node.object.visible => return false,
                Some(node) => at = node.parent,
                None => return at.is_none(),
            }
        }
        false
    }

    /// Where the object is in the world: its transform combined with all
    /// its parents'.
    pub fn world_matrix(&self, id: ObjectId) -> Mat4 {
        let mut matrix = Mat4::IDENTITY;
        let mut at = Some(id);
        for _ in 0..=self.nodes.len() {
            match at.and_then(|i| self.nodes.get(&i)) {
                Some(node) => {
                    matrix = node.object.transform.matrix() * matrix;
                    at = node.parent;
                }
                None => break,
            }
        }
        matrix
    }

    /// The box around everything shown among `ids` and the objects inside
    /// them, in world space, or `None` if none of it has a shape.
    pub fn bounds_of(&self, ids: impl IntoIterator<Item = ObjectId>) -> Option<Bounds> {
        let mut total: Option<Bounds> = None;
        for id in ids {
            for inner in self.subtree(id) {
                if !self.is_shown(inner) {
                    continue;
                }
                let Some(local) = self.object(inner).and_then(|o| o.kind.local_bounds()) else {
                    continue;
                };
                let matrix = self.world_matrix(inner);
                if let Some(world) =
                    Bounds::around(corners(&local).map(|c| matrix.transform_point3(c)))
                {
                    total = Some(match total {
                        Some(t) => t.union(world),
                        None => world,
                    });
                }
            }
        }
        total
    }

    /// The object and everything inside it, top to bottom; empty if it
    /// doesn't exist.
    pub fn subtree(&self, id: ObjectId) -> Vec<ObjectId> {
        let mut out = Vec::new();
        if !self.contains(id) {
            return out;
        }
        let mut stack = vec![id];
        while let Some(at) = stack.pop() {
            // Each object is visited once even in a damaged tree.
            if out.contains(&at) || out.len() > self.nodes.len() {
                continue;
            }
            out.push(at);
            stack.extend(self.children(Some(at)).iter().rev());
        }
        out
    }

    /// The box around everything shown, or `None` if nothing shown has a
    /// shape.
    pub fn bounds(&self) -> Option<Bounds> {
        self.bounds_of(self.roots.clone())
    }

    /// A name for a new object that no other object has yet: `base`, or
    /// `base 2`, `base 3` and so on.
    pub fn unused_name(&self, base: &str) -> String {
        let taken = |name: &str| self.nodes.values().any(|n| n.object.name == name);
        if !taken(base) {
            return base.to_string();
        }
        (2..)
            .map(|n| format!("{base} {n}"))
            .find(|name| !taken(name))
            .unwrap_or_else(|| base.to_string())
    }

    /// The id the next new object gets.
    pub(crate) fn next_id(&self) -> ObjectId {
        ObjectId(self.next_id)
    }

    /// Applies one edit. Fails, changing nothing, if the edit doesn't fit
    /// the scene (which would be a bug in the code that made it).
    pub(crate) fn apply(&mut self, edit: &Edit) -> Result<(), SceneError> {
        match edit {
            Edit::Insert { id, object, place } => {
                if self.nodes.contains_key(id) {
                    return Err(SceneError::History("an object with that id already exists"));
                }
                self.check_parent(place.parent)?;
                let siblings = self.children(place.parent).len();
                if place.index > siblings {
                    return Err(SceneError::History("an object's place no longer exists"));
                }
                self.nodes.insert(
                    *id,
                    Node {
                        object: object.clone(),
                        parent: place.parent,
                        children: Vec::new(),
                    },
                );
                self.siblings_mut(place.parent)?.insert(place.index, *id);
                self.next_id = self.next_id.max(id.0 + 1);
                Ok(())
            }
            Edit::Remove { id, place, .. } => {
                let node = self.nodes.get(id).ok_or(SceneError::UnknownObject(*id))?;
                if !node.children.is_empty() {
                    return Err(SceneError::History("only an empty object can be taken out"));
                }
                if self.place(*id) != Some(*place) {
                    return Err(SceneError::History("an object is not where it was"));
                }
                self.siblings_mut(place.parent)?.remove(place.index);
                self.nodes.remove(id);
                Ok(())
            }
            Edit::Relocate { id, from, to } => {
                if self.place(*id) != Some(*from) {
                    return Err(SceneError::History("an object is not where it was"));
                }
                self.check_parent(to.parent)?;
                if to.parent.is_some_and(|p| self.is_inside(p, *id)) {
                    return Err(SceneError::NestedInItself);
                }
                let len_after =
                    self.children(to.parent).len() - usize::from(to.parent == from.parent);
                if to.index > len_after {
                    return Err(SceneError::History("an object's place no longer exists"));
                }
                self.siblings_mut(from.parent)?.remove(from.index);
                self.siblings_mut(to.parent)?.insert(to.index, *id);
                if let Some(node) = self.nodes.get_mut(id) {
                    node.parent = to.parent;
                }
                Ok(())
            }
            Edit::Rename { id, before, after } => {
                let node = self
                    .nodes
                    .get_mut(id)
                    .ok_or(SceneError::UnknownObject(*id))?;
                if node.object.name != *before {
                    return Err(SceneError::History("an object's name has changed"));
                }
                node.object.name.clone_from(after);
                Ok(())
            }
            Edit::SetVisible { id, before, after } => {
                let node = self
                    .nodes
                    .get_mut(id)
                    .ok_or(SceneError::UnknownObject(*id))?;
                if node.object.visible != *before {
                    return Err(SceneError::History("an object's visibility has changed"));
                }
                node.object.visible = *after;
                Ok(())
            }
        }
    }

    fn check_parent(&self, parent: Option<ObjectId>) -> Result<(), SceneError> {
        match parent {
            Some(p) if !self.nodes.contains_key(&p) => Err(SceneError::UnknownObject(p)),
            _ => Ok(()),
        }
    }

    fn siblings_mut(&mut self, parent: Option<ObjectId>) -> Result<&mut Vec<ObjectId>, SceneError> {
        match parent {
            None => Ok(&mut self.roots),
            Some(p) => self
                .nodes
                .get_mut(&p)
                .map(|n| &mut n.children)
                .ok_or(SceneError::UnknownObject(p)),
        }
    }
}

/// The eight corners of a box.
fn corners(b: &Bounds) -> [glam::Vec3; 8] {
    std::array::from_fn(|i| {
        glam::Vec3::new(
            if i & 1 == 0 { b.min.x } else { b.max.x },
            if i & 2 == 0 { b.min.y } else { b.max.y },
            if i & 4 == 0 { b.min.z } else { b.max.z },
        )
    })
}
