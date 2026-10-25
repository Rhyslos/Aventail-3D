//! Tests for version 63: version 62 with modifier ids and each level of
//! detail's details: reduced or not, budget, changes to the modifiers and
//! a shape made by hand (decision 0165).

use super::tests::{assert_same, header};
use super::v62_tests::rich_document_v62;
use super::*;
use scene::{
    Command, LiveBevelOptions, Lods, ModifierId, ModifierKind, ObjectId, ObjectKind, Primitive,
    SubdivideOptions,
};

fn add(doc: &mut Document, id: ObjectId, kind: ModifierKind) {
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind,
    })
    .unwrap();
}

/// Every object's modifier ids and levels, in tree order.
fn levels(doc: &Document) -> Vec<(ObjectId, Vec<ModifierId>, Lods)> {
    let scene = doc.scene();
    scene
        .walk()
        .into_iter()
        .filter_map(|(id, _)| {
            let object = scene.object(id)?;
            Some((
                id,
                scene.modifiers(id).iter().map(|m| m.id).collect(),
                object.lods.clone(),
            ))
        })
        .collect()
}

/// Everything version 62 stores, plus a sphere with a Subdivide and a
/// Live bevel (added after it but placed before it, so their ids are out
/// of order) and three levels: the first without Subdivide, the second
/// with a one-segment bevel, not reduced and with a budget, the third
/// made by hand and then edited. The version 63 sample holds exactly this.
pub(super) fn rich_document_v63() -> Document {
    let mut doc = rich_document_v62();
    doc.execute(Command::Create(ObjectKind::Primitive(Primitive::Sphere(
        Default::default(),
    ))))
    .unwrap();
    let sphere = doc.selection()[0];
    add(
        &mut doc,
        sphere,
        ModifierKind::Subdivide(SubdivideOptions::default()),
    );
    add(
        &mut doc,
        sphere,
        ModifierKind::LiveBevel(LiveBevelOptions::default()),
    );
    let modifiers = doc.scene().modifiers(sphere).to_vec();
    let (bevel, subdivide) = (modifiers[0].id, modifiers[1].id);
    assert_eq!((bevel, subdivide), (ModifierId(2), ModifierId(1)));
    let mut lods = Lods {
        on: true,
        ..Lods::default()
    };
    lods.levels[0] = lods.levels[0].with_override(&modifiers, subdivide, false, None);
    lods.levels[1] = lods.levels[1].with_override(
        &modifiers,
        bevel,
        true,
        Some(ModifierKind::LiveBevel(LiveBevelOptions {
            segments: 1,
            ..LiveBevelOptions::default()
        })),
    );
    lods.levels[1].reduce = false;
    lods.levels[1].budget = 1_500;
    doc.execute(Command::SetLods {
        ids: vec![sphere],
        lods,
    })
    .unwrap();
    doc.execute(Command::MakeLevelByHand {
        id: sphere,
        level: 2,
    })
    .unwrap();
    let shape = doc.selection()[0];
    doc.execute(Command::Record {
        id: shape,
        operation: scene::Operation::FixFacing,
    })
    .unwrap();
    doc.execute(Command::SetTransforms(vec![(
        shape,
        scene::Transform {
            position: glam::Vec3::new(0.0, 0.0, 30.0),
            ..scene::Transform::default()
        },
    )]))
    .unwrap();
    doc.set_selection([sphere]);
    doc
}

/// The sphere with the levels in `doc`.
fn sphere(doc: &Document) -> ObjectId {
    doc.scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| {
            doc.scene()
                .object(*id)
                .is_some_and(|o| o.lods.levels.iter().any(|l| l.hand.is_some()))
        })
        .unwrap()
}

#[test]
fn levels_and_modifier_ids_survive_a_round_trip() {
    let doc = rich_document_v63();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_same(&doc, &back);
    assert_eq!(levels(&doc), levels(&back));
    let id = sphere(&back);
    let (shape, place) = back.scene().level_shape(id, 2).unwrap();
    assert!(place.is_some(), "moved up");
    assert_eq!(back.scene().level_drifted(id, 2), Some(false));
    assert_eq!(
        back.scene().evaluated(shape).unwrap().mesh,
        doc.scene().evaluated(shape).unwrap().mesh
    );
    assert_eq!(
        back.scene().lod_shapes(id).unwrap().triangles(),
        doc.scene().lod_shapes(id).unwrap().triangles(),
        "the levels come out the same"
    );
}

const SAMPLE_V63: &[u8] = include_bytes!("../../tests/data/sample-v63.avtl");

#[test]
fn a_file_saved_by_format_version_63_opens_with_its_levels() {
    let doc = from_bytes(SAMPLE_V63).expect("the version 63 sample opens");
    let expected = rich_document_v63();
    assert_same(&expected, &doc);
    assert_eq!(levels(&doc), levels(&expected));
    let id = sphere(&doc);
    let lods = &doc.scene().object(id).unwrap().lods;
    assert_eq!(lods.levels[0].overrides.len(), 1);
    assert!(!lods.levels[1].reduce);
    assert_eq!(lods.levels[1].budget, 1_500);
    assert!(doc.scene().level_shape(id, 2).is_some());
    assert_eq!(
        doc.scene().lod_shapes(id).unwrap().triangles(),
        expected.scene().lod_shapes(id).unwrap().triangles()
    );
}

const SAMPLE_V62: &[u8] = include_bytes!("../../tests/data/sample-v62.avtl");

#[test]
fn older_files_open_with_ids_in_order_and_levels_reduced() {
    let doc = from_bytes(SAMPLE_V62).unwrap();
    assert_same(&rich_document_v62(), &doc);
    for (id, ids, lods) in levels(&doc) {
        let in_order: Vec<ModifierId> = (1..=ids.len() as u32).map(ModifierId).collect();
        assert_eq!(ids, in_order, "{id}");
        assert!(lods.levels.iter().all(|l| l.reduce && l.budget == 0));
        assert!(lods.levels.iter().all(|l| l.overrides.is_empty()));
        assert!(lods.levels.iter().all(|l| l.hand.is_none()));
    }
    let again = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(levels(&doc), levels(&again));
}

#[test]
fn damaged_levels_are_refused_or_tidied() {
    let good = v43::ProjectV43::from_document(&rich_document_v63());
    // A hand-made level naming an object that isn't there: damaged.
    let mut missing = good.clone();
    missing.levels[0].levels[2].hand = Some((99_999, 0));
    assert!(missing.into_document().is_err());
    // Listed twice: damaged.
    let mut twice = good.clone();
    let first = twice.levels[0].clone();
    twice.levels.push(first);
    assert!(twice.into_document().is_err());
    // A change naming no modifier, and more levels than there are: left
    // out.
    let mut extra = good.clone();
    extra.levels[0].levels[0].overrides[0].modifier = 77;
    let more = extra.levels[0].levels[0].clone();
    extra.levels[0].levels.push(more);
    let back = extra.into_document().unwrap();
    let id = sphere(&back);
    let lods = &back.scene().object(id).unwrap().lods;
    assert!(lods.levels[0].overrides.is_empty());
    assert_eq!(lods.levels.len(), 3);
    // Repeated modifier ids are given new ones.
    let mut repeated = good;
    let at = repeated
        .modifiers
        .iter()
        .position(|m| m.list.len() == 2 && m.id == id.get())
        .unwrap();
    repeated.modifiers[at].list[1].id = repeated.modifiers[at].list[0].id;
    let back = repeated.into_document().unwrap();
    let ids: Vec<ModifierId> = back.scene().modifiers(id).iter().map(|m| m.id).collect();
    assert_eq!(ids[0], ModifierId(2));
    assert_ne!(ids[0], ids[1]);
}

#[test]
fn an_older_app_is_told_it_needs_updating() {
    let error = super::tests::refused(&header(FORMAT_VERSION + 1, 0, 0));
    assert!(matches!(error, ProjectError::TooNew { .. }));
}

#[test]
fn random_version_63_bodies_never_panic() {
    let mut seed = 0x63be_4d70_7a9e_0165_u64;
    let good = v43::encode(&v43::ProjectV43::from_document(&rich_document_v63())).unwrap();
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
        let mut bytes = header(63, 0, body.len() as u64);
        bytes.extend_from_slice(&body);
        let _ = from_bytes(&bytes);
    }
}
