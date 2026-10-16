//! Layout snapshots ([decision 0114](../../../docs/decisions/0114-layout-snapshots.md)):
//! named versions of the UV layout, kept in the project, to go back to or
//! compare with.
//!
//! A snapshot holds, for each object it was taken of, every polygon's
//! corner UVs and the seams (a [`UvLayout`]). Taking, renaming and
//! removing snapshots are steps to undo ([`crate::Command::SetSnapshots`]);
//! putting one back is one recorded edit per object
//! ([`crate::Operation::RestoreUvs`]), so it is undone like any UV edit and
//! never loses what the shape went through.

use crate::{ObjectId, ObjectKind, Operation, Scene};
use mesh::uvs::UvLayout;
use std::sync::Arc;

/// The most snapshots a project keeps.
pub const MAX_SNAPSHOTS: usize = 32;

/// The longest name a snapshot keeps, in bytes.
pub const MAX_SNAPSHOT_NAME: usize = 64;

/// One named version of the UV layout.
#[derive(Debug, Clone, PartialEq)]
pub struct UvSnapshot {
    /// Its name ("Before packing").
    pub name: String,
    /// The layout of each object it was taken of, in the order they were
    /// selected. Shared, so copies of the list (in the undo history) cost
    /// little.
    pub objects: Vec<(ObjectId, Arc<UvLayout>)>,
}

impl UvSnapshot {
    /// A snapshot named `name` of the UVs of `ids` (those with their own
    /// shape and UVs; instances show their original's), or `None` if none
    /// has UVs.
    pub fn take(scene: &Scene, ids: &[ObjectId], name: &str) -> Option<Self> {
        let objects: Vec<(ObjectId, Arc<UvLayout>)> = ids
            .iter()
            .filter(|id| {
                scene.object(**id).is_some_and(|o| {
                    matches!(o.kind, ObjectKind::Primitive(_) | ObjectKind::Mesh(_))
                })
            })
            .filter_map(|id| {
                let mesh = scene.evaluated(*id)?.mesh;
                Some((*id, Arc::new(UvLayout::of(&mesh)?)))
            })
            .collect();
        (!objects.is_empty()).then(|| Self {
            name: name.to_string(),
            objects,
        })
    }

    /// The edits that put it back: one per object still in `scene`.
    pub fn restore(&self, scene: &Scene) -> Vec<(ObjectId, Operation)> {
        self.objects
            .iter()
            .filter(|(id, _)| {
                scene.object(*id).is_some_and(|o| {
                    matches!(o.kind, ObjectKind::Primitive(_) | ObjectKind::Mesh(_))
                })
            })
            .map(|(id, layout)| {
                (
                    *id,
                    Operation::RestoreUvs {
                        layout: Arc::clone(layout),
                    },
                )
            })
            .collect()
    }

    /// The layout it holds for object `id`, if any.
    pub fn layout(&self, id: ObjectId) -> Option<&UvLayout> {
        self.objects
            .iter()
            .find(|(o, _)| *o == id)
            .map(|(_, l)| l.as_ref())
    }

    /// How many polygons it holds, over every object.
    pub fn polygon_count(&self) -> usize {
        self.objects.iter().map(|(_, l)| l.polygon_count()).sum()
    }
}

/// Whether `name` is one a snapshot can have: not blank, on one line, at
/// most [`MAX_SNAPSHOT_NAME`] bytes.
pub fn name_is_valid(name: &str) -> bool {
    !name.trim().is_empty()
        && name.len() <= MAX_SNAPSHOT_NAME
        && !name.chars().any(char::is_control)
}

/// Whether a project can keep `list`: at most [`MAX_SNAPSHOTS`], each
/// named (names all different) and holding real UVs.
pub fn snapshots_are_valid(list: &[UvSnapshot]) -> bool {
    list.len() <= MAX_SNAPSHOTS
        && list.iter().enumerate().all(|(i, s)| {
            name_is_valid(&s.name)
                && s.objects.iter().all(|(_, l)| l.is_real())
                && !list[..i].iter().any(|t| t.name == s.name)
        })
}

/// A name for a new snapshot that no snapshot in `list` has: `wanted`, or
/// `wanted` with a number after it ("Snapshot 2").
pub fn free_name(list: &[UvSnapshot], wanted: &str) -> String {
    let mut base = wanted.trim().to_string();
    if base.is_empty() {
        base = "Snapshot".into();
    }
    // Room for a number after it.
    while base.len() > MAX_SNAPSHOT_NAME - 4 {
        base.pop();
    }
    let taken = |n: &str| list.iter().any(|s| s.name == n);
    if !taken(&base) {
        return base;
    }
    (2..)
        .map(|k| format!("{base} {k}"))
        .find(|n| !taken(n))
        .unwrap_or(base)
}

#[cfg(test)]
mod tests;
