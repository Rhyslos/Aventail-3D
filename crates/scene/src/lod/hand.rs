//! Levels made by hand (decision 0165): a level's own shape, an object
//! inside the object with the levels, made from what the level showed and
//! then edited like any other shape. It no longer follows the model; a
//! fingerprint of the model it was made from tells when the model has
//! changed since ([`Scene::level_drifted`]).
//!
//! The shape is the level's only while it is inside its object and has a
//! shape of its own; dragged out of it in the Objects panel, or deleted,
//! the level is made from the model again and the object is an ordinary
//! one. It is drawn, picked and edited only while it is selected (which is
//! how a level is edited: the Levels panel's *Edit this level* selects
//! it), and its object is drawn in its place meanwhile; exports write it
//! as its level, never as an object of its own.

use crate::object::ObjectId;
use crate::tree::Scene;
use glam::Mat4;
use mesh::Mesh;
use std::sync::Arc;

/// The name a level's shape made by hand gets: its object's name with
/// `LOD` and the level's number (`Rock LOD1`), as exports name levels.
pub fn level_name(object: &str, level: usize) -> String {
    words::trf!("{} LOD{}", object, level)
}

/// A small, steady hash (FNV-1a), the same on every computer and in every
/// version, so fingerprints can be saved.
struct Hash(u64);

impl Hash {
    fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn add(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }
}

/// A fingerprint of `mesh`: its structure and where its points are.
fn mesh_fingerprint(mesh: &Mesh) -> u64 {
    let mut hash = Hash::new();
    hash.add(&mesh.structure_fingerprint().to_le_bytes());
    for (id, at) in mesh.vertices() {
        hash.add(&id.get().to_le_bytes());
        for c in at.to_array() {
            hash.add(&c.to_bits().to_le_bytes());
        }
    }
    hash.0
}

impl Scene {
    /// The object holding level `k` (0 for Level 1) of object `id`'s
    /// levels made by hand, with where its shape sits in `id`'s shape's
    /// space (`None` when exactly on it): only while that object is inside
    /// `id` and has a shape.
    pub fn level_shape(&self, id: ObjectId, k: usize) -> Option<(ObjectId, Option<Mat4>)> {
        let hand = self.object(id)?.lods.levels.get(k)?.hand?;
        let shape = hand.shape;
        if shape == id
            || !self.is_inside(shape, id)
            || !self.object(shape).is_some_and(|o| o.kind.has_shape())
        {
            return None;
        }
        let into = self.shape_matrix(id);
        if into.determinant().abs() < 1e-12 {
            return None;
        }
        let matrix = into.inverse() * self.shape_matrix(shape);
        let exact = matrix.abs_diff_eq(Mat4::IDENTITY, 1e-5);
        Some((shape, (!exact && matrix.is_finite()).then_some(matrix)))
    }

    /// The object and level (0 for Level 1) whose shape made by hand
    /// object `shape` is, if it is one (see [`Scene::level_shape`]).
    pub fn level_owner(&self, shape: ObjectId) -> Option<(ObjectId, usize)> {
        let mut at = self.parent(shape);
        // Bounded by the number of objects, so a damaged tree can't loop.
        for _ in 0..=self.len() {
            let owner = at?;
            if let Some(k) = self.object(owner).and_then(|o| {
                o.lods
                    .levels
                    .iter()
                    .position(|l| l.hand.is_some_and(|h| h.shape == shape))
            }) && self.level_shape(owner, k).is_some()
            {
                return Some((owner, k));
            }
            at = self.parent(owner);
        }
        None
    }

    /// Whether `id` is a level's shape made by hand: drawn only while
    /// selected, and never exported as an object of its own.
    pub fn is_level_shape(&self, id: ObjectId) -> bool {
        self.level_owner(id).is_some()
    }

    /// A fingerprint of object `id`'s model, what its levels are made from:
    /// its shape before its modifiers (edits, booleans and live mirror),
    /// its modifiers (an instance's original's) and its smoothing. Worked
    /// out here when needed (heavy booleans included).
    pub fn model_fingerprint(&self, id: ObjectId) -> Option<u64> {
        let shape = self.unmodified_shape(id)?;
        Some(self.fingerprint_of(id, &shape))
    }

    /// [`Scene::model_fingerprint`], unless heavy booleans are still to be
    /// worked out (the interface does that on another thread): for the
    /// Levels panel, asked every frame.
    pub fn model_fingerprint_if_ready(&self, id: ObjectId) -> Option<u64> {
        let shape = self.input_if_light(id)?;
        Some(self.fingerprint_of(id, &shape))
    }

    fn fingerprint_of(&self, id: ObjectId, shape: &Arc<Mesh>) -> u64 {
        let geometry = {
            let mut kept = self
                .lods
                .fingerprints
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            match kept.get(&id) {
                Some((mesh, print)) if Arc::ptr_eq(mesh, shape) => *print,
                _ => {
                    let print = mesh_fingerprint(shape);
                    kept.insert(id, (Arc::clone(shape), print));
                    print
                }
            }
        };
        let owner = self.modifier_owner(id);
        let mut hash = Hash::new();
        hash.add(&geometry.to_le_bytes());
        for m in self.modifiers(owner) {
            // Its settings as written out: the same for the same settings.
            hash.add(format!("{:?}{}{}", m.kind, m.on, m.in_viewport).as_bytes());
        }
        if let Some(object) = self.object(owner) {
            hash.add(format!("{:?}", object.hardness).as_bytes());
        }
        hash.0
    }

    /// Whether object `id`'s level `k` (0 for Level 1), made by hand, was
    /// made from a model that has changed since. `None` when it isn't made
    /// by hand, or the model isn't worked out yet.
    pub fn level_drifted(&self, id: ObjectId, k: usize) -> Option<bool> {
        let hand = self.object(id)?.lods.levels.get(k)?.hand?;
        self.level_shape(id, k)?;
        Some(self.model_fingerprint_if_ready(id)? != hand.made_from)
    }

    /// What level `k` (0 for Level 1) of object `id` shows when made from
    /// the model: the model with the level's changes to its modifiers,
    /// reduced if it is, exactly as [`Scene::lod_shapes`] makes it, worked
    /// out here (for making the level by hand, or starting it again).
    /// `None` without levels of detail on.
    pub(crate) fn automatic_level(&self, id: ObjectId, k: usize) -> Option<Arc<Mesh>> {
        let job = self.lod_plan_as(id, false, Some(k))??;
        let done = job.run();
        let level = done.shapes().levels.get(k).cloned();
        self.store_lod_stacks(done);
        level
    }
}
