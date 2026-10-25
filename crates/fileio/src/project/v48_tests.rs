//! Tests for version 48: version 47 with the Bend along curve and
//! Shrinkwrap modifiers (decision 0147), which name the curve or target
//! they read.

use super::tests::{assert_same, header};
use super::v47_tests::rich_document_v47;
use super::*;
use scene::{
    BendAxis, BendOptions, Command, CurveBendSettings, CurveObject, CurveShape, Modifier,
    ModifierKind, ObjectId, ObjectKind, PrimitiveKind, ReadyCurve, ShrinkwrapOptions,
    ShrinkwrapSettings, WrapMode,
};

/// Every object's Bend along curve and Shrinkwrap modifiers in `doc`, in
/// tree order.
fn wraps(doc: &Document) -> Vec<(ObjectId, Vec<Modifier>)> {
    doc.scene()
        .walk()
        .iter()
        .map(|(id, _)| {
            let list: Vec<Modifier> = doc
                .scene()
                .modifiers(*id)
                .iter()
                .filter(|m| {
                    matches!(
                        m.kind,
                        ModifierKind::BendAlongCurve(_) | ModifierKind::Shrinkwrap(_)
                    )
                })
                .cloned()
                .collect();
            (*id, list)
        })
        .filter(|(_, list)| !list.is_empty())
        .collect()
}

/// The sample's odd bend: every setting off its standard.
fn odd_bend() -> BendOptions {
    BendOptions {
        axis: BendAxis::Z,
        stretch: true,
        offset: 12.5,
        twist: 45.0,
    }
}

/// The sample's odd wrap: every setting off its standard.
fn odd_wrap() -> ShrinkwrapOptions {
    ShrinkwrapOptions {
        mode: WrapMode::AxisY,
        offset: -1.5,
    }
}

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

/// Everything version 47 stores, plus a circle curve with a cube bent
/// along it as standard and a cylinder bent along it with every setting
/// changed, and a sphere wrapped onto a plane as standard and a cone
/// wrapped onto it with every setting changed and three chosen points,
/// hidden in the viewport: the version 48 sample holds exactly this.
pub(super) fn rich_document_v48() -> Document {
    let mut doc = rich_document_v47();
    doc.execute(Command::Create(ObjectKind::Curve(CurveObject::line(
        CurveShape::Ready(ReadyCurve::Circle { radius: 150.0 }),
    ))))
    .unwrap();
    let curve = doc.selection()[0];
    let cube = create(&mut doc, PrimitiveKind::Cube);
    let cylinder = create(&mut doc, PrimitiveKind::Cylinder);
    let plane = create(&mut doc, PrimitiveKind::Plane);
    let sphere = create(&mut doc, PrimitiveKind::Sphere);
    let cone = create(&mut doc, PrimitiveKind::Cone);
    let add = |doc: &mut Document, id: ObjectId, kind: ModifierKind| {
        doc.execute(Command::AddModifier {
            ids: vec![id],
            kind,
        })
        .unwrap();
    };
    add(
        &mut doc,
        cube,
        ModifierKind::BendAlongCurve(CurveBendSettings::of(curve)),
    );
    add(
        &mut doc,
        cylinder,
        ModifierKind::BendAlongCurve(CurveBendSettings {
            options: odd_bend(),
            curve,
        }),
    );
    add(
        &mut doc,
        sphere,
        ModifierKind::Shrinkwrap(ShrinkwrapSettings::of(plane)),
    );
    let points = doc
        .scene()
        .unmodified_shape(cone)
        .unwrap()
        .vertices()
        .map(|(v, _)| v)
        .take(3)
        .collect();
    add(
        &mut doc,
        cone,
        ModifierKind::Shrinkwrap(ShrinkwrapSettings {
            options: odd_wrap(),
            target: plane,
            points,
        }),
    );
    let mut hidden = doc.scene().modifiers(cone)[0].clone();
    hidden.in_viewport = false;
    doc.execute(Command::ChangeModifier {
        id: cone,
        index: 0,
        modifier: hidden,
    })
    .unwrap();
    doc
}

#[test]
fn bends_and_wraps_survive_a_round_trip() {
    let doc = rich_document_v48();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_same(&doc, &back);
    assert_eq!(wraps(&doc), wraps(&back));
    assert_eq!(wraps(&back).len(), 4);
    for (id, _) in wraps(&back) {
        assert_eq!(
            back.scene().exported_shape(id),
            doc.scene().exported_shape(id),
            "the bent or wrapped shape comes out the same"
        );
    }
}

const SAMPLE_V48: &[u8] = include_bytes!("../../tests/data/sample-v48.avtl");

#[test]
fn a_file_saved_by_format_version_48_opens_with_its_bends_and_wraps() {
    let doc = from_bytes(SAMPLE_V48).expect("the version 48 sample opens");
    let expected = rich_document_v48();
    assert_same(&expected, &doc);
    assert_eq!(wraps(&expected), wraps(&doc));
    let list: Vec<Modifier> = wraps(&doc).into_iter().flat_map(|(_, l)| l).collect();
    assert!(
        list.iter()
            .any(|m| matches!(&m.kind, ModifierKind::BendAlongCurve(b) if b.options == odd_bend()))
    );
    let odd: Vec<&Modifier> = list
        .iter()
        .filter(|m| matches!(&m.kind, ModifierKind::Shrinkwrap(w) if w.options == odd_wrap()))
        .collect();
    assert_eq!(odd.len(), 1);
    assert!(!odd[0].in_viewport);
    let ModifierKind::Shrinkwrap(settings) = &odd[0].kind else {
        unreachable!()
    };
    assert_eq!(settings.points.len(), 3);
}

const SAMPLE_V47: &[u8] = include_bytes!("../../tests/data/sample-v47.avtl");

#[test]
fn older_files_open_with_no_bends_or_wraps() {
    let doc = from_bytes(SAMPLE_V47).unwrap();
    assert!(wraps(&doc).is_empty());
    assert_same(&rich_document_v47(), &doc);
    let again = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_same(&doc, &again);
}

/// The sample's file model, and the places of its objects with a bend
/// (the last) and a wrap (the last).
fn model() -> (v43::ProjectV43, usize, usize) {
    let model = v43::ProjectV43::from_document(&rich_document_v48());
    let bend = model
        .modifiers
        .iter()
        .rposition(|m| matches!(m.list[0].kind, v43::KindV43::BendAlongCurve { .. }))
        .unwrap();
    let wrap = model
        .modifiers
        .iter()
        .rposition(|m| matches!(m.list[0].kind, v43::KindV43::Shrinkwrap { .. }))
        .unwrap();
    (model, bend, wrap)
}

#[test]
fn settings_out_of_range_are_brought_within_limits() {
    let (mut model, bend, wrap) = model();
    let v43::KindV43::BendAlongCurve { curve, .. } = model.modifiers[bend].list[0].kind.clone()
    else {
        panic!("a bend")
    };
    model.modifiers[bend].list[0].kind = v43::KindV43::BendAlongCurve {
        curve,
        axis: 200,
        stretch: false,
        offset: f32::NAN,
        twist: 1e30,
    };
    let v43::KindV43::Shrinkwrap { target, .. } = model.modifiers[wrap].list[0].kind.clone() else {
        panic!("a wrap")
    };
    model.modifiers[wrap].list[0].kind = v43::KindV43::Shrinkwrap {
        target,
        mode: 99,
        offset: f32::NEG_INFINITY,
        points: [5, 5, 1].into_iter().collect(),
    };
    let (bent, wrapped) = (
        ObjectId::from_raw(model.modifiers[bend].id),
        ObjectId::from_raw(model.modifiers[wrap].id),
    );
    let back = model.into_document().unwrap();
    let ModifierKind::BendAlongCurve(b) = &back.scene().modifiers(bent)[0].kind else {
        panic!("a bend")
    };
    assert_eq!(b.options.axis, BendAxis::Longest);
    assert_eq!(b.options.offset, 0.0);
    assert_eq!(b.options.twist, BendOptions::MAX_TWIST);
    let ModifierKind::Shrinkwrap(w) = &back.scene().modifiers(wrapped)[0].kind else {
        panic!("a wrap")
    };
    assert_eq!(w.options.mode, WrapMode::Nearest);
    assert_eq!(w.options.offset, 0.0);
    assert_eq!(w.points.len(), 2, "each point once");
}

#[test]
fn a_bend_or_wrap_naming_the_wrong_thing_or_a_loop_is_refused_as_damaged() {
    let (model, bend, wrap) = model();
    let bend_id = model.modifiers[bend].id;
    let wrap_id = model.modifiers[wrap].id;
    let with_curve = |curve: u64| {
        let mut model = model.clone();
        if let v43::KindV43::BendAlongCurve { curve: c, .. } =
            &mut model.modifiers[bend].list[0].kind
        {
            *c = curve;
        }
        model.into_document()
    };
    let with_target = |target: u64| {
        let mut model = model.clone();
        if let v43::KindV43::Shrinkwrap { target: t, .. } = &mut model.modifiers[wrap].list[0].kind
        {
            *t = target;
        }
        model.into_document()
    };
    // A missing curve, itself.
    assert!(with_curve(987_654).is_err());
    assert!(with_curve(bend_id).is_err());
    // A shape that isn't a curve (a curve baked by an older app): the bend
    // is taken off with a note, and the rest opens.
    let mut baked = model.clone();
    if let v43::KindV43::BendAlongCurve { curve: c, .. } = &mut baked.modifiers[bend].list[0].kind {
        *c = wrap_id;
    }
    let mut notes = Vec::new();
    let doc = baked.into_document_noting(&mut notes).unwrap();
    assert_eq!(notes.len(), 1);
    assert!(
        !doc.scene()
            .modifiers(scene::ObjectId::from_raw(bend_id))
            .iter()
            .any(|m| matches!(m.kind, scene::ModifierKind::BendAlongCurve(_)))
    );
    // A missing target, itself.
    assert!(with_target(987_654).is_err());
    assert!(with_target(wrap_id).is_err());
    // The sphere wraps onto the plane: the plane wrapping onto the sphere
    // would be a loop.
    let v43::KindV43::Shrinkwrap { target: plane, .. } = model.modifiers[wrap].list[0].kind else {
        panic!("a wrap")
    };
    let sphere = model.modifiers[wrap - 1].id;
    let mut looped = model.clone();
    looped.modifiers[wrap].id = plane;
    if let v43::KindV43::Shrinkwrap { target, .. } = &mut looped.modifiers[wrap].list[0].kind {
        *target = sphere;
    }
    assert!(looped.into_document().is_err());
}

#[test]
fn an_older_app_is_told_it_needs_updating() {
    let error = super::tests::refused(&header(FORMAT_VERSION + 1, 0, 0));
    assert!(matches!(error, ProjectError::TooNew { .. }));
}

#[test]
fn random_version_48_bodies_never_panic() {
    let mut seed = 0x3c6e_f372_fe94_f82b_u64;
    let good = v43::encode(&v43::ProjectV43::from_document(&rich_document_v48())).unwrap();
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
        let mut bytes = header(48, 0, body.len() as u64);
        bytes.extend_from_slice(&body);
        let _ = from_bytes(&bytes);
    }
}
