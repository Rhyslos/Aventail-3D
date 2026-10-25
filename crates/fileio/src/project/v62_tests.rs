//! Tests for version 62: version 61 with the Triangulate modifier
//! (decision 0161).

use super::tests::{assert_same, header};
use super::v61_tests::rich_document_v61;
use super::*;
use scene::{
    Command, ModifierKind, ObjectId, ObjectKind, Primitive, TriangulateMethod, TriangulateOptions,
};

/// Every Triangulate modifier in `doc` with its object, in tree order.
fn cuts(doc: &Document) -> Vec<(ObjectId, TriangulateOptions, bool)> {
    let mut out = Vec::new();
    for (id, _) in doc.scene().walk() {
        for m in doc.scene().modifiers(id) {
            if let ModifierKind::Triangulate(o) = &m.kind {
                out.push((id, *o, m.in_viewport));
            }
        }
    }
    out
}

fn cylinder(doc: &mut Document) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(Primitive::Cylinder(
        Default::default(),
    ))))
    .unwrap();
    doc.selection()[0]
}

/// The sample's odd settings: every one off its standard.
fn odd() -> TriangulateOptions {
    TriangulateOptions {
        method: TriangulateMethod::Fixed,
        quads_only: true,
    }
}

fn add(doc: &mut Document, id: ObjectId, kind: ModifierKind) {
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind,
    })
    .unwrap();
}

/// Everything version 61 stores, plus a cylinder with Weighted normals
/// and a standard Triangulate after it, and a cylinder with a Triangulate
/// with every setting changed, hidden in the viewport: the version 62
/// sample holds exactly this.
pub(super) fn rich_document_v62() -> Document {
    let mut doc = rich_document_v61();
    let cut = cylinder(&mut doc);
    add(
        &mut doc,
        cut,
        ModifierKind::WeightedNormals(Default::default()),
    );
    add(
        &mut doc,
        cut,
        ModifierKind::Triangulate(TriangulateOptions::default()),
    );
    let odd_one = cylinder(&mut doc);
    add(&mut doc, odd_one, ModifierKind::Triangulate(odd()));
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
fn triangulate_survives_a_round_trip() {
    let doc = rich_document_v62();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_same(&doc, &back);
    assert_eq!(cuts(&doc), cuts(&back));
    assert_eq!(cuts(&back).len(), 2);
    for (id, _, _) in cuts(&back) {
        assert_eq!(
            back.scene().exported_shape(id),
            doc.scene().exported_shape(id),
            "the shapes come out the same"
        );
    }
}

const SAMPLE_V62: &[u8] = include_bytes!("../../tests/data/sample-v62.avtl");

#[test]
fn a_file_saved_by_format_version_62_opens_with_its_triangulate() {
    let doc = from_bytes(SAMPLE_V62).expect("the version 62 sample opens");
    let expected = rich_document_v62();
    assert_same(&expected, &doc);
    let list = cuts(&doc);
    assert_eq!(list, cuts(&expected));
    assert_eq!(list.len(), 2);
    assert!(
        list.iter()
            .any(|(_, o, shown)| *o == TriangulateOptions::default() && *shown)
    );
    assert!(list.iter().any(|(_, o, shown)| *o == odd() && !*shown));
    for (id, _, _) in &list {
        assert_eq!(
            doc.scene().exported_shape(*id),
            expected.scene().exported_shape(*id)
        );
    }
}

const SAMPLE_V61: &[u8] = include_bytes!("../../tests/data/sample-v61.avtl");

#[test]
fn older_files_open_with_no_triangulate() {
    let doc = from_bytes(SAMPLE_V61).unwrap();
    assert!(cuts(&doc).is_empty());
    assert_same(&rich_document_v61(), &doc);
    let again = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_same(&doc, &again);
}

#[test]
fn an_unknown_method_reads_as_beauty() {
    for (stored, expected) in [
        (0, TriangulateMethod::Beauty),
        (1, TriangulateMethod::Fixed),
        (2, TriangulateMethod::Beauty),
        (255, TriangulateMethod::Beauty),
    ] {
        let mut model = v43::ProjectV43::from_document(&rich_document_v62());
        let at = model
            .modifiers
            .iter()
            .position(|m| matches!(&m.list[0].kind, v43::KindV43::Triangulate(o) if o.quads_only))
            .unwrap();
        let v43::KindV43::Triangulate(kept) = &mut model.modifiers[at].list[0].kind else {
            panic!("triangulate")
        };
        kept.method = stored;
        let id = ObjectId::from_raw(model.modifiers[at].id);
        let back = model.into_document().unwrap();
        let ModifierKind::Triangulate(o) = &back.scene().modifiers(id)[0].kind else {
            panic!("triangulate")
        };
        assert_eq!(o.method, expected);
        assert!(o.quads_only);
    }
}

#[test]
fn an_older_app_is_told_it_needs_updating() {
    let error = super::tests::refused(&header(FORMAT_VERSION + 1, 0, 0));
    assert!(matches!(error, ProjectError::TooNew { .. }));
}

#[test]
fn random_version_62_bodies_never_panic() {
    let mut seed = 0x62be_4d70_7a9e_0161_u64;
    let good = v43::encode(&v43::ProjectV43::from_document(&rich_document_v62())).unwrap();
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
        let mut bytes = header(62, 0, body.len() as u64);
        bytes.extend_from_slice(&body);
        let _ = from_bytes(&bytes);
    }
}
