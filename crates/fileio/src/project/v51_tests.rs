//! Tests for version 51: version 50 with the Quadify modifier and the
//! one-off Quadify (decision 0150).

use super::tests::{assert_same, header};
use super::v50_tests::rich_document_v50;
use super::*;
use scene::{
    Command, FaceId, Hardness, Modifier, ModifierKind, ObjectId, ObjectKind, Operation,
    PrimitiveKind, QuadifyOptions, SubdivideOptions,
};
use std::sync::Arc;

/// Every object's Quadify modifiers in `doc`, in tree order.
fn quadifies(doc: &Document) -> Vec<(ObjectId, Vec<Modifier>)> {
    doc.scene()
        .walk()
        .iter()
        .map(|(id, _)| {
            let list: Vec<Modifier> = doc
                .scene()
                .modifiers(*id)
                .iter()
                .filter(|m| matches!(m.kind, ModifierKind::Quadify(_)))
                .cloned()
                .collect();
            (*id, list)
        })
        .filter(|(_, list)| !list.is_empty())
        .collect()
}

/// Every recorded one-off Quadify in `doc`, by object, in tree order.
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
                .filter(|op| matches!(op.inner(), Operation::Quadify { .. }))
                .collect();
            (!list.is_empty()).then_some((*id, list))
        })
        .collect()
}

/// The sample's odd settings: every one off its standard.
fn odd() -> QuadifyOptions {
    QuadifyOptions {
        max_angle: 25.0,
        max_distortion: 60.0,
        keep_seams: false,
        keep_hard_edges: false,
        keep_materials: false,
    }
}

/// A plain mesh object: the standard `kind` cut into triangles.
fn triangulated(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    let mesh = mesh::decimate::Decimator::new(
        &kind.standard().mesh(),
        mesh::decimate::Keep::default(),
        Hardness::Marked,
    )
    .unwrap()
    .mesh(false);
    doc.execute(Command::Create(ObjectKind::Mesh(Arc::new(mesh))))
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

/// Everything version 50 stores, plus a triangulated sphere with a
/// standard Quadify before a Subdivide; a triangulated cube with a
/// Quadify with every setting changed, hidden in the viewport; a
/// triangulated cube with a one-off Quadify of the whole shape (shaded by
/// its marks) and one of two chosen polygons with odd settings (shaded
/// by angle): the version 51 sample holds exactly this.
pub(super) fn rich_document_v51() -> Document {
    let mut doc = rich_document_v50();
    let sphere = triangulated(&mut doc, PrimitiveKind::Sphere);
    add(
        &mut doc,
        sphere,
        ModifierKind::Subdivide(SubdivideOptions::default()),
    );
    add(
        &mut doc,
        sphere,
        ModifierKind::Quadify(QuadifyOptions::default()),
    );
    let cube = triangulated(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, cube, ModifierKind::Quadify(odd()));
    let mut hidden = doc.scene().modifiers(cube)[0].clone();
    hidden.in_viewport = false;
    doc.execute(Command::ChangeModifier {
        id: cube,
        index: 0,
        modifier: hidden,
    })
    .unwrap();
    let edited = triangulated(&mut doc, PrimitiveKind::Cube);
    doc.execute(Command::Record {
        id: edited,
        operation: Operation::Quadify {
            faces: vec![FaceId::from_raw(0), FaceId::from_raw(1)],
            options: odd(),
            hardness: Hardness::ByAngle(45.0),
            anchors: None,
        },
    })
    .unwrap();
    doc.execute(Command::Record {
        id: edited,
        operation: Operation::Quadify {
            faces: Vec::new(),
            options: QuadifyOptions::default(),
            hardness: Hardness::Marked,
            anchors: None,
        },
    })
    .unwrap();
    doc
}

#[test]
fn quadify_modifiers_and_edits_survive_a_round_trip() {
    let doc = rich_document_v51();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_same(&doc, &back);
    assert_eq!(quadifies(&doc), quadifies(&back));
    assert_eq!(quadifies(&back).len(), 2);
    assert_eq!(edits(&doc), edits(&back));
    assert_eq!(edits(&back).len(), 1);
    assert_eq!(edits(&back)[0].1.len(), 2);
    let ids = quadifies(&back)
        .into_iter()
        .map(|(id, _)| id)
        .chain(edits(&back).into_iter().map(|(id, _)| id));
    for id in ids {
        assert_eq!(
            back.scene().exported_shape(id),
            doc.scene().exported_shape(id),
            "the quadified shape comes out the same"
        );
    }
}

const SAMPLE_V51: &[u8] = include_bytes!("../../tests/data/sample-v51.avtl");

#[test]
fn a_file_saved_by_format_version_51_opens_with_its_quadifies() {
    let doc = from_bytes(SAMPLE_V51).expect("the version 51 sample opens");
    let expected = rich_document_v51();
    assert_same(&expected, &doc);
    assert_eq!(quadifies(&expected), quadifies(&doc));
    assert_eq!(edits(&expected), edits(&doc));
    let list: Vec<Modifier> = quadifies(&doc).into_iter().flat_map(|(_, l)| l).collect();
    let odd_ones: Vec<&Modifier> = list
        .iter()
        .filter(|m| matches!(&m.kind, ModifierKind::Quadify(o) if *o == odd()))
        .collect();
    assert_eq!(odd_ones.len(), 1);
    assert!(!odd_ones[0].in_viewport);
    // Quadify went before the Subdivide already there.
    let lists: Vec<Vec<&str>> = quadifies(&doc)
        .iter()
        .map(|(id, _)| {
            doc.scene()
                .modifiers(*id)
                .iter()
                .map(|m| m.kind.name())
                .collect()
        })
        .collect();
    assert!(lists.contains(&vec!["Quadify", "Subdivide"]), "{lists:?}");
    // The one-off edits keep their settings and the shading they saw.
    let (_, ops) = &edits(&doc)[0];
    assert!(matches!(
        ops[0],
        Operation::Quadify { options, hardness: Hardness::ByAngle(a), .. }
            if options == odd() && a == 45.0
    ));
    assert!(matches!(
        ops[1],
        Operation::Quadify { hardness: Hardness::Marked, ref faces, .. } if faces.is_empty()
    ));
}

const SAMPLE_V50: &[u8] = include_bytes!("../../tests/data/sample-v50.avtl");

#[test]
fn older_files_open_with_no_quadifies() {
    let doc = from_bytes(SAMPLE_V50).unwrap();
    assert!(quadifies(&doc).is_empty());
    assert!(edits(&doc).is_empty());
    assert_same(&rich_document_v50(), &doc);
    let again = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_same(&doc, &again);
}

/// The sample's file model, and the place of its object with the odd
/// Quadify.
fn model() -> (v43::ProjectV43, usize) {
    let model = v43::ProjectV43::from_document(&rich_document_v51());
    let at = model
        .modifiers
        .iter()
        .position(|m| matches!(&m.list[0].kind, v43::KindV43::Quadify(o) if !o.keep_seams))
        .unwrap();
    (model, at)
}

#[test]
fn settings_out_of_range_are_brought_within_limits() {
    let (mut model, at) = model();
    model.modifiers[at].list[0].kind = v43::KindV43::Quadify(super::v51::QuadifyV51 {
        max_angle: f32::NAN,
        max_distortion: 1e9,
        keep_seams: true,
        keep_hard_edges: true,
        keep_materials: true,
    });
    let id = ObjectId::from_raw(model.modifiers[at].id);
    let back = model.into_document().unwrap();
    let ModifierKind::Quadify(o) = &back.scene().modifiers(id)[0].kind else {
        panic!("a quadify")
    };
    assert_eq!(o.max_angle, QuadifyOptions::default().max_angle);
    assert_eq!(o.max_distortion, mesh::quadify::MAX_DISTORTION);
}

#[test]
fn broken_quadify_edits_are_refused_and_odd_ones_brought_within_limits() {
    use super::v1::FileOperation;
    use super::v37::OperationV37;
    use super::v51::{QuadifyEditV51, QuadifyV51};
    let stored = |max_angle: f32, smoothing_angle: Option<f32>| {
        OperationV37::Quadify(QuadifyEditV51 {
            faces: Default::default(),
            options: QuadifyV51 {
                max_angle,
                ..QuadifyV51::of(&QuadifyOptions::default())
            },
            smoothing_angle,
            anchors: None,
        })
    };
    for (angle, smoothing) in [
        (f32::NAN, None),
        (f32::INFINITY, None),
        (40.0, Some(f32::NAN)),
        (40.0, Some(500.0)),
        (40.0, Some(-1.0)),
    ] {
        assert!(
            stored(angle, smoothing).into_operation().is_err(),
            "{angle} {smoothing:?}"
        );
    }
    let Operation::Quadify { options, .. } = stored(1e6, None).into_operation().unwrap() else {
        panic!("a quadify")
    };
    assert_eq!(options.max_angle, mesh::quadify::MAX_ANGLE);
    let op = Operation::Quadify {
        faces: vec![FaceId::from_raw(3), FaceId::from_raw(7)],
        options: odd(),
        hardness: Hardness::ByAngle(30.0),
        anchors: None,
    };
    let file = OperationV37::to_file(&op).unwrap();
    assert!(matches!(file, OperationV37::Quadify(_)));
    assert_eq!(file.into_operation().unwrap(), op);
}

#[test]
fn an_older_app_is_told_it_needs_updating() {
    let error = super::tests::refused(&header(FORMAT_VERSION + 1, 0, 0));
    assert!(matches!(error, ProjectError::TooNew { .. }));
}

#[test]
fn random_version_51_bodies_never_panic() {
    let mut seed = 0x51a7_d9ab_fb41_bd6b_u64;
    let good = v43::encode(&v43::ProjectV43::from_document(&rich_document_v51())).unwrap();
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
        let mut bytes = header(51, 0, body.len() as u64);
        bytes.extend_from_slice(&body);
        let _ = from_bytes(&bytes);
    }
}
