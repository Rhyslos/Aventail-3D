//! Tests of dabs with an alpha (decision 0308): a stamp's result matches
//! its alpha, turned by its up, its angle and the stroke's way; black does
//! nothing; the grabbing brushes take no alpha; a stroke with an alpha
//! evaluates exactly to the live shape; and a benchmark.

use crate::alpha::{Alpha, DabAlpha, Made, made};
use crate::brush::DRAW_RATE;
use crate::tests::{evaluated, grid};
use crate::*;
use glam::Vec3;
use std::sync::Arc;

fn draw(centre: Vec3, radius: f32) -> Dab {
    Dab {
        kind: BrushKind::Draw,
        centre,
        radius,
        strength: 1.0,
        falloff: Falloff::Flat,
        invert: false,
    }
}

fn laid(alpha: &Alpha) -> DabAlpha<'_> {
    DabAlpha {
        alpha,
        up: Vec3::Y,
        angle: 0.0,
        follow: false,
    }
}

/// White on the left half, black on the right.
fn left_half() -> Alpha {
    let mut values = vec![0u8; 16 * 16];
    for y in 0..16 {
        for x in 0..8 {
            values[y * 16 + x] = 255;
        }
    }
    Alpha::new(16, 16, values, "left").unwrap()
}

#[test]
fn a_stamp_matches_its_alpha() {
    let input = Arc::new(grid(20));
    let rivets = made(Made::Rivets);
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    let r = 8.0;
    live.dab_with(&draw(Vec3::ZERO, r), Some(&laid(&rivets)));
    let mut raised = 0;
    for (v, p) in input.vertices() {
        let now = live.mesh().position(v).unwrap();
        let fall = Falloff::Flat.at(p.length() / r);
        let want = DRAW_RATE * r * fall * rivets.at(p.x / r, p.y / r);
        assert!(
            (now.z - want).abs() < 1e-5,
            "{p}: {} where the alpha gives {want}",
            now.z
        );
        // Only up, never sideways.
        assert!((now.truncate() - p.truncate()).length() < 1e-6);
        if want > 1e-3 {
            raised += 1;
        }
    }
    assert!(raised > 5, "the rivets raised {raised} points");
}

#[test]
fn black_parts_of_an_alpha_leave_the_surface_alone() {
    let left = left_half();
    let mut live = LiveSculpt::new(Arc::new(grid(20)), None);
    let moved = live.dab_with(&draw(Vec3::ZERO, 6.0), Some(&laid(&left)));
    assert!(moved > 0);
    for (_, p) in live.mesh().vertices() {
        if p.x > 0.5 {
            assert_eq!(p.z, 0.0, "the black half moved at {p}");
        }
    }
    let lifted = live
        .mesh()
        .vertices()
        .filter(|(_, p)| p.x < -0.5 && p.z > 0.0)
        .count();
    assert!(lifted > 10);
    // An alpha black wherever the brush reaches does nothing.
    let mut dark = vec![0u8; 16 * 16];
    dark[0] = 255;
    let corner = Alpha::new(16, 16, dark, "corner").unwrap();
    let mut live = LiveSculpt::new(Arc::new(grid(20)), None);
    let moved = live.dab_with(&draw(Vec3::ZERO, 3.0), Some(&laid(&corner)));
    assert_eq!(moved, 0);
    assert!(live.finish().is_none());
}

#[test]
fn an_alpha_turns_with_its_up_its_angle_and_the_stroke() {
    let left = left_half();
    let side = |live: &LiveSculpt| {
        // Which way the raised half lies.
        live.mesh()
            .vertices()
            .filter(|(_, p)| p.z > 0.0)
            .map(|(_, p)| p.truncate())
            .sum::<glam::Vec2>()
            .normalize()
    };
    let mut live = LiveSculpt::new(Arc::new(grid(20)), None);
    live.dab_with(&draw(Vec3::ZERO, 6.0), Some(&laid(&left)));
    assert!(side(&live).x < -0.9, "left: {}", side(&live));
    // Its top pointing right: the white half lies up.
    let mut live = LiveSculpt::new(Arc::new(grid(20)), None);
    let right = DabAlpha {
        up: Vec3::X,
        ..laid(&left)
    };
    live.dab_with(&draw(Vec3::ZERO, 6.0), Some(&right));
    assert!(side(&live).y > 0.9, "up: {}", side(&live));
    // Half a turn: the white half lies right.
    let mut live = LiveSculpt::new(Arc::new(grid(20)), None);
    let half = DabAlpha {
        angle: std::f32::consts::PI,
        ..laid(&left)
    };
    live.dab_with(&draw(Vec3::ZERO, 6.0), Some(&half));
    assert!(side(&live).x > 0.9, "right: {}", side(&live));
    // Following the stroke: a stroke's first dab has no way yet, so the
    // up is used.
    let follow = DabAlpha {
        follow: true,
        ..laid(&left)
    };
    let mut live = LiveSculpt::new(Arc::new(grid(20)), None);
    live.dab_with(&draw(Vec3::ZERO, 6.0), Some(&follow));
    assert!(side(&live).x < -0.9, "first dab: {}", side(&live));
    // A stroke going down (−y): the picture's top points down, so its
    // left (white) half lies at +x. The first dab, too small to reach a
    // point, only gives the way.
    let mut live = LiveSculpt::new(Arc::new(grid(20)), None);
    live.dab_with(&draw(Vec3::new(0.0, 0.5, 0.0), 0.01), Some(&follow));
    live.dab_with(&draw(Vec3::ZERO, 6.0), Some(&follow));
    assert!(side(&live).x > 0.9, "followed: {}", side(&live));
}

#[test]
fn grabbing_brushes_take_no_alpha() {
    let left = left_half();
    let shapes: Vec<_> = [None, Some(laid(&left))]
        .into_iter()
        .map(|alpha| {
            let mut live = LiveSculpt::new(Arc::new(grid(20)), None);
            let grab = Dab {
                kind: BrushKind::Grab,
                ..draw(Vec3::ZERO, 4.0)
            };
            live.dab_with(&grab, alpha.as_ref());
            live.dab_with(
                &Dab {
                    centre: Vec3::new(0.0, 0.0, 2.0),
                    ..grab
                },
                alpha.as_ref(),
            );
            live.mesh().clone()
        })
        .collect();
    assert_eq!(shapes[0], shapes[1]);
}

#[test]
fn a_stroke_with_an_alpha_evaluates_exactly_to_the_live_shape() {
    let input = Arc::new(grid(20));
    let stones = made(Made::Stones);
    for kind in [BrushKind::Draw, BrushKind::Clay, BrushKind::Smooth, BrushKind::Inflate] {
        let mut live = LiveSculpt::new(Arc::clone(&input), None);
        for i in 0..5 {
            let d = Dab {
                kind,
                ..draw(Vec3::new(i as f32 - 2.0, 0.0, 0.0), 5.0)
            };
            live.dab_with(
                &d,
                Some(&DabAlpha {
                    follow: true,
                    ..laid(&stones)
                }),
            );
        }
        if kind == BrushKind::Smooth {
            // Smoothing a flat grid moves nothing.
            assert!(live.finish().is_none());
            continue;
        }
        let sculpt = live.finish().unwrap();
        assert_eq!(&evaluated(&input, &sculpt), live.mesh(), "{kind:?}");
    }
}

/// Run with `cargo test -p sculpt --release -- --ignored --nocapture
/// alpha_benchmark`: a dab of Draw with a 1024-pixel alpha and about
/// 140 000 points in reach on a million points, beside the same dab with
/// no alpha.
#[test]
#[ignore = "a benchmark"]
fn alpha_benchmark() {
    use std::time::Instant;
    let input = Arc::new(mesh::samples::heavy_ball(1_000_000, 100.0));
    let side = alpha::MAX_ALPHA_SIDE;
    let values: Vec<u8> = (0..side * side)
        .map(|i| ((i % side) ^ (i / side)) as u8)
        .collect();
    let big = Alpha::new(side, side, values, "big").unwrap();
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    let at = live
        .hit(Vec3::new(0.0, -500.0, 100.0), Vec3::Y)
        .unwrap()
        .point;
    for alpha in [None, Some(laid(&big))] {
        let start = Instant::now();
        let mut moved = 0;
        for i in 1..=10 {
            moved = live.dab_with(
                &Dab {
                    falloff: Falloff::Smooth,
                    ..draw(at + Vec3::X * i as f32, 90.0)
                },
                alpha.as_ref(),
            );
        }
        println!(
            "Draw {} an alpha, a dab ({moved} points): {:.2} ms",
            if alpha.is_some() { "with" } else { "without" },
            start.elapsed().as_secs_f64() * 1e2
        );
        live.finish();
    }
}

