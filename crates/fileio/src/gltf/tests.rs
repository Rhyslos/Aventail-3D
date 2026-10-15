//! Tests for glTF files: the container, the scene description and the
//! numbers, read back the way an engine reads them.

use super::*;
use crate::export::{ExportFormat, ExportOptions, export_bytes, prepare};
use scene::{Command, Document, ObjectKind, PrimitiveKind, Transform};

fn scene_of(kinds: &[PrimitiveKind]) -> Document {
    let mut doc = Document::new();
    for kind in kinds {
        doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
            .unwrap();
    }
    doc
}

/// The JSON and binary parts of a `.glb`, checking the container.
fn parts(glb: &[u8]) -> (Json, Vec<u8>) {
    let word = |at: usize| u32::from_le_bytes(glb[at..at + 4].try_into().unwrap()) as usize;
    assert_eq!(&glb[0..4], b"glTF");
    assert_eq!(word(4), 2);
    assert_eq!(word(8), glb.len());
    let json_len = word(12);
    assert_eq!(&glb[16..20], b"JSON");
    assert_eq!(json_len % 4, 0);
    let json: Json = serde_json::from_slice(&glb[20..20 + json_len]).unwrap();
    let at = 20 + json_len;
    let bin_len = word(at);
    assert_eq!(&glb[at + 4..at + 8], b"BIN\0");
    assert_eq!(bin_len % 4, 0);
    let bin = glb[at + 8..at + 8 + bin_len].to_vec();
    assert_eq!(at + 8 + bin_len, glb.len());
    (json, bin)
}

/// The floats of an accessor.
fn floats(json: &Json, bin: &[u8], accessor: usize) -> Vec<f32> {
    let a = &json["accessors"][accessor];
    let view = &json["bufferViews"][a["bufferView"].as_u64().unwrap() as usize];
    let offset = view["byteOffset"].as_u64().unwrap() as usize;
    let length = view["byteLength"].as_u64().unwrap() as usize;
    bin[offset..offset + length]
        .chunks(4)
        .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
        .collect()
}

fn glb_of(doc: &Document) -> Vec<u8> {
    export_bytes(
        doc,
        ExportFormat::Glb,
        &ExportOptions::default(),
        "Test App 1.0",
    )
    .unwrap()
    .0
}

#[test]
fn a_cube_becomes_a_valid_binary_gltf() {
    let doc = scene_of(&[PrimitiveKind::Cube]);
    let (json, bin) = parts(&glb_of(&doc));
    assert_eq!(json["asset"]["version"], "2.0");
    assert_eq!(json["asset"]["generator"], "Test App 1.0");
    assert_eq!(
        json["buffers"][0]["byteLength"].as_u64().unwrap() as usize,
        bin.len()
    );
    assert_eq!(json["nodes"][0]["name"], "Cube");
    let primitive = &json["meshes"][0]["primitives"][0];
    assert_eq!(primitive["mode"], 4);
    // Four corners on each of six crisp sides; twelve triangles.
    let position = primitive["attributes"]["POSITION"].as_u64().unwrap() as usize;
    assert_eq!(json["accessors"][position]["count"], 24);
    let indices = primitive["indices"].as_u64().unwrap() as usize;
    assert_eq!(json["accessors"][indices]["count"], 36);
    // Metres, Y up: 1 m tall, standing on the ground.
    let max = &json["accessors"][position]["max"];
    assert!((max[1].as_f64().unwrap() - 1.0).abs() < 1e-5, "{max}");
    let values = floats(&json, &bin, position);
    let top = values.chunks(3).map(|p| p[1]).fold(f32::MIN, f32::max);
    assert!((top - 1.0).abs() < 1e-5);
    // Every buffer view starts on a multiple of 4.
    for view in json["bufferViews"].as_array().unwrap() {
        assert_eq!(view["byteOffset"].as_u64().unwrap() % 4, 0);
    }
}

#[test]
fn smooth_shapes_share_their_vertices() {
    let doc = scene_of(&[PrimitiveKind::Sphere]);
    let objects = prepare(&doc, &ExportFormat::Glb.options(&ExportOptions::default()));
    let v = vertices(&objects[0].local);
    let points = objects[0].local.points.len();
    // A smooth sphere splits only where its UV-less shading breaks: not
    // at all.
    assert_eq!(v.positions.len(), points);
    let triangles: usize = v.triangles.iter().map(|(_, c)| c.len() / 3).sum();
    assert_eq!(triangles, objects[0].local.triangle_count());
}

#[test]
fn each_object_sits_at_its_pivot() {
    let mut doc = scene_of(&[PrimitiveKind::Cube]);
    let id = doc.selection()[0];
    doc.execute(Command::SetTransforms(vec![(
        id,
        Transform {
            position: glam::Vec3::new(200.0, 0.0, 0.0),
            ..Transform::default()
        },
    )]))
    .unwrap();
    let (json, bin) = parts(&glb_of(&doc));
    let m = json["nodes"][0]["matrix"].as_array().unwrap();
    // 2 m along X: the translation is the last column.
    assert!((m[12].as_f64().unwrap() - 2.0).abs() < 1e-5, "{m:?}");
    // The shape is around the pivot: its middle on the node.
    let position = json["meshes"][0]["primitives"][0]["attributes"]["POSITION"]
        .as_u64()
        .unwrap() as usize;
    let xs: Vec<f32> = floats(&json, &bin, position)
        .chunks(3)
        .map(|p| p[0])
        .collect();
    let middle = (xs.iter().copied().fold(f32::MIN, f32::max)
        + xs.iter().copied().fold(f32::MAX, f32::min))
        / 2.0;
    assert!(middle.abs() < 1e-5);
    // An object at the origin has no matrix.
    let (json, _) = parts(&glb_of(&scene_of(&[PrimitiveKind::Cube])));
    assert!(json["nodes"][0].get("matrix").is_none());
}

#[test]
fn uvs_turn_upside_down_for_gltf() {
    let shape = ExportMesh {
        points: vec![Vec3::ZERO, Vec3::X, Vec3::Y],
        normals: vec![Vec3::Z],
        uvs: vec![Vec2::new(0.25, 0.0), Vec2::new(1.0, 0.75)],
        polygons: vec![crate::export::ExportPolygon {
            corners: vec![
                crate::export::ExportCorner {
                    point: 0,
                    normal: 0,
                    uv: Some(0),
                },
                crate::export::ExportCorner {
                    point: 1,
                    normal: 0,
                    uv: Some(1),
                },
                crate::export::ExportCorner {
                    point: 2,
                    normal: 0,
                    uv: Some(1),
                },
            ],
            material: 0,
        }],
    };
    let v = vertices(&shape);
    let uvs = v.uvs.unwrap();
    assert_eq!(uvs[0], Vec2::new(0.25, 1.0));
    assert_eq!(uvs[1], Vec2::new(1.0, 0.25));
    assert_eq!(v.triangles, vec![(0, vec![0, 1, 2])]);
}

#[test]
fn gltf_is_always_metres_y_up_and_triangles() {
    let options = ExportOptions {
        units: crate::export::FileUnits::Centimetres,
        up: crate::export::UpAxis::Z,
        triangulate: false,
        ..ExportOptions::default()
    };
    let fixed = ExportFormat::Glb.options(&options);
    assert_eq!(fixed.units, crate::export::FileUnits::Metres);
    assert_eq!(fixed.up, crate::export::UpAxis::Y);
    assert!(fixed.triangulate);
    assert_eq!(ExportFormat::Obj.options(&options), options);
}

#[test]
fn several_objects_and_the_same_bytes_every_time() {
    let doc = scene_of(&[
        PrimitiveKind::Cube,
        PrimitiveKind::Cylinder,
        PrimitiveKind::Plane,
    ]);
    let first = glb_of(&doc);
    assert_eq!(first, glb_of(&doc));
    let (json, _) = parts(&first);
    assert_eq!(json["nodes"].as_array().unwrap().len(), 3);
    assert_eq!(json["scenes"][0]["nodes"], serde_json::json!([0, 1, 2]));
    // A sample to check with the Khronos validator by hand:
    // GLB_SAMPLE=/path/file.glb cargo test -p fileio several_objects
    if let Some(path) = std::env::var_os("GLB_SAMPLE") {
        std::fs::write(path, &first).unwrap();
    }
}
