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
        "test.glb",
    )
    .unwrap()
    .main
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

/// A flat grid lying on the ground with UVs following X and Y, as an
/// exported shape.
fn uv_grid() -> ExportMesh {
    let mut shape = mesh::Primitive::Plane(mesh::PlaneParams {
        segments: [2, 2],
        ..Default::default()
    })
    .mesh();
    shape
        .add_attribute(
            mesh::attributes::UV,
            mesh::Domain::Corner,
            mesh::Value::Vec2(Vec2::ZERO),
        )
        .unwrap();
    let faces: Vec<(mesh::FaceId, Vec<mesh::VertexId>)> =
        shape.faces().map(|(f, c)| (f, c.to_vec())).collect();
    for (face, corners) in faces {
        for (i, v) in corners.iter().enumerate() {
            let p = shape.position(*v).unwrap();
            shape
                .set_attribute(
                    mesh::attributes::UV,
                    mesh::AttributeElement::Corner(face, i),
                    mesh::Value::Vec2(Vec2::new(p.x / 100.0 + 0.5, p.y / 100.0 + 0.5)),
                )
                .unwrap();
        }
    }
    crate::export::export_mesh(
        &shape,
        ExportOptions::default().to_file(),
        &ExportFormat::Glb.options(&ExportOptions::default()),
    )
}

#[test]
fn shapes_with_uvs_get_mikktspace_tangents() {
    let v = vertices(&uv_grid());
    let tangents = v.tangents.as_ref().expect("tangents with UVs");
    assert_eq!(tangents.len(), v.positions.len());
    for (t, n) in tangents.iter().zip(&v.normals) {
        let dir = t.truncate();
        assert!((dir.length() - 1.0).abs() < 1e-4, "{t}");
        assert!(dir.dot(*n).abs() < 1e-4, "square to the normal");
        assert!(t.w == 1.0 || t.w == -1.0);
        // U runs along X, in the app and in the file.
        assert!(dir.dot(Vec3::X) > 0.999, "{dir}");
    }
    // Every corner of a flat grid has the same tangent: shared vertices
    // stay shared.
    assert_eq!(v.positions.len(), 9);
    assert!(tangents.windows(2).all(|w| w[0] == w[1]));
    // And the file lists them.
    let doc = ExportMesh::default();
    assert!(vertices(&doc).tangents.is_none());
}

#[test]
fn tangents_go_into_the_file() {
    let object = crate::export::ExportedObject {
        id: scene::ObjectId::from_raw(1),
        name: "Grid".into(),
        pivot: glam::Mat4::IDENTITY,
        world: uv_grid(),
        local: uv_grid(),
        materials: vec![scene::Material::standard()],
        collision: scene::Collision::None,
    };
    let glb = write_glb(&[object], "Test App 1.0", false);
    let (json, _) = parts(&glb);
    let attributes = &json["meshes"][0]["primitives"][0]["attributes"];
    let tangent = attributes["TANGENT"].as_u64().unwrap() as usize;
    assert_eq!(json["accessors"][tangent]["type"], "VEC4");
    assert!(attributes.get("TEXCOORD_0").is_some());
    if let Some(path) = std::env::var_os("GLB_UV_SAMPLE") {
        std::fs::write(path, &glb).unwrap();
    }
}

/// A cube with three materials (one used by no polygon) and two polygons
/// given the second, and a plane whose only material shares the cube's
/// second's name.
fn scene_with_materials() -> Document {
    let mut doc = scene_of(&[PrimitiveKind::Cube, PrimitiveKind::Plane]);
    let named = |doc: &Document, name: &str| {
        doc.scene()
            .walk()
            .into_iter()
            .map(|(id, _)| id)
            .find(|id| doc.scene().object(*id).unwrap().name == name)
            .unwrap()
    };
    let ids = [named(&doc, "Cube"), named(&doc, "Plane")];
    let paint = scene::Material {
        name: "Paint".into(),
        color: [1.0, 0.0, 0.0],
    };
    doc.execute(Command::SetMaterials {
        id: ids[0],
        materials: vec![
            scene::Material::standard(),
            paint.clone(),
            scene::Material {
                name: "Unused".into(),
                color: [0.0, 1.0, 0.0],
            },
        ],
    })
    .unwrap();
    doc.execute(Command::Record {
        id: ids[0],
        operation: scene::Operation::SetMaterial {
            faces: (0..2).map(mesh::FaceId::from_raw).collect(),
            slot: 1,
            anchors: None,
        },
    })
    .unwrap();
    doc.execute(Command::SetMaterials {
        id: ids[1],
        materials: vec![scene::Material {
            color: [0.0, 0.0, 1.0],
            ..paint
        }],
    })
    .unwrap();
    doc
}

#[test]
fn materials_go_into_the_file_once_each_by_name() {
    let glb = glb_of(&scene_with_materials());
    if let Some(path) = std::env::var_os("GLB_MATERIAL_SAMPLE") {
        std::fs::write(path, &glb).unwrap();
    }
    let (json, _) = parts(&glb);
    let materials = json["materials"].as_array().unwrap();
    let names: Vec<&str> = materials
        .iter()
        .map(|m| m["name"].as_str().unwrap())
        .collect();
    // "Unused" is used by no polygon; "Paint" is one material, in the
    // order first used (the plane comes first in the Objects panel), its
    // colour the plane's.
    assert_eq!(names, ["Paint", "Default"]);
    let blue = &materials[0]["pbrMetallicRoughness"]["baseColorFactor"];
    assert_eq!(blue[0].as_f64(), Some(0.0));
    assert_eq!(blue[2].as_f64(), Some(1.0));
    // The grey (the viewport's) is converted to linear light.
    let grey = materials[1]["pbrMetallicRoughness"]["baseColorFactor"][0]
        .as_f64()
        .unwrap();
    assert!((grey - 0.5).abs() < 0.01, "{grey}");
    // The cube has one primitive per material; the plane uses Paint.
    let mesh_named = |name: &str| {
        json["meshes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["name"] == name)
            .unwrap()
            .clone()
    };
    let cube = mesh_named("Cube");
    let cube = cube["primitives"].as_array().unwrap();
    let used: Vec<u64> = cube
        .iter()
        .map(|p| p["material"].as_u64().unwrap())
        .collect();
    assert_eq!(used, [1, 0]);
    assert_eq!(mesh_named("Plane")["primitives"][0]["material"], 0);
}

#[test]
fn an_instance_uses_its_originals_materials() {
    let mut doc = scene_with_materials();
    let cube = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == "Cube")
        .unwrap();
    doc.execute(Command::Duplicate {
        ids: vec![cube],
        instances: true,
    })
    .unwrap();
    let objects = prepare(&doc, &ExportOptions::default());
    let instance = objects.iter().find(|o| o.name == "Cube 2").unwrap();
    assert_eq!(instance.materials.len(), 3);
    assert!(instance.world.polygons.iter().any(|p| p.material == 1));
}

#[test]
fn collision_names_and_boxes_go_out_for_godot() {
    let mut doc = scene_of(&[
        PrimitiveKind::Cube,
        PrimitiveKind::Sphere,
        PrimitiveKind::Cylinder,
    ]);
    let named = |doc: &Document, name: &str| {
        doc.scene()
            .walk()
            .into_iter()
            .map(|(id, _)| id)
            .find(|id| doc.scene().object(*id).unwrap().name == name)
            .unwrap()
    };
    for (name, collision) in [
        ("Cube", scene::Collision::Mesh),
        ("Sphere", scene::Collision::Convex),
        ("Cylinder", scene::Collision::Box),
    ] {
        let id = named(&doc, name);
        doc.execute(Command::SetCollision {
            ids: vec![id],
            collision,
        })
        .unwrap();
    }
    let names = |godot_names: bool| {
        let options = ExportOptions {
            godot_names,
            ..ExportOptions::default()
        };
        let files = export_bytes(&doc, ExportFormat::Glb, &options, "T", "t.glb").unwrap();
        if let Some(path) = std::env::var_os("GLB_COLLISION_SAMPLE").filter(|_| godot_names) {
            std::fs::write(path, &files.main).unwrap();
        }
        let (json, _) = parts(&files.main);
        let mut names: Vec<String> = json["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n["name"].as_str().unwrap().to_string())
            .collect();
        names.sort();
        (names, json)
    };
    let (godot, json) = names(true);
    assert_eq!(
        godot,
        [
            "Cube-col",
            "Cylinder",
            "Cylinder-convcolonly",
            "Sphere-convcol"
        ]
    );
    // The box has eight corners, around the cylinder, at its pivot.
    let node = json["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["name"] == "Cylinder-convcolonly")
        .unwrap();
    let mesh = &json["meshes"][node["mesh"].as_u64().unwrap() as usize];
    let position = mesh["primitives"][0]["attributes"]["POSITION"]
        .as_u64()
        .unwrap() as usize;
    assert_eq!(json["accessors"][position]["count"], 24, "a crisp box");
    let (plain, _) = names(false);
    assert_eq!(plain, ["Cube", "Cylinder", "Sphere"]);
}
