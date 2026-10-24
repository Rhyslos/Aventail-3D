//! Tests for version 49: version 48 with the Weighted normals modifier
//! (decision 0148), and custom normals baked into a shape.

use super::tests::{assert_same, header};
use super::v48_tests::rich_document_v48;
use super::*;
use scene::{
    Command, Modifier, ModifierKind, ObjectId, ObjectKind, PrimitiveKind, WeightBy,
    WeightedNormalsOptions,
};

/// Every object's Weighted normals modifiers in `doc`, in tree order.
fn weighted(doc: &Document) -> Vec<(ObjectId, Vec<Modifier>)> {
    doc.scene()
        .walk()
        .iter()
        .map(|(id, _)| {
            let list: Vec<Modifier> = doc
                .scene()
                .modifiers(*id)
                .iter()
                .filter(|m| matches!(m.kind, ModifierKind::WeightedNormals(_)))
                .cloned()
                .collect();
            (*id, list)
        })
        .filter(|(_, list)| !list.is_empty())
        .collect()
}

/// The objects whose own shape (before modifiers) has custom normals.
fn baked(doc: &Document) -> Vec<ObjectId> {
    doc.scene()
        .walk()
        .iter()
        .map(|(id, _)| *id)
        .filter(|id| {
            doc.scene()
                .evaluated(*id)
                .is_some_and(|e| mesh::custom_normals::has_custom_normals(&e.mesh))
        })
        .collect()
}

/// The sample's odd settings: every one off its standard.
fn odd() -> WeightedNormalsOptions {
    WeightedNormalsOptions {
        weight_by: WeightBy::AreaAndAngle,
        strength: 0.6,
        influence: 0.25,
        keep_hard_edges: false,
    }
}

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

fn add(doc: &mut Document, id: ObjectId, options: WeightedNormalsOptions) {
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind: ModifierKind::WeightedNormals(options),
    })
    .unwrap();
}

/// Everything version 48 stores, plus a cube with a standard Weighted
/// normals modifier, a cylinder with one with every setting changed,
/// hidden in the viewport, and a capsule whose weighted normals were
/// baked into its shape: the version 49 sample holds exactly this.
pub(super) fn rich_document_v49() -> Document {
    let mut doc = rich_document_v48();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, cube, WeightedNormalsOptions::default());
    let cylinder = create(&mut doc, PrimitiveKind::Cylinder);
    add(&mut doc, cylinder, odd());
    let mut hidden = doc.scene().modifiers(cylinder)[0].clone();
    hidden.in_viewport = false;
    doc.execute(Command::ChangeModifier {
        id: cylinder,
        index: 0,
        modifier: hidden,
    })
    .unwrap();
    let capsule = create(&mut doc, PrimitiveKind::Capsule);
    add(&mut doc, capsule, WeightedNormalsOptions::default());
    doc.execute(Command::MakeEditable(vec![capsule])).unwrap();
    doc
}

#[test]
fn weighted_normals_survive_a_round_trip() {
    let doc = rich_document_v49();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_same(&doc, &back);
    assert_eq!(weighted(&doc), weighted(&back));
    assert_eq!(weighted(&back).len(), 2);
    for (id, _) in weighted(&back) {
        assert_eq!(
            back.scene().exported_shape(id),
            doc.scene().exported_shape(id),
            "the shape and its normals come out the same"
        );
    }
}

#[test]
fn baked_custom_normals_survive_a_round_trip() {
    let doc = rich_document_v49();
    assert_eq!(baked(&doc).len(), 1);
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(baked(&back), baked(&doc));
    let id = baked(&back)[0];
    assert_eq!(
        mesh::corner_normal_list(&back.scene().shown_shape(id).unwrap()),
        mesh::corner_normal_list(&doc.scene().shown_shape(id).unwrap())
    );
}

const SAMPLE_V49: &[u8] = include_bytes!("../../tests/data/sample-v49.avtl");

#[test]
fn a_file_saved_by_format_version_49_opens_with_its_weighted_normals() {
    let doc = from_bytes(SAMPLE_V49).expect("the version 49 sample opens");
    let expected = rich_document_v49();
    assert_same(&expected, &doc);
    assert_eq!(weighted(&expected), weighted(&doc));
    assert_eq!(baked(&doc).len(), 1);
    let list: Vec<Modifier> = weighted(&doc).into_iter().flat_map(|(_, l)| l).collect();
    let odd: Vec<&Modifier> = list
        .iter()
        .filter(|m| matches!(&m.kind, ModifierKind::WeightedNormals(o) if *o == odd()))
        .collect();
    assert_eq!(odd.len(), 1);
    assert!(!odd[0].in_viewport);
}

const SAMPLE_V48: &[u8] = include_bytes!("../../tests/data/sample-v48.avtl");

#[test]
fn older_files_open_with_no_weighted_normals() {
    let doc = from_bytes(SAMPLE_V48).unwrap();
    assert!(weighted(&doc).is_empty());
    assert!(baked(&doc).is_empty());
    assert_same(&rich_document_v48(), &doc);
    let again = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_same(&doc, &again);
}

/// The sample's file model, and the place of its last object with
/// Weighted normals.
fn model() -> (v43::ProjectV43, usize) {
    let model = v43::ProjectV43::from_document(&rich_document_v49());
    let at = model
        .modifiers
        .iter()
        .rposition(|m| matches!(m.list[0].kind, v43::KindV43::WeightedNormals { .. }))
        .unwrap();
    (model, at)
}

#[test]
fn settings_out_of_range_are_brought_within_limits() {
    let (mut model, at) = model();
    model.modifiers[at].list[0].kind = v43::KindV43::WeightedNormals {
        weight_by: 77,
        strength: f32::NAN,
        influence: -3.0,
        keep_hard_edges: true,
    };
    let id = ObjectId::from_raw(model.modifiers[at].id);
    let back = model.into_document().unwrap();
    let ModifierKind::WeightedNormals(o) = &back.scene().modifiers(id)[0].kind else {
        panic!("weighted normals")
    };
    assert_eq!(o.weight_by, WeightBy::Area);
    assert_eq!(o.strength, WeightedNormalsOptions::default().strength);
    assert_eq!(o.influence, 0.0);
}

#[test]
fn an_older_app_is_told_it_needs_updating() {
    let error = super::tests::refused(&header(FORMAT_VERSION + 1, 0, 0));
    assert!(matches!(error, ProjectError::TooNew { .. }));
}

#[test]
fn random_version_49_bodies_never_panic() {
    let mut seed = 0x5be0_cd19_137e_2179_u64;
    let good = v43::encode(&v43::ProjectV43::from_document(&rich_document_v49())).unwrap();
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
        let mut bytes = header(49, 0, body.len() as u64);
        bytes.extend_from_slice(&body);
        let _ = from_bytes(&bytes);
    }
}
