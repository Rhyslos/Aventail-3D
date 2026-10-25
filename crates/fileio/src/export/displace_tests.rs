//! Tests for a displaced shape going out (decision 0157): read back from
//! a `.glb` as an engine reads it, the points are pushed as shown while
//! the UVs stay those of the plain shape, so a texture painted on it
//! still fits; baking gives the same file. (The vertex count may grow:
//! smoothed by angle, a bump sharper than the angle shades as a crease,
//! and a crease splits vertices.)

use super::normals_tests::{glb, parts, vec3s};
use super::*;
use scene::{
    Command, DisplaceOptions, DisplaceSettings, ModifierKind, ObjectId, ObjectKind, PrimitiveKind,
};
use std::collections::BTreeSet;

/// A document with one standard sphere, and its id.
fn sphere() -> (Document, ObjectId) {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Sphere.standard(),
    )))
    .unwrap();
    let id = doc.selection()[0];
    (doc, id)
}

/// The first primitive's positions, and its distinct UVs (as bits, in
/// order).
fn read(glb: &[u8]) -> (Vec<Vec3>, BTreeSet<[u32; 2]>) {
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
    let uvs = bin[start..start + length]
        .chunks(8)
        .map(|c| {
            let f = |i: usize| u32::from_le_bytes(c[i..i + 4].try_into().unwrap());
            [f(0), f(4)]
        })
        .collect();
    (positions, uvs)
}

/// The point nearest `p` among `points`.
fn nearest(points: &[Vec3], p: Vec3) -> Vec3 {
    points
        .iter()
        .copied()
        .min_by(|a, b| a.distance(p).total_cmp(&b.distance(p)))
        .unwrap()
}

#[test]
fn a_displaced_sphere_goes_out_bumpy_with_its_uvs_kept() {
    let (plain_doc, _) = sphere();
    let (plain, plain_uvs) = read(&glb(&plain_doc));
    let (mut doc, id) = sphere();
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind: ModifierKind::Displace(DisplaceSettings::default()),
    })
    .unwrap();
    let live = glb(&doc);
    let (bumpy, uvs) = read(&live);
    assert_eq!(uvs, plain_uvs, "the UVs are those of the plain sphere");
    // In metres: every point is pushed, but no further than the strength.
    let most = DisplaceOptions::STANDARD_STRENGTH / 100.0;
    let mut moved = 0;
    for p in &bumpy {
        let d = nearest(&plain, *p).distance(*p);
        assert!(d <= most + 1e-4, "{d}");
        moved += usize::from(d > 1e-4);
    }
    assert!(moved > bumpy.len() / 2, "{moved} of {}", bumpy.len());
    // Baked, the file is the same.
    let mut baked = doc.clone();
    baked.execute(Command::MakeEditable(vec![id])).unwrap();
    assert_eq!(glb(&baked), live);
}
