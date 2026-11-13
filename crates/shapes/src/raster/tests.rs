//! Tests for turning drawings into pictures: sizes, sharp edges,
//! cut-outs, the mirror line, wrapping round, lines and odd paths.

use super::*;
use crate::drawing::{Mirror, PieceKind};
use crate::path::Knot;
use crate::{Basic, Drawing, Path};

fn square(at: Vec2, side: f32) -> Path {
    Path::polygon([
        at,
        at + Vec2::new(side, 0.0),
        at + Vec2::splat(side),
        at + Vec2::new(0.0, side),
    ])
}

fn drawn(pieces: &[(Path, PieceKind)]) -> Drawing {
    let mut d = Drawing::new();
    for (path, kind) in pieces {
        d = d.with_piece("P", path.clone(), *kind).unwrap();
    }
    d
}

fn covered(pixels: &[f32]) -> f32 {
    pixels.iter().sum::<f32>() / pixels.len() as f32
}

fn at(pixels: &[f32], size: u32, x: u32, y: u32) -> f32 {
    pixels[(y * size + x) as usize]
}

#[test]
fn a_square_fills_its_share_at_every_size() {
    let d = drawn(&[(square(Vec2::splat(0.25), 0.5), PieceKind::Add)]);
    for size in [1, 16, 32, 64, 256, 1024] {
        let m = mask(&d, size);
        assert_eq!(m.len(), (size * size) as usize);
        assert!((covered(&m) - 0.25).abs() < 0.01, "{size}: {}", covered(&m));
        assert!(m.iter().all(|v| (0.0..=1.0).contains(v)));
    }
    assert!(mask(&d, 0).is_empty());
    assert!(mask(&d, MAX_SIZE + 1).is_empty());
}

#[test]
fn edges_are_sharp_at_any_size_and_smooth_along_curves() {
    let d = drawn(&[(Basic::Circle.path(Vec2::splat(0.5), 0.3), PieceKind::Add)]);
    for size in [64, 512, 2048] {
        let m = mask(&d, size);
        // Only pixels the edge passes through are part covered: about
        // the circle's length in pixels, never a blurred band.
        let partial = m.iter().filter(|v| **v > 0.02 && **v < 0.98).count() as f32;
        let edge = std::f32::consts::TAU * 0.3 * size as f32;
        assert!(partial < edge * 1.6, "{size}: {partial} part covered");
        assert!(partial > edge * 0.3, "{size}: no smoothing ({partial})");
        let true_area = std::f32::consts::PI * 0.09;
        assert!((covered(&m) - true_area).abs() < 0.005, "{size}");
    }
}

#[test]
fn a_cut_out_takes_away_only_what_is_under_it() {
    let leaf = square(Vec2::splat(0.25), 0.5);
    let bite = square(Vec2::splat(0.5), 0.5);
    let cut = mask(
        &drawn(&[
            (leaf.clone(), PieceKind::Add),
            (bite.clone(), PieceKind::Cut),
        ]),
        64,
    );
    assert!((covered(&cut) - 0.1875).abs() < 0.01);
    assert_eq!(at(&cut, 64, 40, 40), 0.0);
    assert_eq!(at(&cut, 64, 20, 20), 1.0);
    // A cut under a piece doesn't take that piece away.
    let under = mask(
        &drawn(&[(bite.clone(), PieceKind::Cut), (leaf, PieceKind::Add)]),
        64,
    );
    assert!((covered(&under) - 0.25).abs() < 0.01);
    // A cut alone leaves nothing.
    assert_eq!(covered(&mask(&drawn(&[(bite, PieceKind::Cut)]), 32)), 0.0);
}

#[test]
fn the_mirror_line_fills_the_other_side_too() {
    let half = Path::polygon([
        Vec2::new(0.5, 0.2),
        Vec2::new(0.2, 0.5),
        Vec2::new(0.5, 0.8),
    ]);
    let d = drawn(&[(half, PieceKind::Add)]);
    let one = covered(&mask(&d, 64));
    let m = d.with_mirror(Mirror { on: true, x: 0.5 }).unwrap();
    let both = mask(&m, 64);
    assert!((covered(&both) - 2.0 * one).abs() < 0.01);
    // Symmetrical about the middle.
    for y in 0..64 {
        for x in 0..32 {
            assert!((at(&both, 64, x, y) - at(&both, 64, 63 - x, y)).abs() < 0.02);
        }
    }
}

#[test]
fn shapes_crossing_an_edge_come_back_on_the_other_side() {
    // A square over the right and bottom edges.
    let d = drawn(&[(square(Vec2::splat(0.875), 0.25), PieceKind::Add)]);
    let m = mask(&d, 64);
    assert!((covered(&m) - 0.0625).abs() < 0.005);
    for (x, y) in [(63, 63), (0, 0), (0, 63), (63, 0)] {
        assert_eq!(at(&m, 64, x, y), 1.0, "({x}, {y})");
    }
    // Seamless: the picture laid twice across matches where it joins.
    for y in 0..64 {
        assert_eq!(at(&m, 64, 63, y), at(&m, 64, 56, y));
        assert_eq!(at(&m, 64, 0, y), at(&m, 64, 7, y));
    }
    // Wholly outside the square: it is drawn where it wraps to.
    let far = drawn(&[(square(Vec2::new(2.25, -0.75), 0.5), PieceKind::Add)]);
    assert!((covered(&mask(&far, 32)) - 0.25).abs() < 0.01);
}

#[test]
fn lines_are_drawn_their_width_with_round_ends_and_wrap() {
    let vein = Path::new(
        vec![
            Knot::corner(Vec2::new(0.1, 0.5)),
            Knot::corner(Vec2::new(0.9, 0.5)),
        ],
        false,
    );
    let d = drawn(&[(vein, PieceKind::Line)]);
    assert_eq!(covered(&mask(&d, 64)), 0.0, "lines are never filled");
    let l = lines(&d, 100, 4.0);
    let expected = (80.0 * 4.0 + std::f32::consts::PI * 4.0) / 10_000.0;
    assert!((covered(&l) - expected).abs() < 0.004, "{}", covered(&l));
    assert!(lines(&d, 0, 4.0).is_empty());
    assert!(!lines(&d, 16, f32::NAN).is_empty());
    let over = Path::new(
        vec![
            Knot::corner(Vec2::new(0.9, 0.5)),
            Knot::corner(Vec2::new(1.1, 0.5)),
        ],
        false,
    );
    let w = lines(&drawn(&[(over, PieceKind::Line)]), 100, 2.0);
    assert!(at(&w, 100, 5, 50) > 0.5 && at(&w, 100, 95, 50) > 0.5);
}

#[test]
fn odd_paths_draw_without_harm() {
    // Two points on one spot (a side of no length), a figure of eight,
    // and a lens of two points.
    let doubled = Path::polygon([
        Vec2::splat(0.25),
        Vec2::splat(0.25),
        Vec2::new(0.75, 0.25),
        Vec2::splat(0.75),
        Vec2::new(0.25, 0.75),
    ]);
    assert!((covered(&mask(&drawn(&[(doubled, PieceKind::Add)]), 64)) - 0.25).abs() < 0.01);
    let eight = Path::polygon([
        Vec2::new(0.1, 0.1),
        Vec2::new(0.9, 0.9),
        Vec2::new(0.9, 0.1),
        Vec2::new(0.1, 0.9),
    ]);
    let e = mask(&drawn(&[(eight, PieceKind::Add)]), 64);
    assert_eq!(at(&e, 64, 51, 32), 1.0);
    assert_eq!(at(&e, 64, 12, 32), 1.0);
    assert_eq!(at(&e, 64, 32, 10), 0.0);
    // A path of one point draws nothing (never built), whatever kind.
    assert!(built(&Path::polygon([Vec2::splat(0.5)])).is_none());
    assert!(built(&Path::default()).is_none());
    assert!(wraps(&Path::default(), 0.0).is_empty());
}

/// Times the mask at 2048 pixels: run by hand with `--ignored`.
#[test]
#[ignore = "timing: run by hand"]
fn mask_speed() {
    let mut d = Drawing::new();
    for i in 0..40 {
        let middle = Vec2::new((i % 7) as f32 / 7.0, (i / 7) as f32 / 6.0);
        d = d
            .with_piece("P", Basic::Star(7).path(middle, 0.12), PieceKind::Add)
            .unwrap();
    }
    let start = std::time::Instant::now();
    let m = mask(&d, 2048);
    let took = start.elapsed();
    assert_eq!(m.len(), 2048 * 2048);
    eprintln!("40 stars at 2048: {took:?}");
}
