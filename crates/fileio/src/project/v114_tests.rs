//! Tests for version 114: version 113 with each Sculpt layer's mask
//! (decision 0310).

use super::v112_tests::{rich_document_v112, sculpt_of, sculptable};
use super::v113_tests::rich_document_v113;
use super::*;
use scene::Command;
use sculpt::mask::{FULL, Mask};

/// Everything version 113 stores, with half the points of the sculpted
/// object's Sculpt layer masked, some only partly. The version 114 sample
/// holds exactly this.
pub(crate) fn rich_document_v114() -> Document {
    let mut doc = rich_document_v113();
    let id = sculptable(&doc);
    let sculpt = sculpt_of(&doc);
    let values: Vec<u8> = (0..sculpt.len())
        .map(|i| match i % 4 {
            0 | 1 => FULL,
            2 => 100,
            _ => 0,
        })
        .collect();
    let sculpt = sculpt.with_mask(Mask::from_values(&values)).unwrap();
    doc.execute(Command::Sculpt { id, sculpt }).unwrap();
    doc
}

fn stored(doc: &Document) -> v112::SculptEditV112 {
    v112::SculptEditV112::of(&scene::Operation::Sculpt {
        sculpt: sculpt_of(doc),
    })
    .unwrap()
}

#[test]
fn a_mask_survives_a_round_trip_exactly() {
    let doc = rich_document_v114();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    let (a, b) = (sculpt_of(&back), sculpt_of(&doc));
    assert!(!a.mask().is_zero());
    assert_eq!(a.mask(), b.mask());
    assert_eq!(a, b);
}

#[test]
fn only_masked_chunks_are_stored() {
    let doc = rich_document_v114();
    let s = stored(&doc);
    assert_eq!(
        s.mask.len(),
        sculpt_of(&doc).mask().chunks().iter().flatten().count()
    );
    // Without a mask, nothing more is stored than version 112 did.
    let plain = stored(&rich_document_v112());
    assert!(plain.mask.is_empty());
}

#[test]
fn damaged_masks_are_refused() {
    let good = stored(&rich_document_v114());
    assert!(good.clone().into_operation().is_ok());
    let mut far = good.clone();
    far.mask[0].index = 1_000;
    assert!(far.into_operation().is_err(), "a chunk past the end");
    let mut twice = good.clone();
    let first = twice.mask[0].clone();
    twice.mask.push(first);
    assert!(twice.into_operation().is_err(), "a chunk twice");
    let mut garbage = good.clone();
    garbage.mask[0].values = v107::PackedBytes(vec![1, 2, 3, 4]);
    assert!(
        garbage.into_operation().is_err(),
        "values that don't unpack"
    );
    let mut short = good.clone();
    short.mask[0].values = v107::PackedBytes(zstd::bulk::compress(&[7u8; 5], 1).unwrap());
    assert!(short.into_operation().is_err(), "too few values");
    let read = crate::limits::within(64, || good.into_operation());
    assert!(read.is_err(), "within a small budget");
}

#[test]
fn flipped_bytes_in_a_file_with_a_mask_never_crash_the_reader() {
    let full = to_bytes(&rich_document_v114()).unwrap();
    let mut state = 114u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        state >> 20
    };
    for _ in 0..200 {
        let mut bytes = full.clone();
        for _ in 0..(next() % 3 + 1) {
            let at = (next() as usize) % bytes.len();
            bytes[at] ^= 1 << (next() % 8);
        }
        if let Ok(doc) = from_bytes(&bytes) {
            for (id, _) in doc.scene().walk() {
                let _ = doc.scene().evaluated(id);
            }
        }
    }
}

const SAMPLE_V114: &[u8] = include_bytes!("../../tests/data/sample-v114.avtl");

#[test]
fn a_file_saved_by_format_version_114_opens_with_its_mask() {
    let doc = from_bytes(SAMPLE_V114).expect("the version 114 sample opens");
    let (stored, made) = (sculpt_of(&doc), sculpt_of(&rich_document_v114()));
    assert_eq!(stored.mask(), made.mask());
    assert!(!stored.mask().is_zero());
    // Version 113's scripts come along too.
    assert_eq!(
        doc.scene().scripts(),
        rich_document_v114().scene().scripts()
    );
    assert!(!doc.scene().scripts().is_empty());
}

#[test]
fn version_112_and_113_files_open_with_no_mask() {
    const SAMPLE_V112: &[u8] = include_bytes!("../../tests/data/sample-v112.avtl");
    const SAMPLE_V113: &[u8] = include_bytes!("../../tests/data/sample-v113.avtl");
    for sample in [SAMPLE_V112, SAMPLE_V113] {
        let doc = from_bytes(sample).unwrap();
        assert!(sculpt_of(&doc).mask().is_zero());
    }
    // A version 113 file keeps its scripts.
    let doc = from_bytes(SAMPLE_V113).unwrap();
    assert_eq!(
        doc.scene().scripts(),
        rich_document_v113().scene().scripts()
    );
}
