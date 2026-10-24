//! Tests for version 44: version 43 with the Subdivide modifier and
//! creases (decision 0143).

use super::tests::{assert_same, header};
use super::v43_tests::rich_document_v43;
use super::*;
use scene::{
    Command, EdgeKey, Modifier, ModifierKind, ObjectId, ObjectKind, Operation, PrimitiveKind,
    SubdivideOptions,
};
use std::sync::Arc;

/// Every object's modifiers in `doc`, in tree order.
fn modifiers(doc: &Document) -> Vec<(ObjectId, Vec<Modifier>)> {
    doc.scene()
        .walk()
        .iter()
        .map(|(id, _)| (*id, doc.scene().modifiers(*id).to_vec()))
        .filter(|(_, list)| !list.is_empty())
        .collect()
}

/// Every object's creases in `doc` (after its edits), in tree order.
fn creases(doc: &Document) -> Vec<(ObjectId, Vec<(EdgeKey, f32)>)> {
    doc.scene()
        .walk()
        .iter()
        .filter_map(|(id, _)| {
            let shape = doc.scene().evaluated(*id)?;
            let list = mesh::crease::creases(&shape.mesh);
            (!list.is_empty()).then_some((*id, list))
        })
        .collect()
}

/// Everything version 43 stores, plus a cube with a Subdivide modifier
/// (its own levels, hard edges not kept sharp) and recorded creases (one
/// full, two partial), and a plain mesh whose own shape holds creases:
/// the version 44 sample holds exactly this.
pub(super) fn rich_document_v44() -> Document {
    let mut doc = rich_document_v43();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let cube = doc.selection()[0];
    doc.execute(Command::AddModifier {
        ids: vec![cube],
        kind: ModifierKind::Subdivide(SubdivideOptions {
            levels: 2,
            export_levels: 3,
            hard_edges_sharp: false,
        }),
    })
    .unwrap();
    let shape = doc.scene().evaluated(cube).unwrap().mesh;
    let edges: Vec<EdgeKey> = shape.topology().edges().collect();
    for (range, crease) in [(0..1, 1.0), (1..3, 0.25)] {
        doc.execute(Command::Record {
            id: cube,
            operation: Operation::SetCreases {
                edges: edges[range].to_vec(),
                crease,
                anchors: None,
            },
        })
        .unwrap();
    }
    let mut plain = PrimitiveKind::Cylinder.standard().mesh();
    let rim: Vec<EdgeKey> = plain.topology().edges().take(4).collect();
    mesh::crease::set_creases(&mut plain, &rim, 0.75);
    doc.execute(Command::Create(ObjectKind::Mesh(Arc::new(plain))))
        .unwrap();
    doc
}

#[test]
fn subdivide_and_creases_survive_a_round_trip() {
    let doc = rich_document_v44();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], 44u32.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_same(&doc, &back);
    assert_eq!(modifiers(&doc), modifiers(&back));
    assert_eq!(creases(&doc), creases(&back));
    // The cube's edits, the new mesh's own, and the older test mesh's
    // "crease" layer (named so from the start, decision 0043).
    assert_eq!(creases(&back).len(), 3);
    let (cube, _) = modifiers(&back)
        .into_iter()
        .find(|(_, list)| matches!(list[0].kind, ModifierKind::Subdivide(_)))
        .expect("the smoothed cube");
    assert_eq!(
        back.scene().exported_shape(cube),
        doc.scene().exported_shape(cube),
        "the smoothed cube comes out the same"
    );
}

const SAMPLE_V44: &[u8] = include_bytes!("../../tests/data/sample-v44.avtl");

#[test]
fn a_file_saved_by_format_version_44_opens_with_its_creases() {
    let doc = from_bytes(SAMPLE_V44).expect("the version 44 sample opens");
    let expected = rich_document_v44();
    assert_same(&expected, &doc);
    assert_eq!(modifiers(&expected), modifiers(&doc));
    assert_eq!(creases(&expected), creases(&doc));
}

const SAMPLE_V43: &[u8] = include_bytes!("../../tests/data/sample-v43.avtl");

#[test]
fn older_files_open_as_they_were() {
    let doc = from_bytes(SAMPLE_V43).unwrap();
    // The only creases are the test mesh's own layer, kept as it was.
    assert_eq!(creases(&doc), creases(&rich_document_v43()));
    assert_eq!(creases(&doc).len(), 1);
    assert_same(&rich_document_v43(), &doc);
    // And save again as version 44.
    let again = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_same(&doc, &again);
}

#[test]
fn levels_out_of_range_are_brought_within_limits() {
    let doc = rich_document_v44();
    let mut model = v43::ProjectV43::from_document(&doc);
    let last = model
        .modifiers
        .iter()
        .position(|m| matches!(m.list[0].kind, v43::KindV43::Subdivide { .. }))
        .unwrap();
    model.modifiers[last].list[0].kind = v43::KindV43::Subdivide {
        levels: 99,
        export_levels: u32::MAX,
        hard_edges_sharp: true,
    };
    let back = model.into_document().unwrap();
    let (_, list) = modifiers(&back)
        .into_iter()
        .find(|(_, list)| matches!(list[0].kind, ModifierKind::Subdivide(_)))
        .unwrap();
    assert_eq!(
        list[0].kind,
        ModifierKind::Subdivide(SubdivideOptions {
            levels: SubdivideOptions::MAX_LEVELS,
            export_levels: SubdivideOptions::MAX_LEVELS,
            hard_edges_sharp: true,
        })
    );
}

#[test]
fn broken_creases_are_refused() {
    use super::v1::FileOperation;
    use super::v37::OperationV37;
    for crease in [f32::NAN, 1.5, -0.5, f32::INFINITY] {
        let bad = OperationV37::SetCreases {
            edges: Default::default(),
            crease,
            anchors: None,
        };
        assert!(bad.into_operation().is_err(), "{crease}");
    }
    let to_itself = OperationV37::SetCreases {
        edges: [[3, 3]].into_iter().collect(),
        crease: 1.0,
        anchors: None,
    };
    assert!(to_itself.into_operation().is_err());
    // A good one survives the trip, stored as version 44's own.
    let op = Operation::SetCreases {
        edges: vec![EdgeKey::new(
            scene::VertexId::from_raw(1),
            scene::VertexId::from_raw(2),
        )],
        crease: 0.5,
        anchors: None,
    };
    let file = OperationV37::to_file(&op).unwrap();
    assert!(matches!(file, OperationV37::SetCreases { .. }));
    assert_eq!(file.into_operation().unwrap(), op);
}

#[test]
fn an_older_app_is_told_it_needs_updating() {
    let error = super::tests::refused(&header(FORMAT_VERSION + 1, 0, 0));
    assert!(matches!(error, ProjectError::TooNew { .. }));
}

#[test]
fn random_version_44_bodies_never_panic() {
    let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
    let good = v43::encode(&v43::ProjectV43::from_document(&rich_document_v44())).unwrap();
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
        let mut bytes = header(44, 0, body.len() as u64);
        bytes.extend_from_slice(&body);
        let _ = from_bytes(&bytes);
    }
}
