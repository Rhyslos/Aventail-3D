//! Version 18 of the project file's contents ([decision 0074](../../../../docs/decisions/0074-material-slots.md)):
//! version 17's, with materials.
//!
//! Beside version 17's file: each object's list of materials (only
//! objects that have one), by the object's id. Among the recorded edits,
//! one more kind: giving polygons a material slot, on its own or
//! mirrored. Older files open with no materials, so every polygon uses
//! the standard one.

use super::v1::{FileOperation, Problem};
use super::v2::{AnchorsV2, anchors_from_file, anchors_to_file};
use super::v5::EntryWith;
use super::v16::ProjectWithPlaces;
use super::v17::OperationV17;
use crate::limits::Bounded;
use glam::Vec3;
use mesh::FaceId;
use mesh::material::MAX_SLOTS;
use mesh::symmetry::Mirror;
use scene::{Document, Material, Operation};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Version 18 of the file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProjectV18 {
    /// Version 17's file, its edits stored as version 18's.
    pub(crate) places: ProjectWithPlaces<EntryV18>,
    /// The objects' materials (only objects that have some).
    pub(crate) materials: Bounded<MaterialsV18>,
}

/// A recorded edit in version 18.
pub(crate) type EntryV18 = EntryWith<OperationV18>;

/// An object's materials.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct MaterialsV18 {
    pub(crate) object: u64,
    pub(crate) list: Bounded<MaterialV18>,
}

/// A material: its name and colour.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct MaterialV18 {
    pub(crate) name: String,
    pub(crate) color: [f32; 3],
}

/// The recorded operations version 18 stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV18 {
    /// Everything version 17 could store, stored the same way.
    V17(OperationV17),
    /// Polygons given a material slot.
    SetMaterial(SetMaterialV18),
    /// The same, made with symmetry on: the edit, the axes mirrored
    /// across, the point the planes go through and whether points are
    /// kept on their side.
    MirroredSetMaterial {
        op: SetMaterialV18,
        axes: [bool; 3],
        centre: [f32; 3],
        clip: bool,
    },
}

/// Polygons given a material slot, and its anchors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct SetMaterialV18 {
    faces: Bounded<u32>,
    slot: u32,
    anchors: Option<AnchorsV2>,
}

impl SetMaterialV18 {
    fn of(op: &Operation) -> Option<Self> {
        let Operation::SetMaterial {
            faces,
            slot,
            anchors,
        } = op
        else {
            return None;
        };
        Some(Self {
            faces: faces.iter().map(|f| f.get()).collect(),
            slot: *slot,
            anchors: anchors.as_ref().map(anchors_to_file),
        })
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        if self.slot >= MAX_SLOTS {
            return Err(format!(
                "polygons are given material {}, but an object has at most {MAX_SLOTS}",
                self.slot + 1
            ));
        }
        Ok(Operation::SetMaterial {
            faces: self.faces.into_iter().map(FaceId::from_raw).collect(),
            slot: self.slot,
            anchors: self.anchors.map(anchors_from_file).transpose()?,
        })
    }
}

impl FileOperation for OperationV18 {
    fn to_file(op: &Operation) -> Option<Self> {
        if let Some(set) = SetMaterialV18::of(op) {
            return Some(Self::SetMaterial(set));
        }
        if let Operation::Mirrored { op, mirror } = op
            && let Some(set) = SetMaterialV18::of(op)
        {
            return Some(Self::MirroredSetMaterial {
                op: set,
                axes: mirror.axes,
                centre: mirror.centre.to_array(),
                clip: mirror.clip,
            });
        }
        Some(Self::V17(OperationV17::to_file(op)?))
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        match self {
            Self::V17(op) => op.into_operation(),
            Self::SetMaterial(set) => set.into_operation(),
            Self::MirroredSetMaterial {
                op,
                axes,
                centre,
                clip,
            } => {
                let centre = Vec3::from_array(centre);
                if !centre.is_finite() {
                    return Err("a mirror's middle isn't a real number".to_string());
                }
                if !axes.contains(&true) {
                    return Err("a mirrored edit mirrors across no axis".to_string());
                }
                Ok(Operation::Mirrored {
                    op: Box::new(op.into_operation()?),
                    mirror: Mirror { axes, centre, clip },
                })
            }
        }
    }
}

impl ProjectV18 {
    /// The file model of a document.
    pub(crate) fn from_document(document: &Document) -> Self {
        let scene = document.scene();
        let materials = scene
            .walk()
            .into_iter()
            .filter_map(|(id, _)| {
                let object = scene.object(id)?;
                (!object.materials.is_empty()).then(|| MaterialsV18 {
                    object: id.get(),
                    list: object
                        .materials
                        .iter()
                        .map(|m| MaterialV18 {
                            name: m.name.clone(),
                            color: m.color,
                        })
                        .collect(),
                })
            })
            .collect();
        Self {
            places: ProjectWithPlaces::from_document(document),
            materials,
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
        mut attach: impl FnMut(u64, &mut scene::Object) -> Result<(), Problem>,
    ) -> Result<Document, Problem> {
        let mut lists: BTreeMap<u64, Vec<Material>> = BTreeMap::new();
        for entry in self.materials {
            if entry.list.len() > MAX_SLOTS as usize {
                return Err(format!(
                    "object {} has more than {MAX_SLOTS} materials",
                    entry.object
                ));
            }
            let list: Vec<Material> = entry
                .list
                .into_iter()
                .map(|m| Material {
                    name: m.name,
                    color: m.color,
                })
                .collect();
            if !list.iter().all(Material::is_valid) {
                return Err("a material has no name or a colour outside 0 to 1".into());
            }
            if lists.insert(entry.object, list).is_some() {
                return Err("an object's materials are stored twice".into());
            }
        }
        let document = self.places.into_document_with(|id, object| {
            attach(id, object)?;
            // (Instances are still stored as groups here; their lists
            // are kept for when they are made editable.)
            if let Some(list) = lists.remove(&id) {
                object.materials = list;
            }
            Ok(())
        })?;
        if let Some(id) = lists.keys().next() {
            return Err(format!(
                "materials are stored for object {id}, which isn't there"
            ));
        }
        Ok(document)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slot_beyond_the_last_is_refused() {
        let set = SetMaterialV18 {
            faces: vec![0].into_iter().collect(),
            slot: MAX_SLOTS,
            anchors: None,
        };
        assert!(set.into_operation().is_err());
        let fine = SetMaterialV18 {
            faces: vec![0].into_iter().collect(),
            slot: MAX_SLOTS - 1,
            anchors: None,
        };
        assert!(fine.into_operation().is_ok());
    }

    #[test]
    fn a_mirrored_material_change_needs_an_axis() {
        let op = OperationV18::MirroredSetMaterial {
            op: SetMaterialV18 {
                faces: Bounded::new(),
                slot: 0,
                anchors: None,
            },
            axes: [false; 3],
            centre: [0.0; 3],
            clip: false,
        };
        assert!(op.into_operation().is_err());
    }
}
