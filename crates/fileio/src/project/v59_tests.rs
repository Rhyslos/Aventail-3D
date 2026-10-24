//! Tests for version 59: version 58 with the *Bend, twist or taper*
//! modifier (decision 0158).

use super::tests::{assert_same, header};
use super::v58_tests::rich_document_v58;
use super::*;
use scene::{
    AxisDeformKind, AxisDeformOptions, Command, DeformAxis, ModifierKind, ObjectId, ObjectKind,
    Primitive,
};

/// Every Bend, twist or taper in `doc` with its object, in tree order.
fn deformers(doc: &Document) -> Vec<(ObjectId, AxisDeformOptions, bool)> {
    let mut out = Vec::new();
    for (id, _) in doc.scene().walk() {
        for m in doc.scene().modifiers(id) {
            if let ModifierKind::BendTwistTaper(o) = &m.kind {
                out.push((id, *o, m.in_viewport));
            }
        }
    }
    out
}

/// A pole cut into slices, to bend.
fn pole(doc: &mut Document) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(Primitive::Cylinder(
        mesh::primitives::CylinderParams {
            height_segments: 12,
            ..Default::default()
        },
    ))))
    .unwrap();
    doc.selection()[0]
}

/// The sample's odd settings: every one off its standard.
fn odd() -> AxisDeformOptions {
    AxisDeformOptions {
        kind: AxisDeformKind::Taper,
        axis: DeformAxis::X,
        angle: -120.0,
        direction: 30.0,
        amount: 0.75,
        curve: -0.25,
        limits: [0.8, 0.2],
        centre: glam::Vec3::new(5.0, -10.0, 20.0),
    }
}

fn add(doc: &mut Document, id: ObjectId, kind: ModifierKind) {
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind,
    })
    .unwrap();
}

/// Everything version 58 stores, plus a pole with a standard bend and
/// a twist after it, and a pole with a taper with every setting changed,
/// hidden in the viewport: the version 59 sample holds exactly this.
pub(super) fn rich_document_v59() -> Document {
    let mut doc = rich_document_v58();
    let bent = pole(&mut doc);
    add(
        &mut doc,
        bent,
        ModifierKind::BendTwistTaper(AxisDeformOptions::default()),
    );
    add(
        &mut doc,
        bent,
        ModifierKind::BendTwistTaper(AxisDeformOptions {
            kind: AxisDeformKind::Twist,
            ..AxisDeformOptions::default()
        }),
    );
    let tapered = pole(&mut doc);
    add(&mut doc, tapered, ModifierKind::BendTwistTaper(odd()));
    let mut hidden = doc.scene().modifiers(tapered)[0].clone();
    hidden.in_viewport = false;
    doc.execute(Command::ChangeModifier {
        id: tapered,
        index: 0,
        modifier: hidden,
    })
    .unwrap();
    doc
}

#[test]
fn deformers_survive_a_round_trip() {
    let doc = rich_document_v59();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_same(&doc, &back);
    assert_eq!(deformers(&doc), deformers(&back));
    assert_eq!(deformers(&back).len(), 3);
    for (id, _, _) in deformers(&back) {
        assert_eq!(
            back.scene().exported_shape(id),
            doc.scene().exported_shape(id),
            "the shapes come out the same"
        );
    }
}

const SAMPLE_V59: &[u8] = include_bytes!("../../tests/data/sample-v59.avtl");

#[test]
fn a_file_saved_by_format_version_59_opens_with_its_deformers() {
    let doc = from_bytes(SAMPLE_V59).expect("the version 59 sample opens");
    let expected = rich_document_v59();
    assert_same(&expected, &doc);
    let list = deformers(&doc);
    assert_eq!(list, deformers(&expected));
    assert_eq!(list.len(), 3);
    assert!(
        list.iter()
            .any(|(_, o, shown)| *o == AxisDeformOptions::default() && *shown)
    );
    assert!(list.iter().any(|(_, o, shown)| *o == odd() && !*shown));
    for (id, _, _) in &list {
        assert_eq!(
            doc.scene().exported_shape(*id),
            expected.scene().exported_shape(*id)
        );
    }
}

const SAMPLE_V58: &[u8] = include_bytes!("../../tests/data/sample-v58.avtl");

#[test]
fn older_files_open_with_no_deformers() {
    let doc = from_bytes(SAMPLE_V58).unwrap();
    assert!(deformers(&doc).is_empty());
    assert_same(&rich_document_v58(), &doc);
    let again = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_same(&doc, &again);
}

/// The sample's file model, and the place of its object with the odd
/// taper.
fn model() -> (v43::ProjectV43, usize) {
    let model = v43::ProjectV43::from_document(&rich_document_v59());
    let at = model
        .modifiers
        .iter()
        .position(|m| matches!(&m.list[0].kind, v43::KindV43::BendTwistTaper(o) if o.kind == 2))
        .unwrap();
    (model, at)
}

#[test]
fn settings_out_of_range_are_brought_within_limits() {
    for (angle, amount, kind, axis, expected) in [
        (f32::NAN, f32::NAN, 9, 9, AxisDeformOptions::STANDARD_ANGLE),
        (1e30, 1e30, 255, 1, AxisDeformOptions::MAX_ANGLE),
        (-1e30, -40.0, 1, 0, -AxisDeformOptions::MAX_ANGLE),
    ] {
        let (mut model, at) = model();
        let v43::KindV43::BendTwistTaper(stored) = &mut model.modifiers[at].list[0].kind else {
            panic!("a taper")
        };
        stored.angle = angle;
        stored.amount = amount;
        stored.kind = kind;
        stored.axis = axis;
        stored.direction = f32::INFINITY;
        stored.curve = 1e9;
        stored.limits = [f32::NAN, 7.0];
        stored.centre = [f32::NAN, 1e20, -1e20];
        let id = ObjectId::from_raw(model.modifiers[at].id);
        let back = model.into_document().unwrap();
        let ModifierKind::BendTwistTaper(o) = &back.scene().modifiers(id)[0].kind else {
            panic!("a bend, twist or taper")
        };
        assert_eq!(o.angle, expected);
        assert!(o.amount.abs() <= AxisDeformOptions::MAX_AMOUNT);
        assert_eq!(o.curve, AxisDeformOptions::MAX_AMOUNT);
        assert_eq!(o.direction, 0.0);
        assert_eq!(o.limits, [0.0, 1.0]);
        let most = AxisDeformOptions::MAX_CENTRE;
        assert_eq!(o.centre, glam::Vec3::new(0.0, most, -most));
        if kind > 2 {
            assert_eq!(o.kind, AxisDeformKind::Bend);
        }
        if axis > 2 {
            assert_eq!(o.axis, DeformAxis::Z);
        }
        assert!(
            back.scene()
                .exported_shape(id)
                .unwrap()
                .vertices()
                .all(|(_, p)| p.is_finite())
        );
    }
}

#[test]
fn an_older_app_is_told_it_needs_updating() {
    let error = super::tests::refused(&header(FORMAT_VERSION + 1, 0, 0));
    assert!(matches!(error, ProjectError::TooNew { .. }));
}

#[test]
fn random_version_59_bodies_never_panic() {
    let mut seed = 0x59be_4d70_7a9e_0001_u64;
    let good = v43::encode(&v43::ProjectV43::from_document(&rich_document_v59())).unwrap();
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
        let mut bytes = header(59, 0, body.len() as u64);
        bytes.extend_from_slice(&body);
        let _ = from_bytes(&bytes);
    }
}
