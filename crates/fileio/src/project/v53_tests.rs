//! Tests for version 53: version 52 with the Weld by distance modifier and
//! the one-off Weld by distance now (decision 0152).

use super::tests::{assert_same, header};
use super::v52_tests::rich_document_v52;
use super::*;
use scene::{
    Command, Mesh, Modifier, ModifierKind, ObjectId, ObjectKind, Operation, SubdivideOptions,
    VertexId, WeldOptions,
};
use std::sync::Arc;

/// Every object's Weld by distance modifiers in `doc`, in tree order.
fn welds(doc: &Document) -> Vec<(ObjectId, Vec<Modifier>)> {
    doc.scene()
        .walk()
        .iter()
        .map(|(id, _)| {
            let list: Vec<Modifier> = doc
                .scene()
                .modifiers(*id)
                .iter()
                .filter(|m| matches!(m.kind, ModifierKind::WeldByDistance(_)))
                .cloned()
                .collect();
            (*id, list)
        })
        .filter(|(_, list)| !list.is_empty())
        .collect()
}

/// Every recorded one-off Weld by distance in `doc`, by object, in tree
/// order.
fn edits(doc: &Document) -> Vec<(ObjectId, Vec<Operation>)> {
    doc.scene()
        .walk()
        .iter()
        .filter_map(|(id, _)| {
            let object = doc.scene().object(*id)?;
            let list: Vec<Operation> = object
                .stack
                .layers()
                .iter()
                .flat_map(|layer| layer.entries.iter())
                .map(|entry| entry.operation.clone())
                .filter(|op| matches!(op.inner(), Operation::WeldByDistance { .. }))
                .collect();
            (!list.is_empty()).then_some((*id, list))
        })
        .collect()
}

/// The sample's odd settings: every one off its standard.
fn odd() -> WeldOptions {
    WeldOptions {
        distance: 2.5,
        only_borders: true,
        keep_seams: false,
    }
}

/// A cube 100 wide whose six sides each have their own four points.
fn split_cube() -> Mesh {
    let corners = [
        [0.0, 0.0, 0.0],
        [100.0, 0.0, 0.0],
        [100.0, 100.0, 0.0],
        [0.0, 100.0, 0.0],
        [0.0, 0.0, 100.0],
        [100.0, 0.0, 100.0],
        [100.0, 100.0, 100.0],
        [0.0, 100.0, 100.0],
    ];
    let sides: [[usize; 4]; 6] = [
        [0, 3, 2, 1],
        [4, 5, 6, 7],
        [0, 1, 5, 4],
        [1, 2, 6, 5],
        [2, 3, 7, 6],
        [3, 0, 4, 7],
    ];
    let mut mesh = Mesh::new();
    for side in sides {
        let ids: Vec<VertexId> = side
            .iter()
            .map(|c| mesh.add_vertex(glam::Vec3::from(corners[*c])).unwrap())
            .collect();
        mesh.add_face(&ids).unwrap();
    }
    mesh
}

fn split(doc: &mut Document) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Mesh(Arc::new(split_cube()))))
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

/// Everything version 52 stores, plus a split cube with a Subdivide and
/// then a standard Weld by distance (which goes before it); a split cube
/// with a Weld by distance with every setting changed, hidden in the
/// viewport; a split cube with a one-off Weld by distance of three chosen
/// points with odd settings and one of the whole shape: the version 53
/// sample holds exactly this.
pub(super) fn rich_document_v53() -> Document {
    let mut doc = rich_document_v52();
    let smooth = split(&mut doc);
    add(
        &mut doc,
        smooth,
        ModifierKind::Subdivide(SubdivideOptions::default()),
    );
    add(
        &mut doc,
        smooth,
        ModifierKind::WeldByDistance(WeldOptions::default()),
    );
    let odd_one = split(&mut doc);
    add(&mut doc, odd_one, ModifierKind::WeldByDistance(odd()));
    let mut hidden = doc.scene().modifiers(odd_one)[0].clone();
    hidden.in_viewport = false;
    doc.execute(Command::ChangeModifier {
        id: odd_one,
        index: 0,
        modifier: hidden,
    })
    .unwrap();
    let edited = split(&mut doc);
    doc.execute(Command::Record {
        id: edited,
        operation: Operation::WeldByDistance {
            points: vec![
                VertexId::from_raw(0),
                VertexId::from_raw(11),
                VertexId::from_raw(21),
            ],
            options: odd(),
            anchors: None,
        },
    })
    .unwrap();
    doc.execute(Command::Record {
        id: edited,
        operation: Operation::WeldByDistance {
            points: Vec::new(),
            options: WeldOptions::default(),
            anchors: None,
        },
    })
    .unwrap();
    doc
}

#[test]
fn weld_modifiers_and_edits_survive_a_round_trip() {
    let doc = rich_document_v53();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_same(&doc, &back);
    assert_eq!(welds(&doc), welds(&back));
    assert_eq!(welds(&back).len(), 2);
    assert_eq!(edits(&doc), edits(&back));
    assert_eq!(edits(&back).len(), 1);
    assert_eq!(edits(&back)[0].1.len(), 2);
    let ids = welds(&back)
        .into_iter()
        .map(|(id, _)| id)
        .chain(edits(&back).into_iter().map(|(id, _)| id));
    for id in ids {
        let shape = back.scene().exported_shape(id);
        assert_eq!(
            shape,
            doc.scene().exported_shape(id),
            "the welded shape comes out the same"
        );
        let report = mesh::MeshReport::new(&shape.unwrap());
        assert!(report.is_closed(), "every crack closed: {report:?}");
    }
}

const SAMPLE_V53: &[u8] = include_bytes!("../../tests/data/sample-v53.avtl");

#[test]
fn a_file_saved_by_format_version_53_opens_with_its_welds() {
    let doc = from_bytes(SAMPLE_V53).expect("the version 53 sample opens");
    let expected = rich_document_v53();
    assert_same(&expected, &doc);
    assert_eq!(welds(&expected), welds(&doc));
    assert_eq!(edits(&expected), edits(&doc));
    let list: Vec<Modifier> = welds(&doc).into_iter().flat_map(|(_, l)| l).collect();
    let odd_ones: Vec<&Modifier> = list
        .iter()
        .filter(|m| matches!(&m.kind, ModifierKind::WeldByDistance(o) if *o == odd()))
        .collect();
    assert_eq!(odd_ones.len(), 1);
    assert!(!odd_ones[0].in_viewport);
    // The weld went before the Subdivide already there.
    let lists: Vec<Vec<&str>> = welds(&doc)
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
        lists.contains(&vec!["Weld by distance", "Subdivide"]),
        "{lists:?}"
    );
    // The one-off edits keep their points and settings.
    let (_, ops) = &edits(&doc)[0];
    assert!(matches!(
        &ops[0],
        Operation::WeldByDistance { options, points, .. } if *options == odd() && points.len() == 3
    ));
    assert!(matches!(
        &ops[1],
        Operation::WeldByDistance { points, .. } if points.is_empty()
    ));
}

const SAMPLE_V52: &[u8] = include_bytes!("../../tests/data/sample-v52.avtl");

#[test]
fn older_files_open_with_no_welds() {
    let doc = from_bytes(SAMPLE_V52).unwrap();
    assert!(welds(&doc).is_empty());
    assert!(edits(&doc).is_empty());
    assert_same(&rich_document_v52(), &doc);
    let again = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_same(&doc, &again);
}

/// The sample's file model, and the place of its object with the odd
/// weld.
fn model() -> (v43::ProjectV43, usize) {
    let model = v43::ProjectV43::from_document(&rich_document_v53());
    let at = model
        .modifiers
        .iter()
        .position(|m| matches!(&m.list[0].kind, v43::KindV43::WeldByDistance(o) if o.only_borders))
        .unwrap();
    (model, at)
}

#[test]
fn settings_out_of_range_are_brought_within_limits() {
    for (stored, read) in [
        (f32::NAN, WeldOptions::STANDARD_DISTANCE),
        (f32::NEG_INFINITY, WeldOptions::STANDARD_DISTANCE),
        (1e9, WeldOptions::MAX_DISTANCE),
        (-4.0, 0.0),
    ] {
        let (mut model, at) = model();
        model.modifiers[at].list[0].kind = v43::KindV43::WeldByDistance(super::v53::WeldV53 {
            distance: stored,
            only_borders: false,
            keep_seams: true,
        });
        let id = ObjectId::from_raw(model.modifiers[at].id);
        let back = model.into_document().unwrap();
        let ModifierKind::WeldByDistance(o) = &back.scene().modifiers(id)[0].kind else {
            panic!("a weld")
        };
        assert_eq!(o.distance, read, "{stored}");
    }
}

#[test]
fn broken_weld_edits_are_refused_and_odd_ones_brought_within_limits() {
    use super::v1::FileOperation;
    use super::v37::OperationV37;
    use super::v53::{WeldEditV53, WeldV53};
    let stored = |distance: f32| {
        OperationV37::WeldByDistance(WeldEditV53 {
            points: Default::default(),
            options: WeldV53 {
                distance,
                ..WeldV53::of(&WeldOptions::default())
            },
            anchors: None,
        })
    };
    for distance in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(stored(distance).into_operation().is_err(), "{distance}");
    }
    let Operation::WeldByDistance { options, .. } = stored(1e6).into_operation().unwrap() else {
        panic!("a weld")
    };
    assert_eq!(options.distance, WeldOptions::MAX_DISTANCE);
    let op = Operation::WeldByDistance {
        points: vec![VertexId::from_raw(3), VertexId::from_raw(7)],
        options: odd(),
        anchors: None,
    };
    let file = OperationV37::to_file(&op).unwrap();
    assert!(matches!(file, OperationV37::WeldByDistance(_)));
    assert_eq!(file.into_operation().unwrap(), op);
}

#[test]
fn an_older_app_is_told_it_needs_updating() {
    let error = super::tests::refused(&header(FORMAT_VERSION + 1, 0, 0));
    assert!(matches!(error, ProjectError::TooNew { .. }));
}

#[test]
fn random_version_53_bodies_never_panic() {
    let mut seed = 0x53e1_d0b1_d157_a9ce_u64;
    let good = v43::encode(&v43::ProjectV43::from_document(&rich_document_v53())).unwrap();
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
        let mut bytes = header(53, 0, body.len() as u64);
        bytes.extend_from_slice(&body);
        let _ = from_bytes(&bytes);
    }
}
