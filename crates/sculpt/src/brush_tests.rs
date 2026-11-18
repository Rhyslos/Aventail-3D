//! Tests of the brushes for forms (decision 0306): each on a flat grid
//! against a known result, with Ctrl, at no strength and bigger than the
//! shape; Grab, Snake hook and Elastic grab following where they are
//! taken; Elastic grab keeping the volume; and a benchmark per brush.

use crate::forms::{self, CLAY_RATE, CLAY_SHARE, CREASE_RATE, INFLATE_RATE};
use crate::tests::{evaluated, grid};
use crate::*;
use glam::Vec3;
use mesh::{Mesh, VertexId};
use std::sync::Arc;

/// A dab of `kind` at full strength.
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

/// The point of a `grid(20)` at (x, y).
fn at(x: i32, y: i32) -> VertexId {
    VertexId::from_raw(((y + 10) * 21 + (x + 10)) as u32)
}

fn place(live: &LiveSculpt, v: VertexId) -> Vec3 {
    live.mesh().position(v).unwrap()
}

/// A live sculpt on a flat 20 × 20 grid, after one dab of `d`.
fn one_dab(d: &Dab) -> LiveSculpt {
    let mut live = LiveSculpt::new(Arc::new(grid(20)), None);
    live.dab(d);
    live
}

#[test]
fn clay_strips_lays_a_square_and_clay_a_round_patch() {
    let level = CLAY_RATE * 4.0 * CLAY_SHARE;
    for kind in [BrushKind::ClayStrips, BrushKind::Clay] {
        let live = one_dab(&dab(kind, Vec3::ZERO, 4.0));
        // The middle rises half way to the clay's level.
        assert!((place(&live, at(0, 0)).z - level).abs() < 1e-5, "{kind:?}");
        // Off the side: nothing.
        assert_eq!(place(&live, at(5, 0)).z, 0.0, "{kind:?}");
    }
    // Towards a corner, (2, 2) is 2.8 from the middle: inside the square
    // (half way to its edge) but further out of the round brush.
    let strips = one_dab(&dab(BrushKind::ClayStrips, Vec3::ZERO, 4.0));
    let round = one_dab(&dab(BrushKind::Clay, Vec3::ZERO, 4.0));
    let half = Falloff::Smooth.at(0.5) * level;
    assert!((place(&strips, at(2, 2)).z - half).abs() < 1e-5);
    assert!(place(&round, at(2, 2)).z < half * 0.5);
    // The square's corner region (3, 3) is past the round brush.
    assert!(place(&strips, at(3, 3)).z > 0.0);
    assert_eq!(place(&round, at(3, 3)).z, 0.0);
    // Ctrl carves down.
    let carved = one_dab(&Dab {
        invert: true,
        ..dab(BrushKind::ClayStrips, Vec3::ZERO, 4.0)
    });
    assert!((place(&carved, at(0, 0)).z + level).abs() < 1e-5);
}

#[test]
fn clay_fills_hollows_before_building_on_top() {
    let mut input = grid(20);
    input.set_position(at(1, 0), Vec3::new(1.0, 0.0, -0.5));
    input.set_position(at(-1, 0), Vec3::new(-1.0, 0.0, 1.5));
    let mut live = LiveSculpt::new(Arc::new(input), None);
    live.dab(&dab(BrushKind::Clay, Vec3::ZERO, 4.0));
    // The hollow rose a lot; the lump above the clay's level stayed.
    assert!(place(&live, at(1, 0)).z > -0.3);
    assert_eq!(place(&live, at(-1, 0)).z, 1.5);
}

#[test]
fn inflate_swells_along_each_points_facing() {
    let live = one_dab(&dab(BrushKind::Inflate, Vec3::ZERO, 4.0));
    let middle = place(&live, at(0, 0));
    assert!((middle.z - INFLATE_RATE * 4.0).abs() < 1e-5);
    assert_eq!(middle.truncate(), glam::Vec2::ZERO);
    let deflated = one_dab(&Dab {
        invert: true,
        ..dab(BrushKind::Inflate, Vec3::ZERO, 4.0)
    });
    assert!((place(&deflated, at(0, 0)).z + INFLATE_RATE * 4.0).abs() < 1e-5);
    // On a ball, every point swells out from the middle.
    let ball = Arc::new(mesh::samples::heavy_ball(2_000, 10.0));
    let mut live = LiveSculpt::new(Arc::clone(&ball), None);
    let top = live.hit(Vec3::new(0.0, 0.0, 50.0), Vec3::NEG_Z).unwrap();
    live.dab(&dab(BrushKind::Inflate, top.point, 6.0));
    for (v, p) in ball.vertices() {
        let now = live.mesh().position(v).unwrap();
        assert!(now.length() >= p.length() - 1e-4, "{v:?} went in");
    }
}

#[test]
fn crease_cuts_a_groove_and_pinches_its_sides() {
    let live = one_dab(&dab(BrushKind::Crease, Vec3::ZERO, 4.0));
    assert!((place(&live, at(0, 0)).z + CREASE_RATE * 4.0).abs() < 1e-5);
    let side = place(&live, at(2, 0));
    assert!(side.z < 0.0 && side.x < 2.0 && side.x > 1.0, "{side}");
    let ridge = one_dab(&Dab {
        invert: true,
        ..dab(BrushKind::Crease, Vec3::ZERO, 4.0)
    });
    assert!(place(&ridge, at(0, 0)).z > 0.0);
    assert!(place(&ridge, at(2, 0)).x < 2.0);
}

#[test]
fn pinch_pulls_along_the_surface_towards_the_middle() {
    let live = one_dab(&dab(BrushKind::Pinch, Vec3::ZERO, 4.0));
    let p = place(&live, at(2, 1));
    assert!(p.x < 2.0 && p.y < 1.0 && p.x > 0.0 && p.y > 0.0);
    assert_eq!(p.z, 0.0);
    assert_eq!(place(&live, at(0, 0)), Vec3::ZERO);
    let apart = one_dab(&Dab {
        invert: true,
        ..dab(BrushKind::Pinch, Vec3::ZERO, 4.0)
    });
    assert!(place(&apart, at(2, 1)).x > 2.0);
}

#[test]
fn flatten_scrape_and_fill_level_bumps_and_hollows() {
    // A grid with a lump in the middle and a pit beside it.
    let mut input = grid(20);
    input.set_position(at(0, 0), Vec3::new(0.0, 0.0, 1.0));
    input.set_position(at(2, 0), Vec3::new(2.0, 0.0, -1.0));
    let input = Arc::new(input);
    let run = |kind, invert| {
        let mut live = LiveSculpt::new(Arc::clone(&input), None);
        live.dab(&Dab {
            invert,
            ..dab(kind, Vec3::ZERO, 4.0)
        });
        (place(&live, at(0, 0)).z, place(&live, at(2, 0)).z)
    };
    let (lump, pit) = run(BrushKind::Flatten, false);
    assert!(lump < 1.0 && pit > -1.0, "{lump} {pit}");
    let (lump, pit) = run(BrushKind::Scrape, false);
    assert!(lump < 1.0 && pit == -1.0, "{lump} {pit}");
    let (lump, pit) = run(BrushKind::Fill, false);
    assert!(lump == 1.0 && pit > -1.0, "{lump} {pit}");
    // Ctrl: Flatten pushes away; Scrape fills; Fill scrapes.
    let (lump, pit) = run(BrushKind::Flatten, true);
    assert!(lump > 1.0 && pit < -1.0, "{lump} {pit}");
    let (lump, pit) = run(BrushKind::Scrape, true);
    assert!(lump == 1.0 && pit > -1.0, "{lump} {pit}");
    let (lump, pit) = run(BrushKind::Fill, true);
    assert!(lump < 1.0 && pit == -1.0, "{lump} {pit}");
    // A flat grid has nothing to flatten.
    let mut flat = LiveSculpt::new(Arc::new(grid(20)), None);
    assert_eq!(flat.dab(&dab(BrushKind::Flatten, Vec3::ZERO, 4.0)), 0);
}

#[test]
fn grab_holds_the_lump_it_took_and_follows_the_mouse() {
    let mut live = LiveSculpt::new(Arc::new(grid(20)), None);
    let g = |centre| dab(BrushKind::Grab, centre, 3.0);
    assert_eq!(live.dab(&g(Vec3::ZERO)), 0, "taking hold moves nothing");
    assert_eq!(live.held(), Some(Vec3::ZERO));
    live.dab(&g(Vec3::new(0.0, 0.0, 2.0)));
    assert_eq!(place(&live, at(0, 0)), Vec3::new(0.0, 0.0, 2.0));
    // Back down half way: where the mouse is now, not added up.
    live.dab(&g(Vec3::new(0.0, 0.0, 1.0)));
    assert_eq!(place(&live, at(0, 0)), Vec3::new(0.0, 0.0, 1.0));
    let side = place(&live, at(2, 0));
    assert!(side.z > 0.0 && side.z < 1.0);
    assert_eq!(place(&live, at(3, 0)).z, 0.0);
    // Ctrl changes nothing for the grabbing brushes.
    let mut inv = LiveSculpt::new(Arc::new(grid(20)), None);
    for c in [Vec3::ZERO, Vec3::new(0.0, 0.0, 1.0)] {
        inv.dab(&Dab {
            invert: true,
            ..g(c)
        });
    }
    assert_eq!(inv.mesh(), live.mesh());
    // The stroke ends: one step, and nothing held.
    let input = Arc::clone(live.input());
    let s = live.finish().unwrap();
    assert_eq!(&evaluated(&input, &s), live.mesh());
    assert_eq!(live.held(), None);
}

#[test]
fn snake_hook_pulls_a_horn_out_along_the_stroke() {
    let mut live = LiveSculpt::new(Arc::new(grid(20)), None);
    let h = |centre| dab(BrushKind::SnakeHook, centre, 2.0);
    live.dab(&h(Vec3::ZERO));
    // Up five, then over to the side: the tip follows each step.
    let mut tip = Vec3::ZERO;
    for i in 1..=10 {
        tip = Vec3::new(0.0, 0.0, i as f32 * 0.5);
        live.dab(&h(tip));
    }
    for i in 1..=4 {
        tip = Vec3::new(i as f32 * 0.5, 0.0, 5.0);
        live.dab(&h(tip));
    }
    assert!(place(&live, at(0, 0)).distance(tip) < 1e-4);
    assert_eq!(live.held(), Some(tip));
    // Far further than the brush's reach from where it started, which
    // Grab can't do with the same brush, and the base stays put.
    assert!(place(&live, at(0, 0)).length() > 5.0);
    assert_eq!(place(&live, at(4, 0)).z, 0.0);
}

/// The volume a closed shape holds.
fn volume(m: &Mesh) -> f32 {
    m.faces()
        .map(|(_, corners)| {
            let p: Vec<Vec3> = corners.iter().map(|v| m.position(*v).unwrap()).collect();
            (1..p.len() - 1)
                .map(|i| p[0].dot(p[i].cross(p[i + 1])))
                .sum::<f32>()
                / 6.0
        })
        .sum()
}

#[test]
fn elastic_grab_bends_the_whole_shape_and_keeps_its_volume() {
    let ball = Arc::new(mesh::samples::heavy_ball(20_000, 1.0));
    let before = volume(&ball);
    let mut live = LiveSculpt::new(Arc::clone(&ball), None);
    let top = live
        .hit(Vec3::new(0.0, 0.0, 5.0), Vec3::NEG_Z)
        .unwrap()
        .point;
    let e = |centre| dab(BrushKind::ElasticGrab, centre, 1.0);
    live.dab(&e(top));
    for i in 1..=10 {
        live.dab(&e(top + Vec3::new(0.03, 0.0, 0.02) * i as f32));
    }
    let after = volume(live.mesh());
    assert!(
        ((after - before) / before).abs() < 0.02,
        "{before} became {after}"
    );
    // The middle went with the mouse; the far side moved a little too.
    let near = live.hit(Vec3::new(0.3, 0.0, 5.0), Vec3::NEG_Z).unwrap();
    assert!(near.point.z > 1.1, "{}", near.point);
    let bottom_before = ball.bounds().unwrap().min.z;
    let moved_far = ball
        .vertices()
        .any(|(v, p)| p.z < bottom_before + 0.2 && live.mesh().position(v).unwrap() != p);
    assert!(moved_far, "Elastic grab bends the whole shape");
}

#[test]
fn no_strength_does_nothing_with_any_brush() {
    for kind in BrushKind::ALL {
        let input = Arc::new(grid(10));
        let mut live = LiveSculpt::new(Arc::clone(&input), None);
        for c in [Vec3::ZERO, Vec3::new(0.5, 0.0, 1.0)] {
            let moved = live.dab(&Dab {
                strength: 0.0,
                ..dab(kind, c, 3.0)
            });
            assert_eq!(moved, 0, "{kind:?}");
        }
        assert_eq!(live.mesh(), &*input, "{kind:?}");
        assert!(live.finish().is_none(), "{kind:?}");
    }
}

#[test]
fn every_brush_bigger_than_the_shape_works_and_evaluates_exactly() {
    for kind in BrushKind::ALL {
        for invert in [false, true] {
            let mut input = grid(4);
            // A little relief, so Flatten and Smooth have work to do.
            input.set_position(VertexId::from_raw(12), Vec3::new(0.0, 0.0, 0.7));
            let input = Arc::new(input);
            let mut live = LiveSculpt::new(Arc::clone(&input), None);
            for c in [
                Vec3::ZERO,
                Vec3::new(0.5, 0.2, 0.8),
                Vec3::new(1.0, 0.4, 1.2),
            ] {
                live.dab(&Dab {
                    invert,
                    ..dab(kind, c, 100.0)
                });
            }
            assert!(
                live.mesh().vertices().all(|(_, p)| p.is_finite()),
                "{kind:?}"
            );
            let s = live
                .finish()
                .unwrap_or_else(|| panic!("{kind:?} moved nothing"));
            assert_eq!(&evaluated(&input, &s), live.mesh(), "{kind:?}");
        }
    }
}

#[test]
fn cancelling_a_grab_puts_the_lump_back_and_lets_go() {
    let input = Arc::new(grid(10));
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    live.dab(&dab(BrushKind::Grab, Vec3::ZERO, 3.0));
    live.dab(&dab(BrushKind::Grab, Vec3::new(0.0, 0.0, 3.0), 3.0));
    live.cancel();
    assert_eq!(live.mesh(), &*input);
    assert_eq!(live.held(), None);
    // A new stroke takes hold afresh.
    assert_eq!(
        live.dab(&dab(BrushKind::Grab, Vec3::new(1.0, 0.0, 0.0), 3.0)),
        0
    );
    assert_eq!(live.held(), Some(Vec3::new(1.0, 0.0, 0.0)));
}

#[test]
fn the_square_turns_with_the_stroke() {
    // A stroke along X lays a square whose edges run along X.
    let mut live = LiveSculpt::new(Arc::new(grid(20)), None);
    live.dab(&dab(BrushKind::ClayStrips, Vec3::new(-0.01, 0.0, 0.0), 4.0));
    live.cancel();
    live.dab(&dab(BrushKind::ClayStrips, Vec3::new(-1.0, 0.0, 0.0), 4.0));
    live.dab(&dab(BrushKind::ClayStrips, Vec3::ZERO, 4.0));
    let plane = forms::Plane {
        point: Vec3::ZERO,
        normal: Vec3::Z,
    };
    let along = forms::along_plane(&plane, Vec3::X);
    assert!((along - Vec3::X).length() < 1e-6);
    assert!(place(&live, at(3, 3)).z > 0.0);
}

/// The benchmark: ten dabs of each brush with a big brush (about 140 000
/// points in reach) on a shape of a million points. Run with
/// `cargo test --release -p sculpt brush_benchmark -- --ignored --nocapture`.
#[test]
#[ignore = "a benchmark"]
fn brush_benchmark() {
    use std::time::Instant;
    let input = Arc::new(mesh::samples::heavy_ball(1_000_000, 100.0));
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    let at = live
        .hit(Vec3::new(0.0, -500.0, 100.0), Vec3::Y)
        .unwrap()
        .point;
    for kind in BrushKind::ALL {
        if kind.grabs() {
            live.dab(&dab(kind, at, 90.0));
        }
        let start = Instant::now();
        let mut moved = 0;
        for i in 1..=10 {
            let way = if kind.grabs() { Vec3::NEG_Y } else { Vec3::X };
            moved = live.dab(&dab(kind, at + way * i as f32, 90.0));
        }
        println!(
            "{:<13} a dab ({moved} points): {:.2} ms",
            kind.name(),
            start.elapsed().as_secs_f64() * 1e2
        );
        live.finish();
    }
}
