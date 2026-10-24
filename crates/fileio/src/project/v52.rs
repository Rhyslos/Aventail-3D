//! Version 52 of the project file ([decision 0151](../../../../docs/decisions/0151-collision-hulls.md)):
//! version 51's, with each object's collision in parts (*Wrapped in
//! parts*) and how it is split, stored with its development settings
//! (`v39::ObjectV39::collision_parts`, as [`CollisionPartsV52`]).
//!
//! Version 20's list of collisions knows only none, the shape itself,
//! wrapped and a box: an object wrapped in parts is listed there as
//! wrapped, its closest older choice, and marked here as in parts. Files
//! before version 52 hold no collision parts.

use scene::ConvexPartsOptions;
use serde::{Deserialize, Serialize};

/// An object's collision parts as stored.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct CollisionPartsV52 {
    /// Whether its collision is in parts (version 20's list then says
    /// wrapped).
    pub(crate) chosen: bool,
    /// Max parts.
    pub(crate) max_parts: u32,
    /// Detail, 0 to 1.
    pub(crate) detail: f32,
}

impl CollisionPartsV52 {
    /// The stored form of an object's collision parts: only when chosen,
    /// or set to something other than the standard.
    pub(crate) fn of(object: &scene::Object) -> Option<Self> {
        let chosen = object.collision == scene::Collision::Parts;
        let options = object.collision_parts;
        (chosen || options != ConvexPartsOptions::default()).then_some(Self {
            chosen,
            max_parts: options.max_parts,
            detail: options.detail,
        })
    }

    /// The settings read back (clamped by whoever uses them).
    pub(crate) fn options(&self) -> ConvexPartsOptions {
        ConvexPartsOptions {
            max_parts: self.max_parts,
            detail: self.detail,
        }
    }
}
