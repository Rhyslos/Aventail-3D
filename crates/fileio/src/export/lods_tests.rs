//! Tests for levels of detail in exports (decision 0155).

use super::*;
use scene::{Command, Lods, ObjectKind, PrimitiveKind};
use serde_json::Value as Json;

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

fn lods_on(doc: &mut Document, ids: Vec<ObjectId>) {
    doc.execute(Command::SetLods {
        ids,
        lods: Lods {
            on: true,
            ..Lods::default()
        },
    })
    .unwrap();
}

/// A sphere with the standard levels and a cube without any.
fn scene() -> Document {
    let mut doc = Document::new();
    let sphere = create(&mut doc, PrimitiveKind::Sphere);
    create(&mut doc, PrimitiveKind::Cube);
    lods_on(&mut doc, vec![sphere]);
    doc
}

fn named(doc: &Document, name: &str) -> ObjectId {
    doc.scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == name)
        .unwrap()
}

fn with_levels(godot_names: bool) -> ExportOptions {
    ExportFormat::Glb.options(&ExportOptions {
        lod_levels: true,
        godot_names,
        ..ExportOptions::default()
    })
}

/// The JSON part of a `.glb`.
fn json_of(glb: &[u8]) -> Json {
    let length = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
    serde_json::from_slice(&glb[20..20 + length]).unwrap()
}

fn node<'a>(json: &'a Json, name: &str) -> &'a Json {
    json["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["name"] == name)
        .unwrap_or_else(|| panic!("no node {name}"))
}

#[test]
fn levels_go_out_only_when_asked_and_only_for_objects_with_them() {
    let doc = scene();
    let plain = prepare(&doc, &ExportFormat::Glb.options(&ExportOptions::default()));
    assert!(plain.iter().all(|o| o.levels.is_empty()));
    let objects = prepare(&doc, &with_levels(false));
    let sphere = objects.iter().find(|o| o.name == "Sphere").unwrap();
    let cube = objects.iter().find(|o| o.name == "Cube").unwrap();
    assert!(cube.levels.is_empty());
    assert_eq!(sphere.levels.len(), 3);
    let counts: Vec<usize> = sphere
        .levels
        .iter()
        .map(|l| l.local.triangle_count())
        .collect();
    assert!(counts[0] < sphere.local.triangle_count());
    assert!(counts.windows(2).all(|w| w[1] < w[0]), "{counts:?}");
    // In the file's units: 10 m, 20 m, 40 m.
    let distances: Vec<f32> = sphere.levels.iter().map(|l| l.distance).collect();
    assert_eq!(distances, [10.0, 20.0, 40.0]);
    // The summary counts the objects themselves.
    let files = write_files(
        &objects,
        ExportFormat::Glb,
        &with_levels(false),
        "T",
        "t.glb",
    )
    .unwrap();
    assert_eq!(files.summary.objects, 2);
}

#[test]
fn each_level_is_a_node_of_its_own_with_its_distances() {
    let doc = scene();
    let options = with_levels(false);
    let files = export_bytes(&doc, ExportFormat::Glb, &options, "T", "t.glb").unwrap();
    let json = json_of(&files.main);
    let mut names: Vec<&str> = json["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["name"].as_str().unwrap())
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "Cube",
            "Sphere",
            "Sphere_LOD1",
            "Sphere_LOD2",
            "Sphere_LOD3"
        ]
    );
    let extras = |name: &str| node(&json, name)["extras"].clone();
    assert_eq!(extras("Cube"), Json::Null, "no levels, no notes");
    assert_eq!(extras("Sphere")["lod_level"], 0);
    assert_eq!(extras("Sphere")["lod_begin"], 0.0);
    assert_eq!(extras("Sphere")["lod_end"], 10.0);
    assert_eq!(extras("Sphere_LOD1")["lod_begin"], 10.0);
    assert_eq!(extras("Sphere_LOD1")["lod_end"], 20.0);
    assert_eq!(extras("Sphere_LOD3")["lod_level"], 3);
    assert_eq!(extras("Sphere_LOD3")["lod_end"], 0.0, "no end");
    // Seen by other engines: nothing hidden.
    assert!(json.get("extensionsUsed").is_none());
    assert!(node(&json, "Sphere_LOD1").get("extensions").is_none());
    // At the same pivot as the sphere.
    assert_eq!(
        node(&json, "Sphere_LOD2")["matrix"],
        node(&json, "Sphere")["matrix"]
    );
}

#[test]
fn for_godot_the_levels_are_hidden_and_the_object_keeps_its_collision_name() {
    let mut doc = scene();
    let sphere = named(&doc, "Sphere");
    doc.execute(Command::SetCollision {
        ids: vec![sphere],
        collision: scene::Collision::Convex,
    })
    .unwrap();
    let files = export_bytes(&doc, ExportFormat::Glb, &with_levels(true), "T", "t.glb").unwrap();
    let json = json_of(&files.main);
    assert_eq!(
        json["extensionsUsed"],
        serde_json::json!(["KHR_node_visibility"])
    );
    assert!(json.get("extensionsRequired").is_none());
    let full = node(&json, "Sphere-convcol");
    assert!(full.get("extensions").is_none(), "the object itself shows");
    assert_eq!(full["extras"]["lod_level"], 0);
    for level in 1..=3 {
        let n = node(&json, &format!("Sphere_LOD{level}"));
        assert_eq!(
            n["extensions"]["KHR_node_visibility"]["visible"],
            Json::Bool(false)
        );
    }
}

#[test]
fn an_obj_lists_each_level_as_an_object() {
    let doc = scene();
    let options = ExportOptions {
        lod_levels: true,
        ..ExportOptions::default()
    };
    let files = export_bytes(&doc, ExportFormat::Obj, &options, "T", "t.obj").unwrap();
    let text = String::from_utf8(files.main).unwrap();
    let objects: Vec<&str> = text.lines().filter(|l| l.starts_with("o ")).collect();
    assert_eq!(
        objects,
        [
            "o Cube",
            "o Sphere",
            "o Sphere_LOD1",
            "o Sphere_LOD2",
            "o Sphere_LOD3"
        ]
    );
}

#[test]
fn a_kit_piece_carries_its_levels_at_its_pivot() {
    let doc = scene();
    let sphere = named(&doc, "Sphere");
    let piece = prepare_piece(&doc, sphere, &with_levels(true), Frames::Both);
    assert_eq!(piece.len(), 1);
    assert_eq!(piece[0].levels.len(), 3);
    let objects = level_objects(&piece);
    let names: Vec<&str> = objects.iter().map(|(o, _)| o.name.as_str()).collect();
    assert_eq!(names, ["Sphere_LOD1", "Sphere_LOD2", "Sphere_LOD3"]);
    assert!(objects.iter().all(|(o, _)| o.pivot == piece[0].pivot));
}

#[test]
fn levels_follow_the_exported_shape_and_its_materials() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    doc.execute(Command::AddModifier {
        ids: vec![cube],
        kind: scene::ModifierKind::Subdivide(scene::SubdivideOptions::default()),
    })
    .unwrap();
    lods_on(&mut doc, vec![cube]);
    let objects = prepare(&doc, &with_levels(false));
    let full = objects[0].local.triangle_count();
    assert!(full > 12, "smoothed for export");
    let first = objects[0].levels[0].local.triangle_count();
    assert!(
        first <= full / 2 + 2 && first >= full / 4,
        "{full} → {first}"
    );
    for level in &objects[0].levels {
        assert!(level.local.polygons.iter().all(|p| p.corners.len() == 3));
        assert!(
            level
                .local
                .polygons
                .iter()
                .all(|p| (p.material as usize) < objects[0].materials.len())
        );
    }
}
