//! Tests for version 50: version 49 with the Live bevel modifier and
//! bevel weights (decision 0149).

use super::tests::{assert_same, header};
use super::v49_tests::rich_document_v49;
use super::*;
use scene::{
    BevelEdges, Command, EdgeKey, LiveBevelOptions, Modifier, ModifierKind, ObjectId, ObjectKind,
    Operation, PrimitiveKind, WeightedNormalsOptions,
};
use std::sync::Arc;

/// Every object's Live bevel modifiers in `doc`, in tree order.
fn bevels(doc: &Document) -> Vec<(ObjectId, Vec<Modifier>)> {
    doc.scene()
        .walk()
        .iter()
        .map(|(id, _)| {
            let list: Vec<Modifier> = doc
                .scene()
                .modifiers(*id)
                .iter()
                .filter(|m| matches!(m.kind, ModifierKind::LiveBevel(_)))
                .cloned()
                .collect();
            (*id, list)
        })
        .filter(|(_, list)| !list.is_empty())
        .collect()
}

/// Every object's bevel weights in `doc` (after its edits), in tree order.
fn weights(doc: &Document) -> Vec<(ObjectId, Vec<(EdgeKey, f32)>)> {
    doc.scene()
        .walk()
        .iter()
        .filter_map(|(id, _)| {
            let shape = doc.scene().evaluated(*id)?;
            let list = mesh::bevel_weight::bevel_weights(&shape.mesh);
            (!list.is_empty()).then_some((*id, list))
        })
        .collect()
}

/// The sample's odd settings: every one off its standard.
fn odd() -> LiveBevelOptions {
    LiveBevelOptions {
        edges: BevelEdges::Marked,
        angle: 45.0,
        width: 3.5,
        segments: 3,
        profile: 0.25,
        clamp: false,
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

/// Everything version 49 stores, plus a cube with a standard Live bevel
/// and Weighted normals after it; a cube with a Live bevel with every
/// setting changed, hidden in the viewport, and recorded bevel weights
/// (one full, two partial); a cylinder bevelling its hard edges; and a
/// plain mesh whose own shape holds bevel weights: the version 50 sample
/// holds exactly this.
pub(super) fn rich_document_v50() -> Document {
    let mut doc = rich_document_v49();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    add(
        &mut doc,
        cube,
        ModifierKind::WeightedNormals(WeightedNormalsOptions::default()),
    );
    add(
        &mut doc,
        cube,
        ModifierKind::LiveBevel(LiveBevelOptions::default()),
    );
    let marked = create(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, marked, ModifierKind::LiveBevel(odd()));
    let mut hidden = doc.scene().modifiers(marked)[0].clone();
    hidden.in_viewport = false;
    doc.execute(Command::ChangeModifier {
        id: marked,
        index: 0,
        modifier: hidden,
    })
    .unwrap();
    let shape = doc.scene().evaluated(marked).unwrap().mesh;
    let edges: Vec<EdgeKey> = shape.topology().edges().collect();
    for (range, weight) in [(0..1, 1.0), (1..3, 0.4)] {
        doc.execute(Command::Record {
            id: marked,
            operation: Operation::SetBevelWeights {
                edges: edges[range].to_vec(),
                weight,
                anchors: None,
            },
        })
        .unwrap();
    }
    let cylinder = create(&mut doc, PrimitiveKind::Cylinder);
    add(
        &mut doc,
        cylinder,
        ModifierKind::LiveBevel(LiveBevelOptions {
            edges: BevelEdges::Hard,
            ..LiveBevelOptions::default()
        }),
    );
    let mut plain = mesh::Primitive::Cube(Default::default()).mesh();
    let some: Vec<EdgeKey> = plain.topology().edges().take(4).collect();
    mesh::bevel_weight::set_bevel_weights(&mut plain, &some, 0.75);
    doc.execute(Command::Create(ObjectKind::Mesh(Arc::new(plain))))
        .unwrap();
    doc
}

#[test]
fn live_bevels_and_bevel_weights_survive_a_round_trip() {
    let doc = rich_document_v50();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_same(&doc, &back);
    assert_eq!(bevels(&doc), bevels(&back));
    assert_eq!(bevels(&back).len(), 3);
    assert_eq!(weights(&doc), weights(&back));
    assert_eq!(weights(&back).len(), 2);
    for (id, _) in bevels(&back) {
        assert_eq!(
            back.scene().exported_shape(id),
            doc.scene().exported_shape(id),
            "the bevelled shape comes out the same"
        );
    }
}

const SAMPLE_V50: &[u8] = include_bytes!("../../tests/data/sample-v50.avtl");

#[test]
fn a_file_saved_by_format_version_50_opens_with_its_live_bevels() {
    let doc = from_bytes(SAMPLE_V50).expect("the version 50 sample opens");
    let expected = rich_document_v50();
    assert_same(&expected, &doc);
    assert_eq!(bevels(&expected), bevels(&doc));
    assert_eq!(weights(&expected), weights(&doc));
    let list: Vec<Modifier> = bevels(&doc).into_iter().flat_map(|(_, l)| l).collect();
    let odd: Vec<&Modifier> = list
        .iter()
        .filter(|m| matches!(&m.kind, ModifierKind::LiveBevel(o) if *o == odd()))
        .collect();
    assert_eq!(odd.len(), 1);
    assert!(!odd[0].in_viewport);
    // The bevel comes first, Weighted normals after it.
    let lists: Vec<Vec<&str>> = bevels(&doc)
        .iter()
        .map(|(id, _)| {
            doc.scene()
                .modifiers(*id)
                .iter()
                .map(|m| m.kind.name())
                .collect()
        })
        .collect();
    assert!(
        lists.contains(&vec!["Live bevel", "Weighted normals"]),
        "{lists:?}"
    );
}

const SAMPLE_V49: &[u8] = include_bytes!("../../tests/data/sample-v49.avtl");

#[test]
fn older_files_open_with_no_live_bevels() {
    let doc = from_bytes(SAMPLE_V49).unwrap();
    assert!(bevels(&doc).is_empty());
    assert!(weights(&doc).is_empty());
    assert_same(&rich_document_v49(), &doc);
    let again = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_same(&doc, &again);
}

/// The sample's file model, and the place of its object with the odd
/// Live bevel.
fn model() -> (v43::ProjectV43, usize) {
    let model = v43::ProjectV43::from_document(&rich_document_v50());
    let at = model
        .modifiers
        .iter()
        .position(|m| matches!(&m.list[0].kind, v43::KindV43::LiveBevel(o) if o.edges == 2))
        .unwrap();
    (model, at)
}

#[test]
fn settings_out_of_range_are_brought_within_limits() {
    let (mut model, at) = model();
    model.modifiers[at].list[0].kind = v43::KindV43::LiveBevel(super::v50::LiveBevelV50 {
        edges: 99,
        angle: f32::NAN,
        width: -4.0,
        segments: 100_000,
        profile: f32::INFINITY,
        clamp: true,
    });
    let id = ObjectId::from_raw(model.modifiers[at].id);
    let back = model.into_document().unwrap();
    let ModifierKind::LiveBevel(o) = &back.scene().modifiers(id)[0].kind else {
        panic!("a live bevel")
    };
    assert_eq!(o.edges, BevelEdges::ByAngle);
    assert_eq!(o.angle, LiveBevelOptions::default().angle);
    assert_eq!(o.width, 0.0);
    assert_eq!(o.segments, mesh::bevel::MAX_SEGMENTS);
    assert_eq!(o.profile, LiveBevelOptions::default().profile);
}

#[test]
fn broken_bevel_weights_are_refused() {
    use super::v1::FileOperation;
    use super::v37::OperationV37;
    for weight in [f32::NAN, 1.5, -0.5, f32::INFINITY] {
        let bad = OperationV37::SetBevelWeights {
            edges: Default::default(),
            weight,
            anchors: None,
        };
        assert!(bad.into_operation().is_err(), "{weight}");
    }
    let to_itself = OperationV37::SetBevelWeights {
        edges: [[3, 3]].into_iter().collect(),
        weight: 1.0,
        anchors: None,
    };
    assert!(to_itself.into_operation().is_err());
    let op = Operation::SetBevelWeights {
        edges: vec![EdgeKey::new(
            scene::VertexId::from_raw(1),
            scene::VertexId::from_raw(2),
        )],
        weight: 0.5,
        anchors: None,
    };
    let file = OperationV37::to_file(&op).unwrap();
    assert!(matches!(file, OperationV37::SetBevelWeights { .. }));
    assert_eq!(file.into_operation().unwrap(), op);
}

#[test]
fn an_older_app_is_told_it_needs_updating() {
    let error = super::tests::refused(&header(FORMAT_VERSION + 1, 0, 0));
    assert!(matches!(error, ProjectError::TooNew { .. }));
}

#[test]
fn random_version_50_bodies_never_panic() {
    let mut seed = 0x1f83_d9ab_fb41_bd6b_u64;
    let good = v43::encode(&v43::ProjectV43::from_document(&rich_document_v50())).unwrap();
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
        let mut bytes = header(50, 0, body.len() as u64);
        bytes.extend_from_slice(&body);
        let _ = from_bytes(&bytes);
    }
}
