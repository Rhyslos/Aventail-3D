//! The Weighted normals modifier in the scene ([decision 0148](../../../docs/decisions/0148-weighted-normals.md)):
//! which of an object's custom normals reach its final shape, and which
//! modifier drops them.
//!
//! Weighted normals sets custom normals on every corner of its result
//! ([`mesh::custom_normals`]). Any modifier after it but Triangulate
//! (decision 0161) changes the shape, so the stack drops them there ([`crate::ModifierKind::carries_custom_normals`])
//! and the shading is worked out again: it is best last, and the
//! Attributes panel says so when it isn't.

use crate::modifier::{ModifierKind, Purpose};
use crate::object::ObjectId;
use crate::tree::Scene;
use mesh::weighted_normals::WeightedNormalsOptions;

impl Scene {
    /// The settings of the Weighted normals modifier whose custom normals
    /// reach object `id`'s final shape for `purpose` (an instance's
    /// original's): the last modifier worked out for `purpose`, when it is
    /// one. `None` when there is none, or a modifier after it drops them.
    pub fn final_weighted_normals(
        &self,
        id: ObjectId,
        purpose: Purpose,
    ) -> Option<WeightedNormalsOptions> {
        match self
            .modifiers(self.modifier_owner(id))
            .iter()
            .rev()
            // A Triangulate after it carries its normals through.
            .find(|m| m.applies(purpose) && !m.kind.keeps_shading())
            .map(|m| &m.kind)
        {
            Some(ModifierKind::WeightedNormals(options)) => Some(options.clamped()),
            _ => None,
        }
    }

    /// The name of the first modifier after the one at `index` on object
    /// `id` that is on: the one that drops the custom normals the modifier
    /// at `index` (a Weighted normals) makes. `None` when it is the last
    /// one on, or only a Triangulate, which carries them through, comes
    /// after it.
    pub fn normals_dropped_by(&self, id: ObjectId, index: usize) -> Option<&'static str> {
        self.modifiers(id)
            .iter()
            .skip(index.saturating_add(1))
            .find(|m| m.on && !m.kind.keeps_shading())
            .map(|m| m.kind.name())
    }
}
