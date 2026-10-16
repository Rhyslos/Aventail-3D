//! Version 20 of the project file's contents ([decision 0078](../../../../docs/decisions/0078-kit-export.md)):
//! version 19's, with each object's collision shape.
//!
//! Beside version 19's file: the collision shape of each object that has
//! one. Older files open with none.

use super::v1::{FileOperation, Problem};
use super::v18::EntryV18;
use super::v19::ProjectV19;
use crate::limits::Bounded;
use scene::{Collision, Document};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Version 20 of the file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProjectV20<E = EntryV18> {
    /// Version 19's file (its edits stored as `E`).
    pub(crate) project: ProjectV19<E>,
    /// The objects' collision shapes (only objects with one).
    pub(crate) collisions: Bounded<CollisionV20>,
}

/// An object's collision shape.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct CollisionV20 {
    pub(crate) object: u64,
    pub(crate) shape: ShapeV20,
}

/// A collision shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ShapeV20 {
    Mesh,
    Convex,
    Box,
}

impl<E: FileOperation> ProjectV20<E> {
    /// The file model of a document.
    pub(crate) fn from_document(document: &Document) -> Self {
        let scene = document.scene();
        let collisions = scene
            .walk()
            .into_iter()
            .filter_map(|(id, _)| {
                let shape = match scene.object(id)?.collision {
                    Collision::None => return None,
                    Collision::Mesh => ShapeV20::Mesh,
                    Collision::Convex => ShapeV20::Convex,
                    Collision::Box => ShapeV20::Box,
                };
                Some(CollisionV20 {
                    object: id.get(),
                    shape,
                })
            })
            .collect();
        Self {
            project: ProjectV19::from_document(document),
            collisions,
        }
    }

    /// The document this file model describes, after checking every value.
    pub(crate) fn into_document(self) -> Result<Document, Problem> {
        self.into_document_with(|_, _| Ok(()))
    }

    /// The same, with `attach` giving each object (by its id in the file)
    /// what later versions store beside this one.
    pub(crate) fn into_document_with(
        self,
        mut attach: impl FnMut(u64, &mut scene::Object) -> Result<(), Problem>,
    ) -> Result<Document, Problem> {
        let mut shapes: BTreeMap<u64, Collision> = BTreeMap::new();
        for entry in self.collisions {
            let shape = match entry.shape {
                ShapeV20::Mesh => Collision::Mesh,
                ShapeV20::Convex => Collision::Convex,
                ShapeV20::Box => Collision::Box,
            };
            if shapes.insert(entry.object, shape).is_some() {
                return Err("an object's collision is stored twice".into());
            }
        }
        let document = self.project.into_document_with(|id, object| {
            if let Some(shape) = shapes.remove(&id) {
                object.collision = shape;
            }
            attach(id, object)
        })?;
        if let Some(id) = shapes.keys().next() {
            return Err(format!(
                "a collision is stored for object {id}, which isn't there"
            ));
        }
        Ok(document)
    }
}
