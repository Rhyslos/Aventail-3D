//! Version 12 of the project file's contents ([decision 0055](../../../../docs/decisions/0055-basics.md)):
//! version 11's, with instances.
//!
//! An instance is written in the object list as a group (the kind version
//! 1 knows), and listed beside it with the object it shows, so the object
//! list keeps its shape from version 1. Recorded edits are version 11's,
//! stored inside `OperationV12::V11` as they were.

use super::v1::{FileOperation, ProjectFile, Problem};
use super::v5::{EntryWith, MeshLayersV5, ProjectWith};
use super::v11::OperationV11;
use crate::limits::Bounded;
use scene::{Document, ObjectId, ObjectKind, Operation};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Version 12 of the file.
pub(crate) type ProjectV12 = ProjectWithInstances<EntryV12>;

/// A recorded edit in version 12.
pub(crate) type EntryV12 = EntryWith<OperationV12>;

/// The recorded operations version 12 stores: version 11's.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV12 {
    /// Everything version 11 could store, stored the same way.
    V11(OperationV11),
}

impl FileOperation for OperationV12 {
    fn to_file(op: &Operation) -> Option<Self> {
        Some(Self::V11(OperationV11::to_file(op)?))
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        match self {
            Self::V11(op) => op.into_operation(),
        }
    }
}

/// An instance: the object, and the object whose shape it shows.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct InstanceV12 {
    pub(crate) object: u64,
    pub(crate) source: u64,
}

/// A file of version 12 or later, recorded edits stored as `E`: version
/// 5's, and the instances.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProjectWithInstances<E> {
    /// The objects, their edits and the selection.
    pub(crate) project: ProjectFile<E>,
    /// The attribute layers of plain meshes, by object.
    pub(crate) layers: Bounded<MeshLayersV5>,
    /// The instances, by object.
    pub(crate) instances: Bounded<InstanceV12>,
}

impl<E: FileOperation> ProjectWithInstances<E> {
    /// The file model of a document.
    pub(crate) fn from_document(document: &Document) -> Self {
        let ProjectWith { project, layers } = ProjectWith::<E>::from_document(document);
        let scene = document.scene();
        let instances = scene
            .walk()
            .into_iter()
            .filter_map(|(id, _)| match scene.object(id)?.kind {
                ObjectKind::Instance(source) => Some(InstanceV12 {
                    object: id.get(),
                    source: source.get(),
                }),
                _ => None,
            })
            .collect();
        Self {
            project,
            layers,
            instances,
        }
    }

    /// The document this file model describes, after checking every value.
    pub(crate) fn into_document(self) -> Result<Document, Problem> {
        let mut instances: BTreeMap<u64, u64> = BTreeMap::new();
        for instance in self.instances {
            if instances.insert(instance.object, instance.source).is_some() {
                return Err("an object is listed as an instance twice".into());
            }
        }
        let document = ProjectWith {
            project: self.project,
            layers: self.layers,
        }
        .into_document_with(|id, object| {
            let Some(source) = instances.remove(&id) else {
                return Ok(());
            };
            if object.kind != ObjectKind::Group || !object.stack.is_empty() {
                return Err("it is listed as an instance but stored as something else".into());
            }
            object.kind = ObjectKind::Instance(ObjectId::from_raw(source));
            Ok(())
        })?;
        if let Some(id) = instances.keys().next() {
            return Err(format!(
                "object {id} is listed as an instance, but isn't there"
            ));
        }
        Ok(document)
    }
}
