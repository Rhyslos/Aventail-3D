//! Tests for the edge randomiser: every kind closed, sane and the same for
//! one seed; amount 0 and Smooth leave the outline alone; the mirror line
//! fade.

use super::*;
use crate::basic::Basic;
use crate::drawing::{Drawing, Mirror, PieceKind};
use crate::raster;

fn edge(kind: EdgeKind, amount: f32) -> Edge {
    Edge {
        kind,
        amount,
        size: 0.03,
        seed: 7,
    }
}

/// A leaf: two curves meeting in points at the top and bottom.
fn leaf() -> Path {
    Path::new(
        vec![
            Knot::corner(Vec2::new(0.5, 0.1)),
            Knot::smooth(Vec2::new(0.25, 0.5), Vec2::new(0.0, 0.2)),
            Knot::corner(Vec2::new(0.5, 0.9)),
            Knot::smooth(Vec2::new(0.75, 0.5), Vec2::new(0.0, -0.2)),
        ],
        true,
    )
}

const ROUGH: [EdgeKind; 4] = [
    EdgeKind::Jagged,
    EdgeKind::Serrated,
    EdgeKind::Wavy,
    EdgeKind::Torn,
];

#[test]
fn every_kind_stays_closed_near_the_outline_and_the_same_for_a_seed() {
    for shape in [leaf(), Basic::Circle.path(Vec2::splat(0.5), 0.3)] {
        let (lo, hi) = shape.bounds().unwrap();
        for kind in ROUGH {
            let e = edge(kind, 1.0);
            let out = edged(&shape, &e, 3, None);
            assert!(out.is_closed(), "{kind:?}");
            assert!(out.knots().len() > shape.knots().len(), "{kind:?}");
            assert!(out.knots().len() <= MAX_POINTS);
            assert!(out.is_finite());
            // Within a tooth's depth of the outline: no wild points.
            let (a, b) = out.bounds().unwrap();
            assert!(a.cmpge(lo - Vec2::splat(0.03)).all(), "{kind:?}");
            assert!(b.cmple(hi + Vec2::splat(0.03)).all(), "{kind:?}");
            // No blow-up: about the same area.
            let ratio = out.area() / shape.area();
            assert!((0.8..1.2).contains(&ratio), "{kind:?} area ratio {ratio}");
            // The same seed gives the same teeth; another seed others.
            assert_eq!(out, edged(&shape, &e, 3, None));
            assert_ne!(out, edged(&shape, &e, 4, None), "{kind:?}");
        }
    }
}

#[test]
fn nothing_changes_with_no_amount_smooth_or_an_open_line() {
    let shape = leaf();
    for kind in EdgeKind::ALL {
        assert_eq!(edged(&shape, &edge(kind, 0.0), 1, None), shape);
    }
    assert_eq!(edged(&shape, &edge(EdgeKind::Smooth, 1.0), 1, None), shape);
    let line = Path::new(shape.knots().to_vec(), false);
    assert_eq!(edged(&line, &edge(EdgeKind::Jagged, 1.0), 1, None), line);
    // Odd paths come back as they are, never a panic.
    let dot = Path::polygon([Vec2::splat(0.5)]);
    assert_eq!(edged(&dot, &edge(EdgeKind::Torn, 1.0), 1, None), dot);
    let flat = Path::polygon([Vec2::ZERO, Vec2::splat(0.5), Vec2::ONE]);
    assert_eq!(edged(&flat, &edge(EdgeKind::Torn, 1.0), 1, None), flat);
    let wild = Edge {
        amount: f32::NAN,
        ..edge(EdgeKind::Jagged, 1.0)
    };
    assert_eq!(edged(&shape, &wild, 1, None), shape);
}

#[test]
fn smaller_teeth_make_more_of_them_and_the_count_is_capped() {
    let shape = leaf();
    let count = |size: f32| {
        let e = Edge {
            size,
            ..edge(EdgeKind::Jagged, 1.0)
        };
        edged(&shape, &e, 1, None).knots().len()
    };
    assert!(count(0.01) > count(0.05));
    assert!(count(0.0001) <= MAX_POINTS);
    // A huge outline still keeps within the cap.
    let big = Basic::Circle.path(Vec2::splat(0.5), 7.0);
    let e = Edge {
        size: *SIZE.start(),
        ..edge(EdgeKind::Torn, 1.0)
    };
    assert!(edged(&big, &e, 1, None).knots().len() <= MAX_POINTS);
}

#[test]
fn a_rough_edge_fills_sharp_and_differs_from_the_smooth_one() {
    let smooth = Drawing::new()
        .with_piece("Leaf", leaf(), PieceKind::Add)
        .unwrap();
    for kind in ROUGH {
        let rough = smooth
            .with_edge(smooth.pieces()[0].id, edge(kind, 1.0))
            .unwrap();
        let a = raster::mask(&smooth, 256);
        let b = raster::mask(&rough, 256);
        let changed = a
            .iter()
            .zip(&b)
            .filter(|(x, y)| (*x - *y).abs() > 0.5)
            .count();
        assert!(changed > 50, "{kind:?}: {changed}");
        // Sharp at a big size: only a thin band of pixels is part-covered.
        let big = raster::mask(&rough, 2048);
        let soft = big.iter().filter(|v| **v > 0.02 && **v < 0.98).count();
        let inside = big.iter().filter(|v| **v >= 0.98).count();
        assert!(soft * 20 < inside, "{kind:?}: {soft} soft of {inside}");
    }
}

#[test]
fn teeth_fade_out_at_the_mirror_line_so_halves_meet() {
    // Half a leaf against the line at 0.5, mirrored.
    let half = Path::new(
        vec![
            Knot::corner(Vec2::new(0.5, 0.1)),
            Knot::smooth(Vec2::new(0.25, 0.5), Vec2::new(0.0, 0.2)),
            Knot::corner(Vec2::new(0.5, 0.9)),
        ],
        true,
    );
    let out = edged(&half, &edge(EdgeKind::Jagged, 1.0), 2, Some(0.5));
    // Points on the line stay on it.
    for k in out.knots() {
        assert!(k.at.x <= 0.5 + 1e-5, "{:?}", k.at);
    }
    let d = Drawing::new()
        .with_piece("Half", half, PieceKind::Add)
        .unwrap()
        .with_mirror(Mirror { on: true, x: 0.5 })
        .unwrap();
    let d = d
        .with_edge(d.pieces()[0].id, edge(EdgeKind::Jagged, 1.0))
        .unwrap();
    let m = raster::mask(&d, 128);
    // Down the middle, between the tips, the leaf is whole: no gap.
    for y in 20..108 {
        assert!(m[y * 128 + 64] > 0.99 && m[y * 128 + 63] > 0.99, "row {y}");
    }
}
