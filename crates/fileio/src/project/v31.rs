//! Version 31 of the project file's contents ([decision 0100](../../../../docs/decisions/0100-texture-size-and-texel-density.md)):
//! version 30's, with the project's texture settings and texel density
//! set on pieces.
//!
//! Beside version 30's file: the texture's size and the texel density
//! aimed for. One more kind of recorded edit: pieces scaled to a texel
//! density. Older files open with the standard texture settings.

use super::v1::{FileOperation, Problem};
use super::v2::{AnchorsV2, anchors_from_file, anchors_to_file};
use super::v5::EntryWith;
use super::v27::ProjectV27;
use super::v30::OperationV30;
use crate::limits::Bounded;
use scene::{Document, FaceId, Operation, TextureSettings};
use serde::{Deserialize, Serialize};

/// A recorded edit in version 31.
pub(crate) type EntryV31 = EntryWith<OperationV31>;

/// Version 31 of the file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProjectV31<E> {
    /// Version 27's file, its edits stored as `E`.
    pub(crate) project: ProjectV27<E>,
    /// The texture settings.
    pub(crate) texture: TextureV31,
}

/// The project's texture: its size in pixels and the texel density aimed
/// for, in pixels per metre.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct TextureV31 {
    pub(crate) size: u32,
    pub(crate) density: f32,
}

impl<E: FileOperation> ProjectV31<E> {
    /// The file model of a document.
    pub(crate) fn from_document(document: &Document) -> Self {
        let t = document.scene().texture();
        Self {
            project: ProjectV27::from_document(document),
            texture: TextureV31 {
                size: t.size,
                density: t.density,
            },
        }
    }

    /// The document this file model describes, after checking every value.
    pub(crate) fn into_document(self) -> Result<Document, Problem> {
        let texture = TextureSettings {
            size: self.texture.size,
            density: self.texture.density,
        };
        if !texture.is_valid() {
            return Err("its texture size or texel density is out of range".into());
        }
        Ok(self.project.into_document()?.with_texture(texture))
    }
}

/// The recorded operations version 31 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV31 {
    /// Everything version 30 could store, stored the same way.
    V30(OperationV30),
    /// Pieces scaled to a texel density.
    SetTexelDensity {
        faces: Bounded<u32>,
        density: f32,
        anchors: Option<AnchorsV2>,
    },
}

impl FileOperation for OperationV31 {
    fn to_file(op: &Operation) -> Option<Self> {
        if let Operation::SetTexelDensity {
            faces,
            density,
            anchors,
        } = op
        {
            return Some(Self::SetTexelDensity {
                faces: faces.iter().map(|f| f.get()).collect(),
                density: *density,
                anchors: anchors.as_ref().map(anchors_to_file),
            });
        }
        Some(Self::V30(OperationV30::to_file(op)?))
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        match self {
            Self::V30(op) => op.into_operation(),
            Self::SetTexelDensity {
                faces,
                density,
                anchors,
            } => {
                let operation = Operation::SetTexelDensity {
                    faces: faces.into_iter().map(FaceId::from_raw).collect(),
                    density,
                    anchors: anchors.map(anchors_from_file).transpose()?,
                };
                if !operation.numbers_are_real() {
                    return Err("a texel density isn't a real number above 0".into());
                }
                Ok(operation)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_texel_density_survives_the_trip_and_nonsense_is_refused() {
        let op = Operation::SetTexelDensity {
            faces: vec![FaceId::from_raw(2)],
            density: 0.0025,
            anchors: None,
        };
        let file = OperationV31::to_file(&op).unwrap();
        assert!(matches!(file, OperationV31::SetTexelDensity { .. }));
        assert_eq!(file.into_operation().unwrap(), op);
        assert!(OperationV30::to_file(&op).is_none(), "older versions can't");
        for density in [0.0, -1.0, f32::NAN] {
            let bad = OperationV31::SetTexelDensity {
                faces: Bounded::default(),
                density,
                anchors: None,
            };
            assert!(bad.into_operation().is_err());
        }
    }

    #[test]
    fn texture_settings_come_back_and_bad_ones_are_refused() {
        let doc = Document::new().with_texture(TextureSettings {
            size: 4096,
            density: 1024.0,
        });
        let file = ProjectV31::<EntryV31>::from_document(&doc);
        let back = file.clone().into_document().unwrap();
        assert_eq!(back.scene().texture(), doc.scene().texture());
        let mut bad = file;
        bad.texture.size = 3;
        assert!(bad.into_document().is_err());
    }
}
