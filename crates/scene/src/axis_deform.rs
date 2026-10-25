//! The *Bend, twist or taper* modifier in the scene ([decision 0158](../../../docs/decisions/0158-bend-twist-taper.md)):
//! one modifier kind ([`crate::ModifierKind::BendTwistTaper`]) whose
//! **Kind** setting picks a bend, a twist or a taper, worked out round
//! the object's pivot (moved by its Centre) by [`mesh::axis_deform`].
//!
//! It reads no other object. What the viewport draws to show it while
//! the object is selected (its axis as it bends, its limits and its
//! centre) comes from [`Scene::deform_helpers`].

use crate::modifier::ModifierKind;
use crate::object::ObjectId;
use crate::tree::Scene;
use mesh::axis_deform::{DeformHelper, helper};

impl Scene {
    /// The helpers for object `id`'s *Bend, twist or taper* modifiers that
    /// are on and shown in the viewport, in its shape's space, first to
    /// last: each measured on the shape that modifier last started from
    /// ([`Scene::modifier_input`]). Nothing is worked out here but the
    /// helpers themselves; empty for an instance (its original shows
    /// them).
    pub fn deform_helpers(&self, id: ObjectId) -> Vec<DeformHelper> {
        if self.modifier_owner(id) != id {
            return Vec::new();
        }
        let pivot = self.pivot_in_shape(id);
        self.modifiers(id)
            .iter()
            .enumerate()
            .filter(|(_, m)| m.applies(crate::Purpose::Viewport))
            .filter_map(|(index, m)| match &m.kind {
                ModifierKind::BendTwistTaper(options) => {
                    let input = self.modifier_input(id, index)?;
                    helper(&input, options, pivot)
                }
                _ => None,
            })
            .collect()
    }
}
