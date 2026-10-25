//! Tests for a bent shape going out (decision 0158): read back from a
//! `.glb` as an engine reads it, the shape is bent as shown (in metres,
//! Y up), its UVs are those of the plain shape, so a texture painted on
//! it still fits, and baking gives the same file.

use super::normals_tests::{glb, parts, vec3s};
use super::*;
use scene::{AxisDeformOptions, Command, ModifierKind, ObjectKind, Primitive};

/// A document with a pole 1 m tall cut into 10 slices, bent 90° if
/// asked.
fn pole(bent: bool) -> Document {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(Primitive::Cylinder(
        mesh::primitives::CylinderParams {
            radius: 5.0,
            height: 100.0,
            height_segments: 10,
            ..Default::default()
        },
    ))))
    .unwrap();
    if bent {
        let id = doc.selection()[0];
        doc.execute(Command::AddModifier {
            ids: vec![id],
            kind: ModifierKind::BendTwistTaper(AxisDeformOptions {
                angle: 90.0,
                ..AxisDeformOptions::default()
            }),
        })
        .unwrap();
    }
    doc
}

/// The first primitive's positions, and its UV bytes.
fn read(glb: &[u8]) -> (Vec<Vec3>, Vec<u8>) {
    let (json, bin) = parts(glb);
    let attributes = &json["meshes"][0]["primitives"][0]["attributes"];
    let positions = vec3s(
        &json,
        &bin,
        attributes["POSITION"].as_u64().unwrap() as usize,
    );
    let uv = attributes["TEXCOORD_0"].as_u64().unwrap() as usize;
    let view = &json["bufferViews"][json["accessors"][uv]["bufferView"].as_u64().unwrap() as usize];
    let start = view["byteOffset"].as_u64().unwrap_or(0) as usize;
    let length = view["byteLength"].as_u64().unwrap() as usize;
    (positions, bin[start..start + length].to_vec())
}

#[test]
fn a_bent_pole_goes_out_bent_with_its_uvs_kept() {
    let (plain, plain_uvs) = read(&glb(&pole(false)));
    let doc = pole(true);
    let live = glb(&doc);
    let (bent, uvs) = read(&live);
    assert_eq!(bent.len(), plain.len());
    assert_eq!(uvs, plain_uvs, "the UVs are those of the plain pole");
    // Its top lies on its side: about 2/π m along X, no higher than that
    // (glTF's up is Y).
    let radius = 2.0 / std::f32::consts::PI;
    let most_x = bent.iter().map(|p| p.x).fold(f32::MIN, f32::max);
    let most_up = bent.iter().map(|p| p.y).fold(f32::MIN, f32::max);
    assert!((most_x - radius).abs() < 0.01, "{most_x}");
    assert!((most_up - (radius + 0.05)).abs() < 0.01, "{most_up}");
    // Baked, the file is the same.
    let mut baked = doc.clone();
    let id = baked.scene().walk()[0].0;
    baked.execute(Command::MakeEditable(vec![id])).unwrap();
    assert_eq!(glb(&baked), live);
}
