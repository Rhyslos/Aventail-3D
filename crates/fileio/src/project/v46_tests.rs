//! Tests for version 46: version 45 with the Decimate modifier
//! (decision 0145).

use super::tests::{assert_same, header};
use super::v45_tests::rich_document_v45;
use super::*;
use scene::{
    Command, DecimateKeep, DecimateOptions, Modifier, ModifierKind, ObjectId, ObjectKind,
    PrimitiveKind, ReduceBy,
};

/// Every object's Decimate modifiers in `doc`, in tree order.
fn decimations(doc: &Document) -> Vec<(ObjectId, Vec<Modifier>)> {
    doc.scene()
        .walk()
        .iter()
        .map(|(id, _)| {
            let list: Vec<Modifier> = doc
                .scene()
                .modifiers(*id)
                .iter()
                .filter(|m| matches!(m.kind, ModifierKind::Decimate(_)))
                .cloned()
                .collect();
            (*id, list)
        })
        .filter(|(_, list)| !list.is_empty())
        .collect()
}

/// The settings the sample's second Decimate modifier has: every one off
/// its standard.
fn odd_settings() -> DecimateOptions {
    DecimateOptions {
        reduce_by: ReduceBy::Triangles,
        ratio: 0.3,
        triangles: 120,
        keep: DecimateKeep {
            seams: false,
            hard_edges: true,
            materials: false,
            borders: true,
        },
        quads_where_flat: true,
    }
}

/// Everything version 45 stores, plus a sphere with a standard Decimate
/// modifier and a cylinder with one of every setting changed, switched
/// off in the viewport: the version 46 sample holds exactly this.
pub(super) fn rich_document_v46() -> Document {
    let mut doc = rich_document_v45();
    for (kind, options) in [
        (PrimitiveKind::Sphere, DecimateOptions::default()),
        (PrimitiveKind::Cylinder, odd_settings()),
    ] {
        doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
            .unwrap();
        let id = doc.selection()[0];
        doc.execute(Command::AddModifier {
            ids: vec![id],
            kind: ModifierKind::Decimate(options),
        })
        .unwrap();
        if options != DecimateOptions::default() {
            let mut hidden = doc.scene().modifiers(id)[0].clone();
            hidden.in_viewport = false;
            doc.execute(Command::ChangeModifier {
                id,
                index: 0,
                modifier: hidden,
            })
            .unwrap();
        }
    }
    doc
}

#[test]
fn decimate_modifiers_survive_a_round_trip() {
    let doc = rich_document_v46();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_same(&doc, &back);
    assert_eq!(decimations(&doc), decimations(&back));
    assert_eq!(decimations(&back).len(), 2);
    for (id, _) in decimations(&back) {
        assert_eq!(
            back.scene().exported_shape(id),
            doc.scene().exported_shape(id),
            "the decimated shape comes out the same"
        );
    }
}

const SAMPLE_V46: &[u8] = include_bytes!("../../tests/data/sample-v46.avtl");

#[test]
fn a_file_saved_by_format_version_46_opens_with_its_decimation() {
    let doc = from_bytes(SAMPLE_V46).expect("the version 46 sample opens");
    let expected = rich_document_v46();
    assert_same(&expected, &doc);
    assert_eq!(decimations(&expected), decimations(&doc));
    let odd: Vec<Modifier> = decimations(&doc)
        .into_iter()
        .flat_map(|(_, list)| list)
        .filter(|m| m.kind == ModifierKind::Decimate(odd_settings()))
        .collect();
    assert_eq!(odd.len(), 1);
    assert!(!odd[0].in_viewport);
}

const SAMPLE_V45: &[u8] = include_bytes!("../../tests/data/sample-v45.avtl");

#[test]
fn older_files_open_with_no_decimation() {
    let doc = from_bytes(SAMPLE_V45).unwrap();
    assert!(decimations(&doc).is_empty());
    assert_same(&rich_document_v45(), &doc);
    // And save again as version 46.
    let again = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_same(&doc, &again);
}

#[test]
fn settings_out_of_range_are_brought_within_limits() {
    let doc = rich_document_v46();
    let model = v43::ProjectV43::from_document(&doc);
    let last = model
        .modifiers
        .iter()
        .rposition(|m| matches!(m.list[0].kind, v43::KindV43::Decimate { .. }))
        .unwrap();
    for (ratio, triangles, expected) in [
        (
            f32::NAN,
            0,
            (
                DecimateOptions::STANDARD_RATIO,
                DecimateOptions::MIN_TRIANGLES,
            ),
        ),
        (
            -1.0,
            u32::MAX,
            (DecimateOptions::MIN_RATIO, DecimateOptions::MAX_TRIANGLES),
        ),
        (f32::INFINITY, 7, (DecimateOptions::STANDARD_RATIO, 7)),
        (40.0, 7, (1.0, 7)),
    ] {
        let mut model = model.clone();
        model.modifiers[last].list[0].kind = v43::KindV43::Decimate {
            by_triangles: false,
            ratio,
            triangles,
            keep_seams: true,
            keep_hard_edges: true,
            keep_materials: true,
            keep_borders: true,
            quads_where_flat: false,
        };
        let id = ObjectId::from_raw(model.modifiers[last].id);
        let back = model.into_document().unwrap();
        let list = back.scene().modifiers(id);
        let ModifierKind::Decimate(o) = list[0].kind else {
            panic!("decimate")
        };
        assert_eq!((o.ratio, o.triangles), expected);
    }
}

#[test]
fn an_older_app_is_told_it_needs_updating() {
    let error = super::tests::refused(&header(FORMAT_VERSION + 1, 0, 0));
    assert!(matches!(error, ProjectError::TooNew { .. }));
}

#[test]
fn random_version_46_bodies_never_panic() {
    let mut seed = 0x6a09_e667_f3bc_c908_u64;
    let good = v43::encode(&v43::ProjectV43::from_document(&rich_document_v46())).unwrap();
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
        let mut bytes = header(46, 0, body.len() as u64);
        bytes.extend_from_slice(&body);
        let _ = from_bytes(&bytes);
    }
}
