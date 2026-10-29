//! Tests for version 83: version 82 with each stroke's alignment
//! (decision 0217).

use super::v82_tests::rich_document_v82;
use super::*;
use std::sync::Arc;

/// Everything version 82 stores, with a stroke laid on the UVs. The
/// version 83 sample holds exactly this.
pub(super) fn rich_document_v83() -> Document {
    let mut doc = rich_document_v82();
    let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
    let mut stroke = (*set.layers[0].strokes[0]).clone();
    stroke.alignment = paint::Alignment::Uv;
    set.layers[0].strokes.push(Arc::new(stroke));
    doc.execute(scene::Command::SetTextureSet {
        material: "Default".into(),
        set: Some(set),
    })
    .unwrap();
    doc
}

fn alignments(doc: &Document) -> Vec<paint::Alignment> {
    doc.scene().paint().set("Default").unwrap().layers[0]
        .strokes
        .iter()
        .map(|s| s.alignment)
        .collect()
}

#[test]
fn each_strokes_alignment_survives_a_round_trip() {
    let doc = rich_document_v83();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(alignments(&back), alignments(&doc));
    assert_eq!(alignments(&back).last(), Some(&paint::Alignment::Uv));
}

const SAMPLE_V83: &[u8] = include_bytes!("../../tests/data/sample-v83.avtl");

#[test]
fn a_file_saved_by_format_version_83_opens_with_its_alignments() {
    let doc = from_bytes(SAMPLE_V83).expect("the version 83 sample opens");
    assert_eq!(alignments(&doc), alignments(&rich_document_v83()));
}

#[test]
fn strokes_in_a_version_82_file_were_painted_from_the_camera() {
    const SAMPLE_V82: &[u8] = include_bytes!("../../tests/data/sample-v82.avtl");
    let doc = from_bytes(SAMPLE_V82).unwrap();
    let all = alignments(&doc);
    assert!(!all.is_empty());
    assert!(all.iter().all(|a| *a == paint::Alignment::Camera));
}

#[test]
fn an_unknown_alignment_is_refused() {
    let good = v43::ProjectV43::from_document(&rich_document_v83());
    let mut bad = good.clone();
    if let Some(p) = bad.paint.as_mut() {
        p.sets[0].layers[0].strokes[0].alignment = 9;
    }
    assert!(bad.into_document().is_err());
    assert!(good.into_document().is_ok());
}
