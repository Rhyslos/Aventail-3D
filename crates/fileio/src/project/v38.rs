//! Version 38 of the project file's contents ([decision 0114](../../../../docs/decisions/0114-layout-snapshots.md)):
//! version 37's, with layout snapshots.
//!
//! Beside version 35's model (as version 37 stores it): the project's
//! layout snapshots, each a name and, for each object it was taken of,
//! every polygon's corner UVs and the seams. One more kind of recorded
//! edit: a layout put back. Older files open with no snapshots.

use super::v1::{FileOperation, Problem};
use super::v5::EntryWith;
use super::v35::ProjectV35;
use super::v37::OperationV37;
use crate::limits::Bounded;
use glam::Vec2;
use scene::{
    Document, EdgeKey, FaceId, ObjectId, Operation, UvLayout, UvSnapshot, VertexId,
    snapshot::snapshots_are_valid,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A recorded edit in version 38.
pub(crate) type EntryV38 = EntryWith<OperationV38>;

/// Version 38 of the file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProjectV38<E> {
    /// Version 35's file (as version 37 stores it), its edits stored as
    /// `E`.
    pub(crate) project: ProjectV35<E>,
    /// The layout snapshots, oldest first.
    pub(crate) snapshots: Bounded<SnapshotV38>,
}

/// A layout snapshot, as stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct SnapshotV38 {
    pub(crate) name: String,
    pub(crate) objects: Bounded<SnapshotObjectV38>,
}

/// One object's layout in a snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct SnapshotObjectV38 {
    pub(crate) id: u64,
    pub(crate) layout: LayoutV38,
}

/// A UV layout, as stored: each polygon's corner UVs, and the seams.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct LayoutV38 {
    pub(crate) faces: Bounded<FaceUvsV38>,
    pub(crate) seams: Bounded<[u32; 2]>,
}

/// One polygon's corner UVs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct FaceUvsV38 {
    pub(crate) face: u32,
    pub(crate) uvs: Bounded<[f32; 2]>,
}

fn layout_to_file(layout: &UvLayout) -> LayoutV38 {
    LayoutV38 {
        faces: layout
            .faces
            .iter()
            .map(|(f, uvs)| FaceUvsV38 {
                face: f.get(),
                uvs: uvs.iter().map(|uv| uv.to_array()).collect(),
            })
            .collect(),
        seams: layout
            .seams
            .iter()
            .map(|e| {
                let (a, b) = e.ends();
                [a.get(), b.get()]
            })
            .collect(),
    }
}

fn layout_from_file(layout: LayoutV38) -> Result<UvLayout, Problem> {
    let layout = UvLayout {
        faces: layout
            .faces
            .into_iter()
            .map(|f| {
                (
                    FaceId::from_raw(f.face),
                    f.uvs.into_iter().map(Vec2::from_array).collect(),
                )
            })
            .collect(),
        seams: layout
            .seams
            .into_iter()
            .map(|[a, b]| EdgeKey::new(VertexId::from_raw(a), VertexId::from_raw(b)))
            .collect(),
    };
    if !layout.is_real() {
        return Err("a saved UV layout holds a UV that isn't a real number".into());
    }
    Ok(layout)
}

impl<E: FileOperation> ProjectV38<E> {
    /// The file model of a document.
    pub(crate) fn from_document(document: &Document) -> Self {
        Self {
            project: ProjectV35::from_document(document),
            snapshots: document
                .scene()
                .snapshots()
                .iter()
                .map(|s| SnapshotV38 {
                    name: s.name.clone(),
                    objects: s
                        .objects
                        .iter()
                        .map(|(id, layout)| SnapshotObjectV38 {
                            id: id.get(),
                            layout: layout_to_file(layout),
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    /// The document this file model describes, after checking every value.
    pub(crate) fn into_document(self) -> Result<Document, Problem> {
        let mut snapshots = Vec::new();
        for s in self.snapshots {
            let mut objects = Vec::new();
            for o in s.objects {
                objects.push((
                    ObjectId::from_raw(o.id),
                    Arc::new(layout_from_file(o.layout)?),
                ));
            }
            snapshots.push(UvSnapshot {
                name: s.name,
                objects,
            });
        }
        if !snapshots_are_valid(&snapshots) {
            return Err("its layout snapshots are too many, unnamed or share a name".into());
        }
        Ok(self.project.into_document()?.with_snapshots(snapshots))
    }
}

/// The recorded operations version 38 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV38 {
    /// Everything version 37 could store, stored the same way.
    V37(OperationV37),
    /// A saved UV layout put back.
    RestoreUvs { layout: LayoutV38 },
}

impl FileOperation for OperationV38 {
    fn to_file(op: &Operation) -> Option<Self> {
        match op {
            Operation::RestoreUvs { layout } => Some(Self::RestoreUvs {
                layout: layout_to_file(layout),
            }),
            _ => Some(Self::V37(OperationV37::to_file(op)?)),
        }
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        match self {
            Self::V37(op) => op.into_operation(),
            Self::RestoreUvs { layout } => Ok(Operation::RestoreUvs {
                layout: Arc::new(layout_from_file(layout)?),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> UvLayout {
        UvLayout {
            faces: vec![
                (FaceId::from_raw(0), vec![Vec2::ZERO, Vec2::X, Vec2::ONE]),
                (
                    FaceId::from_raw(3),
                    vec![Vec2::splat(0.25), Vec2::Y, Vec2::X, Vec2::ONE, Vec2::ZERO],
                ),
            ],
            seams: vec![EdgeKey::new(VertexId::from_raw(1), VertexId::from_raw(2))],
        }
    }

    #[test]
    fn a_restore_survives_the_trip() {
        let op = Operation::RestoreUvs {
            layout: Arc::new(layout()),
        };
        let file = OperationV38::to_file(&op).unwrap();
        assert!(matches!(file, OperationV38::RestoreUvs { .. }));
        assert_eq!(file.into_operation().unwrap(), op);
        assert!(OperationV37::to_file(&op).is_none(), "older versions can't");
        let empty = Operation::RestoreUvs {
            layout: Arc::new(UvLayout::default()),
        };
        let file = OperationV38::to_file(&empty).unwrap();
        assert_eq!(file.into_operation().unwrap(), empty);
    }

    #[test]
    fn a_uv_that_isnt_real_is_refused() {
        for bad in [f32::NAN, f32::INFINITY] {
            let mut layout = layout_to_file(&layout());
            layout.faces[1].uvs[2] = [0.0, bad];
            let op = OperationV38::RestoreUvs { layout };
            assert!(op.into_operation().is_err(), "{bad}");
        }
    }
}
