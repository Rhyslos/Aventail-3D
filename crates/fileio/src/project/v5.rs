//! Version 5 of the project file's contents ([decision 0044](../../../../docs/decisions/0044-edits-that-hold.md)):
//! version 4's structures, with three additions.
//!
//! - **Recorded edits keep their id and switches.** Each is an
//!   [`EntryV5`]: its id, on/off and pinned, and version 4's operation
//!   stored as it was.
//! - **Anchors keep the structure** the edit was made on (its fingerprint),
//!   so the edit can trust its ids while that structure is unchanged. It
//!   sits beside the operation in the entry ([`AnchorsMoreV5`]), as
//!   version 4's operations are unchanged.
//! - **Plain meshes keep their attribute layers** (UVs, materials and the
//!   like, once tools make them), beside the objects ([`MeshLayersV5`]).
//!   Hard edges are stored with the mesh, as before.
//!
//! Every value read back is checked: a file may be damaged or hostile.

use super::v1::{FileOperation, Problem, ProjectFile};
use super::v4::OperationV4;
use crate::limits::Bounded;
use glam::{Vec2, Vec3, Vec4};
use mesh::{AttributeValues, Domain, EdgeKey, Mesh, Value, VertexId};
use scene::{Document, EditId, Entry, ObjectKind, Operation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Version 5 of the file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProjectV5 {
    /// Everything version 4 kept, with recorded edits as entries.
    pub(crate) project: ProjectFile<EntryV5>,
    /// The attribute layers of plain meshes, by object.
    pub(crate) layers: Bounded<MeshLayersV5>,
}

/// A recorded edit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct EntryV5 {
    pub(crate) id: u64,
    pub(crate) enabled: bool,
    pub(crate) pinned: bool,
    pub(crate) operation: OperationV4,
    /// What the operation's anchors keep beyond version 4.
    pub(crate) anchors: Option<AnchorsMoreV5>,
}

/// The parts of an operation's anchors new in version 5.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct AnchorsMoreV5 {
    pub(crate) structure: Option<u64>,
}

/// One plain mesh's attribute layers (besides hard edges).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct MeshLayersV5 {
    /// The object whose base the mesh is.
    pub(crate) object: u64,
    pub(crate) layers: Bounded<LayerV5>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct LayerV5 {
    pub(crate) name: String,
    pub(crate) domain: DomainV5,
    pub(crate) default: ValueV5,
    pub(crate) values: ValuesV5,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) enum DomainV5 {
    Point,
    Edge,
    Face,
    Corner,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) enum ValueV5 {
    Bool(bool),
    Int(i32),
    Float(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum ValuesV5 {
    Bool(Bounded<bool>),
    Int(Bounded<i32>),
    Float(Bounded<f32>),
    Vec2(Bounded<[f32; 2]>),
    Vec3(Bounded<[f32; 3]>),
    Vec4(Bounded<[f32; 4]>),
    Edges(Bounded<([u32; 2], ValueV5)>),
}

// ------------------------------------------------------------ entries ---

impl FileOperation for EntryV5 {
    fn to_file(op: &Operation) -> Option<Self> {
        Self::entry_to_file(&Entry::new(EditId::from_raw(1), op.clone()))
    }

    fn into_operation(self) -> Result<Operation, Problem> {
        Ok(self.into_entry(EditId::from_raw(1))?.operation)
    }

    fn entry_to_file(entry: &Entry) -> Option<Self> {
        let anchors = entry.operation.anchors().map(|a| AnchorsMoreV5 {
            structure: a.structure,
        });
        Some(Self {
            id: entry.id.get(),
            enabled: entry.enabled,
            pinned: entry.pinned,
            operation: OperationV4::to_file(&entry.operation)?,
            anchors,
        })
    }

    fn into_entry(self, _fallback: EditId) -> Result<Entry, Problem> {
        let mut operation = self.operation.into_operation()?;
        if let (Some(more), Some(anchors)) = (self.anchors, operation.anchors_mut()) {
            anchors.structure = more.structure;
        }
        Ok(Entry {
            id: EditId::from_raw(self.id),
            enabled: self.enabled,
            pinned: self.pinned,
            operation,
        })
    }
}

// -------------------------------------------------------------- layers ---

fn value_to_file(v: Value) -> ValueV5 {
    match v {
        Value::Bool(x) => ValueV5::Bool(x),
        Value::Int(x) => ValueV5::Int(x),
        Value::Float(x) => ValueV5::Float(x),
        Value::Vec2(x) => ValueV5::Vec2(x.to_array()),
        Value::Vec3(x) => ValueV5::Vec3(x.to_array()),
        Value::Vec4(x) => ValueV5::Vec4(x.to_array()),
    }
}

fn value_from_file(v: ValueV5) -> Value {
    match v {
        ValueV5::Bool(x) => Value::Bool(x),
        ValueV5::Int(x) => Value::Int(x),
        ValueV5::Float(x) => Value::Float(x),
        ValueV5::Vec2(x) => Value::Vec2(Vec2::from_array(x)),
        ValueV5::Vec3(x) => Value::Vec3(Vec3::from_array(x)),
        ValueV5::Vec4(x) => Value::Vec4(Vec4::from_array(x)),
    }
}

fn domain_to_file(d: Domain) -> DomainV5 {
    match d {
        Domain::Point => DomainV5::Point,
        Domain::Edge => DomainV5::Edge,
        Domain::Face => DomainV5::Face,
        Domain::Corner => DomainV5::Corner,
    }
}

fn domain_from_file(d: DomainV5) -> Domain {
    match d {
        DomainV5::Point => Domain::Point,
        DomainV5::Edge => Domain::Edge,
        DomainV5::Face => Domain::Face,
        DomainV5::Corner => Domain::Corner,
    }
}

fn values_to_file(v: AttributeValues) -> ValuesV5 {
    let arrays2 = |l: Vec<Vec2>| l.iter().map(Vec2::to_array).collect();
    let arrays3 = |l: Vec<Vec3>| l.iter().map(Vec3::to_array).collect();
    let arrays4 = |l: Vec<Vec4>| l.iter().map(Vec4::to_array).collect();
    match v {
        AttributeValues::Bool(l) => ValuesV5::Bool(l.into()),
        AttributeValues::Int(l) => ValuesV5::Int(l.into()),
        AttributeValues::Float(l) => ValuesV5::Float(l.into()),
        AttributeValues::Vec2(l) => ValuesV5::Vec2(arrays2(l)),
        AttributeValues::Vec3(l) => ValuesV5::Vec3(arrays3(l)),
        AttributeValues::Vec4(l) => ValuesV5::Vec4(arrays4(l)),
        AttributeValues::Edges(l) => ValuesV5::Edges(
            l.into_iter()
                .map(|(e, v)| {
                    let (a, b) = e.ends();
                    ([a.get(), b.get()], value_to_file(v))
                })
                .collect(),
        ),
    }
}

fn values_from_file(v: ValuesV5) -> AttributeValues {
    match v {
        ValuesV5::Bool(l) => AttributeValues::Bool(l.0),
        ValuesV5::Int(l) => AttributeValues::Int(l.0),
        ValuesV5::Float(l) => AttributeValues::Float(l.0),
        ValuesV5::Vec2(l) => AttributeValues::Vec2(l.into_iter().map(Vec2::from_array).collect()),
        ValuesV5::Vec3(l) => AttributeValues::Vec3(l.into_iter().map(Vec3::from_array).collect()),
        ValuesV5::Vec4(l) => AttributeValues::Vec4(l.into_iter().map(Vec4::from_array).collect()),
        ValuesV5::Edges(l) => AttributeValues::Edges(
            l.into_iter()
                .map(|([a, b], v)| {
                    (
                        EdgeKey::new(VertexId::from_raw(a), VertexId::from_raw(b)),
                        value_from_file(v),
                    )
                })
                .collect(),
        ),
    }
}

/// A mesh's attribute layers, other than hard edges (stored with it).
fn mesh_layers(mesh: &Mesh) -> Vec<LayerV5> {
    mesh.attributes()
        .filter(|(name, _, _)| *name != mesh::attributes::HARD)
        .filter_map(|(name, _, _)| {
            let (domain, default, values) = mesh.attribute_values(name)?;
            Some(LayerV5 {
                name: name.to_owned(),
                domain: domain_to_file(domain),
                default: value_to_file(default),
                values: values_to_file(values),
            })
        })
        .collect()
}

// ------------------------------------------------------- the whole file ---

impl ProjectV5 {
    /// The file model of a document.
    pub(crate) fn from_document(document: &Document) -> Self {
        let scene = document.scene();
        let layers = scene
            .walk()
            .into_iter()
            .filter_map(|(id, _)| match &scene.object(id)?.kind {
                ObjectKind::Mesh(mesh) => {
                    let layers = mesh_layers(mesh);
                    (!layers.is_empty()).then_some(MeshLayersV5 {
                        object: id.get(),
                        layers: layers.into(),
                    })
                }
                _ => None,
            })
            .collect();
        Self {
            project: ProjectFile::from_document(document),
            layers,
        }
    }

    /// The document this file model describes, after checking every value.
    pub(crate) fn into_document(self) -> Result<Document, Problem> {
        let mut layers: std::collections::BTreeMap<u64, Vec<LayerV5>> =
            std::collections::BTreeMap::new();
        for mesh in self.layers {
            if layers.insert(mesh.object, mesh.layers.0).is_some() {
                return Err("an object's mesh layers are listed twice".into());
            }
        }
        let document = self.project.into_document_with(|id, object| {
            let Some(list) = layers.remove(&id) else {
                return Ok(());
            };
            let ObjectKind::Mesh(mesh) = &mut object.kind else {
                return Err("it has mesh layers but no mesh".into());
            };
            let mesh = Arc::make_mut(mesh);
            for layer in list {
                let name = layer.name.clone();
                mesh.set_attribute_values(
                    &layer.name,
                    domain_from_file(layer.domain),
                    value_from_file(layer.default),
                    values_from_file(layer.values),
                )
                .map_err(|e| format!("its mesh layer \"{name}\" is damaged ({e})"))?;
            }
            Ok(())
        })?;
        if let Some(id) = layers.keys().next() {
            return Err(format!(
                "mesh layers are stored for object {id}, which isn't there"
            ));
        }
        Ok(document)
    }
}
