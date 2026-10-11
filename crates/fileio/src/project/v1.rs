//! Version 1 of the project file's contents: plain data that mirrors the
//! scene, kept separate from the app's own types on purpose. The app's
//! types can change freely; this model only changes with a new format
//! version (and an upgrade step from this one), so old files always open.
//!
//! Every value read back is checked: a file may be damaged or hostile.

use glam::Vec3;
use mesh::{
    ConeParams, CubeParams, CylinderParams, DiscParams, EdgeKey, Mesh, PlaneParams, Primitive,
    SphereParams, TubeParams, VertexId,
};
use scene::{Document, Layer, Object, ObjectId, ObjectKind, Operation, Scene, Stack, Transform};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A whole project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProjectV1 {
    /// Every object, parents before their children, children in order.
    pub(crate) objects: Vec<ObjectV1>,
    /// The id the next new object gets (ids are never reused).
    pub(crate) next_id: u64,
    /// The selected objects' ids.
    pub(crate) selection: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ObjectV1 {
    pub(crate) id: u64,
    pub(crate) parent: Option<u64>,
    pub(crate) name: String,
    pub(crate) visible: bool,
    pub(crate) transform: TransformV1,
    pub(crate) kind: KindV1,
    pub(crate) layers: Vec<LayerV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct TransformV1 {
    pub(crate) position: [f32; 3],
    pub(crate) rotation: [f32; 3],
    pub(crate) scale: [f32; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum KindV1 {
    Group,
    Primitive(PrimitiveV1),
    Mesh(MeshV1),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) enum PrimitiveV1 {
    Cube {
        size: [f32; 3],
        segments: [u32; 3],
    },
    Plane {
        width: f32,
        depth: f32,
        segments: [u32; 2],
    },
    Cylinder {
        radius: f32,
        height: f32,
        sides: u32,
        height_segments: u32,
        caps: bool,
        cap_segments: u32,
        cap_centre: bool,
    },
    Sphere {
        radius: f32,
        sides: u32,
        rings: u32,
    },
    Disc {
        radius: f32,
        inner_radius: f32,
        sides: u32,
        ring_segments: u32,
        centre_point: bool,
    },
    Tube {
        outer_radius: f32,
        inner_radius: f32,
        height: f32,
        sides: u32,
        height_segments: u32,
    },
    Cone {
        bottom_radius: f32,
        top_radius: f32,
        height: f32,
        sides: u32,
        height_segments: u32,
        caps: bool,
        cap_segments: u32,
        cap_centre: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct MeshV1 {
    /// Point positions by id number; `None` where a point was deleted.
    pub(crate) points: Vec<Option<[f32; 3]>>,
    /// Faces by id number, as point id numbers; `None` where deleted.
    pub(crate) faces: Vec<Option<Vec<u32>>>,
    /// Hard edges, as pairs of point id numbers.
    pub(crate) hard: Vec<[u32; 2]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct LayerV1 {
    pub(crate) name: String,
    pub(crate) enabled: bool,
    pub(crate) operations: Vec<OperationV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum OperationV1 {
    MovePoints { points: Vec<u32>, offset: [f32; 3] },
}

// ------------------------------------------------------------- writing ---

impl ProjectV1 {
    /// The file model of a document.
    pub(crate) fn from_document(document: &Document) -> Self {
        let scene = document.scene();
        let objects = scene
            .walk()
            .into_iter()
            .filter_map(|(id, _)| {
                let object = scene.object(id)?;
                Some(ObjectV1 {
                    id: id.get(),
                    parent: scene.parent(id).map(ObjectId::get),
                    name: object.name.clone(),
                    visible: object.visible,
                    transform: TransformV1 {
                        position: object.transform.position.to_array(),
                        rotation: object.transform.rotation.to_array(),
                        scale: object.transform.scale.to_array(),
                    },
                    kind: kind_to_file(&object.kind),
                    layers: object.stack.layers().iter().map(layer_to_file).collect(),
                })
            })
            .collect();
        Self {
            objects,
            next_id: scene.next_object_id(),
            selection: document
                .selection()
                .into_iter()
                .map(ObjectId::get)
                .collect(),
        }
    }
}

fn kind_to_file(kind: &ObjectKind) -> KindV1 {
    match kind {
        ObjectKind::Primitive(p) => KindV1::Primitive(primitive_to_file(p)),
        ObjectKind::Mesh(mesh) => KindV1::Mesh(mesh_to_file(mesh)),
        // Every other kind is a group, or arrives with a new format version.
        _ => KindV1::Group,
    }
}

fn primitive_to_file(p: &Primitive) -> PrimitiveV1 {
    match *p {
        Primitive::Cube(c) => PrimitiveV1::Cube {
            size: c.size.to_array(),
            segments: c.segments,
        },
        Primitive::Plane(c) => PrimitiveV1::Plane {
            width: c.width,
            depth: c.depth,
            segments: c.segments,
        },
        Primitive::Cylinder(c) => PrimitiveV1::Cylinder {
            radius: c.radius,
            height: c.height,
            sides: c.sides,
            height_segments: c.height_segments,
            caps: c.caps,
            cap_segments: c.cap_segments,
            cap_centre: c.cap_centre,
        },
        Primitive::Sphere(c) => PrimitiveV1::Sphere {
            radius: c.radius,
            sides: c.sides,
            rings: c.rings,
        },
        Primitive::Disc(c) => PrimitiveV1::Disc {
            radius: c.radius,
            inner_radius: c.inner_radius,
            sides: c.sides,
            ring_segments: c.ring_segments,
            centre_point: c.centre_point,
        },
        Primitive::Tube(c) => PrimitiveV1::Tube {
            outer_radius: c.outer_radius,
            inner_radius: c.inner_radius,
            height: c.height,
            sides: c.sides,
            height_segments: c.height_segments,
        },
        Primitive::Cone(c) => PrimitiveV1::Cone {
            bottom_radius: c.bottom_radius,
            top_radius: c.top_radius,
            height: c.height,
            sides: c.sides,
            height_segments: c.height_segments,
            caps: c.caps,
            cap_segments: c.cap_segments,
            cap_centre: c.cap_centre,
        },
    }
}

fn mesh_to_file(mesh: &Mesh) -> MeshV1 {
    MeshV1 {
        points: mesh
            .point_slots()
            .iter()
            .map(|p| p.map(|p| p.to_array()))
            .collect(),
        faces: mesh
            .face_slots()
            .iter()
            .map(|f| f.as_ref().map(|c| c.iter().map(|v| v.get()).collect()))
            .collect(),
        hard: mesh
            .hard_edges()
            .map(|e| {
                let (a, b) = e.ends();
                [a.get(), b.get()]
            })
            .collect(),
    }
}

fn layer_to_file(layer: &Layer) -> LayerV1 {
    LayerV1 {
        name: layer.name.clone(),
        enabled: layer.enabled,
        operations: layer
            .operations
            .iter()
            .filter_map(|op| match op {
                Operation::MovePoints { points, offset } => Some(OperationV1::MovePoints {
                    points: points.iter().map(|v| v.get()).collect(),
                    offset: offset.to_array(),
                }),
                // New operations arrive with a new format version.
                _ => None,
            })
            .collect(),
    }
}

// ------------------------------------------------------------- reading ---

/// Why the contents couldn't become a document, in plain words.
pub(crate) type Problem = String;

impl ProjectV1 {
    /// The document this file model describes, after checking every value.
    pub(crate) fn into_document(self) -> Result<Document, Problem> {
        let mut objects = Vec::with_capacity(self.objects.len());
        for object in self.objects {
            let name = object.name.clone();
            objects
                .push(object_from_file(object).map_err(|why| format!("object \"{name}\": {why}"))?);
        }
        let scene = Scene::from_objects(objects, self.next_id).map_err(|e| e.to_string())?;
        let selection = self.selection.into_iter().map(ObjectId::from_raw);
        Ok(Document::from_scene(scene, selection))
    }
}

fn finite(values: [f32; 3], what: &str) -> Result<Vec3, Problem> {
    let v = Vec3::from_array(values);
    if v.is_finite() {
        Ok(v)
    } else {
        Err(format!("its {what} isn't a real number"))
    }
}

fn object_from_file(object: ObjectV1) -> Result<(ObjectId, Option<ObjectId>, Object), Problem> {
    let transform = Transform {
        position: finite(object.transform.position, "position")?,
        rotation: finite(object.transform.rotation, "rotation")?,
        scale: finite(object.transform.scale, "scale")?,
    };
    let kind = match object.kind {
        KindV1::Group => ObjectKind::Group,
        KindV1::Primitive(p) => ObjectKind::Primitive(primitive_from_file(p).clamped()),
        KindV1::Mesh(m) => ObjectKind::Mesh(Arc::new(mesh_from_file(m)?)),
    };
    let mut layers = Vec::with_capacity(object.layers.len());
    for layer in object.layers {
        layers.push(layer_from_file(layer)?);
    }
    let mut built = Object::new(kind);
    built.name = object.name;
    built.visible = object.visible;
    built.transform = transform;
    built.stack = Stack::from_layers(layers);
    Ok((
        ObjectId::from_raw(object.id),
        object.parent.map(ObjectId::from_raw),
        built,
    ))
}

/// A primitive's settings. Out-of-range values (including ones that
/// aren't numbers) are brought within limits by the caller.
fn primitive_from_file(p: PrimitiveV1) -> Primitive {
    match p {
        PrimitiveV1::Cube { size, segments } => Primitive::Cube(CubeParams {
            size: Vec3::from_array(size),
            segments,
        }),
        PrimitiveV1::Plane {
            width,
            depth,
            segments,
        } => Primitive::Plane(PlaneParams {
            width,
            depth,
            segments,
        }),
        PrimitiveV1::Cylinder {
            radius,
            height,
            sides,
            height_segments,
            caps,
            cap_segments,
            cap_centre,
        } => Primitive::Cylinder(CylinderParams {
            radius,
            height,
            sides,
            height_segments,
            caps,
            cap_segments,
            cap_centre,
        }),
        PrimitiveV1::Sphere {
            radius,
            sides,
            rings,
        } => Primitive::Sphere(SphereParams {
            radius,
            sides,
            rings,
        }),
        PrimitiveV1::Disc {
            radius,
            inner_radius,
            sides,
            ring_segments,
            centre_point,
        } => Primitive::Disc(DiscParams {
            radius,
            inner_radius,
            sides,
            ring_segments,
            centre_point,
        }),
        PrimitiveV1::Tube {
            outer_radius,
            inner_radius,
            height,
            sides,
            height_segments,
        } => Primitive::Tube(TubeParams {
            outer_radius,
            inner_radius,
            height,
            sides,
            height_segments,
        }),
        PrimitiveV1::Cone {
            bottom_radius,
            top_radius,
            height,
            sides,
            height_segments,
            caps,
            cap_segments,
            cap_centre,
        } => Primitive::Cone(ConeParams {
            bottom_radius,
            top_radius,
            height,
            sides,
            height_segments,
            caps,
            cap_segments,
            cap_centre,
        }),
    }
}

fn mesh_from_file(mesh: MeshV1) -> Result<Mesh, Problem> {
    let points = mesh
        .points
        .into_iter()
        .map(|p| p.map(Vec3::from_array))
        .collect();
    let faces = mesh
        .faces
        .into_iter()
        .map(|f| f.map(|c| c.into_iter().map(VertexId::from_raw).collect()))
        .collect();
    let hard = mesh
        .hard
        .into_iter()
        .map(|[a, b]| EdgeKey::new(VertexId::from_raw(a), VertexId::from_raw(b)));
    Mesh::from_slots(points, faces, hard).map_err(|e| format!("its mesh is damaged ({e})"))
}

fn layer_from_file(layer: LayerV1) -> Result<Layer, Problem> {
    let mut built = Layer::new(layer.name);
    built.enabled = layer.enabled;
    for op in layer.operations {
        built.operations.push(match op {
            OperationV1::MovePoints { points, offset } => Operation::MovePoints {
                points: points.into_iter().map(VertexId::from_raw).collect(),
                offset: finite(offset, "recorded move")?,
            },
        });
    }
    Ok(built)
}
