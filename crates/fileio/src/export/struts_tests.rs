//! Tests for struts going out (decision 0159): read back from a `.glb`
//! as an engine reads it, a cube with struts is its frame (in metres, Y
//! up), with UVs inside the texture square, and baking gives the same
//! file.

use super::normals_tests::{glb, parts, vec3s};
use super::*;
use scene::{Command, ModifierKind, ObjectKind, Primitive, StrutsOptions};

/// A document with a 1 m cube turned into struts.
fn frame() -> Document {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(Primitive::Cube(
        Default::default(),
    ))))
    .unwrap();
    let id = doc.selection()[0];
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind: ModifierKind::Struts(StrutsOptions::default()),
    })
    .unwrap();
    doc
}

#[test]
fn a_cube_with_struts_goes_out_as_its_frame() {
    let doc = frame();
    let live = glb(&doc);
    let (json, bin) = parts(&live);
    let attributes = &json["meshes"][0]["primitives"][0]["attributes"];
    let positions = vec3s(
        &json,
        &bin,
        attributes["POSITION"].as_u64().unwrap() as usize,
    );
    // The joints stick out √2 cm past the cube's corners.
    let most_x = positions.iter().map(|p| p.x).fold(f32::MIN, f32::max);
    assert!(
        (most_x - (0.5 + 0.02_f32.sqrt() / 10.0)).abs() < 1e-4,
        "{most_x}"
    );
    // Something sits in the middle of each side's edge, nothing in the
    // middle of a side.
    assert!(
        positions
            .iter()
            .all(|p| p.x.abs() > 0.48 || p.y.abs() > 0.48 || p.z.abs() > 0.48)
    );
    let uv = attributes["TEXCOORD_0"].as_u64().unwrap() as usize;
    let view = &json["bufferViews"][json["accessors"][uv]["bufferView"].as_u64().unwrap() as usize];
    let start = view["byteOffset"].as_u64().unwrap_or(0) as usize;
    let length = view["byteLength"].as_u64().unwrap() as usize;
    for bytes in bin[start..start + length].chunks_exact(4) {
        let value = f32::from_le_bytes(bytes.try_into().unwrap());
        assert!((0.0..=1.0).contains(&value), "{value}");
    }
    // Baked, the file is the same.
    let mut baked = doc.clone();
    let id = baked.scene().walk()[0].0;
    baked.execute(Command::MakeEditable(vec![id])).unwrap();
    assert_eq!(glb(&baked), live);
}
