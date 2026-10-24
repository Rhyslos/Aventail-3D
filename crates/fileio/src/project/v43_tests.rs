//! Tests for version 43: version 41's model with the modifier stack
//! (decision 0142).

use super::tests::{assert_same, header};
use super::v42_tests::rich_document_v42;
use super::*;
use scene::{Command, Modifier, ModifierKind, ObjectId, ObjectKind, PrimitiveKind, RemeshOptions};

/// Every object's modifiers in `doc`, in tree order.
fn modifiers(doc: &Document) -> Vec<(ObjectId, Vec<Modifier>)> {
    doc.scene()
        .walk()
        .iter()
        .map(|(id, _)| (*id, doc.scene().modifiers(*id).to_vec()))
        .filter(|(_, list)| !list.is_empty())
        .collect()
}

/// Everything version 42 stores, plus a sphere with two remesh modifiers
/// (the second switched off) and a cube whose remesh is hidden in the
/// viewport: the version 43 sample holds exactly this.
pub(super) fn rich_document_v43() -> Document {
    let mut doc = rich_document_v42();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Sphere.standard(),
    )))
    .unwrap();
    let sphere = doc.selection()[0];
    for (voxel_size, smooth) in [(4.0, 3), (12.5, 0)] {
        doc.execute(Command::AddModifier {
            ids: vec![sphere],
            kind: ModifierKind::Remesh(RemeshOptions { voxel_size, smooth }),
        })
        .unwrap();
    }
    let mut off = doc.scene().modifiers(sphere)[1].clone();
    off.on = false;
    doc.execute(Command::ChangeModifier {
        id: sphere,
        index: 1,
        modifier: off,
    })
    .unwrap();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let cube = doc.selection()[0];
    doc.execute(Command::AddModifier {
        ids: vec![cube],
        kind: ModifierKind::Remesh(RemeshOptions::default()),
    })
    .unwrap();
    let mut hidden = doc.scene().modifiers(cube)[0].clone();
    hidden.in_viewport = false;
    doc.execute(Command::ChangeModifier {
        id: cube,
        index: 0,
        modifier: hidden,
    })
    .unwrap();
    doc
}

#[test]
fn modifiers_survive_a_round_trip() {
    let doc = rich_document_v43();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], 43u32.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_same(&doc, &back);
    assert_eq!(modifiers(&doc), modifiers(&back));
    assert_eq!(modifiers(&back).len(), 2);
    let (sphere, _) = modifiers(&back)[0].clone();
    assert_eq!(
        back.scene().shown_shape(sphere),
        doc.scene().shown_shape(sphere),
        "the remeshed sphere comes out the same"
    );
}

const SAMPLE_V43: &[u8] = include_bytes!("../../tests/data/sample-v43.avtl");

#[test]
fn a_file_saved_by_format_version_43_opens_with_its_modifiers() {
    let doc = from_bytes(SAMPLE_V43).expect("the version 43 sample opens");
    let expected = rich_document_v43();
    assert_same(&expected, &doc);
    assert_eq!(modifiers(&expected), modifiers(&doc));
}

const SAMPLE_V42: &[u8] = include_bytes!("../../tests/data/sample-v42.avtl");

#[test]
fn older_files_open_with_no_modifiers() {
    let doc = from_bytes(SAMPLE_V42).unwrap();
    assert!(modifiers(&doc).is_empty());
    assert_same(&rich_document_v42(), &doc);
    // And save again as version 43.
    let again = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_same(&doc, &again);
}

#[test]
fn settings_out_of_range_are_brought_within_limits() {
    let doc = rich_document_v43();
    let mut model = v43::ProjectV43::from_document(&doc);
    model.modifiers[0].list[0].kind = v43::KindV43::Remesh {
        voxel_size: f32::NAN,
        smooth: 9_999,
    };
    let back = model.into_document().unwrap();
    let (_, list) = modifiers(&back)[0].clone();
    assert_eq!(
        list[0].kind,
        ModifierKind::Remesh(RemeshOptions {
            voxel_size: RemeshOptions::STANDARD_VOXEL,
            smooth: RemeshOptions::MAX_SMOOTH,
        })
    );
}

#[test]
fn damaged_modifier_lists_are_refused() {
    let doc = rich_document_v43();
    let good = v43::ProjectV43::from_document(&doc);
    // An object listed twice.
    let mut twice = good.clone();
    let first = twice.modifiers[0].clone();
    twice.modifiers.push(first);
    assert!(twice.into_document().is_err());
    // An object that isn't there.
    let mut missing = good.clone();
    missing.modifiers[0].id = 999_999;
    assert!(missing.into_document().is_err());
    // A group with modifiers.
    let mut group = good.clone();
    let group_id = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().kind == ObjectKind::Group)
        .expect("the rich document has a group");
    group.modifiers[0].id = group_id.get();
    assert!(group.into_document().is_err());
    // Too many.
    let mut many = good;
    let one = many.modifiers[0].list[0].clone();
    for _ in 0..scene::MAX_MODIFIERS {
        many.modifiers[0].list.push(one.clone());
    }
    assert!(many.into_document().is_err());
}

#[test]
fn an_older_app_is_told_it_needs_updating() {
    let error = super::tests::refused(&header(FORMAT_VERSION + 1, 0, 0));
    assert!(matches!(error, ProjectError::TooNew { .. }));
}

#[test]
fn random_version_43_bodies_never_panic() {
    let mut seed = 0x2545_f491_4f6c_dd1d_u64;
    let good = v43::encode(&v43::ProjectV43::from_document(&rich_document_v43())).unwrap();
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
        let mut bytes = header(43, 0, body.len() as u64);
        bytes.extend_from_slice(&body);
        let _ = from_bytes(&bytes);
    }
}
