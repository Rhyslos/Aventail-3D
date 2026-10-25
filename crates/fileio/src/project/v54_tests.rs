//! Tests for version 54: version 53 with the Array modifier (decision
//! 0153).

use super::tests::{assert_same, header};
use super::v53_tests::rich_document_v53;
use super::*;
use glam::Vec3;
use scene::{
    ArrayAxis, ArrayLayout, ArrayOptions, ArraySettings, Command, Modifier, ModifierKind, ObjectId,
    ObjectKind, PrimitiveKind,
};

/// Every object's Array modifiers in `doc`, in tree order.
fn arrays(doc: &Document) -> Vec<(ObjectId, Vec<Modifier>)> {
    doc.scene()
        .walk()
        .iter()
        .map(|(id, _)| {
            let list: Vec<Modifier> = doc
                .scene()
                .modifiers(*id)
                .iter()
                .filter(|m| matches!(m.kind, ModifierKind::Array(_)))
                .cloned()
                .collect();
            (*id, list)
        })
        .filter(|(_, list)| !list.is_empty())
        .collect()
}

/// The sample's odd settings: every one off its standard.
fn odd() -> ArrayOptions {
    ArrayOptions {
        layout: ArrayLayout::Radial,
        count: 7,
        counts: [2, 4, 3],
        relative: false,
        offset: Vec3::new(10.0, -20.0, 30.0),
        spacing: Vec3::new(1.5, 2.5, 0.5),
        angle: 180.0,
        axis: ArrayAxis::X,
        radius: 75.0,
        merge: true,
        merge_distance: 0.5,
        uvs_side_by_side: true,
    }
}

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

fn add(doc: &mut Document, id: ObjectId, kind: ModifierKind) {
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind,
    })
    .unwrap();
}

/// Everything version 53 stores, plus a cube with a standard Array; a
/// cube with an Array with every setting changed, turning round a sphere,
/// hidden in the viewport; and a plane in a merged grid: the version 54
/// sample holds exactly this.
pub(super) fn rich_document_v54() -> Document {
    let mut doc = rich_document_v53();
    let plain = create(&mut doc, PrimitiveKind::Cube);
    add(
        &mut doc,
        plain,
        ModifierKind::Array(ArraySettings::default()),
    );
    let hub = create(&mut doc, PrimitiveKind::Sphere);
    let odd_one = create(&mut doc, PrimitiveKind::Cube);
    add(
        &mut doc,
        odd_one,
        ModifierKind::Array(ArraySettings {
            options: odd(),
            centre: Some(hub),
        }),
    );
    let mut hidden = doc.scene().modifiers(odd_one)[0].clone();
    hidden.in_viewport = false;
    doc.execute(Command::ChangeModifier {
        id: odd_one,
        index: 0,
        modifier: hidden,
    })
    .unwrap();
    let floor = create(&mut doc, PrimitiveKind::Plane);
    add(
        &mut doc,
        floor,
        ModifierKind::Array(ArraySettings {
            options: ArrayOptions {
                layout: ArrayLayout::Grid,
                merge: true,
                ..ArrayOptions::default()
            },
            centre: None,
        }),
    );
    doc
}

#[test]
fn arrays_survive_a_round_trip() {
    let doc = rich_document_v54();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_same(&doc, &back);
    assert_eq!(arrays(&doc), arrays(&back));
    assert_eq!(arrays(&back).len(), 3);
    for (id, _) in arrays(&back) {
        assert_eq!(
            back.scene().exported_shape(id),
            doc.scene().exported_shape(id),
            "the copies come out the same"
        );
    }
}

const SAMPLE_V54: &[u8] = include_bytes!("../../tests/data/sample-v54.avtl");

#[test]
fn a_file_saved_by_format_version_54_opens_with_its_arrays() {
    let doc = from_bytes(SAMPLE_V54).expect("the version 54 sample opens");
    let expected = rich_document_v54();
    assert_same(&expected, &doc);
    assert_eq!(arrays(&expected), arrays(&doc));
    let list: Vec<Modifier> = arrays(&doc).into_iter().flat_map(|(_, l)| l).collect();
    let odd_ones: Vec<&Modifier> = list
        .iter()
        .filter(|m| matches!(&m.kind, ModifierKind::Array(a) if a.options == odd()))
        .collect();
    assert_eq!(odd_ones.len(), 1);
    assert!(!odd_ones[0].in_viewport);
    let ModifierKind::Array(settings) = &odd_ones[0].kind else {
        panic!("an array")
    };
    let centre = settings.centre.expect("a centre object");
    assert!(matches!(
        doc.scene().object(centre).unwrap().kind,
        ObjectKind::Primitive(scene::Primitive::Sphere(_))
    ));
}

const SAMPLE_V53: &[u8] = include_bytes!("../../tests/data/sample-v53.avtl");

#[test]
fn older_files_open_with_no_arrays() {
    let doc = from_bytes(SAMPLE_V53).unwrap();
    assert!(arrays(&doc).is_empty());
    assert_same(&rich_document_v53(), &doc);
    let again = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_same(&doc, &again);
}

/// The sample's file model, and the place of its object with the odd
/// array.
fn model() -> (v43::ProjectV43, usize) {
    let model = v43::ProjectV43::from_document(&rich_document_v54());
    let at = model
        .modifiers
        .iter()
        .position(|m| matches!(&m.list[0].kind, v43::KindV43::Array(a) if a.centre.is_some()))
        .unwrap();
    (model, at)
}

#[test]
fn settings_out_of_range_are_brought_within_limits() {
    let (mut model, at) = model();
    let v43::KindV43::Array(stored) = &mut model.modifiers[at].list[0].kind else {
        panic!("an array")
    };
    stored.layout = 99;
    stored.axis = 42;
    stored.count = 0;
    stored.counts = [u32::MAX, 0, 5];
    stored.offset = [f32::NAN, 1e30, -1e30];
    stored.spacing = [f32::INFINITY; 3];
    stored.angle = f32::NAN;
    stored.radius = f32::NEG_INFINITY;
    stored.merge_distance = 1e9;
    let id = ObjectId::from_raw(model.modifiers[at].id);
    let back = model.into_document().unwrap();
    let ModifierKind::Array(a) = &back.scene().modifiers(id)[0].kind else {
        panic!("an array")
    };
    let o = a.options;
    assert_eq!(o.layout, ArrayLayout::Line);
    assert_eq!(o.axis, ArrayAxis::Z);
    assert_eq!(o.count, 1);
    assert_eq!(o.counts, [ArrayOptions::MAX_COUNT, 1, 5]);
    assert_eq!(
        o.offset,
        Vec3::new(1.0, ArrayOptions::MAX_DISTANCE, -ArrayOptions::MAX_DISTANCE)
    );
    assert_eq!(o.spacing, Vec3::ONE);
    assert_eq!(o.angle, 360.0);
    assert_eq!(o.radius, 0.0);
    assert_eq!(o.merge_distance, scene::WeldOptions::MAX_DISTANCE);
}

#[test]
fn a_centre_that_is_not_there_or_itself_is_refused_as_damaged() {
    for centre in [Some(987_654_321), None] {
        let (mut model, at) = model();
        let own = model.modifiers[at].id;
        let v43::KindV43::Array(stored) = &mut model.modifiers[at].list[0].kind else {
            panic!("an array")
        };
        stored.centre = Some(centre.unwrap_or(own));
        assert!(model.into_document().is_err(), "{centre:?}");
    }
}

#[test]
fn an_older_app_is_told_it_needs_updating() {
    let error = super::tests::refused(&header(FORMAT_VERSION + 1, 0, 0));
    assert!(matches!(error, ProjectError::TooNew { .. }));
}

#[test]
fn random_version_54_bodies_never_panic() {
    let mut seed = 0x54a7_7a75_1c0d_e5ee_u64;
    let good = v43::encode(&v43::ProjectV43::from_document(&rich_document_v54())).unwrap();
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
        let mut bytes = header(54, 0, body.len() as u64);
        bytes.extend_from_slice(&body);
        let _ = from_bytes(&bytes);
    }
}
