//! Tests for version 69: version 68 with each dab's strength (decision
//! 0184).

use super::v68_tests::rich_document_v68;
use super::*;
use std::sync::Arc;

/// Everything version 68 stores, with the first stroke's dabs at
/// different strengths. The version 69 sample holds exactly this.
pub(super) fn rich_document_v69() -> Document {
    let mut doc = rich_document_v68();
    let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
    let mut stroke = (*set.layers[0].strokes[0]).clone();
    for (i, dab) in stroke.dabs.iter_mut().enumerate() {
        dab.strength = 0.25 + 0.5 * i as f32;
    }
    set.layers[0].strokes[0] = Arc::new(stroke);
    doc.execute(scene::Command::SetTextureSet {
        material: "Default".into(),
        set: Some(set),
    })
    .unwrap();
    doc
}

#[test]
fn dab_strengths_survive_a_round_trip() {
    let doc = rich_document_v69();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_eq!(doc.scene().paint(), back.scene().paint());
}

const SAMPLE_V69: &[u8] = include_bytes!("../../tests/data/sample-v69.avtl");

#[test]
fn a_file_saved_by_format_version_69_opens_with_its_strengths() {
    let doc = from_bytes(SAMPLE_V69).expect("the version 69 sample opens");
    assert_eq!(rich_document_v69().scene().paint(), doc.scene().paint());
}

#[test]
fn a_version_68_file_has_full_strength() {
    const SAMPLE_V68: &[u8] = include_bytes!("../../tests/data/sample-v68.avtl");
    let doc = from_bytes(SAMPLE_V68).unwrap();
    let set = doc.scene().paint().set("Default").unwrap();
    assert!(
        set.layers[0].strokes[0]
            .dabs
            .iter()
            .all(|d| d.strength == 1.0)
    );
}

#[test]
fn damaged_strengths_are_refused() {
    let good = v43::ProjectV43::from_document(&rich_document_v69());
    let with = |change: &dyn Fn(&mut super::v65::StrokeV65)| {
        let mut model = good.clone();
        if let Some(p) = model.paint.as_mut() {
            change(&mut p.sets[0].layers[0].strokes[0]);
        }
        model.into_document()
    };
    assert!(with(&|s| s.strengths = std::iter::once(0.5).collect()).is_err());
    assert!(
        with(&|s| {
            let n = s.dabs.len();
            s.strengths = std::iter::repeat_n(2.0, n).collect();
        })
        .is_err()
    );
    assert!(with(&|_| ()).is_ok());
}
