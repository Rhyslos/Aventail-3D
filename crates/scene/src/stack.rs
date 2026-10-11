//! An object's operation stack: the edits recorded on top of its base
//! (a primitive's settings, or a plain mesh), and how the final shape is
//! worked out from them (DESIGN.md §4.2, decisions
//! [0006](../../../docs/decisions/0006-base-plus-operation-stack.md) and
//! [0021](../../../docs/decisions/0021-object-stack-foundation.md)).
//!
//! The shape is evaluated from the bottom up: the base's mesh, then each
//! layer's operations in order. Each layer's result is cached, so a change
//! in one layer only re-evaluates from that layer upward, and a change to
//! the base (a cube's segments, say) replays every recorded edit on the
//! new shape. Until the Edit Layers interface arrives, every edit goes
//! into one layer, so the user sees a plain object.

use crate::object::{ObjectId, ObjectKind};
use glam::Vec3;
use mesh::edit::{self, Outcome};
use mesh::{Mesh, VertexId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// The name of the layer edits go into until layers can be managed.
pub const EDITS_LAYER: &str = "Edits";

/// One recorded edit. Elements are named by id, so replaying the same
/// history gives the same result; ids that are no longer in the shape are
/// skipped and reported (see [`StackWarning`]). More kinds arrive with the
/// modelling tools.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Operation {
    /// Moves points by an offset, in the object's own space (centimetres).
    MovePoints {
        /// The points.
        points: Vec<VertexId>,
        /// How far.
        offset: Vec3,
    },
}

impl Operation {
    /// A short description, such as "Move 3 points".
    pub fn describe(&self) -> String {
        match self {
            Self::MovePoints { points, .. } => match points.len() {
                1 => "Move 1 point".to_string(),
                n => format!("Move {n} points"),
            },
        }
    }

    /// Applies the operation to `mesh`.
    fn apply(&self, mesh: &mut Mesh) -> Outcome {
        match self {
            Self::MovePoints { points, offset } => edit::move_points(mesh, points, *offset),
        }
    }
}

/// A named group of operations that can be switched off as a whole.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    /// Its name, for the future Edit Layers panel.
    pub name: String,
    /// A switched-off layer passes the shape below it through unchanged.
    pub enabled: bool,
    /// Its operations, in the order they apply.
    pub operations: Vec<Operation>,
}

impl Layer {
    /// An empty, switched-on layer.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            enabled: true,
            operations: Vec::new(),
        }
    }
}

/// The layers on top of an object's base, bottom first. Empty for a new
/// object. Changed only through commands.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Stack {
    pub(crate) layers: Vec<Layer>,
}

impl Stack {
    /// A stack of these layers, bottom first (for opening saved files;
    /// in use, the stack only changes through commands).
    pub fn from_layers(layers: Vec<Layer>) -> Self {
        Self { layers }
    }

    /// The layers, bottom first.
    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    /// Whether nothing is recorded.
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    /// How many operations are recorded, in every layer.
    pub fn operation_count(&self) -> usize {
        self.layers.iter().map(|l| l.operations.len()).sum()
    }
}

/// What evaluation can depend on besides the object itself. Empty for
/// now; a point in time joins it with animation, without every function
/// that evaluates having to change (DESIGN.md §4.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EvalContext {}

/// A recorded operation that no longer fully matches the shape under it:
/// some of the elements it names are gone (a cube was given fewer
/// segments, for example). The rest of the operation still applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackWarning {
    /// Which layer, from the bottom (0).
    pub layer: usize,
    /// Which operation in the layer, from the first (0).
    pub operation: usize,
    /// How many of its elements are missing.
    pub missing: usize,
}

/// An object's final shape and any warnings from working it out.
#[derive(Debug, Clone, PartialEq)]
pub struct Evaluated {
    /// The shape, in the object's own space.
    pub mesh: Arc<Mesh>,
    /// Operations that no longer fully match, bottom first.
    pub warnings: Vec<StackWarning>,
}

/// The shape a base gives on its own, or `None` for a kind without one.
fn base_mesh(base: &ObjectKind) -> Option<Arc<Mesh>> {
    match base {
        ObjectKind::Group => None,
        ObjectKind::Primitive(p) => Some(Arc::new(p.mesh())),
        ObjectKind::Mesh(mesh) => Some(Arc::clone(mesh)),
    }
}

/// Whether two bases give the same shape, without comparing whole meshes
/// when they are the same one.
fn same_base(a: &ObjectKind, b: &ObjectKind) -> bool {
    match (a, b) {
        (ObjectKind::Mesh(x), ObjectKind::Mesh(y)) => Arc::ptr_eq(x, y) || x == y,
        _ => a == b,
    }
}

/// Evaluates `stack` on top of `input`, layers from `from` upward, adding
/// each layer's result to `out`.
fn evaluate_layers(input: &Arc<Mesh>, layers: &[Layer], from: usize, out: &mut Vec<CachedLayer>) {
    let mut below = out
        .last()
        .map_or_else(|| Arc::clone(input), |l| Arc::clone(&l.mesh));
    for (index, layer) in layers.iter().enumerate().skip(from) {
        let mut warnings = Vec::new();
        let mesh = if layer.enabled && !layer.operations.is_empty() {
            let mut mesh = (*below).clone();
            for (op_index, operation) in layer.operations.iter().enumerate() {
                let outcome = operation.apply(&mut mesh);
                if outcome.missing > 0 {
                    warnings.push(StackWarning {
                        layer: index,
                        operation: op_index,
                        missing: outcome.missing,
                    });
                }
            }
            Arc::new(mesh)
        } else {
            Arc::clone(&below)
        };
        below = Arc::clone(&mesh);
        out.push(CachedLayer {
            layer: layer.clone(),
            mesh,
            warnings,
        });
    }
}

/// What the cache had to work out on its last evaluation of an object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Rebuilt {
    /// Nothing: the cached shape was still right.
    Nothing,
    /// The base and every layer.
    Everything,
    /// The layers from this one upward.
    From(usize),
}

/// One layer's cached result.
#[derive(Debug, Clone)]
struct CachedLayer {
    /// The layer as it was evaluated, to tell when it changes.
    layer: Layer,
    mesh: Arc<Mesh>,
    warnings: Vec<StackWarning>,
}

/// One object's cached evaluation.
#[derive(Debug, Clone)]
struct Entry {
    base: ObjectKind,
    base_mesh: Arc<Mesh>,
    layers: Vec<CachedLayer>,
    last: Rebuilt,
}

impl Entry {
    fn result(&self) -> Evaluated {
        Evaluated {
            mesh: self
                .layers
                .last()
                .map_or_else(|| Arc::clone(&self.base_mesh), |l| Arc::clone(&l.mesh)),
            warnings: self
                .layers
                .iter()
                .flat_map(|l| l.warnings.clone())
                .collect(),
        }
    }
}

/// Every object's evaluated shape, layer by layer, worked out when first
/// asked for and kept until the object changes.
#[derive(Debug, Default)]
pub(crate) struct StackCache {
    entries: HashMap<ObjectId, Entry>,
}

impl StackCache {
    /// The shape of object `id` with base `base` and `stack`, from the
    /// cache where it still matches. `None` for a kind without a shape.
    pub(crate) fn evaluate(
        &mut self,
        id: ObjectId,
        base: &ObjectKind,
        stack: &Stack,
        _context: &EvalContext,
    ) -> Option<Evaluated> {
        if let Some(entry) = self.entries.get_mut(&id)
            && same_base(&entry.base, base)
        {
            // The first layer that changed (or is new), if any.
            let first_changed = stack
                .layers
                .iter()
                .zip(&entry.layers)
                .position(|(now, cached)| *now != cached.layer)
                .unwrap_or(entry.layers.len().min(stack.layers.len()));
            if first_changed == stack.layers.len() && entry.layers.len() == stack.layers.len() {
                entry.last = Rebuilt::Nothing;
                return Some(entry.result());
            }
            entry.layers.truncate(first_changed);
            let base_mesh = Arc::clone(&entry.base_mesh);
            evaluate_layers(&base_mesh, &stack.layers, first_changed, &mut entry.layers);
            entry.last = Rebuilt::From(first_changed);
            return Some(entry.result());
        }
        let base_mesh = base_mesh(base)?;
        let mut layers = Vec::new();
        evaluate_layers(&base_mesh, &stack.layers, 0, &mut layers);
        let entry = Entry {
            base: base.clone(),
            base_mesh,
            layers,
            last: Rebuilt::Everything,
        };
        let result = entry.result();
        self.entries.insert(id, entry);
        Some(result)
    }

    /// Forgets an object (it was deleted).
    pub(crate) fn forget(&mut self, id: ObjectId) {
        self.entries.remove(&id);
    }

    /// What the last evaluation of `id` had to work out.
    #[cfg(test)]
    pub(crate) fn last_rebuilt(&self, id: ObjectId) -> Option<Rebuilt> {
        self.entries.get(&id).map(|e| e.last)
    }
}

/// The cache as the scene keeps it: shared behind a lock so the shape can
/// be worked out while the scene is only being read. Not part of what the
/// scene *is*: a copy of a scene starts with an empty cache, and two
/// scenes compare equal whatever their caches hold.
#[derive(Default)]
pub(crate) struct SharedCache(Mutex<StackCache>);

impl SharedCache {
    /// Runs `f` with the cache. A panic elsewhere while it was held leaves
    /// a cache that is still usable (at worst it re-evaluates).
    pub(crate) fn with<T>(&self, f: impl FnOnce(&mut StackCache) -> T) -> T {
        let mut cache = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        f(&mut cache)
    }

    /// The cache, when the scene is being changed (no locking needed).
    pub(crate) fn get_mut(&mut self) -> &mut StackCache {
        self.0
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl Clone for SharedCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl PartialEq for SharedCache {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl std::fmt::Debug for SharedCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SharedCache")
    }
}
