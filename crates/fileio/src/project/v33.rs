//! Version 33 of the project file's contents ([decision 0106](../../../../docs/decisions/0106-trim-sheets.md)):
//! version 32's, with the project's trim sheet and pieces laid on its
//! rows.
//!
//! Beside version 32's file: the trim sheet's rows (a name, and where each
//! starts and ends as shares of the texture's height). One more kind of
//! recorded edit: pieces laid onto a row. Older files open with no trim
//! sheet.

use super::v1::{FileOperation, Problem};
use super::v2::{AnchorsV2, anchors_from_file, anchors_to_file};
use super::v5::EntryWith;
use super::v31::ProjectV31;
use super::v32::OperationV32;
use crate::limits::Bounded;
use scene::{Document, FaceId, Operation, TrimRow, TrimSheet};
use serde::{Deserialize, Serialize};

/// A recorded edit in version 33.
pub(crate) type EntryV33 = EntryWith<OperationV33>;

/// Version 33 of the file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProjectV33<E> {
    /// Version 31's file (as version 32 stores it), its edits stored as
    /// `E`.
    pub(crate) project: ProjectV31<E>,
    /// The trim sheet's rows.
    pub(crate) trim: Bounded<RowV33>,
}

/// One row of the trim sheet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct RowV33 {
    pub(crate) name: String,
    pub(crate) bottom: f32,
    pub(crate) top: f32,
}

impl<E: FileOperation> ProjectV33<E> {
    /// The file model of a document.
    pub(crate) fn from_document(document: &Document) -> Self {
        Self {
            project: ProjectV31::from_document(document),
            trim: document
                .scene()
                .trim()
                .rows
                .iter()
                .map(|r| RowV33 {
                    name: r.name.clone(),
                    bottom: r.bottom,
                    top: r.top,
                })
                .collect(),
        }
    }

    /// The document this file model describes, after checking every value.
    pub(crate) fn into_document(self) -> Result<Document, Problem> {
        let trim = TrimSheet {
            rows: self
                .trim
                .iter()
                .map(|r| TrimRow {
                    name: r.name.clone(),
                    bottom: r.bottom,
                    top: r.top,
                })
                .collect(),
        };
        if !trim.is_valid() {
            return Err(
                "its trim sheet has rows that are unnamed, overlap or lie outside the texture"
                    .into(),
            );
        }
        Ok(self.project.into_document()?.with_trim(trim))
    }
}

/// The recorded operations version 33 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV33 {
    /// Everything version 32 could store, stored the same way.
    V32(OperationV32),
    /// Pieces laid onto a trim row.
    PlaceOnTrim {
        faces: Bounded<u32>,
        bottom: f32,
        top: f32,
        anchors: Option<AnchorsV2>,
    },
}

impl FileOperation for OperationV33 {
    fn to_file(op: &Operation) -> Option<Self> {
        if let Operation::PlaceOnTrim {
            faces,
            bottom,
            top,
            anchors,
        } = op
        {
            return Some(Self::PlaceOnTrim {
                faces: faces.iter().map(|f| f.get()).collect(),
                bottom: *bottom,
                top: *top,
                anchors: anchors.as_ref().map(anchors_to_file),
            });
        }
        Some(Self::V32(OperationV32::to_file(op)?))
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        match self {
            Self::V32(op) => op.into_operation(),
            Self::PlaceOnTrim {
                faces,
                bottom,
                top,
                anchors,
            } => {
                let operation = Operation::PlaceOnTrim {
                    faces: faces.into_iter().map(FaceId::from_raw).collect(),
                    bottom,
                    top,
                    anchors: anchors.map(anchors_from_file).transpose()?,
                };
                if !operation.numbers_are_real() {
                    return Err("a trim row doesn't lie within the texture".into());
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
    fn a_piece_on_a_trim_row_survives_the_trip_and_nonsense_is_refused() {
        let op = Operation::PlaceOnTrim {
            faces: vec![FaceId::from_raw(1), FaceId::from_raw(4)],
            bottom: 0.25,
            top: 0.5,
            anchors: None,
        };
        let file = OperationV33::to_file(&op).unwrap();
        assert!(matches!(file, OperationV33::PlaceOnTrim { .. }));
        assert_eq!(file.into_operation().unwrap(), op);
        assert!(OperationV32::to_file(&op).is_none(), "older versions can't");
        for (bottom, top) in [(0.5, 0.25), (-0.1, 0.5), (0.0, 1.5), (f32::NAN, 0.5)] {
            let bad = OperationV33::PlaceOnTrim {
                faces: Bounded::default(),
                bottom,
                top,
                anchors: None,
            };
            assert!(bad.into_operation().is_err(), "{bottom} {top}");
        }
    }

    #[test]
    fn the_trim_sheet_comes_back_and_bad_ones_are_refused() {
        let doc = Document::new().with_trim(TrimSheet::even(3));
        let file = ProjectV33::<EntryV33>::from_document(&doc);
        let back = file.clone().into_document().unwrap();
        assert_eq!(back.scene().trim(), doc.scene().trim());
        let mut bad = file;
        bad.trim[1].bottom = 0.9;
        assert!(bad.into_document().is_err());
    }
}
