//! Version 25 of the project file's contents ([decision 0088](../../../../docs/decisions/0088-live-mirror.md)):
//! version 24's, with each object's live mirror.
//!
//! Beside version 24's file: the live mirror of each object that has one
//! (its axes, the point its planes go through and its weld distance).
//! Older files open with none.

use super::v1::{FileOperation, Problem};
use super::v20::ProjectV20;
use super::v24::EntryV24;
use crate::limits::Bounded;
use glam::Vec3;
use scene::{Document, LiveMirror};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Version 25 of the file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProjectV25<E = EntryV24> {
    /// Version 24's file (its edits stored as `E`).
    pub(crate) project: ProjectV20<E>,
    /// The objects' live mirrors (only objects with one).
    pub(crate) mirrors: Bounded<MirrorV25>,
}

/// An object's live mirror.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct MirrorV25 {
    pub(crate) object: u64,
    pub(crate) axes: [bool; 3],
    pub(crate) centre: [f32; 3],
    pub(crate) weld_distance: f32,
}

impl<E: FileOperation> ProjectV25<E> {
    /// The file model of a document.
    pub(crate) fn from_document(document: &Document) -> Self {
        let scene = document.scene();
        let mirrors = scene
            .walk()
            .into_iter()
            .filter_map(|(id, _)| {
                let mirror = scene.object(id)?.mirror?;
                Some(MirrorV25 {
                    object: id.get(),
                    axes: mirror.axes,
                    centre: mirror.centre.to_array(),
                    weld_distance: mirror.weld_distance,
                })
            })
            .collect();
        Self {
            project: ProjectV20::from_document(document),
            mirrors,
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
        let mut mirrors: BTreeMap<u64, LiveMirror> = BTreeMap::new();
        for entry in self.mirrors {
            let mirror = LiveMirror {
                axes: entry.axes,
                centre: Vec3::from_array(entry.centre),
                weld_distance: entry.weld_distance,
                // Version 25's mirrors are in the world; version 39 says
                // which belong to their object.
                on_object: false,
            };
            if !mirror.is_valid() {
                return Err(
                    "a mirror has no axis, or a number that isn't real or is out of range".into(),
                );
            }
            if mirrors.insert(entry.object, mirror).is_some() {
                return Err("an object's mirror is stored twice".into());
            }
        }
        let document = self.project.into_document_with(|id, object| {
            if let Some(mirror) = mirrors.remove(&id) {
                if !object.kind.has_shape() {
                    return Err("a mirror is stored for an object without a shape".into());
                }
                object.mirror = Some(mirror);
            }
            attach(id, object)
        })?;
        if let Some(id) = mirrors.keys().next() {
            return Err(format!(
                "a mirror is stored for object {id}, which isn't there"
            ));
        }
        Ok(document)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scene::{Command, ObjectKind, PrimitiveKind};

    fn mirrored() -> Document {
        let mut doc = Document::new();
        doc.execute(Command::Create(ObjectKind::Primitive(
            PrimitiveKind::Cube.standard(),
        )))
        .unwrap();
        let id = doc.selection()[0];
        doc.execute(Command::SetMirror {
            ids: vec![id],
            mirror: Some(LiveMirror {
                axes: [true, false, true],
                centre: Vec3::new(10.0, -5.0, 0.0),
                weld_distance: 0.5,
            }),
        })
        .unwrap();
        doc
    }

    #[test]
    fn a_mirror_survives_the_trip() {
        let doc = mirrored();
        let back = ProjectV25::<EntryV24>::from_document(&doc)
            .into_document()
            .unwrap();
        let id = back.scene().walk()[0].0;
        assert_eq!(
            back.scene().object(id).unwrap().mirror,
            doc.scene().object(doc.scene().walk()[0].0).unwrap().mirror
        );
    }

    #[test]
    fn broken_mirrors_are_refused() {
        let good = ProjectV25::<EntryV24>::from_document(&mirrored());
        let entry = good.mirrors.iter().next().copied().unwrap();
        for broken in [
            MirrorV25 {
                axes: [false; 3],
                ..entry
            },
            MirrorV25 {
                centre: [f32::NAN, 0.0, 0.0],
                ..entry
            },
            MirrorV25 {
                weld_distance: -1.0,
                ..entry
            },
            MirrorV25 {
                object: 999,
                ..entry
            },
        ] {
            let model = ProjectV25 {
                mirrors: vec![broken].into_iter().collect(),
                ..good.clone()
            };
            assert!(model.into_document().is_err(), "{broken:?}");
        }
        let twice = ProjectV25 {
            mirrors: vec![entry, entry].into_iter().collect(),
            ..good
        };
        assert!(twice.into_document().is_err());
    }
}
