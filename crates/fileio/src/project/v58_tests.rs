//! Tests for version 58: version 57 with the Displace modifier (decision
//! 0157).

use super::tests::{assert_same, header};
use super::v57_tests::rich_document_v57;
use super::*;
use scene::{
    Command, DisplaceDirection, DisplaceOptions, DisplaceSettings, ModifierKind, NoiseKind,
    ObjectId, ObjectKind, PrimitiveKind, VertexId,
};

/// Every Displace in `doc` with its object, in tree order.
fn displaces(doc: &Document) -> Vec<(ObjectId, DisplaceSettings, bool)> {
    let mut out = Vec::new();
    for (id, _) in doc.scene().walk() {
        for m in doc.scene().modifiers(id) {
            if let ModifierKind::Displace(d) = &m.kind {
                out.push((id, d.clone(), m.in_viewport));
            }
        }
    }
    out
}

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

/// The sample's odd settings: every one off its standard, with three
/// chosen points.
fn odd() -> DisplaceSettings {
    DisplaceSettings {
        options: DisplaceOptions {
            strength: -7.5,
            scale: 12.0,
            seed: 42,
            detail: 5,
            noise: NoiseKind::Cells,
            direction: DisplaceDirection::AxisZ,
            falloff: 35.0,
        },
        points: [0, 2, 5].into_iter().map(VertexId::from_raw).collect(),
    }
}

fn add(doc: &mut Document, id: ObjectId, kind: ModifierKind) {
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind,
    })
    .unwrap();
}

/// Everything version 57 stores, plus a sphere with a standard Displace
/// and a cube with a Displace with every setting changed (ridged would
/// do as well; cells is the one furthest from standard), hidden in the
/// viewport: the version 58 sample holds exactly this.
pub(super) fn rich_document_v58() -> Document {
    let mut doc = rich_document_v57();
    let sphere = create(&mut doc, PrimitiveKind::Sphere);
    add(
        &mut doc,
        sphere,
        ModifierKind::Displace(DisplaceSettings::default()),
    );
    let cube = create(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, cube, ModifierKind::Displace(odd()));
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
fn displaces_survive_a_round_trip() {
    let doc = rich_document_v58();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_same(&doc, &back);
    assert_eq!(displaces(&doc), displaces(&back));
    assert_eq!(displaces(&back).len(), 2);
    for (id, _, _) in displaces(&back) {
        assert_eq!(
            back.scene().exported_shape(id),
            doc.scene().exported_shape(id),
            "the bumps come out the same"
        );
    }
}

const SAMPLE_V58: &[u8] = include_bytes!("../../tests/data/sample-v58.avtl");

#[test]
fn a_file_saved_by_format_version_58_opens_with_its_displaces() {
    let doc = from_bytes(SAMPLE_V58).expect("the version 58 sample opens");
    let expected = rich_document_v58();
    assert_same(&expected, &doc);
    let list = displaces(&doc);
    assert_eq!(list, displaces(&expected));
    assert_eq!(list.len(), 2);
    assert!(
        list.iter()
            .any(|(_, d, shown)| *d == DisplaceSettings::default() && *shown)
    );
    assert!(list.iter().any(|(_, d, shown)| *d == odd() && !*shown));
    // The noise is the same as when the sample was saved: the shapes come
    // out exactly as they did.
    for (id, _, _) in &list {
        assert_eq!(
            doc.scene().exported_shape(*id),
            expected.scene().exported_shape(*id)
        );
    }
}

const SAMPLE_V57: &[u8] = include_bytes!("../../tests/data/sample-v57.avtl");

#[test]
fn older_files_open_with_no_displaces() {
    let doc = from_bytes(SAMPLE_V57).unwrap();
    assert!(displaces(&doc).is_empty());
    assert_same(&rich_document_v57(), &doc);
    let again = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_same(&doc, &again);
}

/// The sample's file model, and the place of its object with the odd
/// displace.
fn model() -> (v43::ProjectV43, usize) {
    let model = v43::ProjectV43::from_document(&rich_document_v58());
    let at = model
        .modifiers
        .iter()
        .position(|m| matches!(&m.list[0].kind, v43::KindV43::Displace(d) if d.seed == 42))
        .unwrap();
    (model, at)
}

#[test]
fn settings_out_of_range_are_brought_within_limits() {
    for (strength, scale, detail, noise, expected) in [
        (f32::NAN, f32::NAN, 0, 9, DisplaceOptions::STANDARD_STRENGTH),
        (1e30, 1e30, 99, 255, DisplaceOptions::MAX_STRENGTH),
        (-1e30, -4.0, 3, 1, -DisplaceOptions::MAX_STRENGTH),
    ] {
        let (mut model, at) = model();
        let v43::KindV43::Displace(stored) = &mut model.modifiers[at].list[0].kind else {
            panic!("a displace")
        };
        stored.strength = strength;
        stored.scale = scale;
        stored.detail = detail;
        stored.noise = noise;
        stored.direction = 200;
        stored.seed = u32::MAX;
        stored.falloff = -3.0;
        stored.points = [7, 7, 1].into_iter().collect();
        let id = ObjectId::from_raw(model.modifiers[at].id);
        let back = model.into_document().unwrap();
        let ModifierKind::Displace(d) = &back.scene().modifiers(id)[0].kind else {
            panic!("a displace")
        };
        let o = d.options;
        assert_eq!(o.strength, expected);
        assert!((DisplaceOptions::MIN_SCALE..=DisplaceOptions::MAX_SCALE).contains(&o.scale));
        assert!((1..=DisplaceOptions::MAX_DETAIL).contains(&o.detail));
        assert_eq!(o.seed, DisplaceOptions::MAX_SEED);
        assert_eq!(o.falloff, 0.0);
        assert_eq!(o.direction, DisplaceDirection::Normals);
        if noise > 2 {
            assert_eq!(o.noise, NoiseKind::Smooth);
        }
        assert_eq!(
            d.points,
            vec![VertexId::from_raw(1), VertexId::from_raw(7)],
            "each point once"
        );
    }
}

#[test]
fn an_older_app_is_told_it_needs_updating() {
    let error = super::tests::refused(&header(FORMAT_VERSION + 1, 0, 0));
    assert!(matches!(error, ProjectError::TooNew { .. }));
}

#[test]
fn random_version_58_bodies_never_panic() {
    let mut seed = 0x58d1_5b1a_ce00_7a75_u64;
    let good = v43::encode(&v43::ProjectV43::from_document(&rich_document_v58())).unwrap();
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
        let mut bytes = header(58, 0, body.len() as u64);
        bytes.extend_from_slice(&body);
        let _ = from_bytes(&bytes);
    }
}
