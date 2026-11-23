//! Tests of detail levels (decision 0315), the Relax brush and Rebuild
//! lower levels.

use super::levels::{ExportLevel, LevelProblem, MAX_LEVELS};
use super::rebuild::{RebuildProblem, RebuildProgress, levels_for, rebuild_lower};
use super::tests::{evaluated, grid};
use super::*;
use glam::Vec3;
use mesh::{Mesh, PrimitiveKind, VertexId};
use std::sync::Arc;

fn dab(kind: BrushKind, centre: Vec3, radius: f32) -> Dab {
    Dab {
        kind,
        centre,
        radius,
        strength: 1.0,
        falloff: Falloff::Smooth,
        invert: false,
    }
}

fn sphere() -> Arc<Mesh> {
    Arc::new(PrimitiveKind::Sphere.standard().mesh())
}

/// The standard sphere's middle (it sits on the ground).
const MIDDLE: Vec3 = Vec3::new(0.0, 0.0, 50.0);

/// Where a line from `from` (measured from the sphere's middle) towards
/// the middle meets the live shape.
fn hit(live: &LiveSculpt, from: Vec3) -> Vec3 {
    let way = -from + Vec3::new(0.013, 0.017, 0.011) * from.length();
    live.hit(MIDDLE + from, way).expect("the shape").point
}

/// The point of `mesh` nearest `p`.
fn nearest(mesh: &Mesh, p: Vec3) -> VertexId {
    mesh.vertices()
        .min_by(|a, b| a.1.distance(p).total_cmp(&b.1.distance(p)))
        .unwrap()
        .0
}

/// One stroke of `kind` at `from` (see [`hit`]), finished.
fn stroke(live: &mut LiveSculpt, kind: BrushKind, from: Vec3, radius: f32) -> Sculpt {
    let at = hit(live, from);
    for i in 0..3 {
        live.dab(&dab(kind, at + Vec3::Z * i as f32, radius));
    }
    live.finish().expect("a stroke")
}

#[test]
fn a_level_has_four_times_the_polygons_and_keeps_every_points_number() {
    let input = Arc::new(grid(4));
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    assert_eq!((live.level(), live.top_level()), (0, 0));
    assert_eq!(live.polygons_if_added(), 64);
    let sculpt = live.add_level().unwrap();
    assert_eq!((sculpt.shown(), sculpt.top(), sculpt.level_count()), (1, 1, 2));
    assert_eq!((live.level(), live.top_level()), (1, 1));
    assert_eq!(live.mesh().face_count(), 64);
    assert_eq!(live.polygons_if_added(), 256);
    // The first points are the shape's own, each where subdividing puts
    // it: a flat sheet stays flat and keeps its corners.
    for (v, p) in input.vertices() {
        let now = live.mesh().position(v).unwrap();
        assert!(now.z.abs() < 1e-6);
        assert!(now.distance(p) < 0.5, "{v:?}");
    }
    assert_eq!(live.mesh().position(VertexId::from_raw(0)).unwrap(), Vec3::new(-2.0, -2.0, 0.0));
    assert_eq!(&evaluated(&input, &sculpt), live.mesh());
    // A fresh level holds no moves.
    assert!(sculpt.level_moves(1).unwrap().is_zero());
    assert_eq!(sculpt.len(), live.mesh().point_slot_count());
}

#[test]
fn sculpting_on_level_1_moves_the_detail_on_level_3_with_it() {
    let input = sphere();
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    for _ in 0..3 {
        live.add_level().unwrap();
    }
    assert_eq!(live.level(), 3);
    let smooth = live.mesh().clone();
    // A small bump of detail on the front, on level 3.
    let front = Vec3::new(0.0, -300.0, 0.0);
    let detail = stroke(&mut live, BrushKind::Draw, front, 6.0);
    let top = nearest(live.mesh(), hit(&live, front));
    let bump = live.mesh().position(top).unwrap() - smooth.position(top).unwrap();
    assert!(bump.length() > 0.3, "the detail is {bump:?}");
    let with_detail = live.mesh().clone();
    // A big push on level 1, right under it.
    live.set_level(1).unwrap();
    let big = stroke(&mut live, BrushKind::Draw, front, 40.0);
    assert!(big.level_moves(3) == detail.level_moves(3), "level 3's moves are untouched");
    assert!(!big.level_moves(1).unwrap().is_zero());
    let sculpt = live.set_level(3).unwrap();
    let now = live.mesh().position(top).unwrap();
    let carried = now - with_detail.position(top).unwrap();
    assert!(carried.length() > 2.0, "the detail moved by {carried:?}");
    // The same sculpt without level 3's detail: the bump stands on it as
    // it stood on the smooth shape.
    let mut plain = (*input).clone();
    sculpt
        .with_moves(Moves::zero(sculpt.len()))
        .unwrap()
        .apply(&mut plain);
    let bump_now = now - plain.position(top).unwrap();
    assert!(
        (bump_now.length() - bump.length()).abs() < bump.length() * 0.05,
        "{bump_now:?} vs {bump:?}"
    );
    assert!(bump_now.normalize().dot(bump.normalize()) > 0.95);
    assert_eq!(&evaluated(&input, &sculpt), live.mesh());
}

#[test]
fn the_live_shape_is_exactly_what_working_the_sculpt_out_gives_on_every_level() {
    let input = sphere();
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    let mut sculpt;
    let check = |live: &LiveSculpt, sculpt: &Sculpt, what: &str| {
        assert_eq!(&evaluated(&input, sculpt), live.mesh(), "{what}");
        let fresh = LiveSculpt::new(Arc::clone(&input), Some(sculpt));
        assert_eq!(fresh.mesh(), live.mesh(), "{what}, made afresh");
        assert_eq!(fresh.level(), sculpt.shown());
    };
    let places = [
        Vec3::new(0.0, -300.0, 0.0),
        Vec3::new(300.0, 0.0, 30.0),
        Vec3::new(0.0, 0.0, 300.0),
        Vec3::new(-200.0, -200.0, 0.0),
    ];
    // Up a level and a stroke, three times; then strokes going down and
    // up again, each on a level whose finer levels hold detail.
    for (i, from) in places.iter().take(3).enumerate() {
        live.add_level().unwrap();
        sculpt = stroke(&mut live, BrushKind::Draw, *from, 12.0);
        check(&live, &sculpt, &format!("a stroke on level {}", i + 1));
    }
    for (level, kind) in [
        (0, BrushKind::Inflate),
        (2, BrushKind::Smooth),
        (1, BrushKind::ClayStrips),
        (3, BrushKind::Crease),
        (0, BrushKind::Grab),
    ] {
        sculpt = live.set_level(level).unwrap();
        check(&live, &sculpt, &format!("on level {level}"));
        let from = places[level];
        sculpt = stroke(&mut live, kind, from, 25.0);
        check(&live, &sculpt, &format!("{kind:?} on level {level}"));
    }
    sculpt = live.set_level(3).unwrap();
    check(&live, &sculpt, "back on the finest");
    // Working it out for another level than the one shown.
    for level in 0..=3 {
        let mut at = (*input).clone();
        sculpt.apply_at(&mut at, level);
        let mut shown = (*input).clone();
        sculpt.showing(level).unwrap().apply(&mut shown);
        assert_eq!(at, shown);
        assert_eq!(
            at.face_count() as u64,
            mesh::subdivide::polygon_count(&input, level as u32)
        );
    }
}

#[test]
fn undo_and_redo_follow_through_strokes_and_level_changes() {
    let input = sphere();
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    let front = Vec3::new(0.0, -300.0, 0.0);
    let mut steps = vec![(live.sculpt().clone(), live.mesh().clone())];
    let mut keep = |live: &LiveSculpt, sculpt: Sculpt| steps.push((sculpt, live.mesh().clone()));
    let s = stroke(&mut live, BrushKind::Draw, front, 30.0);
    keep(&live, s);
    let s = live.add_level().unwrap();
    keep(&live, s);
    let s = stroke(&mut live, BrushKind::Draw, front, 10.0);
    keep(&live, s);
    let s = live.add_level().unwrap();
    keep(&live, s);
    let s = stroke(&mut live, BrushKind::Crease, front, 8.0);
    keep(&live, s);
    let s = live.set_level(0).unwrap();
    keep(&live, s);
    let s = stroke(&mut live, BrushKind::Grab, front, 30.0);
    keep(&live, s);
    let s = live.set_level(2).unwrap();
    keep(&live, s);
    let s = live.set_level(1).unwrap();
    keep(&live, s);
    let s = live.delete_higher().unwrap();
    keep(&live, s);
    // Back to the start one step at a time, forward again, then in
    // jumps: each time the shape is exactly what it was.
    let last = steps.len() - 1;
    let order: Vec<usize> = (0..last)
        .rev()
        .chain(1..=last)
        .chain([3, 9, 0, 6, last, 1, 7, 4])
        .collect();
    for i in order {
        let (sculpt, shape) = &steps[i];
        live.follow(sculpt);
        assert_eq!(live.mesh(), shape, "step {i}");
        assert_eq!(live.level(), sculpt.shown(), "step {i}");
        assert_eq!(live.shape().as_ref(), shape, "step {i}, as handed out");
    }
}

#[test]
fn a_stroke_on_a_level_is_undone_from_its_kept_places() {
    let input = sphere();
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    live.add_level().unwrap();
    live.add_level().unwrap();
    live.set_level(1).unwrap();
    let before = live.sculpt().clone();
    let was = live.mesh().clone();
    let after = stroke(&mut live, BrushKind::Draw, Vec3::new(0.0, -300.0, 0.0), 30.0);
    let is = live.mesh().clone();
    // Only level 1's chunks, and the stroke's places, are kept apart.
    assert_eq!(after.level_moves(0), before.level_moves(0));
    assert_eq!(after.level_moves(2), before.level_moves(2));
    assert!(after.memory_apart(&before) > 0);
    assert!(after.memory_apart(&before) < after.memory_size() / 2);
    live.follow(&before);
    assert_eq!(live.mesh(), &was);
    live.follow(&after);
    assert_eq!(live.mesh(), &is);
    // A stroke taken back leaves the level as it was.
    live.dab(&dab(BrushKind::Draw, hit(&live, Vec3::new(300.0, 0.0, 0.0)), 30.0));
    live.cancel();
    assert_eq!(live.mesh(), &is);
    assert!(live.finish().is_none());
}

#[test]
fn levels_that_arent_there_and_too_many_are_refused() {
    let mut one = Mesh::new();
    let corners: Vec<VertexId> = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
        .iter()
        .map(|(x, y)| one.add_vertex(Vec3::new(*x, *y, 0.0)).unwrap())
        .collect();
    one.add_face(&corners).unwrap();
    let mut live = LiveSculpt::new(Arc::new(one), None);
    assert!(live.set_level(1).is_none());
    assert!(live.set_level(0).is_none());
    assert!(live.delete_higher().is_none());
    assert!(live.without_lower().is_none());
    for level in 1..=MAX_LEVELS {
        assert_eq!(live.add_level().unwrap().top(), level);
    }
    assert_eq!(live.mesh().face_count(), 4096);
    assert_eq!(live.add_level(), Err(LevelProblem::TooManyLevels));
    assert_eq!(live.level(), MAX_LEVELS);
    assert!(live.set_level(MAX_LEVELS + 1).is_none());
    // An empty shape has nothing to subdivide.
    let mut empty = LiveSculpt::new(Arc::new(Mesh::new()), None);
    assert_eq!(empty.add_level(), Err(LevelProblem::NoShape));
    // A sculpt showing a level it hasn't isn't made.
    assert!(live.sculpt().showing(MAX_LEVELS + 1).is_none());
    assert!(live.sculpt().with_level(0, Vec::new(), Moves::zero(0)).is_none());
}

#[test]
fn deleting_higher_levels_keeps_the_shape_shown() {
    let input = sphere();
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    let front = Vec3::new(0.0, -300.0, 0.0);
    for _ in 0..3 {
        live.add_level().unwrap();
        stroke(&mut live, BrushKind::Draw, front, 10.0);
    }
    // A mask on the finest level comes down with the levels.
    let values: Vec<u8> = (0..live.mesh().point_slot_count())
        .map(|i| (i % 251) as u8)
        .collect();
    live.set_mask(Mask::from_values(&values)).unwrap();
    live.set_level(1).unwrap();
    let shown = live.mesh().clone();
    let points = shown.point_slot_count();
    assert_eq!(live.mask().values(), values[..points]);
    let sculpt = live.delete_higher().unwrap();
    assert_eq!((sculpt.top(), sculpt.shown(), sculpt.mask_level()), (1, 1, 1));
    assert_eq!(sculpt.mask().values(), values[..points]);
    assert_eq!(live.mesh(), &shown);
    assert_eq!(&evaluated(&input, &sculpt), &shown);
    assert!(live.delete_higher().is_none());
    // A level added again starts with nothing on it.
    let again = live.add_level().unwrap();
    assert!(again.level_moves(2).unwrap().is_zero());
    assert_eq!(&evaluated(&input, &again), live.mesh());
}

#[test]
fn deleting_lower_levels_keeps_every_levels_shape() {
    let input = sphere();
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    let front = Vec3::new(0.0, -300.0, 0.0);
    stroke(&mut live, BrushKind::Inflate, front, 40.0);
    for _ in 0..3 {
        live.add_level().unwrap();
        stroke(&mut live, BrushKind::Draw, front, 12.0);
    }
    live.set_level(2).unwrap();
    let values: Vec<u8> = (0..live.mesh().point_slot_count())
        .map(|i| if i % 2 == 0 { 255 } else { 40 })
        .collect();
    live.set_mask(Mask::from_values(&values)).unwrap();
    let old = live.set_level(1).unwrap().with_export(ExportLevel::Level(3));
    live.follow(&old);
    let shapes: Vec<Mesh> = (0..=3)
        .map(|k| {
            let mut m = (*input).clone();
            old.apply_at(&mut m, k);
            m
        })
        .collect();
    let (base, new) = live.without_lower().expect("lower levels to delete");
    // Level 1 is the shape itself now; levels 2 and 3 are levels 1 and 2.
    assert_eq!(*base, shapes[1]);
    assert_eq!((new.level_count(), new.shown()), (3, 0));
    assert!(new.fits(&base));
    assert!(new.level_moves(0).unwrap().is_zero());
    for k in 0..=2 {
        let mut m = (*base).clone();
        new.apply_at(&mut m, k);
        assert_eq!(m, shapes[k + 1], "level {k}");
        // Every finer level fits as it is: nothing is found again.
        let mut m = (*base).clone();
        assert!(!new.showing(k).unwrap().apply(&mut m).found_again);
    }
    // The mask stays on its level, and the export's level moves down.
    assert_eq!(new.mask_level(), 1);
    assert_eq!(new.mask().values(), values);
    assert_eq!(new.export(), ExportLevel::Level(2));
    // Nothing changed in the live sculpt itself.
    assert_eq!(live.level(), 1);
    assert_eq!(live.mesh(), &shapes[1]);
    // On the finest level, only that level is left.
    live.set_level(3).unwrap();
    let (base, new) = live.without_lower().unwrap();
    assert_eq!(*base, shapes[3]);
    assert_eq!(new.level_count(), 1);
    assert!(new.mask().len() == base.point_slot_count());
}

#[test]
fn the_mask_shows_on_every_level_and_stays_where_it_was_changed() {
    let input = Arc::new(grid(4));
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    live.add_level().unwrap();
    live.add_level().unwrap();
    live.set_level(1).unwrap();
    // Half of level 1 masked.
    let points = live.mesh().point_slot_count();
    let values: Vec<u8> = (0..points)
        .map(|i| if live.mesh().position(VertexId::from_raw(i as u32)).unwrap().x < 0.0 { 255 } else { 0 })
        .collect();
    let sculpt = live.set_mask(Mask::from_values(&values)).unwrap();
    assert_eq!(sculpt.mask_level(), 1);
    // On the level above, the new points take what lies between.
    let up = live.set_level(2).unwrap();
    assert_eq!(up.mask_level(), 1, "showing it changes nothing");
    assert_eq!(live.mask().len(), live.mesh().point_slot_count());
    assert_eq!(live.mask().values()[..points], values[..]);
    for (v, p) in live.mesh().vertices() {
        let value = live.mask().get(v.get() as usize);
        if p.x < -0.3 {
            assert_eq!(value, 255, "{p:?}");
        } else if p.x > 0.3 {
            assert_eq!(value, 0, "{p:?}");
        }
    }
    // A masked point holds still there, and a free one moves.
    let moved = live.dab(&dab(BrushKind::Draw, Vec3::ZERO, 10.0));
    assert!(moved > 0);
    for (v, p) in live.mesh().vertices() {
        if live.mask().get(v.get() as usize) == 255 {
            assert_eq!(p.z, 0.0);
        }
    }
    live.cancel();
    // On the level below, its own points' values.
    live.set_level(0).unwrap();
    let low = live.mesh().point_slot_count();
    assert_eq!(live.mask().values(), values[..low]);
    // Changed there, it lives there: coarser on the levels above.
    let cleared = live.set_mask(mask::invert(live.mask())).unwrap();
    assert_eq!(cleared.mask_level(), 0);
    assert_eq!(cleared.mask().len(), low);
    live.set_level(2).unwrap();
    assert_eq!(live.mask().len(), live.mesh().point_slot_count());
    assert_eq!(live.mask().get(0), 255 - values[0]);
    // The Mask brush on a level keeps the mask on that level.
    let painted = {
        live.dab(&dab(BrushKind::Mask, Vec3::new(-1.0, -1.0, 0.0), 0.8));
        live.finish().unwrap()
    };
    assert_eq!(painted.mask_level(), 2);
    assert_eq!(painted.mask().len(), live.mesh().point_slot_count());
    // Undo brings the coarser mask back as it showed.
    let shown = live.mask().clone();
    live.follow(&cleared.showing(2).unwrap());
    assert_ne!(live.mask(), &shown);
    live.follow(&painted);
    assert_eq!(live.mask(), &shown);
}

#[test]
fn a_changed_shape_under_levels_is_fitted_level_by_level() {
    let input = Arc::new(grid(8));
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    live.dab(&dab(BrushKind::Draw, Vec3::ZERO, 3.0));
    live.finish().unwrap();
    live.add_level().unwrap();
    live.add_level().unwrap();
    for _ in 0..4 {
        live.dab(&dab(BrushKind::Draw, Vec3::new(1.0, 1.0, 0.0), 1.0));
    }
    live.finish().unwrap();
    let values: Vec<u8> = live
        .mesh()
        .vertices()
        .map(|(_, p)| if p.x < -2.0 { 255 } else { 0 })
        .collect();
    let sculpt = live.set_mask(Mask::from_values(&values)).unwrap();
    let was = live.mesh().clone();
    let height = |m: &Mesh, at: Vec3| m.position(nearest_flat(m, at)).unwrap().z;
    let fine = height(&was, Vec3::new(1.0, 1.0, 0.0));
    assert!(fine > height(&was, Vec3::new(-1.0, -1.0, 0.0)) + 0.05);
    // The same sheet with more polygons: every level finds its place by
    // position, and says so.
    let finer = Arc::new(scaled(&grid(12), 8.0 / 12.0));
    let mut out = (*finer).clone();
    assert!(sculpt.apply(&mut out).found_again);
    assert_eq!(out.face_count(), 144 * 16);
    assert!(out.vertices().all(|(_, p)| p.is_finite()));
    for at in [Vec3::new(1.0, 1.0, 0.0), Vec3::ZERO, Vec3::new(-3.0, 2.0, 0.0)] {
        let (a, b) = (height(&was, at), height(&out, at));
        assert!((a - b).abs() < 0.08, "at {at:?}: {a} then {b}");
    }
    // The live sculpt fits it the same way, and its mask with it.
    let mut live = LiveSculpt::new(Arc::clone(&finer), Some(&sculpt));
    assert!(live.was_fitted());
    assert_eq!(live.mesh(), &out);
    assert_eq!(live.mask().len(), out.point_slot_count());
    for (v, p) in live.mesh().vertices() {
        let value = live.mask().get(v.get() as usize);
        if p.x < -2.3 {
            assert_eq!(value, 255, "{p:?}");
        } else if p.x > -1.7 {
            assert_eq!(value, 0, "{p:?}");
        }
    }
    // The next stroke's sculpt is made for the new shape: it fits as it
    // is, and evaluates exactly.
    live.dab(&dab(BrushKind::Draw, Vec3::new(2.0, -2.0, 0.0), 1.0));
    let fitted = live.finish().unwrap();
    assert!(fitted.fits(&finer));
    let mut again = (*finer).clone();
    assert!(!fitted.apply(&mut again).found_again);
    assert_eq!(&again, live.mesh());
    // A mask on a finer level than the one shown comes down to it.
    let lower = sculpt.showing(1).unwrap();
    let live = LiveSculpt::new(Arc::clone(&finer), Some(&lower));
    assert_eq!(live.level(), 1);
    assert_eq!(live.sculpt().mask_level(), 1);
    assert_eq!(live.mask().len(), live.mesh().point_slot_count());
    assert!(!live.mask().is_zero());
}

/// The point of `m` nearest `at`, looking from above.
fn nearest_flat(m: &Mesh, at: Vec3) -> VertexId {
    m.vertices()
        .min_by(|a, b| {
            a.1.truncate()
                .distance(at.truncate())
                .total_cmp(&b.1.truncate().distance(at.truncate()))
        })
        .unwrap()
        .0
}

/// `m` scaled about the middle.
fn scaled(m: &Mesh, by: f32) -> Mesh {
    let mut out = m.clone();
    for (v, p) in m.vertices() {
        out.set_position(v, p * by);
    }
    out
}

#[test]
fn an_export_takes_the_level_chosen() {
    let input = Arc::new(grid(4));
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    live.add_level().unwrap();
    live.add_level().unwrap();
    let sculpt = live.set_level(1).unwrap();
    assert_eq!(sculpt.export(), ExportLevel::Shown);
    assert_eq!(sculpt.export_level(), 1);
    assert!(sculpt.for_export().is_none());
    assert!(live.set_export(ExportLevel::Shown).is_none());
    let finest = live.set_export(ExportLevel::Finest).unwrap();
    assert_eq!(finest.export_level(), 2);
    assert_eq!(finest.shown(), 1, "the view keeps its level");
    assert_eq!(finest.for_export().unwrap().shown(), 2);
    assert_ne!(finest, sculpt);
    let low = live.set_export(ExportLevel::Level(0)).unwrap();
    assert_eq!(low.for_export().unwrap().shown(), 0);
    // A level it hasn't means the finest; the level shown, nothing else.
    let past = low.with_export(ExportLevel::Level(9));
    assert_eq!(past.export_level(), 2);
    assert!(low.with_export(ExportLevel::Level(1)).for_export().is_none());
    // Going to another level keeps the choice.
    let up = live.set_level(2).unwrap();
    assert_eq!(up.export(), ExportLevel::Level(0));
}

#[test]
fn relax_evens_out_a_flat_sheets_polygons_and_keeps_it_flat() {
    let mut uneven = grid(10);
    // The inner points pushed about along the sheet.
    let n = 11;
    for y in 1..n - 1 {
        for x in 1..n - 1 {
            let v = VertexId::from_raw((y * n + x) as u32);
            let p = uneven.position(v).unwrap();
            let push = Vec3::new(
                (((x * 7 + y * 3) % 5) as f32 - 2.0) * 0.15,
                (((x * 3 + y * 11) % 7) as f32 - 3.0) * 0.1,
                0.0,
            );
            uneven.set_position(v, p + push);
        }
    }
    let even = grid(10);
    let off = |m: &Mesh| -> f32 {
        m.vertices()
            .map(|(v, p)| p.distance(even.position(v).unwrap()))
            .sum()
    };
    let input = Arc::new(uneven);
    let before = off(&input);
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    for _ in 0..20 {
        live.dab(&dab(BrushKind::Relax, Vec3::ZERO, 20.0));
    }
    let after = off(live.mesh());
    assert!(after < before * 0.2, "{before} then {after}");
    // Flat still, and the open edge where it was, corners and all.
    for (v, p) in live.mesh().vertices() {
        assert_eq!(p.z, 0.0);
        let start = even.position(v).unwrap();
        if start.x.abs() == 5.0 || start.y.abs() == 5.0 {
            assert!(p.distance(start) < 1e-5, "{v:?} on the edge moved");
        }
    }
    // Ctrl changes nothing about it.
    let mut inverted = LiveSculpt::new(Arc::clone(&input), None);
    let mut plain = LiveSculpt::new(Arc::clone(&input), None);
    inverted.dab(&Dab {
        invert: true,
        ..dab(BrushKind::Relax, Vec3::ZERO, 20.0)
    });
    plain.dab(&dab(BrushKind::Relax, Vec3::ZERO, 20.0));
    assert_eq!(inverted.mesh(), plain.mesh());
    let sculpt = live.finish().unwrap();
    assert_eq!(&evaluated(&input, &sculpt), live.mesh());
}

#[test]
fn relax_keeps_a_balls_shape() {
    let input = Arc::new(mesh::samples::heavy_ball(20_000, 50.0));
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    // Polygons pulled long by a grab along the surface.
    let front = live
        .hit(Vec3::new(0.0, -500.0, 50.0), Vec3::Y)
        .unwrap()
        .point;
    live.dab(&dab(BrushKind::Grab, front, 20.0));
    live.dab(&dab(BrushKind::Grab, front + Vec3::X * 8.0, 20.0));
    live.finish().unwrap();
    let pulled = live.mesh().clone();
    let surface = mesh::bvh::TriangleTree::of_mesh(&pulled, glam::Mat4::IDENTITY);
    for _ in 0..10 {
        live.dab(&dab(BrushKind::Relax, front, 30.0));
    }
    let (mut slid, mut off) = (0.0f32, 0.0f32);
    for (v, p) in pulled.vertices() {
        let now = live.mesh().position(v).unwrap();
        slid = slid.max(now.distance(p));
        off = off.max(surface.nearest(now).unwrap().distance);
    }
    // The points slide along the surface, and stay on it near enough.
    assert!(slid > 0.5, "points slid {slid}");
    assert!(off < slid * 0.1, "points slid {slid} and left the surface by {off}");
}

#[test]
fn lower_levels_are_rebuilt_for_a_shape_without_any() {
    // A ball with a bump: an even shape of about 12 000 polygons, as a
    // remesh leaves it.
    let ball = Arc::new(mesh::samples::heavy_ball(12_000, 100.0));
    let mut live = LiveSculpt::new(Arc::clone(&ball), None);
    let front = live
        .hit(Vec3::new(0.0, -500.0, 100.0), Vec3::Y)
        .unwrap()
        .point;
    for _ in 0..6 {
        live.dab(&dab(BrushKind::Draw, front, 40.0));
    }
    live.finish().unwrap();
    let values: Vec<u8> = live
        .mesh()
        .point_slots()
        .map(|p| if p.is_some_and(|p| p.x > 0.0) { 255 } else { 0 })
        .collect();
    let mask = Mask::from_values(&values);
    let shape = live.mesh().clone();
    let progress = RebuildProgress::default();
    let rebuilt = rebuild_lower(&shape, 2, Some(&mask), &progress).unwrap();
    assert!((progress.share() - 1.0).abs() < 1e-3);
    let (base, sculpt) = (rebuilt.base, rebuilt.sculpt);
    assert_eq!((sculpt.level_count(), sculpt.shown()), (3, 2));
    assert!(sculpt.fits(&base));
    // About a sixteenth of the polygons at the bottom, about as many at
    // the top.
    let polygons = shape.face_count() as f32;
    let low = base.face_count() as f32;
    assert!(low > polygons / 16.0 * 0.5 && low < polygons / 16.0 * 1.6, "{low}");
    let mut finest = base.clone();
    assert!(!sculpt.apply(&mut finest).found_again);
    assert_eq!(finest.face_count(), base.face_count() * 16);
    // The finest level lies on the shape, bump and all.
    let surface = mesh::bvh::TriangleTree::of_mesh(&shape, glam::Mat4::IDENTITY);
    let mut worst = 0.0f32;
    for (_, p) in finest.vertices() {
        worst = worst.max(surface.nearest(p).unwrap().distance);
    }
    assert!(worst < 0.05, "a point is {worst} off the shape");
    let rebuilt_surface = mesh::bvh::TriangleTree::of_mesh(&finest, glam::Mat4::IDENTITY);
    let mut worst = 0.0f32;
    for (_, p) in shape.vertices() {
        worst = worst.max(rebuilt_surface.nearest(p).unwrap().distance);
    }
    let mut at = Vec3::ZERO;
    for (_, p) in shape.vertices() {
        let d = rebuilt_surface.nearest(p).unwrap().distance;
        if d >= worst { at = p; }
    }
    assert!(worst < 2.0, "the shape is {worst} off the rebuilt one at {at:?}");
    // The lowest level is a coarse copy: near the shape, but not on it.
    let mut lowest = base.clone();
    sculpt.apply_at(&mut lowest, 0);
    assert_eq!(lowest, base);
    // The mask is carried by position, on the finest level.
    assert_eq!(sculpt.mask_level(), 2);
    for (v, p) in finest.vertices() {
        let value = sculpt.mask().get(v.get() as usize);
        if p.x > 6.0 {
            assert_eq!(value, 255, "{p:?}");
        } else if p.x < -6.0 {
            assert_eq!(value, 0, "{p:?}");
        }
    }
    // The live sculpt goes down them and back.
    let base = Arc::new(base);
    let mut live = LiveSculpt::new(Arc::clone(&base), Some(&sculpt));
    assert_eq!(live.mesh(), &finest);
    let lower = live.set_level(0).unwrap();
    assert_eq!(&evaluated(&base, &lower), live.mesh());
}

#[test]
fn a_rebuild_makes_as_many_levels_as_fit_and_can_be_stopped() {
    assert_eq!(levels_for(12_000, 2), 2);
    assert_eq!(levels_for(12_000, 4), 2);
    assert_eq!(levels_for(3_000_000, 4), 4);
    assert_eq!(levels_for(3_000_000, 9), MAX_LEVELS);
    assert_eq!(levels_for(799, 2), 0);
    assert_eq!(levels_for(800, 2), 1);
    assert_eq!(levels_for(0, 2), 0);
    let cube = PrimitiveKind::Cube.standard().mesh();
    let progress = RebuildProgress::default();
    assert_eq!(
        rebuild_lower(&cube, 2, None, &progress).unwrap_err(),
        RebuildProblem::TooFew
    );
    let ball = mesh::samples::heavy_ball(12_000, 100.0);
    progress.stop();
    assert!(progress.cancelled());
    assert_eq!(
        rebuild_lower(&ball, 2, None, &progress).unwrap_err(),
        RebuildProblem::Cancelled
    );
    // Wanting nothing gives nothing.
    assert_eq!(
        rebuild_lower(&ball, 0, None, &RebuildProgress::default()).unwrap_err(),
        RebuildProblem::TooFew
    );
}
