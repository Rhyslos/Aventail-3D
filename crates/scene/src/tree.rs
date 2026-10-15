//! The object tree, and the small reversible edits every change to it is
//! made of.

use crate::object::{Object, ObjectId, ObjectKind, Transform};
use crate::stack::{Entry, EvalContext, Evaluated, Layer, SharedCache};
use crate::units::Units;
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
    /// Settings for one kind of primitive were given to something else.
    WrongKind(&'static str),
    /// Edits can only be recorded on an object with a shape, not a group.
    NoShape,
    /// An instance shows its original's shape and has no edits of its own.
    Instance,
    /// A position, turn or size that isn't a number.
    NotANumber,
    /// More materials than an object can have.
    TooManyMaterials,
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
            Self::WrongKind(kind) => write!(
                f,
                "{} settings can only be given to a {}",
                kind,
                kind.to_lowercase()
            ),
            Self::NoShape => f.write_str("a group has no shape to edit"),
            Self::Instance => f.write_str(
                "it's an instance: edit its original, or make it editable to give it its own shape",
            ),
            Self::NotANumber => f.write_str("a value isn't a number"),
            Self::TooManyMaterials => write!(
                f,
                "an object can have at most {} materials",
                mesh::material::MAX_SLOTS
            ),
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
    /// Moves, turns or scales an object (its transform, relative to its
    /// parent).
    SetTransform {
        id: ObjectId,
        before: Transform,
        after: Transform,
    },
    /// Changes the project's units.
    SetUnits { before: Units, after: Units },
    /// Changes the project's game project folder.
    SetExportTarget {
        before: Option<crate::ExportTarget>,
        after: Option<crate::ExportTarget>,
    },
    /// Changes an object's list of materials.
    SetMaterials {
        id: ObjectId,
        before: Vec<crate::Material>,
        after: Vec<crate::Material>,
    },
    /// Moves an object's shape relative to its pivot (its offset).
    SetOffset {
        id: ObjectId,
        before: Transform,
        after: Transform,
    },
    /// Changes what an object is: a primitive's settings, for example.
    SetKind {
        id: ObjectId,
        before: ObjectKind,
        after: ObjectKind,
    },
    /// Adds a layer to an object's stack, at `index` (from the bottom).
    InsertLayer {
        id: ObjectId,
        index: usize,
        layer: Layer,
    },
    /// Takes a layer, with its operations, out of an object's stack.
    RemoveLayer {
        id: ObjectId,
        index: usize,
        layer: Layer,
    },
    /// Adds a recorded edit to one of an object's layers, at `index`.
    InsertEntry {
        id: ObjectId,
        layer: usize,
        index: usize,
        entry: Entry,
    },
    /// Takes a recorded edit out of one of an object's layers.
    RemoveEntry {
        id: ObjectId,
        layer: usize,
        index: usize,
        entry: Entry,
    },
}

impl Edit {
    /// Whether the edit leaves everything as it was.
    pub(crate) fn changes_nothing(&self) -> bool {
        match self {
            Self::Insert { .. }
            | Self::Remove { .. }
            | Self::InsertLayer { .. }
            | Self::RemoveLayer { .. }
            | Self::InsertEntry { .. }
            | Self::RemoveEntry { .. } => false,
            Self::Relocate { from, to, .. } => from == to,
            Self::Rename { before, after, .. } => before == after,
            Self::SetVisible { before, after, .. } => before == after,
            Self::SetKind { before, after, .. } => before == after,
            Self::SetUnits { before, after } => before == after,
            Self::SetExportTarget { before, after } => before == after,
            Self::SetMaterials { before, after, .. } => before == after,
            Self::SetTransform { before, after, .. } | Self::SetOffset { before, after, .. } => {
                before == after
            }
        }
    }

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
            Self::SetKind { id, before, after } => Self::SetKind {
                id,
                before: after,
                after: before,
            },
            Self::SetUnits { before, after } => Self::SetUnits {
                before: after,
                after: before,
            },
            Self::SetExportTarget { before, after } => Self::SetExportTarget {
                before: after,
                after: before,
            },
            Self::SetMaterials { id, before, after } => Self::SetMaterials {
                id,
                before: after,
                after: before,
            },
            Self::SetOffset { id, before, after } => Self::SetOffset {
                id,
                before: after,
                after: before,
            },
            Self::SetTransform { id, before, after } => Self::SetTransform {
                id,
                before: after,
                after: before,
            },
            Self::InsertLayer { id, index, layer } => Self::RemoveLayer { id, index, layer },
            Self::RemoveLayer { id, index, layer } => Self::InsertLayer { id, index, layer },
            Self::InsertEntry {
                id,
                layer,
                index,
                entry,
            } => Self::RemoveEntry {
                id,
                layer,
                index,
                entry,
            },
            Self::RemoveEntry {
                id,
                layer,
                index,
                entry,
            } => Self::InsertEntry {
                id,
                layer,
                index,
                entry,
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

/// Object ids at or above this are refused when a scene is rebuilt from
/// a file: far more than any scene could use, and safe from counting
/// past the largest number.
const MAX_OBJECT_ID: u64 = u64::MAX / 2;

/// Every object in the scene, arranged in a tree: objects can sit inside
/// other objects, and the order among brothers and sisters matters (it is
/// the order the Objects panel lists them in).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Scene {
    nodes: BTreeMap<ObjectId, Node>,
    /// The objects at the top of the tree, in order.
    roots: Vec<ObjectId>,
    /// The next new object's id.
    next_id: u64,
    /// The project's units (decision 0068).
    units: Units,
    /// The project's game project folder, if any (decision 0076).
    export_target: Option<crate::ExportTarget>,
    /// Each object's evaluated shape, layer by layer (not part of what the
    /// scene is: copies start empty, and it never affects equality).
    cache: SharedCache,
}

impl Scene {
    /// An empty scene.
    pub fn new() -> Self {
        Self::default()
    }

    /// Rebuilds a saved scene. `objects` lists every object with its id
    /// and parent, parents before their children and children in order
    /// (as [`Self::walk`] lists them). `next_id` is the id the next new
    /// object gets; it is raised past every id listed, so ids are never
    /// reused. Everything is checked, as it may come from a damaged file:
    /// ids must be unique, parents listed earlier, names not empty, and
    /// groups without edits.
    pub fn from_objects(
        objects: impl IntoIterator<Item = (ObjectId, Option<ObjectId>, Object)>,
        next_id: u64,
    ) -> Result<Self, SceneError> {
        let mut scene = Self::new();
        for (id, parent, object) in objects {
            if object.name.trim().is_empty() {
                return Err(SceneError::EmptyName);
            }
            if !object.kind.has_shape() && !object.stack.is_empty() {
                return Err(SceneError::NoShape);
            }
            if matches!(object.kind, ObjectKind::Instance(_)) && !object.stack.is_empty() {
                return Err(SceneError::Instance);
            }
            if object.stack.repeated_edit_id().is_some() {
                return Err(SceneError::History(
                    "two of an object's edits have the same id",
                ));
            }
            if scene.contains(id) {
                return Err(SceneError::History("two objects have the same id"));
            }
            if id.0 >= MAX_OBJECT_ID {
                return Err(SceneError::History("an object's id is impossibly large"));
            }
            if let Some(p) = parent
                && !scene.contains(p)
            {
                return Err(SceneError::UnknownObject(p));
            }
            let index = scene.children(parent).len();
            scene.apply(&Edit::Insert {
                id,
                object,
                place: Place { parent, index },
            })?;
        }
        // Every instance shows an object that is there, with a shape of
        // its own (not another instance).
        for (id, _) in scene.walk() {
            if let Some(ObjectKind::Instance(source)) = scene.object(id).map(|o| &o.kind) {
                match scene.object(*source).map(|o| &o.kind) {
                    None => return Err(SceneError::UnknownObject(*source)),
                    Some(ObjectKind::Instance(_)) => {
                        return Err(SceneError::History("an instance shows another instance"));
                    }
                    Some(kind) if !kind.has_shape() => return Err(SceneError::NoShape),
                    Some(_) => {}
                }
            }
        }
        scene.next_id = scene.next_id.max(next_id.min(MAX_OBJECT_ID));
        Ok(scene)
    }

    /// The project's units (decision 0068).
    pub fn units(&self) -> Units {
        self.units
    }

    /// The same scene measured in `units`, for a new project or one
    /// being opened (not a step to undo: use [`crate::Command::SetUnits`]
    /// for that).
    #[must_use]
    pub fn with_units(mut self, units: Units) -> Self {
        self.units = units;
        self
    }

    /// The project's game project folder, if any (decision 0076).
    pub fn export_target(&self) -> Option<&crate::ExportTarget> {
        self.export_target.as_ref()
    }

    /// The same scene exporting to `target`, for a project being opened
    /// (not a step to undo: use [`crate::Command::SetExportTarget`]).
    #[must_use]
    pub fn with_export_target(mut self, target: Option<crate::ExportTarget>) -> Self {
        self.export_target = target;
        self
    }

    /// The id the next new object will get (for saving).
    pub fn next_object_id(&self) -> u64 {
        self.next_id
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

    /// The materials `id`'s polygons use (decision 0074): its own list,
    /// an instance's original's, or the standard material while there are
    /// none. Empty for an unknown id.
    pub fn material_list(&self, id: ObjectId) -> Vec<crate::Material> {
        let Some(object) = self.object(id) else {
            return Vec::new();
        };
        match object.kind {
            crate::ObjectKind::Instance(source) => self
                .object(source)
                .map_or_else(|| object.material_list(), crate::Object::material_list),
            _ => object.material_list(),
        }
    }

    /// The object's final shape: its base with every recorded edit
    /// applied, from the cache where nothing has changed. `None` for an
    /// object without a shape (a group) or an unknown id.
    pub fn evaluated(&self, id: ObjectId) -> Option<Evaluated> {
        let object = self.object(id)?;
        if let ObjectKind::Instance(source) = object.kind {
            // An instance's original is never an instance itself, so this
            // goes one step at most.
            return self
                .object(source)
                .filter(|o| !matches!(o.kind, ObjectKind::Instance(_)))
                .and_then(|_| self.evaluated(source));
        }
        self.cache
            .with(|cache| cache.evaluate(id, &object.kind, &object.stack, &EvalContext::default()))
    }

    /// The box around the object's final shape, in its own space, or
    /// `None` without a shape. A primitive with no recorded edits gives
    /// its box without building its mesh.
    pub fn local_bounds(&self, id: ObjectId) -> Option<Bounds> {
        let object = self.object(id)?;
        match &object.kind {
            ObjectKind::Primitive(p) if object.stack.is_empty() => Some(p.bounds()),
            ObjectKind::Group => None,
            _ => self.evaluated(id)?.mesh.bounds(),
        }
    }

    /// Whether [`crate::Command::MakeEditable`] would change `id`: a
    /// primitive (whose settings become a plain mesh), or a mesh with
    /// edits recorded on it. Not a group, or a mesh with nothing recorded.
    pub fn can_make_editable(&self, id: ObjectId) -> bool {
        self.object(id).is_some_and(|o| match o.kind {
            ObjectKind::Primitive(_) | ObjectKind::Instance(_) => true,
            ObjectKind::Mesh(_) => !o.stack.is_empty(),
            ObjectKind::Group => false,
        })
    }

    /// What the cache had to work out the last time `id` was evaluated.
    #[cfg(test)]
    pub(crate) fn last_rebuilt(&self, id: ObjectId) -> Option<crate::stack::Rebuilt> {
        self.cache.with(|cache| cache.last_rebuilt(id))
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

    /// Every object, top to bottom as the Objects panel lists them (each
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

    /// Where the object's shape is in the world: its pivot's place
    /// ([`Self::world_matrix`]) with the shape's offset from the pivot
    /// (decision 0068). Drawing, picking, editing points and joining use
    /// it; children sit relative to the pivot instead.
    pub fn shape_matrix(&self, id: ObjectId) -> Mat4 {
        let offset = self
            .nodes
            .get(&id)
            .map_or(Mat4::IDENTITY, |node| node.object.offset.matrix());
        self.world_matrix(id) * offset
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
                let Some(local) = self.local_bounds(inner) else {
                    continue;
                };
                let matrix = self.shape_matrix(inner);
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
                self.next_id = self.next_id.max(id.0.saturating_add(1));
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
                self.cache.get_mut().forget(*id);
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
            Edit::SetKind { id, before, after } => {
                let node = self
                    .nodes
                    .get_mut(id)
                    .ok_or(SceneError::UnknownObject(*id))?;
                if node.object.kind != *before {
                    return Err(SceneError::History("an object's settings have changed"));
                }
                node.object.kind = after.clone();
                Ok(())
            }
            Edit::SetTransform { id, before, after } => {
                let node = self
                    .nodes
                    .get_mut(id)
                    .ok_or(SceneError::UnknownObject(*id))?;
                if node.object.transform != *before {
                    return Err(SceneError::History("an object's place has changed"));
                }
                node.object.transform = *after;
                Ok(())
            }
            Edit::SetMaterials { id, before, after } => {
                let node = self
                    .nodes
                    .get_mut(id)
                    .ok_or(SceneError::UnknownObject(*id))?;
                if node.object.materials != *before {
                    return Err(SceneError::History("an object's materials have changed"));
                }
                node.object.materials.clone_from(after);
                Ok(())
            }
            Edit::SetExportTarget { before, after } => {
                if self.export_target != *before {
                    return Err(SceneError::History(
                        "the project's game project folder has changed",
                    ));
                }
                self.export_target.clone_from(after);
                Ok(())
            }
            Edit::SetUnits { before, after } => {
                if self.units != *before {
                    return Err(SceneError::History("the project's units have changed"));
                }
                self.units = *after;
                Ok(())
            }
            Edit::SetOffset { id, before, after } => {
                let node = self
                    .nodes
                    .get_mut(id)
                    .ok_or(SceneError::UnknownObject(*id))?;
                if node.object.offset != *before {
                    return Err(SceneError::History("an object's shape has moved"));
                }
                node.object.offset = *after;
                Ok(())
            }
            Edit::InsertLayer { id, index, layer } => {
                let object = self.shaped_mut(*id)?;
                if *index > object.stack.layers.len() {
                    return Err(SceneError::History("a layer's place no longer exists"));
                }
                object.stack.layers.insert(*index, layer.clone());
                Ok(())
            }
            Edit::RemoveLayer { id, index, layer } => {
                let object = self.shaped_mut(*id)?;
                if object.stack.layers.get(*index) != Some(layer) {
                    return Err(SceneError::History("a layer has changed"));
                }
                object.stack.layers.remove(*index);
                Ok(())
            }
            Edit::InsertEntry {
                id,
                layer,
                index,
                entry,
            } => {
                let object = self.shaped_mut(*id)?;
                if object.stack.entry(entry.id).is_some() {
                    return Err(SceneError::History("an edit with that id already exists"));
                }
                let Some(layer) = object.stack.layers.get_mut(*layer) else {
                    return Err(SceneError::History("a layer no longer exists"));
                };
                if *index > layer.entries.len() {
                    return Err(SceneError::History("an edit's place no longer exists"));
                }
                layer.entries.insert(*index, entry.clone());
                Ok(())
            }
            Edit::RemoveEntry {
                id,
                layer,
                index,
                entry,
            } => {
                let object = self.shaped_mut(*id)?;
                let Some(layer) = object.stack.layers.get_mut(*layer) else {
                    return Err(SceneError::History("a layer no longer exists"));
                };
                if layer.entries.get(*index) != Some(entry) {
                    return Err(SceneError::History("an edit has changed"));
                }
                layer.entries.remove(*index);
                Ok(())
            }
        }
    }

    /// An object with a shape, to change its stack.
    fn shaped_mut(&mut self, id: ObjectId) -> Result<&mut Object, SceneError> {
        let node = self
            .nodes
            .get_mut(&id)
            .ok_or(SceneError::UnknownObject(id))?;
        if !node.object.kind.has_shape() {
            return Err(SceneError::NoShape);
        }
        Ok(&mut node.object)
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
