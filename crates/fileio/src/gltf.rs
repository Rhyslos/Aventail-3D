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
//! says. A glTF vertex is one point with one normal and one UV, so
//! points are split where their shading breaks, as every engine does.

use crate::export::{ExportMesh, ExportedObject};
use glam::{Vec2, Vec3};
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

/// A glTF vertex list: one vertex for each distinct point, normal and UV
/// a shape's corners use, and the triangles over them, by material slot.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Vertices {
    /// Where each vertex is.
    pub positions: Vec<Vec3>,
    /// Its normal.
    pub normals: Vec<Vec3>,
    /// Its UV (glTF's way up: V counts down from the top), when the shape
    /// has UVs.
    pub uvs: Option<Vec<Vec2>>,
    /// Triangle corners for each material slot, in slot order.
    pub triangles: Vec<(u32, Vec<u32>)>,
}

/// The glTF vertices and triangles of `shape` (its polygons must be
/// triangles; others are cut as a fan).
pub fn vertices(shape: &ExportMesh) -> Vertices {
    let mut out = Vertices {
        uvs: (!shape.uvs.is_empty()).then(Vec::new),
        ..Vertices::default()
    };
    let mut index: HashMap<(u32, u32, Option<u32>), u32> = HashMap::new();
    let mut by_slot: std::collections::BTreeMap<u32, Vec<u32>> = std::collections::BTreeMap::new();
    for polygon in &shape.polygons {
        let ids: Vec<u32> = polygon
            .corners
            .iter()
            .map(|c| {
                let next = out.positions.len() as u32;
                let id = *index.entry((c.point, c.normal, c.uv)).or_insert(next);
                if id == next {
                    out.positions.push(shape.points[c.point as usize]);
                    out.normals.push(shape.normals[c.normal as usize]);
                    if let Some(uvs) = &mut out.uvs {
                        let uv = c.uv.map_or(Vec2::ZERO, |t| shape.uvs[t as usize]);
                        uvs.push(Vec2::new(uv.x, 1.0 - uv.y));
                    }
                }
                id
            })
            .collect();
        let list = by_slot.entry(polygon.material).or_default();
        for i in 1..ids.len().saturating_sub(1) {
            list.extend([ids[0], ids[i], ids[i + 1]]);
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

/// The bytes of a binary glTF file holding `objects` (made with metres,
/// Y up and triangles), naming `generator` as the program that made it.
pub fn write_glb(objects: &[ExportedObject], generator: &str) -> Vec<u8> {
    let mut b = Builder::default();
    let mut nodes = Vec::new();
    let mut meshes = Vec::new();
    for object in objects {
        let vertices = vertices(&object.local);
        if vertices.positions.is_empty() {
            continue;
        }
        let position = b.vec3s(&vertices.positions, true);
        let normal = b.vec3s(&vertices.normals, false);
        let uv = vertices.uvs.as_ref().map(|uvs| b.vec2s(uvs));
        let primitives: Vec<Json> = vertices
            .triangles
            .iter()
            .filter(|(_, corners)| !corners.is_empty())
            .map(|(_, corners)| {
                let indices = b.indices(corners);
                let mut attributes = json!({ "POSITION": position, "NORMAL": normal });
                if let Some(uv) = uv {
                    attributes["TEXCOORD_0"] = json!(uv);
                }
                json!({
                    "attributes": attributes,
                    "indices": indices,
                    "material": 0,
                    "mode": TRIANGLES,
                })
            })
            .collect();
        meshes.push(json!({ "name": object.name, "primitives": primitives }));
        let mut node = json!({ "name": object.name, "mesh": meshes.len() - 1 });
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
        "materials": [{
            "name": "Default",
            "pbrMetallicRoughness": {
                "baseColorFactor": [0.8, 0.8, 0.8, 1.0],
                "metallicFactor": 0.0,
                "roughnessFactor": 0.8,
            },
        }],
        "buffers": [{ "byteLength": b.bin.len() }],
    });
    if !b.views.is_empty() {
        document["bufferViews"] = json!(b.views);
        document["accessors"] = json!(b.accessors);
    }
    glb(&document, b.bin)
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
