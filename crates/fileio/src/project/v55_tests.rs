//! Tests for version 55: version 54 with the Shell modifier (decision
//! 0154).

use super::tests::{assert_same, header};
use super::v54_tests::rich_document_v54;
use super::*;
use mesh::primitives::CylinderParams;
use scene::{
    Command, Modifier, ModifierKind, ObjectId, ObjectKind, Primitive, PrimitiveKind,
    ShellDirection, ShellOptions,
};

/// Every object's Shell modifiers in `doc`, in tree order.
fn shells(doc: &Document) -> Vec<(ObjectId, Vec<Modifier>)> {
    doc.scene()
        .walk()
        .iter()
        .map(|(id, _)| {
            let list: Vec<Modifier> = doc
                .scene()
                .modifiers(*id)
                .iter()
                .filter(|m| matches!(m.kind, ModifierKind::Shell(_)))
                .cloned()
                .collect();
            (*id, list)
        })
        .filter(|(_, list)| !list.is_empty())
        .collect()
}

/// The sample's odd settings: every one off its standard.
fn odd() -> ShellOptions {
    ShellOptions {
        thickness: 3.5,
        direction: ShellDirection::Both,
        even: false,
        fill_rims: false,
    }
}

fn add(doc: &mut Document, id: ObjectId, kind: ModifierKind) {
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind,
    })
    .unwrap();
}

/// Everything version 54 stores, plus a plane with a standard Shell and
/// an open pipe with a Shell with every setting changed, hidden in the
/// viewport: the version 55 sample holds exactly this.
pub(super) fn rich_document_v55() -> Document {
    let mut doc = rich_document_v54();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Plane.standard(),
    )))
    .unwrap();
    let plane = doc.selection()[0];
    add(
        &mut doc,
        plane,
        ModifierKind::Shell(ShellOptions::default()),
    );
    doc.execute(Command::Create(ObjectKind::Primitive(Primitive::Cylinder(
        CylinderParams {
            caps: false,
            ..CylinderParams::default()
        },
    ))))
    .unwrap();
    let pipe = doc.selection()[0];
    add(&mut doc, pipe, ModifierKind::Shell(odd()));
    let mut hidden = doc.scene().modifiers(pipe)[0].clone();
    hidden.in_viewport = false;
    doc.execute(Command::ChangeModifier {
        id: pipe,
        index: 0,
        modifier: hidden,
    })
    .unwrap();
    doc
}

#[test]
fn shells_survive_a_round_trip() {
    let doc = rich_document_v55();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_same(&doc, &back);
    assert_eq!(shells(&doc), shells(&back));
    assert_eq!(shells(&back).len(), 2);
    for (id, _) in shells(&back) {
        assert_eq!(
            back.scene().exported_shape(id),
            doc.scene().exported_shape(id),
            "the shells come out the same"
        );
    }
}

const SAMPLE_V55: &[u8] = include_bytes!("../../tests/data/sample-v55.avtl");

#[test]
fn a_file_saved_by_format_version_55_opens_with_its_shells() {
    let doc = from_bytes(SAMPLE_V55).expect("the version 55 sample opens");
    let expected = rich_document_v55();
    assert_same(&expected, &doc);
    assert_eq!(shells(&expected), shells(&doc));
    let list: Vec<Modifier> = shells(&doc).into_iter().flat_map(|(_, l)| l).collect();
    assert_eq!(list.len(), 2);
    let standard = ModifierKind::Shell(ShellOptions::default());
    let plain = list.iter().find(|m| m.kind == standard).unwrap();
    assert!(plain.in_viewport);
    let odd_one = list
        .iter()
        .find(|m| m.kind == ModifierKind::Shell(odd()))
        .unwrap();
    assert!(!odd_one.in_viewport);
}

const SAMPLE_V54: &[u8] = include_bytes!("../../tests/data/sample-v54.avtl");

#[test]
fn older_files_open_with_no_shells() {
    let doc = from_bytes(SAMPLE_V54).unwrap();
    assert!(shells(&doc).is_empty());
    assert_same(&rich_document_v54(), &doc);
    let again = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_same(&doc, &again);
}

/// The sample's file model, and the place of its object with the odd
/// shell.
fn model() -> (v43::ProjectV43, usize) {
    let model = v43::ProjectV43::from_document(&rich_document_v55());
    let at = model
        .modifiers
        .iter()
        .position(|m| matches!(&m.list[0].kind, v43::KindV43::Shell(s) if !s.even))
        .unwrap();
    (model, at)
}

#[test]
fn settings_out_of_range_are_brought_within_limits() {
    for (thickness, direction, expected) in [
        (f32::NAN, 7, ShellOptions::STANDARD_THICKNESS),
        (1e30, 255, ShellOptions::MAX_THICKNESS),
        (-2.0, 3, 2.0),
    ] {
        let (mut model, at) = model();
        let v43::KindV43::Shell(stored) = &mut model.modifiers[at].list[0].kind else {
            panic!("a shell")
        };
        stored.thickness = thickness;
        stored.direction = direction;
        let id = ObjectId::from_raw(model.modifiers[at].id);
        let back = model.into_document().unwrap();
        let ModifierKind::Shell(o) = &back.scene().modifiers(id)[0].kind else {
            panic!("a shell")
        };
        assert_eq!(o.thickness, expected);
        assert_eq!(o.direction, ShellDirection::Inward);
    }
}

#[test]
fn an_older_app_is_told_it_needs_updating() {
    let error = super::tests::refused(&header(FORMAT_VERSION + 1, 0, 0));
    assert!(matches!(error, ProjectError::TooNew { .. }));
}

#[test]
fn random_version_55_bodies_never_panic() {
    let mut seed = 0x55e1_1c0d_e5ee_7a75_u64;
    let good = v43::encode(&v43::ProjectV43::from_document(&rich_document_v55())).unwrap();
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
        let mut bytes = header(55, 0, body.len() as u64);
        bytes.extend_from_slice(&body);
        let _ = from_bytes(&bytes);
    }
}
