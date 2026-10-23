//! Tests for version 40: the development format with live booleans
//! (decision 0135).

use super::tests::{assert_same, header, refused, rich_document_v39};
use super::*;
use glam::Vec3;
use scene::{Boolean, BooleanKind, Command, ObjectId, ObjectKind, PrimitiveKind, Transform};

/// The id of the object called `name`.
fn named(doc: &Document, name: &str) -> ObjectId {
    doc.scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == name)
        .unwrap()
}

/// Everything version 39 stores, plus two cutters: the cube is cut by
/// "Cutter" and joined with "Cutter 2" (the version 40 sample holds
/// exactly this).
pub(super) fn rich_document_v40() -> Document {
    let mut doc = rich_document_v39();
    let cube = named(&doc, "Cube");
    let mut cutters = Vec::new();
    for (name, x) in [("Cutter", 50.0), ("Cutter 2", -80.0)] {
        doc.execute(Command::Create(ObjectKind::Primitive(
            PrimitiveKind::Cube.standard(),
        )))
        .unwrap();
        let id = doc.selection()[0];
        doc.execute(Command::Rename {
            id,
            name: name.into(),
        })
        .unwrap();
        doc.execute(Command::SetTransforms(vec![(
            id,
            Transform {
                position: Vec3::new(x, 50.0, 50.0),
                ..Transform::default()
            },
        )]))
        .unwrap();
        cutters.push(id);
    }
    doc.execute(Command::AddBoolean {
        kept: cube,
        cutters: vec![cutters[0]],
        kind: BooleanKind::Subtract,
    })
    .unwrap();
    doc.execute(Command::AddBoolean {
        kept: cube,
        cutters: vec![cutters[1]],
        kind: BooleanKind::Union,
    })
    .unwrap();
    doc
}

/// Each object's booleans, by name.
fn booleans_of(doc: &Document) -> Vec<(String, Vec<Boolean>)> {
    let scene = doc.scene();
    scene
        .walk()
        .into_iter()
        .map(|(id, _)| {
            (
                scene.object(id).unwrap().name.clone(),
                scene.booleans(id).to_vec(),
            )
        })
        .collect()
}

#[test]
fn booleans_are_saved_and_opened_in_order() {
    let doc = rich_document_v40();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_same(&doc, &back);
    assert_eq!(booleans_of(&doc), booleans_of(&back));
    let cube = named(&back, "Cube");
    let list = back.scene().booleans(cube);
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].kind, BooleanKind::Subtract);
    assert_eq!(list[1].kind, BooleanKind::Union);
    assert_eq!(list[0].cutters, [named(&back, "Cutter")]);
    assert_eq!(
        back.scene().shown_shape(cube),
        doc.scene().shown_shape(cube),
        "the cut comes out the same"
    );
}

const SAMPLE_V40: &[u8] = include_bytes!("../../tests/data/sample-v40.avtl");

#[test]
fn a_file_saved_by_format_version_40_opens_with_its_booleans() {
    let doc = from_bytes(SAMPLE_V40).expect("the version 40 sample opens");
    let expected = rich_document_v40();
    assert_same(&expected, &doc);
    assert_eq!(booleans_of(&expected), booleans_of(&doc));
}

#[test]
fn older_development_files_open_without_booleans() {
    let doc = rich_document_v39();
    let body = v39::encode(&v39::ProjectV39::from_document(&doc)).unwrap();
    let mut bytes = header(39, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    let back = from_bytes(&bytes).unwrap();
    assert_same(&doc, &back);
    assert!(booleans_of(&back).iter().all(|(_, b)| b.is_empty()));
}

/// A version 40 file holding `model`, uncompressed.
fn file(model: &v40::ProjectV40) -> Vec<u8> {
    let body = v40::encode(model).unwrap();
    let mut bytes = header(40, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    bytes
}

#[test]
fn bad_booleans_are_refused_in_plain_words() {
    let doc = rich_document_v40();
    let cube = named(&doc, "Cube").get();
    let cutter = named(&doc, "Cutter").get();
    let plain = || v40::ProjectV40::from_document(&doc);
    // Listed twice.
    let mut model = plain();
    let again = model.booleans[0].clone();
    model.booleans.push(again);
    assert!(refused(&file(&model)).to_string().contains("twice"));
    // An object that isn't there, as the kept object or a cutter.
    let mut model = plain();
    model.booleans[0].id = 999_999;
    assert!(refused(&file(&model)).to_string().contains("isn't there"));
    let mut model = plain();
    model.booleans[0].list[0].cutters[0] = 999_999;
    assert!(refused(&file(&model)).to_string().contains("boolean"));
    // Cutting itself, or a loop.
    let mut model = plain();
    model.booleans[0].list[0].cutters[0] = cube;
    assert!(refused(&file(&model)).to_string().contains("itself"));
    let mut model = plain();
    model.booleans.push(v40::BooleansV40 {
        id: cutter,
        list: vec![v40::BooleanV40 {
            kind: v40::KindV40::Union,
            cutters: vec![cube].into(),
        }]
        .into(),
    });
    assert!(refused(&file(&model)).to_string().contains("wait"));
    // A boolean with no cutters.
    let mut model = plain();
    model.booleans[0].list[0].cutters = Default::default();
    assert!(refused(&file(&model)).to_string().contains("cutter"));
}

#[test]
fn random_version_40_bodies_never_panic() {
    let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
    let good = v40::encode(&v40::ProjectV40::from_document(&rich_document_v40())).unwrap();
    for round in 0..300 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut body = good.clone();
        let at = (seed as usize) % body.len();
        body[at] ^= (seed >> 32) as u8 | 1;
        if round % 3 == 0 {
            body.truncate(at);
        }
        let mut bytes = header(40, 0, body.len() as u64);
        bytes.extend_from_slice(&body);
        let _ = from_bytes(&bytes);
    }
}
