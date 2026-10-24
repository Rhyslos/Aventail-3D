//! Tests for version 60: version 59 with the Struts modifier (decision
//! 0159).

use super::tests::{assert_same, header};
use super::v59_tests::rich_document_v59;
use super::*;
use scene::{Command, ModifierKind, ObjectId, ObjectKind, Primitive, StrutsOptions};

/// Every Struts modifier in `doc` with its object, in tree order.
fn frames(doc: &Document) -> Vec<(ObjectId, StrutsOptions, bool)> {
    let mut out = Vec::new();
    for (id, _) in doc.scene().walk() {
        for m in doc.scene().modifiers(id) {
            if let ModifierKind::Struts(o) = &m.kind {
                out.push((id, *o, m.in_viewport));
            }
        }
    }
    out
}

fn cube(doc: &mut Document) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(Primitive::Cube(
        Default::default(),
    ))))
    .unwrap();
    doc.selection()[0]
}

/// The sample's odd settings: every one off its standard.
fn odd() -> StrutsOptions {
    StrutsOptions {
        thickness: 3.5,
        sides: 7,
        even: false,
        keep_faces: true,
    }
}

fn add(doc: &mut Document, id: ObjectId, kind: ModifierKind) {
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind,
    })
    .unwrap();
}

/// Everything version 59 stores, plus a cube with standard struts, and a
/// cube with struts with every setting changed, hidden in the viewport:
/// the version 60 sample holds exactly this.
pub(super) fn rich_document_v60() -> Document {
    let mut doc = rich_document_v59();
    let framed = cube(&mut doc);
    add(
        &mut doc,
        framed,
        ModifierKind::Struts(StrutsOptions::default()),
    );
    let odd_one = cube(&mut doc);
    add(&mut doc, odd_one, ModifierKind::Struts(odd()));
    let mut hidden = doc.scene().modifiers(odd_one)[0].clone();
    hidden.in_viewport = false;
    doc.execute(Command::ChangeModifier {
        id: odd_one,
        index: 0,
        modifier: hidden,
    })
    .unwrap();
    doc
}

#[test]
fn struts_survive_a_round_trip() {
    let doc = rich_document_v60();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_same(&doc, &back);
    assert_eq!(frames(&doc), frames(&back));
    assert_eq!(frames(&back).len(), 2);
    for (id, _, _) in frames(&back) {
        assert_eq!(
            back.scene().exported_shape(id),
            doc.scene().exported_shape(id),
            "the shapes come out the same"
        );
    }
}

const SAMPLE_V60: &[u8] = include_bytes!("../../tests/data/sample-v60.avtl");

#[test]
fn a_file_saved_by_format_version_60_opens_with_its_struts() {
    let doc = from_bytes(SAMPLE_V60).expect("the version 60 sample opens");
    let expected = rich_document_v60();
    assert_same(&expected, &doc);
    let list = frames(&doc);
    assert_eq!(list, frames(&expected));
    assert_eq!(list.len(), 2);
    assert!(
        list.iter()
            .any(|(_, o, shown)| *o == StrutsOptions::default() && *shown)
    );
    assert!(list.iter().any(|(_, o, shown)| *o == odd() && !*shown));
    for (id, _, _) in &list {
        assert_eq!(
            doc.scene().exported_shape(*id),
            expected.scene().exported_shape(*id)
        );
    }
}

const SAMPLE_V59: &[u8] = include_bytes!("../../tests/data/sample-v59.avtl");

#[test]
fn older_files_open_with_no_struts() {
    let doc = from_bytes(SAMPLE_V59).unwrap();
    assert!(frames(&doc).is_empty());
    assert_same(&rich_document_v59(), &doc);
    let again = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_same(&doc, &again);
}

#[test]
fn settings_out_of_range_are_brought_within_limits() {
    for (thickness, sides, expected_thickness, expected_sides) in [
        (
            f32::NAN,
            0,
            StrutsOptions::STANDARD_THICKNESS,
            StrutsOptions::MIN_SIDES,
        ),
        (
            1e30,
            u32::MAX,
            StrutsOptions::MAX_THICKNESS,
            StrutsOptions::MAX_SIDES,
        ),
        (-4.0, 5, 4.0, 5),
    ] {
        let mut model = v43::ProjectV43::from_document(&rich_document_v60());
        let at = model
            .modifiers
            .iter()
            .position(|m| matches!(&m.list[0].kind, v43::KindV43::Struts(o) if o.keep_faces))
            .unwrap();
        let v43::KindV43::Struts(stored) = &mut model.modifiers[at].list[0].kind else {
            panic!("struts")
        };
        stored.thickness = thickness;
        stored.sides = sides;
        let id = ObjectId::from_raw(model.modifiers[at].id);
        let back = model.into_document().unwrap();
        let ModifierKind::Struts(o) = &back.scene().modifiers(id)[0].kind else {
            panic!("struts")
        };
        assert_eq!(o.thickness, expected_thickness);
        assert_eq!(o.sides, expected_sides);
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
fn random_version_60_bodies_never_panic() {
    let mut seed = 0x60be_4d70_7a9e_0159_u64;
    let good = v43::encode(&v43::ProjectV43::from_document(&rich_document_v60())).unwrap();
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
        let mut bytes = header(60, 0, body.len() as u64);
        bytes.extend_from_slice(&body);
        let _ = from_bytes(&bytes);
    }
}
