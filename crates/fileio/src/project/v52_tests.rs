//! Tests for version 52: version 51 with collision in parts and its
//! settings (decision 0151).

use super::tests::{assert_same, header};
use super::v51_tests::rich_document_v51;
use super::*;
use scene::{Collision, Command, ConvexPartsOptions, ObjectId, ObjectKind, PrimitiveKind};

/// Every object's collision and collision parts in `doc` that aren't the
/// standard ones, in tree order.
fn collisions(doc: &Document) -> Vec<(ObjectId, Collision, ConvexPartsOptions)> {
    doc.scene()
        .walk()
        .iter()
        .filter_map(|(id, _)| {
            let o = doc.scene().object(*id)?;
            (o.collision != Collision::None || o.collision_parts != ConvexPartsOptions::default())
                .then_some((*id, o.collision, o.collision_parts))
        })
        .collect()
}

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

/// The sample's odd settings: both off their standard.
fn odd() -> ConvexPartsOptions {
    ConvexPartsOptions {
        max_parts: 5,
        detail: 0.75,
    }
}

/// Everything version 51 stores, plus an arch wrapped in parts (standard
/// settings), a torus wrapped in parts with odd settings, and a cube
/// wrapped as a box whose parts settings were changed before: the version
/// 52 sample holds exactly this.
pub(super) fn rich_document_v52() -> Document {
    let mut doc = rich_document_v51();
    let arch = create(&mut doc, PrimitiveKind::Arch);
    let torus = create(&mut doc, PrimitiveKind::Torus);
    let cube = create(&mut doc, PrimitiveKind::Cube);
    doc.execute(Command::SetCollision {
        ids: vec![arch, torus, cube],
        collision: Collision::Parts,
    })
    .unwrap();
    doc.execute(Command::SetCollisionParts {
        ids: vec![torus, cube],
        options: odd(),
    })
    .unwrap();
    doc.execute(Command::SetCollision {
        ids: vec![cube],
        collision: Collision::Box,
    })
    .unwrap();
    doc
}

#[test]
fn collision_parts_survive_a_round_trip() {
    let doc = rich_document_v52();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_same(&doc, &back);
    assert_eq!(collisions(&doc), collisions(&back));
    let parts: Vec<_> = collisions(&back)
        .into_iter()
        .filter(|(_, c, _)| *c == Collision::Parts)
        .collect();
    assert_eq!(parts.len(), 2);
}

const SAMPLE_V52: &[u8] = include_bytes!("../../tests/data/sample-v52.avtl");

#[test]
fn a_file_saved_by_format_version_52_opens_with_its_collision_parts() {
    let doc = from_bytes(SAMPLE_V52).expect("the version 52 sample opens");
    let expected = rich_document_v52();
    assert_same(&expected, &doc);
    let found = collisions(&doc);
    assert_eq!(collisions(&expected), found);
    let has = |c: Collision, p: ConvexPartsOptions| {
        found
            .iter()
            .filter(|(_, fc, fp)| *fc == c && *fp == p)
            .count()
    };
    // The arch with the standard settings; the torus's odd ones; the
    // cube's odd ones, kept though it collides as a box.
    assert_eq!(has(Collision::Parts, ConvexPartsOptions::default()), 1);
    assert_eq!(has(Collision::Parts, odd()), 1);
    assert_eq!(has(Collision::Box, odd()), 1);
}

const SAMPLE_V51: &[u8] = include_bytes!("../../tests/data/sample-v51.avtl");

#[test]
fn older_files_open_with_no_collision_parts() {
    let doc = from_bytes(SAMPLE_V51).unwrap();
    assert!(
        collisions(&doc)
            .iter()
            .all(|(_, c, p)| *c != Collision::Parts && *p == ConvexPartsOptions::default())
    );
    assert_same(&rich_document_v51(), &doc);
    let again = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_same(&doc, &again);
}

/// The sample's file model, and the place of the torus's settings.
fn model() -> (v43::ProjectV43, usize) {
    let model = v43::ProjectV43::from_document(&rich_document_v52());
    let at = objects(&model)
        .iter()
        .position(|o| {
            o.collision_parts
                .is_some_and(|p| p.chosen && p.max_parts == 5)
        })
        .unwrap();
    (model, at)
}

/// The development settings of every object in `model`.
fn objects(model: &v43::ProjectV43) -> &Vec<super::v39::ObjectV39> {
    &model.project.project.project.objects.0
}

fn objects_mut(model: &mut v43::ProjectV43) -> &mut Vec<super::v39::ObjectV39> {
    &mut model.project.project.project.objects.0
}

#[test]
fn settings_out_of_range_are_brought_within_limits() {
    let (mut model, at) = model();
    let id = ObjectId::from_raw(objects(&model)[at].id);
    objects_mut(&mut model)[at].collision_parts = Some(super::v52::CollisionPartsV52 {
        chosen: true,
        max_parts: 0,
        detail: f32::NAN,
    });
    let back = model.into_document().unwrap();
    let parts = back.scene().object(id).unwrap().collision_parts;
    assert_eq!(parts.max_parts, 1);
    assert_eq!(parts.detail, ConvexPartsOptions::STANDARD_DETAIL);
}

#[test]
fn parts_for_an_object_with_no_shape_are_refused() {
    let mut doc = rich_document_v52();
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    let group = doc.selection()[0];
    let mut model = v43::ProjectV43::from_document(&doc);
    let list = objects_mut(&mut model);
    let at = list.iter().position(|o| o.id == group.get()).unwrap();
    list[at].collision_parts = Some(super::v52::CollisionPartsV52 {
        chosen: true,
        max_parts: 8,
        detail: 0.5,
    });
    // Refused once every part of the file is in (an empty group may stand
    // in for a curve until then).
    assert!(matches!(
        from_bytes(&super::tests::current_file(&model)),
        Err(ProjectError::Damaged(_))
    ));
}

#[test]
fn random_version_52_bodies_never_panic() {
    let mut seed = 0x52c0_11d1_e5ab_0b1e_u64;
    let good = v43::encode(&v43::ProjectV43::from_document(&rich_document_v52())).unwrap();
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
        let mut bytes = header(52, 0, body.len() as u64);
        bytes.extend_from_slice(&body);
        let _ = from_bytes(&bytes);
    }
}
