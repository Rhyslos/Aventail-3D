//! Version 39 of the project file's contents: the development format
//! ([decision 0126](../../../../docs/decisions/0126-rework-and-first-use.md)).
//!
//! Until the alpha the file format doesn't get a new version for every
//! feature. Version 39 holds version 38's model, encoded with CBOR (a
//! self-describing encoding) instead of postcard, so fields can be added
//! here with a standard value (`#[serde(default)]`) and older development
//! files still open. Proper versioning starts again at the alpha.
//!
//! Beside version 38's model, each object's development settings:
//! its smoothing (by angle, or by marked edges only) and whether its live
//! mirror belongs to the object (moves and turns with it) or to the world.

use super::v1::Problem;
use super::v38::{EntryV38, ProjectV38};
use crate::limits::Bounded;
use scene::{Document, Hardness, ObjectId};
use serde::{Deserialize, Serialize};

/// Version 39 of the file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProjectV39 {
    /// Version 38's model, as it was.
    pub(crate) project: ProjectV38<EntryV38>,
    /// Each object's development settings (objects not listed keep the
    /// standard ones).
    #[serde(default)]
    pub(crate) objects: Bounded<ObjectV39>,
}

/// One object's development settings. New settings are added here, each
/// with a standard value, so files without them still open.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct ObjectV39 {
    /// The object.
    pub(crate) id: u64,
    /// Smoothed by this angle (with marked hard edges added), or only by
    /// marked hard edges (`None`).
    pub(crate) smoothing: Option<f32>,
    /// Whether its live mirror belongs to the object.
    pub(crate) mirror_on_object: bool,
}

impl Default for ObjectV39 {
    fn default() -> Self {
        Self {
            id: 0,
            smoothing: Some(scene::STANDARD_SMOOTHING_ANGLE),
            mirror_on_object: false,
        }
    }
}

impl ProjectV39 {
    /// The file model of a document.
    pub(crate) fn from_document(document: &Document) -> Self {
        let scene = document.scene();
        let objects = scene
            .walk()
            .iter()
            .filter_map(|(id, _)| {
                let object = scene.object(*id)?;
                Some(ObjectV39 {
                    id: id.get(),
                    smoothing: match object.hardness {
                        Hardness::ByAngle(angle) => Some(angle),
                        Hardness::Marked => None,
                    },
                    mirror_on_object: object.mirror.is_some_and(|m| m.on_object),
                })
            })
            .collect();
        Self {
            project: ProjectV38::from_document(document),
            objects,
        }
    }

    /// The document this file model describes, after checking every value.
    pub(crate) fn into_document(self) -> Result<Document, Problem> {
        let mut document = self.project.into_document()?;
        let mut seen = std::collections::BTreeSet::new();
        for object in self.objects {
            if !seen.insert(object.id) {
                return Err("an object's settings are listed twice".into());
            }
            let hardness = object.smoothing.map_or(Hardness::Marked, Hardness::ByAngle);
            document = document
                .with_object_settings(
                    ObjectId::from_raw(object.id),
                    hardness,
                    object.mirror_on_object,
                )
                .map_err(Problem::from)?;
        }
        Ok(document)
    }
}

/// The CBOR bytes of `model`.
pub(crate) fn encode(model: &ProjectV39) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    ciborium::into_writer(model, &mut bytes).map_err(|e| e.to_string())?;
    Ok(bytes)
}

/// The model in CBOR `bytes`; anything after it is refused.
pub(crate) fn decode(bytes: &[u8]) -> Result<ProjectV39, String> {
    let mut reader = std::io::Cursor::new(bytes);
    let model = ciborium::from_reader(&mut reader).map_err(|e| e.to_string())?;
    if reader.position() as usize != bytes.len() {
        return Err("there is unexpected data after the scene".into());
    }
    Ok(model)
}
