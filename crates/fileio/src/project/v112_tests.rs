//! Tests for version 112: version 111 with Sculpt layers (decision 0304).

use super::v111_tests::rich_document_v111;
use super::*;
use glam::Vec3;
use scene::{Command, ObjectId};
use sculpt::{BrushKind, Dab, Falloff, LiveSculpt};
use std::sync::Arc;

/// The first object that can be sculpted.
pub(crate) fn sculptable(doc: &Document) -> ObjectId {
    let scene = doc.scene();
    scene
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| scene.sculpt_target(*id).is_some())
        .expect("an object to sculpt")
}

/// One Draw stroke across the middle of object `id`'s shape.
fn stroke(doc: &mut Document, id: ObjectId) {
    let target = doc.scene().sculpt_target(id).unwrap();
    let mut live = LiveSculpt::new(Arc::clone(&target.input), target.sculpt.as_ref());
    let bounds = target.input.bounds().unwrap();
    let middle = (bounds.min + bounds.max) / 2.0;
    let size = (bounds.max - bounds.min).max_element();
    let hit = live
        .hit(middle + Vec3::Z * size * 4.0, Vec3::NEG_Z)
        .or_else(|| live.hit(middle - Vec3::Y * size * 4.0, Vec3::Y))
        .expect("the shape");
    for i in 0..4 {
        live.dab(&Dab {
            kind: BrushKind::Draw,
            centre: hit.point + Vec3::X * (i as f32 * size * 0.05),
            radius: size * 0.3,
            strength: 1.0,
            falloff: Falloff::Smooth,
            invert: false,
        });
    }
    let sculpt = live.finish().expect("points moved");
    doc.execute(Command::Sculpt { id, sculpt }).unwrap();
}

/// Everything version 111 stores, with a Sculpt layer of one stroke on
/// the first object that can be sculpted. The version 112 sample holds
/// exactly this.
pub(crate) fn rich_document_v112() -> Document {
    let mut doc = rich_document_v111();
    let id = sculptable(&doc);
    stroke(&mut doc, id);
    doc
}

pub(crate) fn sculpt_of(doc: &Document) -> sculpt::Sculpt {
    let id = sculptable(doc);
    let object = doc.scene().object(id).unwrap();
    object
        .stack
        .layers()
        .iter()
        .find_map(|l| l.sculpt().cloned())
        .expect("a Sculpt layer")
}

#[test]
fn a_sculpt_layer_survives_a_round_trip_exactly() {
    let doc = rich_document_v112();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_eq!(sculpt_of(&back), sculpt_of(&doc));
    let id = sculptable(&doc);
    assert_eq!(
        *back.scene().evaluated(id).unwrap().mesh,
        *doc.scene().evaluated(id).unwrap().mesh
    );
}

#[test]
fn only_chunks_that_moved_are_stored() {
    let doc = rich_document_v112();
    let sculpt = sculpt_of(&doc);
    let op = scene::Operation::Sculpt {
        sculpt: sculpt.clone(),
    };
    let stored = v112::SculptEditV112::of(&op).unwrap();
    let moved = sculpt.moves().chunks().iter().flatten().count();
    assert_eq!(stored.chunks.len(), moved);
    assert!(moved >= 1);
    // A fresh sculpt stores no chunks.
    let fresh = sculpt::Sculpt::new(&scene::PrimitiveKind::Cube.standard().mesh());
    let stored = v112::SculptEditV112::of(&scene::Operation::Sculpt { sculpt: fresh }).unwrap();
    assert!(stored.chunks.is_empty());
    assert!(v112::SculptEditV112::of(&scene::Operation::FixFacing).is_none());
}

#[test]
fn damaged_sculpts_are_refused() {
    let doc = rich_document_v112();
    let op = scene::Operation::Sculpt {
        sculpt: sculpt_of(&doc),
    };
    let good = v112::SculptEditV112::of(&op).unwrap();
    assert!(good.clone().into_operation().is_ok());
    let mut more = good.clone();
    more.points += 1;
    assert!(more.into_operation().is_err(), "places don't unpack to it");
    let mut far = good.clone();
    far.chunks[0].index = 1_000;
    assert!(far.into_operation().is_err(), "a chunk past the end");
    let mut twice = good.clone();
    let first = twice.chunks[0].clone();
    twice.chunks.push(first);
    assert!(twice.into_operation().is_err(), "a chunk twice");
    let mut huge = good.clone();
    huge.points = u64::MAX;
    assert!(huge.into_operation().is_err(), "too many points");
    let mut garbage = good.clone();
    garbage.chunks[0].moves = v107::PackedBytes(vec![1, 2, 3, 4]);
    assert!(garbage.into_operation().is_err(), "moves that don't unpack");
    let mut nan = good;
    let n = sculpt::moves::chunk_len(nan.points as usize, nan.chunks[0].index as usize);
    let mut raw = Vec::new();
    for _ in 0..n * 3 {
        raw.extend_from_slice(&f32::NAN.to_le_bytes());
    }
    nan.chunks[0].moves = v107::PackedBytes(zstd::bulk::compress(&raw, 1).unwrap());
    assert!(nan.into_operation().is_err(), "moves that aren't numbers");
}

#[test]
fn flipped_bytes_in_a_file_with_a_sculpt_never_crash_the_reader() {
    let full = to_bytes(&rich_document_v112()).unwrap();
    let mut state = 112u64;
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

#[test]
fn stored_sculpts_read_within_a_small_budget_fail_plainly() {
    let doc = rich_document_v112();
    let op = scene::Operation::Sculpt {
        sculpt: sculpt_of(&doc),
    };
    let stored = v112::SculptEditV112::of(&op).unwrap();
    let read = crate::limits::within(64, || stored.into_operation());
    assert!(read.is_err());
}

const SAMPLE_V112: &[u8] = include_bytes!("../../tests/data/sample-v112.avtl");

#[test]
fn a_file_saved_by_format_version_112_opens_with_its_sculpt() {
    let doc = from_bytes(SAMPLE_V112).expect("the version 112 sample opens");
    let rich = rich_document_v112();
    // The same sculpt, within rounding: where a line meets the shape may
    // differ in its last bits as the hit test changes (decision 0308 gave
    // it a little slack at polygon edges).
    let (stored, made) = (sculpt_of(&doc), sculpt_of(&rich));
    assert_eq!(stored.len(), made.len());
    let mut moved = 0;
    for i in 0..stored.len() {
        let (a, b) = (stored.moves().get(i), made.moves().get(i));
        assert!(a.distance(b) < 1e-3, "point {i}: {a} against {b}");
        if a != Vec3::ZERO {
            moved += 1;
        }
    }
    assert!(moved > 0, "the sample holds a sculpt");
}

#[test]
fn a_version_111_file_opens_without_sculpt_layers() {
    const SAMPLE_V111: &[u8] = include_bytes!("../../tests/data/sample-v111.avtl");
    let doc = from_bytes(SAMPLE_V111).unwrap();
    for (id, _) in doc.scene().walk() {
        let object = doc.scene().object(id).unwrap();
        assert!(object.stack.layers().iter().all(|l| !l.is_sculpt()));
    }
}
