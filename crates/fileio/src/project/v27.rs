//! Version 27 of the project file's contents ([decision 0090](../../../../docs/decisions/0090-stable-element-names.md)):
//! version 26's, with the stable names of the points each edit was made
//! on.
//!
//! Beside version 26's file: for each recorded edit whose anchors carry
//! stable names, the object, the edit's id and the names (one per anchored
//! point, in order). An edit finding its points again by name then works
//! the same after saving and opening. Older files open with no names, and
//! their edits find their points by place, as they always did.

use super::v1::{FileOperation, Problem};
use super::v25::ProjectV25;
use super::v26::EntryV26;
use crate::limits::Bounded;
use scene::{Document, Stack};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use words::{tr, trf};

/// Version 27 of the file (and, with their own edits `E`, the versions
/// after it that change only the kinds of edit).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProjectV27<E = EntryV26> {
    /// Version 26's file (or a later one's edits).
    pub(crate) project: ProjectV25<E>,
    /// The edits' anchored points' names (only edits with them).
    pub(crate) names: Bounded<NamesV27>,
}

/// One edit's anchored points' stable names.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct NamesV27 {
    pub(crate) object: u64,
    pub(crate) edit: u64,
    pub(crate) names: Bounded<u64>,
}

impl<E: FileOperation> ProjectV27<E> {
    /// The file model of a document.
    pub(crate) fn from_document(document: &Document) -> Self {
        let scene = document.scene();
        let mut names = Vec::new();
        for (id, _) in scene.walk() {
            let Some(object) = scene.object(id) else {
                continue;
            };
            for layer in object.stack.layers() {
                for entry in &layer.entries {
                    if let Some(list) = entry.operation.anchors().and_then(|a| a.names.clone()) {
                        names.push(NamesV27 {
                            object: id.get(),
                            edit: entry.id.get(),
                            names: list.into_iter().collect(),
                        });
                    }
                }
            }
        }
        Self {
            project: ProjectV25::from_document(document),
            names: names.into_iter().collect(),
        }
    }

    /// The document this file model describes, after checking every value.
    pub(crate) fn into_document(self) -> Result<Document, Problem> {
        let mut by_object: BTreeMap<u64, BTreeMap<u64, Vec<u64>>> = BTreeMap::new();
        for entry in self.names {
            let edits = by_object.entry(entry.object).or_default();
            if edits
                .insert(entry.edit, entry.names.into_iter().collect())
                .is_some()
            {
                return Err(tr("an edit's names are stored twice").into());
            }
        }
        let document = self.project.into_document_with(|id, object| {
            let Some(mut edits) = by_object.remove(&id) else {
                return Ok(());
            };
            let mut layers = object.stack.layers().to_vec();
            for layer in &mut layers {
                for entry in &mut layer.entries {
                    let Some(names) = edits.remove(&entry.id.get()) else {
                        continue;
                    };
                    let Some(anchors) = entry.operation.anchors_mut() else {
                        return Err(tr("names are stored for an edit without anchors").into());
                    };
                    if names.len() != anchors.points.len() || names.contains(&0) {
                        return Err(tr("an edit's names don't fit its anchored points").into());
                    }
                    anchors.names = Some(names);
                }
            }
            if let Some(edit) = edits.keys().next() {
                return Err(trf!(
                    "names are stored for edit {}, which isn't there",
                    edit
                ));
            }
            object.stack = Stack::from_layers(layers);
            Ok(())
        })?;
        if let Some(id) = by_object.keys().next() {
            return Err(trf!(
                "names are stored for object {}, which isn't there",
                id
            ));
        }
        Ok(document)
    }
}
