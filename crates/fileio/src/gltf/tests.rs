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
    // A smooth sphere splits only where its UVs are cut (down the back
    // and at the poles, decision 0095), never for its shading: one vertex
    // for each point and UV that go together.
    let pairs: std::collections::BTreeSet<(u32, Option<u32>)> = objects[0]
        .local
        .polygons
        .iter()
        .flat_map(|p| p.corners.iter().map(|c| (c.point, c.uv)))
        .collect();
    assert!(pairs.len() > objects[0].local.points.len());
    assert_eq!(v.positions.len(), pairs.len());
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
        collision_parts: Vec::new(),
        levels: Vec::new(),
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

#[test]
fn collision_parts_go_out_beside_their_object_for_godot() {
    let mut doc = scene_of(&[PrimitiveKind::Arch, PrimitiveKind::Cube]);
    let ids: Vec<_> = doc.scene().walk().into_iter().map(|(id, _)| id).collect();
    doc.execute(Command::SetCollision {
        ids,
        collision: scene::Collision::Parts,
    })
    .unwrap();
    let export = |godot_names: bool| {
        let options = ExportOptions {
            godot_names,
            ..ExportOptions::default()
        };
        let files = export_bytes(&doc, ExportFormat::Glb, &options, "T", "t.glb").unwrap();
        let (json, _) = parts(&files.main);
        json
    };
    let json = export(true);
    let nodes = json["nodes"].as_array().unwrap();
    let names: Vec<&str> = nodes.iter().map(|n| n["name"].as_str().unwrap()).collect();
    // The shapes keep their own names; their parts sit beside them.
    assert!(
        names.contains(&"Arch") && names.contains(&"Cube"),
        "{names:?}"
    );
    let arch_parts: Vec<&&str> = names
        .iter()
        .filter(|n| n.starts_with("Arch-part") && n.ends_with("-convcolonly"))
        .collect();
    assert!((3..=8).contains(&arch_parts.len()), "{names:?}");
    assert!(names.contains(&"Arch-part1-convcolonly"));
    // A cube is convex: one part.
    assert!(names.contains(&"Cube-part1-convcolonly"));
    assert!(!names.contains(&"Cube-part2-convcolonly"));
    // Each part is a closed hull of at most 64 corners, at its object's
    // pivot.
    let arch = nodes.iter().find(|n| n["name"] == "Arch").unwrap();
    for node in nodes
        .iter()
        .filter(|n| n["name"].as_str().unwrap().contains("-part"))
    {
        if node["name"].as_str().unwrap().starts_with("Arch") {
            assert_eq!(node.get("matrix"), arch.get("matrix"));
        }
        let mesh = &json["meshes"][node["mesh"].as_u64().unwrap() as usize];
        let position = mesh["primitives"][0]["attributes"]["POSITION"]
            .as_u64()
            .unwrap() as usize;
        let count = json["accessors"][position]["count"].as_u64().unwrap();
        assert!((12..=64 * 6).contains(&count), "{count}");
    }
    // Not for other engines.
    let plain = export(false);
    let names: Vec<&str> = plain["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["name"].as_str().unwrap())
        .collect();
    assert_eq!(names.len(), 2, "{names:?}");
}

#[test]
fn the_summary_before_exporting_matches_the_export() {
    use crate::export::{ExportFormat, ExportWhat, export_bytes, summary};
    let mut doc = scene_with_materials();
    for what in [ExportWhat::Everything, ExportWhat::Selected] {
        let options = ExportOptions {
            what,
            ..ExportOptions::default()
        };
        let before = summary(&doc, &options);
        let made = export_bytes(&doc, ExportFormat::Glb, &options, "T", "a.glb")
            .unwrap()
            .summary;
        assert_eq!(before, made, "{what:?}");
    }
    // Two objects, Default and Paint used ("Unused" isn't), Paint once by
    // name.
    let all = summary(&doc, &ExportOptions::default());
    assert_eq!((all.objects, all.materials), (2, 2));
    // Nothing selected: nothing to export.
    doc.set_selection([]);
    let none = summary(
        &doc,
        &ExportOptions {
            what: ExportWhat::Selected,
            ..ExportOptions::default()
        },
    );
    assert_eq!(none, crate::export::ExportSummary::default());
}

/// Writes `.glb` files for checking by programs outside the app: the
/// Khronos glTF Validator and a Godot import, run by CI (the `gltf-check`
/// job in `.github/workflows/ci.yml`). Every primitive side by side, each
/// primitive on its own, an edited shape with materials, and a scene with
/// Godot's collision names:
/// `GLB_CHECK_OUT=folder cargo test -p fileio files_for_outside_checkers -- --ignored`.
/// Without `GLB_CHECK_OUT` it does nothing, so a plain `--ignored` run
/// writes no files.
#[test]
#[ignore = "writes files for CI's glTF Validator and Godot import"]
fn files_for_outside_checkers() {
    let Some(out) = std::env::var_os("GLB_CHECK_OUT").map(std::path::PathBuf::from) else {
        return;
    };
    std::fs::create_dir_all(&out).unwrap();
    let write = |name: &str, doc: &Document, options: &ExportOptions| {
        let files = export_bytes(doc, ExportFormat::Glb, options, "Test App 1.0", name).unwrap();
        assert!(files.beside.is_empty(), "a .glb is one file");
        std::fs::write(out.join(name), &files.main).unwrap();
    };
    let plain = ExportOptions::default();

    // Every primitive side by side, turned and stretched a little so the
    // nodes carry real matrices.
    let mut all = scene_of(&PrimitiveKind::ALL);
    let ids: Vec<_> = all.scene().walk().into_iter().map(|(id, _)| id).collect();
    let placed = ids
        .iter()
        .enumerate()
        .map(|(k, id)| {
            let (col, row) = ((k % 6) as f32, (k / 6) as f32);
            let transform = Transform {
                position: glam::Vec3::new(col * 200.0 - 500.0, row * 200.0 - 200.0, 0.0),
                rotation: glam::Vec3::new(k as f32 * 0.2, 0.0, 0.0),
                scale: glam::Vec3::new(1.0, 1.0 + k as f32 * 0.05, 1.0),
            };
            (*id, transform)
        })
        .collect();
    all.execute(Command::SetTransforms(placed)).unwrap();
    write("every-shape.glb", &all, &plain);

    // Each primitive on its own, so a problem names its shape.
    for kind in PrimitiveKind::ALL {
        let name = format!("shape-{}.glb", kind.name().to_lowercase().replace(' ', "-"));
        write(&name, &scene_of(&[kind]), &plain);
    }

    // Curves' solids (decision 0138): a pipe swept round a spiral, a
    // lathed vase from a part turn, and a line, which goes out as nothing.
    let mut curves = Document::new();
    for curve in [
        scene::CurveObject {
            shape: scene::CurveShape::Ready(scene::ReadyCurveKind::Spiral.standard()),
            solid: scene::CurveSolid::Sweep(scene::Sweep::default()),
        },
        scene::CurveObject {
            shape: scene::CurveShape::Ready(scene::ReadyCurve::Arc {
                radius: 40.0,
                angle: 120.0,
            }),
            solid: scene::CurveSolid::Lathe(scene::LatheShape {
                axis: scene::LatheAxis::X,
                angle: 270.0,
                ..scene::LatheShape::default()
            }),
        },
        scene::CurveObject::line(scene::CurveShape::Ready(
            scene::ReadyCurveKind::Circle.standard(),
        )),
    ] {
        curves
            .execute(Command::Create(scene::ObjectKind::Curve(curve)))
            .unwrap();
    }
    write("curves.glb", &curves, &plain);

    // An edited shape with materials: the cube's top pushed out, two
    // polygons painted, and a plane sharing a material's name.
    let mut edited = scene_with_materials();
    let cube = edited
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| edited.scene().object(*id).unwrap().name == "Cube")
        .unwrap();
    edited
        .execute(Command::Record {
            id: cube,
            operation: scene::Operation::ExtrudePolygons {
                faces: vec![mesh::FaceId::from_raw(4)],
                distance: 40.0,
                grouping: scene::Grouping::Together,
                shape: scene::ExtrudeShape::EARLIER,
                stretch: glam::Vec3::ONE,
                anchors: None,
            },
        })
        .unwrap();
    write("edited-with-materials.glb", &edited, &plain);

    // Godot's collision names and the box node, as the Godot preset
    // writes them.
    let mut godot = scene_of(&[
        PrimitiveKind::Cube,
        PrimitiveKind::Sphere,
        PrimitiveKind::Cylinder,
    ]);
    let ids: Vec<_> = godot.scene().walk().into_iter().map(|(id, _)| id).collect();
    for (id, collision) in ids.into_iter().zip([
        scene::Collision::Mesh,
        scene::Collision::Convex,
        scene::Collision::Box,
    ]) {
        godot
            .execute(Command::SetCollision {
                ids: vec![id],
                collision,
            })
            .unwrap();
    }
    let godot_options = ExportOptions {
        godot_names: true,
        ..ExportOptions::default()
    };
    write("godot-collision.glb", &godot, &godot_options);

    // Collision in parts (decision 0151): an arch and a ring, each part
    // a `-convcolonly` node beside them.
    let mut in_parts = scene_of(&[PrimitiveKind::Arch, PrimitiveKind::Torus]);
    let ids: Vec<_> = in_parts
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    in_parts
        .execute(Command::SetCollision {
            ids,
            collision: scene::Collision::Parts,
        })
        .unwrap();
    write("godot-collision-parts.glb", &in_parts, &godot_options);

    // Levels of detail (decision 0155): a sphere and a ring, each with
    // three levels beside it, hidden for Godot, with their distances.
    let mut levelled = scene_of(&[PrimitiveKind::Sphere, PrimitiveKind::Torus]);
    let ids: Vec<_> = levelled
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    levelled
        .execute(Command::SetLods {
            ids,
            lods: scene::Lods {
                on: true,
                ..scene::Lods::default()
            },
        })
        .unwrap();
    let lod_options = ExportOptions {
        lod_levels: true,
        ..godot_options
    };
    write("godot-lods.glb", &levelled, &lod_options);

    // Weighted normals (decision 0148): custom normals in NORMAL, and the
    // tangents made from them.
    let mut weighted = scene_of(&[PrimitiveKind::Cylinder, PrimitiveKind::Cube]);
    let ids: Vec<_> = weighted
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    for (id, keep_hard_edges) in ids.into_iter().zip([true, false]) {
        weighted
            .execute(Command::AddModifier {
                ids: vec![id],
                kind: scene::ModifierKind::WeightedNormals(scene::WeightedNormalsOptions {
                    keep_hard_edges,
                    ..Default::default()
                }),
            })
            .unwrap();
    }
    write("weighted-normals.glb", &weighted, &plain);

    // Live bevel (decision 0149), then Weighted normals: the game bevel's
    // order, a bevelled box with flat big faces.
    let mut bevelled = scene_of(&[PrimitiveKind::Cube, PrimitiveKind::Cylinder]);
    let ids: Vec<_> = bevelled
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    for kind in [
        scene::ModifierKind::WeightedNormals(Default::default()),
        scene::ModifierKind::LiveBevel(Default::default()),
    ] {
        bevelled
            .execute(Command::AddModifier {
                ids: ids.clone(),
                kind,
            })
            .unwrap();
    }
    write("live-bevel.glb", &bevelled, &plain);

    // The game bevel (decision 0156): a cube, a cylinder, an arch, a cube
    // with a round hole and a cylinder with a square one, side by side.
    let mut game = Document::new();
    let mut add = |kind: PrimitiveKind, position: glam::Vec3, scale: glam::Vec3| {
        game.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
            .unwrap();
        let id = game.selection()[0];
        let place = Transform {
            position,
            scale,
            ..Transform::default()
        };
        game.execute(Command::SetTransforms(vec![(id, place)]))
            .unwrap();
        id
    };
    let one = glam::Vec3::ONE;
    let narrow = glam::Vec3::new(0.5, 0.5, 1.5);
    let at = |x: f32, z: f32| glam::Vec3::new(x, 0.0, z);
    let mut shapes = vec![
        add(PrimitiveKind::Cube, at(-400.0, 0.0), one),
        add(PrimitiveKind::Cylinder, at(-200.0, 0.0), one),
        add(PrimitiveKind::Arch, at(0.0, 0.0), one),
    ];
    let cut = [
        (PrimitiveKind::Cube, PrimitiveKind::Cylinder, 200.0),
        (PrimitiveKind::Cylinder, PrimitiveKind::Cube, 400.0),
    ];
    let mut pairs = Vec::new();
    for (kept, cutter, x) in cut {
        let kept = add(kept, at(x, 0.0), one);
        let cutter = add(cutter, at(x, -25.0), narrow);
        pairs.push((kept, cutter));
        shapes.push(kept);
    }
    for (kept, cutter) in pairs {
        game.execute(Command::AddBoolean {
            kept,
            cutters: vec![cutter],
            kind: scene::BooleanKind::Subtract,
        })
        .unwrap();
    }
    game.game_bevel(&shapes, "Game bevel", 1).unwrap();
    write("game-bevel.glb", &game, &plain);
}
