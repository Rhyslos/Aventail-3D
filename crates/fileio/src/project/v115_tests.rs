//! Tests for version 115: version 114 with each Sculpt layer's detail
//! levels (decision 0315).

use super::v112_tests::{sculpt_of, sculptable};
use super::v114_tests::rich_document_v114;
use super::*;
use glam::Vec3;
use scene::Command;
use sculpt::levels::ExportLevel;
use sculpt::mask::Mask;
use sculpt::{BrushKind, Dab, Falloff, LiveSculpt};
use std::sync::Arc;

/// One Draw dab in the middle of what `live` shows.
fn dab(live: &mut LiveSculpt) {
    let bounds = live.mesh().bounds().unwrap();
    let middle = (bounds.min + bounds.max) / 2.0;
    let size = (bounds.max - bounds.min).max_element();
    let hit = live
        .hit(middle + Vec3::Z * size * 4.0, Vec3::NEG_Z)
        .or_else(|| live.hit(middle - Vec3::Y * size * 4.0, Vec3::Y))
        .expect("the shape");
    live.dab(&Dab {
        kind: BrushKind::Draw,
        centre: hit.point,
        radius: size * 0.3,
        strength: 1.0,
        falloff: Falloff::Smooth,
        invert: false,
    });
}

/// Everything version 114 stores, with three detail levels added to the
/// sculpted object's Sculpt layer (four levels with the shape it was
/// sculpted on): a stroke on level 1 and one on level 3, a mask painted
/// on level 2, level 2 shown, and exports taking the finest. The version
/// 115 sample holds exactly this.
pub(crate) fn rich_document_v115() -> Document {
    let mut doc = rich_document_v114();
    let id = sculptable(&doc);
    let target = doc.scene().sculpt_target(id).unwrap();
    let mut live = LiveSculpt::new(Arc::clone(&target.input), target.sculpt.as_ref());
    for _ in 0..3 {
        live.add_level().expect("a level");
    }
    dab(&mut live);
    live.finish().expect("a stroke on level 3");
    live.set_level(1).expect("level 1");
    dab(&mut live);
    live.finish().expect("a stroke on level 1");
    live.set_level(2).expect("level 2");
    let values: Vec<u8> = (0..live.mesh().point_slot_count())
        .map(|i| if i % 3 == 0 { 200 } else { 0 })
        .collect();
    live.set_mask(Mask::from_values(&values)).expect("a mask");
    let sculpt = live.set_export(ExportLevel::Finest).expect("an export level");
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
fn four_levels_survive_a_round_trip_exactly() {
    let doc = rich_document_v115();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    let (a, b) = (sculpt_of(&back), sculpt_of(&doc));
    assert_eq!(a.level_count(), 4);
    assert_eq!(a.shown(), 2);
    assert_eq!(a.mask_level(), 2);
    assert_eq!(a.export(), ExportLevel::Finest);
    assert!(!a.mask().is_zero());
    for k in 0..4 {
        assert_eq!(a.level_moves(k), b.level_moves(k), "level {k}");
        assert_eq!(a.level_structure(k), b.level_structure(k), "level {k}");
    }
    assert!(!a.level_moves(1).unwrap().is_zero());
    assert!(!a.level_moves(3).unwrap().is_zero());
    assert_eq!(a, b);
    // The shape shown, and the shape an export takes, are the same too.
    let id = sculptable(&doc);
    assert_eq!(
        *back.scene().evaluated(id).unwrap().mesh,
        *doc.scene().evaluated(id).unwrap().mesh
    );
    let (exported, shown) = (
        back.scene().evaluated_for_export(id).unwrap().mesh,
        back.scene().evaluated(id).unwrap().mesh,
    );
    assert_eq!(exported.face_count(), shown.face_count() * 4);
    assert_eq!(
        *exported,
        *doc.scene().evaluated_for_export(id).unwrap().mesh
    );
}

#[test]
fn a_sculpt_without_levels_stores_none() {
    let plain = stored(&rich_document_v114());
    assert!(plain.levels.above.is_empty());
    assert_eq!(plain.levels, v115::LevelsV115::default());
    let levels = stored(&rich_document_v115());
    assert_eq!(levels.levels.above.len(), 3);
    assert_eq!(levels.levels.shown, 2);
    assert_eq!(levels.levels.mask_level, 2);
    // A level nothing was sculpted on stores its places but no chunks.
    assert!(levels.levels.above[1].chunks.is_empty());
    assert!(!levels.levels.above[0].chunks.is_empty());
}

#[test]
fn damaged_levels_are_refused() {
    let good = stored(&rich_document_v115());
    assert!(good.clone().into_operation().is_ok());
    let level = good.levels.above[0].clone();
    let mut many = good.clone();
    while many.levels.above.len() <= sculpt::levels::MAX_LEVELS {
        many.levels.above.push(level.clone());
    }
    assert!(many.into_operation().is_err(), "too many levels");
    let mut shown = good.clone();
    shown.levels.shown = 4;
    assert!(shown.into_operation().is_err(), "a level shown it hasn't");
    let mut masked = good.clone();
    masked.levels.mask_level = 9;
    assert!(masked.into_operation().is_err(), "a mask on a level it hasn't");
    let mut on_another = good.clone();
    on_another.levels.mask_level = 3;
    assert!(
        on_another.into_operation().is_err(),
        "a mask that doesn't fit its level"
    );
    let mut export = good.clone();
    export.levels.export = v115::ExportV115::Level(7);
    assert!(export.into_operation().is_err(), "an export level it hasn't");
    let mut more = good.clone();
    more.levels.above[1].points += 1;
    assert!(more.into_operation().is_err(), "places don't unpack to it");
    let mut huge = good.clone();
    huge.levels.above[2].points = u64::MAX;
    assert!(huge.into_operation().is_err(), "too many points");
    let mut fewer = good.clone();
    fewer.levels.above.swap(0, 2);
    assert!(
        fewer.into_operation().is_err(),
        "a level with fewer points than the one below"
    );
    let mut far = good.clone();
    far.levels.above[0].chunks[0].index = 1_000;
    assert!(far.into_operation().is_err(), "a chunk past the end");
    let mut garbage = good.clone();
    garbage.levels.above[0].chunks[0].moves = v107::PackedBytes(vec![1, 2, 3, 4]);
    assert!(garbage.into_operation().is_err(), "moves that don't unpack");
    let read = crate::limits::within(64, || good.into_operation());
    assert!(read.is_err(), "within a small budget");
}

#[test]
fn flipped_bytes_in_a_file_with_levels_never_crash_the_reader() {
    let full = to_bytes(&rich_document_v115()).unwrap();
    let mut state = 115u64;
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
                let _ = doc.scene().evaluated_for_export(id);
            }
        }
    }
}

#[test]
fn any_level_count_and_level_numbers_read_without_a_crash() {
    // Every level number a byte can hold, on sculpts of none to seven
    // levels above: read as it is, or refused.
    let good = stored(&rich_document_v115());
    let level = good.levels.above[0].clone();
    for count in 0..=7usize {
        for n in [0u8, 1, 2, 3, 6, 7, 200, 255] {
            let mut model = good.clone();
            model.mask = crate::limits::Bounded::new();
            model.levels.above = vec![level.clone(); count].into();
            model.levels.shown = n;
            model.levels.mask_level = n.wrapping_mul(3);
            model.levels.export = v115::ExportV115::Level(n.wrapping_add(1));
            if let Ok(scene::Operation::Sculpt { sculpt }) = model.into_operation() {
                assert!(sculpt.shown() <= sculpt.top());
                assert!(sculpt.top() <= sculpt::levels::MAX_LEVELS);
            }
        }
    }
}

const SAMPLE_V115: &[u8] = include_bytes!("../../tests/data/sample-v115.avtl");

#[test]
fn a_file_saved_by_format_version_115_opens_with_its_levels() {
    let doc = from_bytes(SAMPLE_V115).expect("the version 115 sample opens");
    let (stored, made) = (sculpt_of(&doc), sculpt_of(&rich_document_v115()));
    assert_eq!(stored.level_count(), 4);
    assert_eq!(stored, made);
}

#[test]
fn files_before_version_115_open_with_one_level() {
    const SAMPLE_V112: &[u8] = include_bytes!("../../tests/data/sample-v112.avtl");
    const SAMPLE_V114: &[u8] = include_bytes!("../../tests/data/sample-v114.avtl");
    for sample in [SAMPLE_V112, SAMPLE_V114] {
        let doc = from_bytes(sample).unwrap();
        let sculpt = sculpt_of(&doc);
        assert_eq!(sculpt.level_count(), 1);
        assert_eq!(sculpt.shown(), 0);
        assert_eq!(sculpt.export(), ExportLevel::Shown);
        assert_eq!(sculpt.mask_level(), 0);
    }
    // A version 114 file keeps its mask, on the shape it was painted on.
    let doc = from_bytes(SAMPLE_V114).unwrap();
    assert_eq!(
        sculpt_of(&doc).mask(),
        sculpt_of(&rich_document_v114()).mask()
    );
}
