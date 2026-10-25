//! Tests for custom normals going out (decision 0148): a bevelled cube
//! with the Weighted normals modifier, read back from a `.glb` and an
//! `.obj` the way an engine or another program reads them.

use super::*;
use glam::Vec3;
use scene::{Command, ModifierKind, ObjectKind, WeightedNormalsOptions};
use std::sync::Arc;

/// A 100 cm cube with every edge bevelled 5 cm in two segments, every edge
/// soft, shaded by its marks; with Weighted normals when `weighted`.
fn bevelled_cube_document(weighted: bool) -> Document {
    let mut cube = mesh::Primitive::Cube(Default::default()).mesh();
    let edges: Vec<mesh::EdgeKey> = cube.edges().to_vec();
    mesh::bevel::bevel_edges(
        &mut cube,
        &edges,
        scene::BevelShape {
            width: 5.0,
            segments: 2,
            profile: 1.0,
            options: scene::BevelOptions::default(),
        },
    );
    let all: Vec<mesh::EdgeKey> = cube.edges().to_vec();
    for e in all {
        let (a, b) = e.ends();
        cube.set_hard(a, b, false);
    }
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Mesh(Arc::new(cube))))
        .unwrap();
    let id = doc.selection()[0];
    doc.execute(Command::SetHardness {
        ids: vec![id],
        hardness: Hardness::Marked,
    })
    .unwrap();
    if weighted {
        doc.execute(Command::AddModifier {
            ids: vec![id],
            kind: ModifierKind::WeightedNormals(WeightedNormalsOptions::default()),
        })
        .unwrap();
    }
    doc
}

/// The JSON and binary parts of a `.glb`.
pub(super) fn parts(glb: &[u8]) -> (serde_json::Value, Vec<u8>) {
    let word = |at: usize| u32::from_le_bytes(glb[at..at + 4].try_into().unwrap()) as usize;
    let json_len = word(12);
    let json = serde_json::from_slice(&glb[20..20 + json_len]).unwrap();
    let at = 20 + json_len;
    let bin_len = word(at);
    (json, glb[at + 8..at + 8 + bin_len].to_vec())
}

/// An accessor's bytes.
fn bytes<'a>(json: &serde_json::Value, bin: &'a [u8], accessor: usize) -> &'a [u8] {
    let a = &json["accessors"][accessor];
    let view = &json["bufferViews"][a["bufferView"].as_u64().unwrap() as usize];
    let offset = view["byteOffset"].as_u64().unwrap() as usize;
    let length = view["byteLength"].as_u64().unwrap() as usize;
    &bin[offset..offset + length]
}

pub(super) fn vec3s(json: &serde_json::Value, bin: &[u8], accessor: usize) -> Vec<Vec3> {
    bytes(json, bin, accessor)
        .chunks(12)
        .map(|c| {
            let f = |i: usize| f32::from_le_bytes(c[i..i + 4].try_into().unwrap());
            Vec3::new(f(0), f(4), f(8))
        })
        .collect()
}

pub(super) fn vec4s(json: &serde_json::Value, bin: &[u8], accessor: usize) -> Vec<glam::Vec4> {
    bytes(json, bin, accessor)
        .chunks(16)
        .map(|c| {
            let f = |i: usize| f32::from_le_bytes(c[i..i + 4].try_into().unwrap());
            glam::Vec4::new(f(0), f(4), f(8), f(12))
        })
        .collect()
}

pub(super) fn indices(json: &serde_json::Value, bin: &[u8], accessor: usize) -> Vec<usize> {
    let raw = bytes(json, bin, accessor);
    match json["accessors"][accessor]["componentType"]
        .as_u64()
        .unwrap()
    {
        5123 => raw
            .chunks(2)
            .map(|c| u16::from_le_bytes(c.try_into().unwrap()) as usize)
            .collect(),
        5125 => raw
            .chunks(4)
            .map(|c| u32::from_le_bytes(c.try_into().unwrap()) as usize)
            .collect(),
        other => panic!("indices of type {other}"),
    }
}

/// The largest angle, in degrees, between a big triangle's (the cube's
/// big faces, in square metres) vertex normals and its facing, in a
/// `.glb`; and how many big triangles there are.
fn glb_lean(glb: &[u8]) -> (f32, usize) {
    let (json, bin) = parts(glb);
    let primitive = &json["meshes"][0]["primitives"][0];
    let attributes = &primitive["attributes"];
    let positions = vec3s(
        &json,
        &bin,
        attributes["POSITION"].as_u64().unwrap() as usize,
    );
    let normals = vec3s(&json, &bin, attributes["NORMAL"].as_u64().unwrap() as usize);
    assert!(attributes.get("TANGENT").is_none() || attributes["TANGENT"].is_u64());
    let list = indices(&json, &bin, primitive["indices"].as_u64().unwrap() as usize);
    let (mut worst, mut big) = (0.0f32, 0);
    for t in list.chunks(3) {
        let [a, b, c] = [positions[t[0]], positions[t[1]], positions[t[2]]];
        let cross = (b - a).cross(c - a);
        if cross.length() / 2.0 < 0.2 {
            continue;
        }
        big += 1;
        let facing = cross.normalize();
        for i in t {
            assert!((normals[*i].length() - 1.0).abs() < 1e-4);
            worst = worst.max(normals[*i].angle_between(facing).to_degrees());
        }
    }
    (worst, big)
}

pub(super) fn glb(doc: &Document) -> Vec<u8> {
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
fn a_glb_carries_the_weighted_normals() {
    let (plain, plain_big) = glb_lean(&glb(&bevelled_cube_document(false)));
    let (weighted, big) = glb_lean(&glb(&bevelled_cube_document(true)));
    // Two triangles per big face.
    assert_eq!((plain_big, big), (12, 12));
    assert!(plain > 5.0, "the usual shading leans: {plain}");
    assert!(
        weighted < 0.05,
        "weighted, the big faces are flat: {weighted}"
    );
}

#[test]
fn tangents_follow_the_weighted_normals() {
    // MikkTSpace makes each tangent square to its own (custom) normal.
    // Where a polygon's UVs have no area (a bevel's strips and corners
    // carry their edge's UVs) it can't, with or without weighted normals:
    // no more such places.
    let crooked = |weighted: bool| {
        let doc = bevelled_cube_document(weighted);
        let objects = prepare(&doc, &ExportFormat::Glb.options(&ExportOptions::default()));
        let v = crate::gltf::vertices(&objects[0].local);
        let tangents = v.tangents.expect("the cube has UVs");
        let crooked = tangents
            .iter()
            .zip(&v.normals)
            .filter(|(t, n)| t.truncate().dot(**n).abs() > 1e-3)
            .count();
        (crooked, tangents.len())
    };
    let (plain, _) = crooked(false);
    let (weighted, all) = crooked(true);
    assert!(
        weighted <= plain && weighted < all,
        "{weighted} {plain} {all}"
    );
}

/// The largest angle, in degrees, between a big polygon's corner normals
/// and its facing, in an `.obj`'s text (positions and normals read back).
fn obj_lean(text: &str) -> f32 {
    let mut points: Vec<Vec3> = Vec::new();
    let mut normals: Vec<Vec3> = Vec::new();
    let mut worst = 0.0f32;
    let three = |rest: &[&str]| {
        Vec3::new(
            rest[0].parse().unwrap(),
            rest[1].parse().unwrap(),
            rest[2].parse().unwrap(),
        )
    };
    for line in text.lines() {
        let words: Vec<&str> = line.split_whitespace().collect();
        match words.first() {
            Some(&"v") => points.push(three(&words[1..])),
            Some(&"vn") => normals.push(three(&words[1..])),
            Some(&"f") => {
                let corners: Vec<(Vec3, Vec3)> = words[1..]
                    .iter()
                    .map(|c| {
                        let parts: Vec<&str> = c.split('/').collect();
                        let p: usize = parts[0].parse().unwrap();
                        let n: usize = parts[2].parse().unwrap();
                        (points[p - 1], normals[n - 1])
                    })
                    .collect();
                let mut newell = Vec3::ZERO;
                for (i, (p, _)) in corners.iter().enumerate() {
                    let q = corners[(i + 1) % corners.len()].0;
                    newell += p.cross(q);
                }
                if newell.length() / 2.0 < 0.5 {
                    continue;
                }
                let facing = newell.normalize();
                for (_, n) in &corners {
                    worst = worst.max(n.normalize().angle_between(facing).to_degrees());
                }
            }
            _ => {}
        }
    }
    worst
}

#[test]
fn an_obj_carries_the_weighted_normals() {
    let obj = |doc: &Document| {
        let files = export_bytes(
            doc,
            ExportFormat::Obj,
            &ExportOptions::default(),
            "Test App 1.0",
            "test.obj",
        )
        .unwrap();
        String::from_utf8(files.main).unwrap()
    };
    assert!(obj_lean(&obj(&bevelled_cube_document(false))) > 5.0);
    assert!(obj_lean(&obj(&bevelled_cube_document(true))) < 0.05);
}

#[test]
fn baked_custom_normals_go_out_as_the_modifier_did() {
    let live = bevelled_cube_document(true);
    let mut baked = live.clone();
    let id = baked.scene().walk()[0].0;
    baked.execute(Command::MakeEditable(vec![id])).unwrap();
    assert!(baked.scene().modifiers(id).is_empty());
    assert_eq!(glb(&live), glb(&baked));
}

#[test]
fn a_modifier_after_weighted_normals_sends_the_usual_shading() {
    let mut doc = bevelled_cube_document(true);
    let id = doc.scene().walk()[0].0;
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind: ModifierKind::Decimate(scene::DecimateOptions {
            ratio: 1.0,
            ..Default::default()
        }),
    })
    .unwrap();
    assert_eq!(glb(&doc), glb(&bevelled_cube_document(false)));
}

/// A standard cube (shaded by its marks, every edge softened, as the game
/// bevel will) with Weighted normals and then a standard Live bevel
/// (decision 0149), which goes before it.
fn live_bevelled_cube_document() -> Document {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(
        scene::PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let id = doc.selection()[0];
    doc.execute(Command::SetHardness {
        ids: vec![id],
        hardness: Hardness::Marked,
    })
    .unwrap();
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    doc.execute(Command::Record {
        id,
        operation: scene::Operation::SetHardEdges {
            edges: shape.edges().to_vec(),
            hard: false,
            anchors: None,
        },
    })
    .unwrap();
    for kind in [
        ModifierKind::WeightedNormals(WeightedNormalsOptions::default()),
        ModifierKind::LiveBevel(Default::default()),
    ] {
        doc.execute(Command::AddModifier {
            ids: vec![id],
            kind,
        })
        .unwrap();
    }
    doc
}

#[test]
fn a_live_bevel_then_weighted_normals_exports_flat_big_faces() {
    let doc = live_bevelled_cube_document();
    let id = doc.scene().walk()[0].0;
    assert_eq!(doc.scene().modifiers(id)[0].kind.name(), "Live bevel");
    let (lean, big) = glb_lean(&glb(&doc));
    assert_eq!(big, 12);
    assert!(lean < 0.05, "the big faces are flat: {lean}");
    // Switch Weighted normals off: they lean again.
    let mut off = doc.scene().modifiers(id)[1].clone();
    off.on = false;
    let mut doc = doc;
    doc.execute(Command::ChangeModifier {
        id,
        index: 1,
        modifier: off,
    })
    .unwrap();
    let (lean, _) = glb_lean(&glb(&doc));
    assert!(lean > 1.0, "the usual shading leans: {lean}");
}
