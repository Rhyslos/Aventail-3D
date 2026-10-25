//! Tests for version 57: version 56 with the Live bevel's *Soft bevels*
//! switch, which the game bevel turns on (decision 0156).

use super::tests::{assert_same, header};
use super::v56_tests::rich_document_v56;
use super::*;
use scene::{
    Command, Hardness, LiveBevelOptions, ModifierKind, ObjectId, ObjectKind, PrimitiveKind,
    WeightedNormalsOptions,
};

/// Every Live bevel in `doc` with its object, in tree order.
fn bevels(doc: &Document) -> Vec<(ObjectId, LiveBevelOptions)> {
    let mut out = Vec::new();
    for (id, _) in doc.scene().walk() {
        for m in doc.scene().modifiers(id) {
            if let ModifierKind::LiveBevel(o) = m.kind {
                out.push((id, o));
            }
        }
    }
    out
}

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

/// Everything version 56 stores, plus a cube and an arch given the game
/// bevel (each shaded by its marked edges, its look kept as a recorded
/// edit, a Live bevel with Soft bevels on and Weighted normals last), the
/// arch's bevel then made 4 cm wide: the version 57 sample holds exactly
/// this.
pub(super) fn rich_document_v57() -> Document {
    let mut doc = rich_document_v56();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    let arch = create(&mut doc, PrimitiveKind::Arch);
    doc.game_bevel(&[cube, arch], "Game bevel", 57).unwrap();
    let mut wider = doc.scene().modifiers(arch)[0].clone();
    wider.kind = ModifierKind::LiveBevel(LiveBevelOptions {
        width: 4.0,
        ..scene::game_bevel_options()
    });
    doc.execute(Command::ChangeModifier {
        id: arch,
        index: 0,
        modifier: wider,
    })
    .unwrap();
    doc
}

#[test]
fn soft_bevels_survive_a_round_trip() {
    let doc = rich_document_v57();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_same(&doc, &back);
    assert_eq!(bevels(&doc), bevels(&back));
    let soft: Vec<_> = bevels(&back)
        .into_iter()
        .filter(|(_, o)| o.soften)
        .collect();
    assert_eq!(soft.len(), 2);
    for (id, _) in soft {
        assert_eq!(back.scene().object(id).unwrap().hardness, Hardness::Marked);
        assert_eq!(
            back.scene().modifiers(id)[1].kind,
            ModifierKind::WeightedNormals(WeightedNormalsOptions::default())
        );
        assert_eq!(
            back.scene().exported_shape(id),
            doc.scene().exported_shape(id),
            "the game bevel comes out the same"
        );
    }
}

const SAMPLE_V57: &[u8] = include_bytes!("../../tests/data/sample-v57.avtl");

#[test]
fn a_file_saved_by_format_version_57_opens_with_its_soft_bevels() {
    let doc = from_bytes(SAMPLE_V57).expect("the version 57 sample opens");
    let expected = rich_document_v57();
    assert_same(&expected, &doc);
    assert_eq!(bevels(&expected), bevels(&doc));
    let found = bevels(&doc);
    assert!(found.iter().any(|(_, o)| *o == scene::game_bevel_options()));
    assert!(found.iter().any(|(_, o)| o.soften && o.width == 4.0));
    // The version 50 sample's plain Live bevels stay as they were.
    assert!(found.iter().any(|(_, o)| !o.soften));
}

const SAMPLE_V56: &[u8] = include_bytes!("../../tests/data/sample-v56.avtl");
const SAMPLE_V50: &[u8] = include_bytes!("../../tests/data/sample-v50.avtl");

#[test]
fn older_files_open_with_soft_bevels_off() {
    for sample in [SAMPLE_V56, SAMPLE_V50] {
        let doc = from_bytes(sample).unwrap();
        let found = bevels(&doc);
        assert!(!found.is_empty(), "the samples hold Live bevels");
        assert!(found.iter().all(|(_, o)| !o.soften));
        let again = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
        assert_same(&doc, &again);
    }
    assert_same(&rich_document_v56(), &from_bytes(SAMPLE_V56).unwrap());
}

#[test]
fn an_older_app_is_told_it_needs_updating() {
    let error = super::tests::refused(&header(FORMAT_VERSION + 1, 0, 0));
    assert!(matches!(error, ProjectError::TooNew { .. }));
}

#[test]
fn random_version_57_bodies_never_panic() {
    let mut seed = 0x0057_5047_be7e_1a5c_u64;
    let good = v43::encode(&v43::ProjectV43::from_document(&rich_document_v57())).unwrap();
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
        let mut bytes = header(57, 0, body.len() as u64);
        bytes.extend_from_slice(&body);
        let _ = from_bytes(&bytes);
    }
}
