//! Tests of a Sculpt layer's data and the live sculpt.

use super::*;
use glam::Vec3;
use mesh::{Mesh, PrimitiveKind, VertexId};
use std::sync::Arc;

/// A flat grid of `n` × `n` quads, 1 apart, facing up (Z), its corner at
/// the middle.
pub(crate) fn grid(n: usize) -> Mesh {
    let mut m = Mesh::new();
    let half = n as f32 / 2.0;
    for y in 0..=n {
        for x in 0..=n {
            m.add_vertex(Vec3::new(x as f32 - half, y as f32 - half, 0.0))
                .unwrap();
        }
    }
    let at = |x: usize, y: usize| VertexId::from_raw((y * (n + 1) + x) as u32);
    for y in 0..n {
        for x in 0..n {
            m.add_face(&[at(x, y), at(x + 1, y), at(x + 1, y + 1), at(x, y + 1)])
                .unwrap();
        }
    }
    m
}

fn draw(centre: Vec3, radius: f32) -> Dab {
    Dab {
        kind: BrushKind::Draw,
        centre,
        radius,
        strength: 1.0,
        falloff: Falloff::Smooth,
        invert: false,
    }
}

/// Evaluating `sculpt` on `input`, as the scene does.
pub(crate) fn evaluated(input: &Mesh, sculpt: &Sculpt) -> Mesh {
    let mut m = input.clone();
    sculpt.apply(&mut m);
    m
}

#[test]
fn a_dab_moves_only_the_points_in_reach_by_its_falloff() {
    let input = Arc::new(grid(20));
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    let moved = live.dab(&draw(Vec3::ZERO, 3.0));
    assert!(moved > 0);
    for (v, p) in input.vertices() {
        let now = live.mesh().position(v).unwrap();
        let d = p.distance(Vec3::ZERO);
        if d >= 3.0 {
            assert_eq!(now, p, "{v:?} out of reach moved");
        } else {
            // Straight up, by the falloff.
            let want = Falloff::Smooth.at(d / 3.0) * brush::DRAW_RATE * 3.0;
            assert!((now.z - want).abs() < 1e-5, "{v:?}: {} vs {want}", now.z);
            assert!((now.truncate() - p.truncate()).length() < 1e-6);
        }
    }
}

#[test]
fn ctrl_digs_in_and_no_strength_does_nothing() {
    let input = Arc::new(grid(10));
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    live.dab(&Dab {
        invert: true,
        ..draw(Vec3::ZERO, 2.0)
    });
    assert!(live.mesh().position(VertexId::from_raw(60)).unwrap().z < 0.0);
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    let none = live.dab(&Dab {
        strength: 0.0,
        ..draw(Vec3::ZERO, 2.0)
    });
    assert_eq!(none, 0);
    assert!(live.finish().is_none(), "nothing to undo");
}

#[test]
fn a_finished_stroke_evaluates_to_exactly_the_live_shape() {
    let input = Arc::new(PrimitiveKind::Sphere.standard().mesh());
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    let top = input
        .vertices()
        .map(|(_, p)| p)
        .fold(Vec3::splat(f32::NEG_INFINITY), Vec3::max);
    for i in 0..10 {
        live.dab(&draw(Vec3::new(i as f32 * 3.0, 0.0, top.z), 20.0));
    }
    let sculpt = live.finish().expect("a stroke");
    assert_eq!(&evaluated(&input, &sculpt), live.mesh());
    // A second stroke on top, with Smooth, keeps the promise.
    for i in 0..5 {
        live.dab(&Dab {
            kind: BrushKind::Smooth,
            ..draw(Vec3::new(i as f32 * 3.0, 0.0, top.z), 20.0)
        });
    }
    let second = live.finish().expect("a second stroke");
    assert_eq!(&evaluated(&input, &second), live.mesh());
    assert_ne!(sculpt, second);
}

#[test]
fn a_stroke_keeps_only_the_chunks_it_touched() {
    let input = Arc::new(mesh::samples::heavy_ball(200_000, 100.0));
    assert!(input.vertex_count() > 3 * CHUNK);
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    let before = live.sculpt().clone();
    let at = live
        .hit(Vec3::new(0.0, -500.0, 100.0), Vec3::Y)
        .expect("the ball's side");
    live.dab(&draw(at.point, 5.0));
    let after = live.finish().unwrap();
    let held = after.memory_apart(&before);
    assert!(held > 0);
    assert!(
        held < after.memory_size() / 2,
        "{held} of {}",
        after.memory_size()
    );
    let differing = after.moves().differing(before.moves());
    assert!(
        !differing.is_empty() && differing.len() <= 4,
        "{differing:?}"
    );
}

#[test]
fn a_dab_at_the_edge_of_an_open_shape_keeps_the_edge_in_line() {
    let input = Arc::new(grid(10));
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    // Push the edge up, then smooth it: the edge points stay on the edge
    // line (y = -5), only rising and falling.
    live.dab(&draw(Vec3::new(0.0, -5.0, 0.0), 3.0));
    for _ in 0..5 {
        live.dab(&Dab {
            kind: BrushKind::Smooth,
            ..draw(Vec3::new(0.0, -5.0, 0.0), 3.0)
        });
    }
    for x in 0..=10 {
        let v = VertexId::from_raw(x);
        let p = live.mesh().position(v).unwrap();
        assert!((p.y + 5.0).abs() < 1e-5, "{p}");
    }
    assert!(live.finish().is_some());
}

#[test]
fn one_polygon_and_an_empty_shape_are_sculpted_without_trouble() {
    let mut one = Mesh::new();
    let a = one.add_vertex(Vec3::ZERO).unwrap();
    let b = one.add_vertex(Vec3::X).unwrap();
    let c = one.add_vertex(Vec3::Y).unwrap();
    one.add_face(&[a, b, c]).unwrap();
    let one = Arc::new(one);
    let mut live = LiveSculpt::new(Arc::clone(&one), None);
    assert!(live.hit(Vec3::new(0.2, 0.2, 5.0), Vec3::NEG_Z).is_some());
    live.dab(&draw(Vec3::new(0.3, 0.3, 0.0), 5.0));
    live.dab(&Dab {
        kind: BrushKind::Smooth,
        ..draw(Vec3::ZERO, 5.0)
    });
    let s = live.finish().unwrap();
    assert_eq!(&evaluated(&one, &s), live.mesh());

    let empty = Arc::new(Mesh::new());
    let mut live = LiveSculpt::new(Arc::clone(&empty), None);
    assert!(live.hit(Vec3::ZERO, Vec3::X).is_none());
    assert_eq!(live.dab(&draw(Vec3::ZERO, 1.0)), 0);
    assert!(live.finish().is_none());
    // Broken dabs do nothing.
    let mut live = LiveSculpt::new(Arc::new(grid(4)), None);
    assert_eq!(live.dab(&draw(Vec3::NAN, 1.0)), 0);
    assert_eq!(live.dab(&draw(Vec3::ZERO, -1.0)), 0);
    assert_eq!(live.dab(&draw(Vec3::ZERO, f32::NAN)), 0);
}

#[test]
fn smooth_flattens_a_bump() {
    let input = Arc::new(grid(20));
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    for _ in 0..10 {
        live.dab(&Dab {
            falloff: Falloff::Sharp,
            ..draw(Vec3::ZERO, 2.0)
        });
    }
    let peak = |live: &LiveSculpt| {
        live.mesh()
            .vertices()
            .map(|(_, p)| p.z)
            .fold(f32::NEG_INFINITY, f32::max)
    };
    let before = peak(&live);
    for _ in 0..20 {
        live.dab(&Dab {
            kind: BrushKind::Smooth,
            ..draw(Vec3::ZERO, 4.0)
        });
    }
    assert!(peak(&live) < before * 0.7, "{} vs {before}", peak(&live));
}

#[test]
fn cancelling_a_stroke_puts_every_point_back() {
    let input = Arc::new(grid(10));
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    live.dab(&draw(Vec3::ZERO, 3.0));
    let first = live.finish().unwrap();
    let kept = live.mesh().clone();
    live.dab(&draw(Vec3::ONE, 3.0));
    assert_ne!(live.mesh(), &kept);
    live.cancel();
    assert_eq!(live.mesh(), &kept);
    assert!(!live.stroking());
    assert_eq!(live.sculpt(), &first);
}

#[test]
fn following_an_undo_and_a_redo_restores_exact_places() {
    let input = Arc::new(grid(12));
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    let empty = live.sculpt().clone();
    live.dab(&draw(Vec3::ZERO, 3.0));
    let one = live.finish().unwrap();
    let after_one = live.mesh().clone();
    live.dab(&draw(Vec3::new(2.0, 2.0, 0.0), 3.0));
    let two = live.finish().unwrap();
    let after_two = live.mesh().clone();
    // Undo, undo, redo, redo.
    live.follow(&one);
    assert_eq!(live.mesh(), &after_one);
    live.follow(&empty);
    assert_eq!(live.mesh(), &*input);
    live.follow(&one);
    assert_eq!(live.mesh(), &after_one);
    live.follow(&two);
    assert_eq!(live.mesh(), &after_two);
    assert_eq!(&evaluated(&input, &two), &after_two);
}

#[test]
fn a_changed_shape_under_the_sculpt_is_fitted_by_position() {
    let input = Arc::new(grid(10));
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    live.dab(&draw(Vec3::ZERO, 3.0));
    let sculpt = live.finish().unwrap();
    // The same flat square, more finely divided: renumbered points.
    let mut finer = Mesh::new();
    let n = 20;
    for y in 0..=n {
        for x in 0..=n {
            finer
                .add_vertex(Vec3::new(x as f32 * 0.5 - 5.0, y as f32 * 0.5 - 5.0, 0.0))
                .unwrap();
        }
    }
    let at = |x: usize, y: usize| VertexId::from_raw((y * (n + 1) + x) as u32);
    for y in 0..n {
        for x in 0..n {
            finer
                .add_face(&[at(x, y), at(x + 1, y), at(x + 1, y + 1), at(x, y + 1)])
                .unwrap();
        }
    }
    assert!(!sculpt.fits(&finer));
    let mut shaped = finer.clone();
    let fit = sculpt.apply(&mut shaped);
    assert!(fit.found_again);
    // The middle is still raised; the far corner isn't.
    let middle = shaped.position(at(10, 10)).unwrap();
    assert!(middle.z > 0.05, "{middle}");
    assert_eq!(shaped.position(at(0, 0)).unwrap().z, 0.0);
    // Sculpting on it starts from the fitted sculpt.
    let live = LiveSculpt::new(Arc::new(finer), Some(&sculpt));
    assert!(live.was_fitted());
    assert_eq!(live.mesh(), &shaped);
}

#[test]
fn moves_are_measured_along_the_surface_so_detail_rides_a_change_below() {
    let input = Arc::new(grid(10));
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    live.dab(&draw(Vec3::ZERO, 3.0));
    let sculpt = live.finish().unwrap();
    // The same grid stood up on its side (facing Y): the bump now
    // points along Y.
    let mut turned = (*input).clone();
    for (v, p) in input.vertices() {
        turned.set_position(v, Vec3::new(p.x, 0.0, p.y));
    }
    let middle = VertexId::from_raw(60);
    let out = evaluated(&turned, &sculpt);
    let p = out.position(middle).unwrap();
    assert!(p.y.abs() > 0.05 && p.x.abs() < 1e-5, "{p}");
}

#[test]
fn damaged_parts_are_refused() {
    let m = grid(2);
    let s = Sculpt::new(&m);
    assert!(Sculpt::from_parts(1, vec![Vec3::ZERO; 3], Moves::zero(4)).is_none());
    let mut bad = Moves::zero(9);
    bad.set_chunk(0, vec![Vec3::NAN; 9]);
    assert!(Sculpt::from_parts(1, vec![Vec3::ZERO; 9], bad.clone()).is_none());
    assert!(s.with_moves(bad).is_none());
    assert!(s.with_moves(Moves::zero(3)).is_none());
    let ok = Sculpt::from_parts(s.structure(), s.places().to_vec(), Moves::zero(9)).unwrap();
    assert_eq!(ok, s);
}

/// The benchmark: one dab of a big and a small brush on a shape of a
/// million points, and finishing the stroke. Run with
/// `cargo test --release -p sculpt dab_benchmark -- --ignored --nocapture`.
#[test]
#[ignore = "a benchmark"]
fn dab_benchmark() {
    use std::time::Instant;
    let input = Arc::new(mesh::samples::heavy_ball(1_000_000, 100.0));
    let start = Instant::now();
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    println!(
        "Start sculpting {} points: {:.1} ms",
        input.vertex_count(),
        start.elapsed().as_secs_f64() * 1e3
    );
    let at = live
        .hit(Vec3::new(0.0, -500.0, 100.0), Vec3::Y)
        .unwrap()
        .point;
    for (radius, name) in [(28.0, "small"), (90.0, "big")] {
        let start = Instant::now();
        let mut moved = 0;
        for i in 0..10 {
            moved = live.dab(&draw(at + Vec3::X * i as f32, radius));
        }
        println!(
            "A dab of the {name} brush ({moved} points): {:.2} ms",
            start.elapsed().as_secs_f64() * 1e2
        );
    }
    let start = Instant::now();
    live.finish();
    println!(
        "Finish the stroke: {:.1} ms",
        start.elapsed().as_secs_f64() * 1e3
    );
}
