//! Version 16 of the project file's contents ([decision 0068](../../../../docs/decisions/0068-pivot-editing-and-scene-units.md)):
//! version 15's, with pivots placed apart from their shapes and the
//! project's units.
//!
//! The objects, their recorded edits (version 15's operations, stored as
//! they were) and the instances are version 15's file as it was. Beside
//! it: where each shape sits relative to its pivot, for objects whose
//! pivot has been moved on its own (the others' shapes sit on their
//! pivots), and the units the project is measured in. Older files open
//! with every shape on its pivot, in metres.

use super::v1::{Problem, TransformV1, finite};
use super::v12::ProjectWithInstances;
use super::v15::EntryV15;
use crate::limits::Bounded;
use scene::{Document, Transform, Units};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Version 16 of the file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProjectV16 {
    /// Version 15's file: the objects, their edits, the selection, the
    /// mesh layers and the instances.
    pub(crate) project: ProjectWithInstances<EntryV15>,
    /// Where shapes sit relative to their pivots, by object (only those
    /// not on their pivots).
    pub(crate) offsets: Bounded<OffsetV16>,
    /// The project's units.
    pub(crate) units: UnitsV16,
}

/// Where an object's shape sits relative to its pivot.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct OffsetV16 {
    pub(crate) object: u64,
    pub(crate) offset: TransformV1,
}

/// The units a project is measured in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum UnitsV16 {
    Metres,
    Centimetres,
}

impl ProjectV16 {
    /// The file model of a document.
    pub(crate) fn from_document(document: &Document) -> Self {
        let scene = document.scene();
        let offsets = scene
            .walk()
            .into_iter()
            .filter_map(|(id, _)| {
                let offset = scene.object(id)?.offset;
                (offset != Transform::default()).then(|| OffsetV16 {
                    object: id.get(),
                    offset: TransformV1 {
                        position: offset.position.to_array(),
                        rotation: offset.rotation.to_array(),
                        scale: offset.scale.to_array(),
                    },
                })
            })
            .collect();
        Self {
            project: ProjectWithInstances::from_document(document),
            offsets,
            units: match scene.units() {
                Units::Metres => UnitsV16::Metres,
                Units::Centimetres => UnitsV16::Centimetres,
            },
        }
    }

    /// The document this file model describes, after checking every value.
    pub(crate) fn into_document(self) -> Result<Document, Problem> {
        let mut offsets: BTreeMap<u64, TransformV1> = BTreeMap::new();
        for entry in self.offsets {
            if offsets.insert(entry.object, entry.offset).is_some() {
                return Err("an object's shape is placed twice".into());
            }
        }
        let document = self.project.into_document_with(|id, object| {
            let Some(offset) = offsets.remove(&id) else {
                return Ok(());
            };
            let offset = Transform {
                position: finite(offset.position, "shape's place")?,
                rotation: finite(offset.rotation, "shape's turn")?,
                scale: finite(offset.scale, "shape's size")?,
            };
            if offset.matrix().determinant().abs() < 1e-12 {
                return Err("its shape is squashed flat against its pivot".into());
            }
            object.offset = offset;
            Ok(())
        })?;
        if let Some(id) = offsets.keys().next() {
            return Err(format!(
                "a shape's place is stored for object {id}, which isn't there"
            ));
        }
        Ok(document.with_units(match self.units {
            UnitsV16::Metres => Units::Metres,
            UnitsV16::Centimetres => Units::Centimetres,
        }))
    }
}
