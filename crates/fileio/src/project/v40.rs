//! Version 40 of the project file's contents: version 39's development
//! model with each object's live booleans
//! ([decision 0135](../../../../docs/decisions/0135-live-booleans.md)).
//!
//! Booleans get a version of their own, against the development format's
//! habit of adding settings with a standard value, because an older app
//! would skip them without a word: it would show the shapes uncut and
//! the cutters as solid shapes, and saving would lose the booleans. With
//! a new version, the older app says it needs updating instead. The
//! encoding is still CBOR, so settings can again be added here with a
//! standard value until the alpha.

use super::v1::Problem;
use super::v39::ProjectV39;
use crate::limits::Bounded;
use scene::{Boolean, BooleanKind, Document, ObjectId};
use serde::{Deserialize, Serialize};
use words::{tr, trf};

/// Version 40 of the file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProjectV40 {
    /// Version 39's model, as it was.
    pub(crate) project: ProjectV39,
    /// Each object's booleans (objects not listed have none).
    #[serde(default = "Bounded::new")]
    pub(crate) booleans: Bounded<BooleansV40>,
}

/// One object's booleans, in order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct BooleansV40 {
    /// The object cut.
    pub(crate) id: u64,
    /// Its booleans.
    pub(crate) list: Bounded<BooleanV40>,
}

/// One boolean: what it does, and the objects that do it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct BooleanV40 {
    /// What it does.
    pub(crate) kind: KindV40,
    /// The cutters, by object id.
    pub(crate) cutters: Bounded<u64>,
}

/// What a boolean does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum KindV40 {
    Union,
    Subtract,
    Intersect,
}

impl KindV40 {
    fn of(kind: BooleanKind) -> Self {
        match kind {
            BooleanKind::Union => Self::Union,
            BooleanKind::Subtract => Self::Subtract,
            BooleanKind::Intersect => Self::Intersect,
        }
    }

    fn kind(self) -> BooleanKind {
        match self {
            Self::Union => BooleanKind::Union,
            Self::Subtract => BooleanKind::Subtract,
            Self::Intersect => BooleanKind::Intersect,
        }
    }
}

impl ProjectV40 {
    /// The file model of a document.
    pub(crate) fn from_document(document: &Document) -> Self {
        let scene = document.scene();
        let booleans = scene
            .walk()
            .iter()
            .filter(|(id, _)| !scene.booleans(*id).is_empty())
            .map(|(id, _)| BooleansV40 {
                id: id.get(),
                list: scene
                    .booleans(*id)
                    .iter()
                    .map(|b| BooleanV40 {
                        kind: KindV40::of(b.kind),
                        cutters: b.cutters.iter().map(|c| c.get()).collect(),
                    })
                    .collect(),
            })
            .collect();
        Self {
            project: ProjectV39::from_document(document),
            booleans,
        }
    }

    /// The document this file model describes, after checking every value.
    pub(crate) fn into_document(self) -> Result<Document, Problem> {
        let mut document = self.project.into_document()?;
        let mut seen = std::collections::BTreeSet::new();
        for entry in self.booleans {
            if !seen.insert(entry.id) {
                return Err(tr("an object's booleans are listed twice").into());
            }
            let id = ObjectId::from_raw(entry.id);
            if !document.scene().contains(id) {
                return Err(tr("its booleans name an object that isn't there").into());
            }
            let list: Vec<Boolean> = entry
                .list
                .0
                .into_iter()
                .map(|b| Boolean {
                    kind: b.kind.kind(),
                    cutters: b.cutters.0.into_iter().map(ObjectId::from_raw).collect(),
                })
                .collect();
            document = document
                .with_booleans(id, list)
                .map_err(|why| Problem::from(trf!("a boolean can't be used: {}", why)))?;
        }
        Ok(document)
    }
}

/// The CBOR bytes of `model` (version 41 is written now; tests still
/// write version 40 files).
#[cfg(test)]
pub(crate) fn encode(model: &ProjectV40) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    ciborium::into_writer(model, &mut bytes).map_err(|e| e.to_string())?;
    Ok(bytes)
}

/// The model in CBOR `bytes`; anything after it is refused.
pub(crate) fn decode(bytes: &[u8]) -> Result<ProjectV40, String> {
    let mut reader = std::io::Cursor::new(bytes);
    let model = ciborium::from_reader(&mut reader).map_err(|e| e.to_string())?;
    if reader.position() as usize != bytes.len() {
        return Err(tr("there is unexpected data after the scene").into());
    }
    Ok(model)
}
