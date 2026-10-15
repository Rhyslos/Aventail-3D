//! Version 19 of the project file's contents ([decision 0076](../../../../docs/decisions/0076-engine-export-paths.md)):
//! version 18's, with the project's game project folder.
//!
//! Beside version 18's file: the folder the project exports into and
//! whether every save exports there, if one was chosen. Older files open
//! with none.

use super::v1::{FileOperation, Problem};
use super::v18::{EntryV18, ProjectV18};
use scene::{Document, ExportTarget};
use serde::{Deserialize, Serialize};

/// Version 19 of the file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProjectV19<E = EntryV18> {
    /// Version 18's file (its edits stored as `E`).
    pub(crate) project: ProjectV18<E>,
    /// The game project folder, if one was chosen.
    pub(crate) target: Option<TargetV19>,
}

/// A game project folder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct TargetV19 {
    pub(crate) folder: String,
    pub(crate) on_save: bool,
}

impl<E: FileOperation> ProjectV19<E> {
    /// The file model of a document.
    pub(crate) fn from_document(document: &Document) -> Self {
        Self {
            project: ProjectV18::from_document(document),
            target: document.scene().export_target().map(|t| TargetV19 {
                folder: t.folder.clone(),
                on_save: t.on_save,
            }),
        }
    }

    /// The document this file model describes, after checking every value.
    pub(crate) fn into_document(self) -> Result<Document, Problem> {
        self.into_document_with(|_, _| Ok(()))
    }

    /// The same, giving each object to `attach` (with its id in the file)
    /// as it is read, for later versions to add what they store.
    pub(crate) fn into_document_with(
        self,
        attach: impl FnMut(u64, &mut scene::Object) -> Result<(), Problem>,
    ) -> Result<Document, Problem> {
        let target = self.target.map(|t| ExportTarget {
            folder: t.folder,
            on_save: t.on_save,
        });
        if target.as_ref().is_some_and(|t| !t.is_valid()) {
            return Err("its game project folder is empty, too long or broken over lines".into());
        }
        Ok(self
            .project
            .into_document_with(attach)?
            .with_export_target(target))
    }
}
