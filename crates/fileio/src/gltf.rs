//! glTF files: writing exported objects as binary glTF (`.glb`)
//! ([decision 0072](../../../docs/decisions/0072-gltf-export.md)).
//!
//! glTF is the open format Godot and Substance Painter read best. A
//! `.glb` is one file in three parts: a 12-byte header, a JSON part
//! describing the scene (nodes, meshes, materials, and *accessors* that
//! say where each list of numbers sits), and a binary part holding the
//! numbers themselves.
//!
//! Each object becomes a **node** placed at its pivot (so Godot places,
//! turns and snaps it by the pivot set in the app), holding a **mesh**
//! of its shape around that pivot. glTF is always metres with Y up, and
//! always triangles: the export is made that way whatever the window
//! says. A glTF vertex is one point with one normal, one UV and (with
//! UVs) one MikkTSpace tangent, so points are split where those break,
//! as every engine does.

use crate::export::{ExportCorner, ExportMesh, ExportedObject};
use glam::{Vec2, Vec3, Vec4};
use serde_json::{Value as Json, json};
use std::collections::HashMap;

/// glTF's number for a list of vertex values.
const ARRAY_BUFFER: u32 = 34962;
/// glTF's number for a list of triangle corners.
const ELEMENT_ARRAY_BUFFER: u32 = 34963;
/// glTF's number for 32-bit floating point values.
const FLOAT: u32 = 5126;
/// glTF's number for unsigned 32-bit whole numbers.
const UNSIGNED_INT: u32 = 5125;
/// glTF's number for drawing triangles.
const TRIANGLES: u32 = 4;

/// The binary part being built, and the JSON lists that describe it.
#[derive(Default)]
struct Builder {
    bin: Vec<u8>,
    views: Vec<Json>,
    accessors: Vec<Json>,
}

impl Builder {
    /// Adds `bytes` as a buffer view (aligned to 4 bytes), and returns its
    /// index.
    fn view(&mut self, bytes: &[u8], target: u32) -> usize {
        while !self.bin.len().is_multiple_of(4) {
            self.bin.push(0);
        }
        let offset = self.bin.len();
        self.bin.extend_from_slice(bytes);
        self.views.push(json!({
            "buffer": 0,
            "byteOffset": offset,
            "byteLength": bytes.len(),
            "target": target,
        }));
        self.views.len() - 1
    }

    /// Adds a list of 3-number vectors, with its bounds when `bounds`
    /// (glTF requires them for positions), and returns its accessor.
    fn vec3s(&mut self, values: &[Vec3], bounds: bool) -> usize {
        let bytes: Vec<u8> = values
            .iter()
            .flat_map(|v| v.to_array())
            .flat_map(f32::to_le_bytes)
            .collect();
        let view = self.view(&bytes, ARRAY_BUFFER);
        let mut accessor = json!({
            "bufferView": view,
            "componentType": FLOAT,
            "count": values.len(),
            "type": "VEC3",
        });
        if bounds {
            let min = values
                .iter()
                .copied()
                .fold(Vec3::splat(f32::MAX), Vec3::min);
            let max = values
                .iter()
                .copied()
                .fold(Vec3::splat(f32::MIN), Vec3::max);
            accessor["min"] = json!(min.to_array());
            accessor["max"] = json!(max.to_array());
        }
        self.accessors.push(accessor);
        self.accessors.len() - 1
    }

    /// Adds a list of 2-number vectors, and returns its accessor.
    fn vec2s(&mut self, values: &[Vec2]) -> usize {
        let bytes: Vec<u8> = values
            .iter()
            .flat_map(|v| v.to_array())
            .flat_map(f32::to_le_bytes)
            .collect();
        let view = self.view(&bytes, ARRAY_BUFFER);
        self.accessors.push(json!({
            "bufferView": view,
            "componentType": FLOAT,
            "count": values.len(),
            "type": "VEC2",
        }));
        self.accessors.len() - 1
    }

    /// Adds a list of 4-number vectors, and returns its accessor.
    fn vec4s(&mut self, values: &[Vec4]) -> usize {
        let bytes: Vec<u8> = values
            .iter()
            .flat_map(|v| v.to_array())
            .flat_map(f32::to_le_bytes)
            .collect();
        let view = self.view(&bytes, ARRAY_BUFFER);
        self.accessors.push(json!({
            "bufferView": view,
            "componentType": FLOAT,
            "count": values.len(),
            "type": "VEC4",
        }));
        self.accessors.len() - 1
    }

    /// Adds triangle corners, and returns their accessor.
    fn indices(&mut self, values: &[u32]) -> usize {
        let bytes: Vec<u8> = values.iter().flat_map(|i| i.to_le_bytes()).collect();
        let view = self.view(&bytes, ELEMENT_ARRAY_BUFFER);
        self.accessors.push(json!({
            "bufferView": view,
            "componentType": UNSIGNED_INT,
            "count": values.len(),
            "type": "SCALAR",
        }));
        self.accessors.len() - 1
    }
}

/// A glTF vertex list: one vertex for each distinct point, normal, UV
/// and tangent a shape's corners use, and the triangles over them, by
/// material slot.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Vertices {
    /// Where each vertex is.
    pub positions: Vec<Vec3>,
    /// Its normal.
    pub normals: Vec<Vec3>,
    /// Its UV (glTF's way up: V counts down from the top), when the shape
    /// has UVs.
    pub uvs: Option<Vec<Vec2>>,
    /// Its MikkTSpace tangent and handedness (decision 0073), when the
    /// shape has UVs: what a normal map baked in Substance Painter needs
    /// to look the same in the engine.
    pub tangents: Option<Vec<Vec4>>,
    /// Triangle corners for each material slot, in slot order.
    pub triangles: Vec<(u32, Vec<u32>)>,
}

/// A shape's triangles, for MikkTSpace to read and write back.
struct Mikk<'a> {
    shape: &'a ExportMesh,
    triangles: &'a [(u32, [ExportCorner; 3])],
    tangents: Vec<[Vec4; 3]>,
}

impl Mikk<'_> {
    fn corner(&self, face: usize, vert: usize) -> ExportCorner {
        self.triangles[face].1[vert]
    }
}

impl bevy_mikktspace::Geometry for Mikk<'_> {
    fn num_faces(&self) -> usize {
        self.triangles.len()
    }

    fn num_vertices_of_face(&self, _face: usize) -> usize {
        3
    }

    fn position(&self, face: usize, vert: usize) -> [f32; 3] {
        self.shape.points[self.corner(face, vert).point as usize].to_array()
    }

    fn normal(&self, face: usize, vert: usize) -> [f32; 3] {
        self.shape.normals[self.corner(face, vert).normal as usize].to_array()
    }

    fn tex_coord(&self, face: usize, vert: usize) -> [f32; 2] {
        gltf_uv(self.shape, self.corner(face, vert)).to_array()
    }

    fn set_tangent(
        &mut self,
        tangent: Option<bevy_mikktspace::TangentSpace>,
        face: usize,
        vert: usize,
    ) {
        let t = tangent.unwrap_or_default().tangent_encoded();
        self.tangents[face][vert] = Vec4::from_array(t);
    }
}

/// A corner's UV the way glTF holds it: V counting down from the top.
fn gltf_uv(shape: &ExportMesh, corner: ExportCorner) -> Vec2 {
    let uv = corner.uv.map_or(Vec2::ZERO, |t| shape.uvs[t as usize]);
    Vec2::new(uv.x, 1.0 - uv.y)
}

/// The glTF vertices and triangles of `shape` (its polygons must be
/// triangles; others are cut as a fan). With UVs, every corner gets its
/// MikkTSpace tangent, worked out on the UVs as the file holds them, so a
/// program that works them out again from the file gets the same.
pub fn vertices(shape: &ExportMesh) -> Vertices {
    let triangles: Vec<(u32, [ExportCorner; 3])> = shape
        .polygons
        .iter()
        .flat_map(|p| {
            let c = &p.corners;
            (1..c.len().saturating_sub(1)).map(move |i| (p.material, [c[0], c[i], c[i + 1]]))
        })
        .collect();
    let has_uvs = !shape.uvs.is_empty();
    let tangents = if has_uvs {
        let mut mikk = Mikk {
            shape,
            triangles: &triangles,
            tangents: vec![[Vec4::new(1.0, 0.0, 0.0, 1.0); 3]; triangles.len()],
        };
        // Never fails: its error type has no values.
        let _ = bevy_mikktspace::generate_tangents(&mut mikk);
        Some(mikk.tangents)
    } else {
        None
    };
    let mut out = Vertices {
        uvs: has_uvs.then(Vec::new),
        tangents: has_uvs.then(Vec::new),
        ..Vertices::default()
    };
    type Key = (u32, u32, Option<u32>, Option<[u32; 4]>);
    let mut index: HashMap<Key, u32> = HashMap::new();
    let mut by_slot: std::collections::BTreeMap<u32, Vec<u32>> = std::collections::BTreeMap::new();
    for (t, (material, corners)) in triangles.iter().enumerate() {
        let list = by_slot.entry(*material).or_default();
        for (k, c) in corners.iter().enumerate() {
            let tangent = tangents.as_ref().map(|all| all[t][k]);
            let key = (
                c.point,
                c.normal,
                c.uv,
                tangent.map(|v| v.to_array().map(f32::to_bits)),
            );
            let next = out.positions.len() as u32;
            let id = *index.entry(key).or_insert(next);
            if id == next {
                out.positions.push(shape.points[c.point as usize]);
                out.normals.push(shape.normals[c.normal as usize]);
                if let Some(uvs) = &mut out.uvs {
                    uvs.push(gltf_uv(shape, *c));
                }
                if let (Some(list), Some(tangent)) = (&mut out.tangents, tangent) {
                    list.push(tangent);
                }
            }
            list.push(id);
        }
    }
    out.triangles = by_slot.into_iter().collect();
    out
}

/// The column-major 4 × 4 matrix of a node, as glTF lists it, with the
/// crumbs of turning (a 0 worked out as 0.00000004) swept to 0.
fn matrix(m: glam::Mat4) -> [f32; 16] {
    m.to_cols_array()
        .map(|x| if x.abs() < 1e-6 { 0.0 } else { x })
}

/// The name an object's node gets: with `godot_names`, its collision as
/// Godot reads it from the name (decision 0078): `-col` for the shape
/// itself, `-convcol` for it wrapped.
fn node_name(object: &ExportedObject, godot_names: bool) -> String {
    match (godot_names, object.collision) {
        (true, scene::Collision::Mesh) => format!("{}-col", object.name),
        (true, scene::Collision::Convex) => format!("{}-convcol", object.name),
        _ => object.name.clone(),
    }
}

/// A box around `shape`'s points, as a shape of six sides facing out.
fn box_around(shape: &crate::export::ExportMesh) -> crate::export::ExportMesh {
    use crate::export::{ExportCorner, ExportMesh, ExportPolygon};
    let (min, max) = shape.points.iter().fold(
        (glam::Vec3::splat(f32::MAX), glam::Vec3::splat(f32::MIN)),
        |(lo, hi), p| (lo.min(*p), hi.max(*p)),
    );
    let corner = |i: usize| {
        glam::Vec3::new(
            if i & 1 == 0 { min.x } else { max.x },
            if i & 2 == 0 { min.y } else { max.y },
            if i & 4 == 0 { min.z } else { max.z },
        )
    };
    // Each side: its corners counter-clockwise seen from outside, and the
    // way it faces.
    let sides: [([u32; 4], glam::Vec3); 6] = [
        ([0, 2, 6, 4], -glam::Vec3::X),
        ([1, 5, 7, 3], glam::Vec3::X),
        ([0, 4, 5, 1], -glam::Vec3::Y),
        ([2, 3, 7, 6], glam::Vec3::Y),
        ([0, 1, 3, 2], -glam::Vec3::Z),
        ([4, 6, 7, 5], glam::Vec3::Z),
    ];
    ExportMesh {
        points: (0..8).map(corner).collect(),
        normals: sides.iter().map(|(_, n)| *n).collect(),
        uvs: Vec::new(),
        polygons: sides
            .iter()
            .enumerate()
            .map(|(n, (c, _))| ExportPolygon {
                corners: c
                    .iter()
                    .map(|p| ExportCorner {
                        point: *p,
                        normal: n as u32,
                        uv: None,
                    })
                    .collect(),
                material: 0,
            })
            .collect(),
    }
}

/// The bytes of a binary glTF file holding `objects` (made with metres,
/// Y up and triangles), naming `generator` as the program that made it.
/// With `godot_names`, objects' names carry their collision for Godot,
/// and an object that collides as a box gets a box beside it called
/// `name-convcolonly` (decision 0078).
pub fn write_glb(objects: &[ExportedObject], generator: &str, godot_names: bool) -> Vec<u8> {
    let materials = crate::export::file_materials(objects);
    let boxes: Vec<ExportedObject> = if godot_names {
        objects
            .iter()
            .filter(|o| o.collision == scene::Collision::Box && !o.local.points.is_empty())
            .map(|o| {
                let shape = box_around(&o.local);
                ExportedObject {
                    name: format!("{}-convcolonly", o.name),
                    world: shape.clone(),
                    local: shape,
                    collision: scene::Collision::None,
                    ..o.clone()
                }
            })
            .collect()
    } else {
        Vec::new()
    };
    let mut b = Builder::default();
    let mut nodes = Vec::new();
    let mut meshes = Vec::new();
    for object in objects.iter().chain(&boxes) {
        let vertices = vertices(&object.local);
        if vertices.positions.is_empty() {
            continue;
        }
        let position = b.vec3s(&vertices.positions, true);
        let normal = b.vec3s(&vertices.normals, false);
        let uv = vertices.uvs.as_ref().map(|uvs| b.vec2s(uvs));
        let tangent = vertices.tangents.as_ref().map(|t| b.vec4s(t));
        let primitives: Vec<Json> = vertices
            .triangles
            .iter()
            .filter(|(_, corners)| !corners.is_empty())
            .map(|(slot, corners)| {
                let material = object
                    .materials
                    .get(*slot as usize)
                    .and_then(|m| materials.iter().position(|f| f.name == m.name))
                    .unwrap_or(0);
                let indices = b.indices(corners);
                let mut attributes = json!({ "POSITION": position, "NORMAL": normal });
                if let Some(uv) = uv {
                    attributes["TEXCOORD_0"] = json!(uv);
                }
                if let Some(tangent) = tangent {
                    attributes["TANGENT"] = json!(tangent);
                }
                json!({
                    "attributes": attributes,
                    "indices": indices,
                    "material": material,
                    "mode": TRIANGLES,
                })
            })
            .collect();
        let name = node_name(object, godot_names);
        meshes.push(json!({ "name": name, "primitives": primitives }));
        let mut node = json!({ "name": name, "mesh": meshes.len() - 1 });
        if !object.pivot.abs_diff_eq(glam::Mat4::IDENTITY, 1e-6) {
            node["matrix"] = json!(matrix(object.pivot));
        }
        nodes.push(node);
    }
    let mut document = json!({
        "asset": { "version": "2.0", "generator": generator },
        "scene": 0,
        "scenes": [{ "nodes": (0..nodes.len()).collect::<Vec<_>>() }],
        "nodes": nodes,
        "meshes": meshes,
        "materials": gltf_materials(&materials),
        "buffers": [{ "byteLength": b.bin.len() }],
    });
    if !b.views.is_empty() {
        document["bufferViews"] = json!(b.views);
        document["accessors"] = json!(b.accessors);
    }
    glb(&document, b.bin)
}

/// The file's materials: each one's name and colour (a plain, matte,
/// non-metal surface). glTF colours are linear, so the colour picked
/// (sRGB, as seen on screen) is converted. Without any, the standard one.
fn gltf_materials(materials: &[scene::Material]) -> Json {
    let standard = [scene::Material::standard()];
    let list = if materials.is_empty() {
        &standard[..]
    } else {
        materials
    };
    Json::Array(
        list.iter()
            .map(|m| {
                let [r, g, b] = m.color.map(linear);
                json!({
                    "name": m.name,
                    "pbrMetallicRoughness": {
                        "baseColorFactor": [r, g, b, 1.0],
                        "metallicFactor": 0.0,
                        "roughnessFactor": 0.8,
                    },
                })
            })
            .collect(),
    )
}

/// An sRGB colour channel (0 to 1) as linear light.
fn linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// The binary container: header, JSON part (padded with spaces) and
/// binary part (padded with zeros).
fn glb(document: &Json, mut bin: Vec<u8>) -> Vec<u8> {
    let mut text = document.to_string().into_bytes();
    while !text.len().is_multiple_of(4) {
        text.push(b' ');
    }
    while !bin.len().is_multiple_of(4) {
        bin.push(0);
    }
    let total = 12 + 8 + text.len() + 8 + bin.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(text.len() as u32).to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&text);
    out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    out.extend_from_slice(b"BIN\0");
    out.extend_from_slice(&bin);
    out
}

#[cfg(test)]
mod tests;
