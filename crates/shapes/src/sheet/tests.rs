//! Tests for leaf sheets: which pieces make a card, cards laid out apart
//! and inside the sheet at matched sizes, pictures on the sheet, and a
//! fuzz run of random drawings.

use super::*;
use crate::basic::Basic;
use crate::drawing::{Mirror, Vary, Vein};
use crate::edge::{Edge, EdgeKind};
use crate::path::Knot;
use crate::raster;

fn line(a: Vec2, b: Vec2) -> Path {
    Path::new(vec![Knot::corner(a), Knot::corner(b)], false)
}

/// Two leaves apart: a big one with a vein and a bite, a small one.
fn two_leaves() -> Drawing {
    Drawing::new()
        .with_piece(
            "Big",
            Basic::Circle.path(Vec2::new(0.3, 0.3), 0.2),
            PieceKind::Add,
        )
        .unwrap()
        .with_piece(
            "Vein",
            line(Vec2::new(0.3, 0.15), Vec2::new(0.3, 0.45)),
            PieceKind::Line,
        )
        .unwrap()
        .with_piece(
            "Bite",
            Basic::Circle.path(Vec2::new(0.48, 0.3), 0.05),
            PieceKind::Cut,
        )
        .unwrap()
        .with_piece(
            "Small",
            Basic::Circle.path(Vec2::new(0.75, 0.75), 0.1),
            PieceKind::Add,
        )
        .unwrap()
}

fn ids(d: &Drawing) -> Vec<PieceId> {
    d.pieces().iter().map(|p| p.id).collect()
}

#[test]
fn touching_pieces_and_groups_make_one_card_and_loose_lines_and_cuts_none() {
    let d = two_leaves();
    let id = ids(&d);
    assert_eq!(cards(&d), vec![vec![id[0], id[1], id[2]], vec![id[3]]]);
    // A line and a cut touching nothing are left off.
    let d = d
        .with_piece(
            "Stray line",
            line(Vec2::new(0.05, 0.9), Vec2::new(0.2, 0.95)),
            PieceKind::Line,
        )
        .unwrap()
        .with_piece(
            "Stray cut",
            Basic::Circle.path(Vec2::new(0.9, 0.1), 0.04),
            PieceKind::Cut,
        )
        .unwrap();
    assert_eq!(cards(&d).len(), 2);
    let sheet = Sheet::of(&d, SheetOptions::default());
    assert!(sheet.change(ids(&d)[4]).is_none() && sheet.change(ids(&d)[5]).is_none());
    // Grouped, the two leaves are one card although they don't touch.
    let (grouped, _) = d.with_group(&[id[0], id[3]], "Twig").unwrap();
    assert_eq!(cards(&grouped).len(), 1);
    // Two crossing thin bars touch though neither's corners are inside the
    // other.
    let bar = |a: Vec2, b: Vec2| Path::polygon([a, Vec2::new(b.x, a.y), b, Vec2::new(a.x, b.y)]);
    let plus = Drawing::new()
        .with_piece(
            "Across",
            bar(Vec2::new(0.1, 0.48), Vec2::new(0.9, 0.52)),
            PieceKind::Add,
        )
        .unwrap()
        .with_piece(
            "Down",
            bar(Vec2::new(0.48, 0.1), Vec2::new(0.52, 0.9)),
            PieceKind::Add,
        )
        .unwrap();
    assert_eq!(cards(&plus).len(), 1);
}

/// Every point of every piece's outline on the sheet.
fn sheet_points(d: &Drawing, sheet: &Sheet, id: PieceId) -> Vec<Vec2> {
    sheet
        .drawn_paths(d, d.piece(id).unwrap())
        .iter()
        .flat_map(|p| p.flattened(0.001))
        .collect()
}

#[test]
fn cards_lie_inside_the_sheet_apart_and_at_the_same_scale() {
    let d = two_leaves();
    let id = ids(&d);
    let gap = 0.04;
    let sheet = Sheet::of(&d, SheetOptions { gap, turn: true });
    assert_eq!(sheet.cards().len(), 2);
    for i in &id {
        for p in sheet_points(&d, &sheet, *i) {
            assert!(
                p.cmpge(Vec2::splat(-1e-3)).all() && p.cmple(Vec2::splat(1.0 + 1e-3)).all(),
                "{p}"
            );
        }
    }
    // Pieces on a card move together; the cards are sized alike.
    assert_eq!(sheet.change(id[0]), sheet.change(id[1]));
    assert_eq!(sheet.change(id[0]), sheet.change(id[2]));
    assert!((sheet.scale_of(id[0]) - sheet.scale_of(id[3])).abs() < 1e-4);
    // The sheet fills more of the texture than the drawing did.
    assert!(sheet.scale_of(id[0]) > 1.2);
    // No pixel is covered by both cards, and they keep about a gap apart.
    let n = 256;
    let only = |keep: PieceId| {
        let mut one = d.clone();
        for other in ids(&d) {
            if !sheet.cards().iter().any(|c| c.contains(&keep) && c.contains(&other)) {
                one = one.without(&[other]).unwrap();
            }
        }
        let laid = Sheet {
            moves: sheet.moves.clone(),
            cards: sheet.cards.clone(),
        };
        raster::mask_on(&one, Some(&laid), n)
    };
    let (a, b) = (only(id[0]), only(id[3]));
    let gap_px = (gap * n as f32) as i64 - 2;
    let near = |m: &[f32], x: i64, y: i64| {
        (-gap_px..=gap_px).any(|dy| {
            (-gap_px..=gap_px).any(|dx| {
                let (u, v) = (x + dx, y + dy);
                (0..n as i64).contains(&u) && (0..n as i64).contains(&v) && m[(v * n as i64 + u) as usize] > 0.0
            })
        })
    };
    for y in 0..n as i64 {
        for x in 0..n as i64 {
            if a[(y * n as i64 + x) as usize] > 0.0 {
                assert!(!near(&b, x, y), "too close at {x}, {y}");
            }
        }
    }
    // Sizes against each other are kept: the big leaf about four times the
    // small one's area.
    let area = |m: &[f32]| m.iter().sum::<f32>();
    let ratio = area(&a) / area(&b);
    assert!((3.0..4.6).contains(&ratio), "{ratio}");
}

#[test]
fn the_sheet_is_the_same_every_time_and_at_every_size() {
    let d = two_leaves();
    let a = Sheet::of(&d, SheetOptions::default());
    assert_eq!(a, Sheet::of(&d, SheetOptions::default()));
    let small = raster::mask_on(&d, Some(&a), 64);
    let big = raster::mask_on(&d, Some(&a), 128);
    // The big picture averaged down matches the small one.
    let mut off = 0.0f32;
    for y in 0..64 {
        for x in 0..64 {
            let k = |dx: usize, dy: usize| big[(2 * y + dy) * 128 + 2 * x + dx];
            let mean = (k(0, 0) + k(1, 0) + k(0, 1) + k(1, 1)) / 4.0;
            off = off.max((mean - small[y * 64 + x]).abs());
        }
    }
    assert!(off < 0.35, "{off}");
}

#[test]
fn a_card_kept_upright_is_only_moved_and_sized() {
    let d = two_leaves();
    let sheet = Sheet::of(&d, SheetOptions { gap: 0.02, turn: false });
    for id in ids(&d).into_iter().take(1) {
        let m = sheet.change(id).unwrap().matrix;
        assert!(m.x_axis.y.abs() < 1e-5 && m.y_axis.x.abs() < 1e-5);
        assert!((m.x_axis.x - m.y_axis.y).abs() < 1e-5 && m.x_axis.x > 0.0);
    }
}

#[test]
fn edges_varied_copies_mirror_images_and_veins_come_along_onto_the_sheet() {
    let half = Path::polygon([
        Vec2::new(0.5, 0.1),
        Vec2::new(0.7, 0.3),
        Vec2::new(0.5, 0.5),
    ]);
    let d = Drawing::new()
        .with_mirror(Mirror { on: true, x: 0.5 })
        .unwrap()
        .with_piece("Half leaf", half, PieceKind::Add)
        .unwrap()
        .with_piece(
            "Vein",
            line(Vec2::new(0.5, 0.12), Vec2::new(0.5, 0.48)),
            PieceKind::Line,
        )
        .unwrap();
    let (leaf, vein) = (d.pieces()[0].id, d.pieces()[1].id);
    let d = d
        .with_mirrored(leaf, true)
        .unwrap()
        .with_edge(
            leaf,
            Edge {
                kind: EdgeKind::Serrated,
                amount: 0.7,
                size: 0.02,
                seed: 3,
            },
        )
        .unwrap()
        .with_vein(
            vein,
            Vein {
                width: 0.01,
                depth: -0.5,
                shade: -0.4,
            },
        )
        .unwrap()
        .with_vary(
            leaf,
            Vary {
                on: true,
                size: 0.2,
                angle: 0.3,
                colour: 0.2,
            },
        )
        .unwrap();
    // The half and its mirror image (touching the line) and the vein: one
    // card, with every toothed point of the drawn outline moved along.
    assert_eq!(cards(&d), vec![vec![leaf, vein]]);
    let sheet = Sheet::of(&d, SheetOptions::default());
    let drawn = d.drawn_paths(d.piece(leaf).unwrap());
    let laid = sheet.drawn_paths(&d, d.piece(leaf).unwrap());
    assert_eq!(drawn.len(), 2);
    assert_eq!(laid.len(), 2);
    assert_eq!(drawn[0].knots().len(), laid[0].knots().len());
    // The veins follow the card, only on the leaf, as wide as it's sized.
    let v = raster::veins_on(&d, Some(&sheet), 128);
    let m = raster::mask_on(&d, Some(&sheet), 128);
    assert!(!v.is_empty());
    let deepest = v
        .raise
        .iter()
        .enumerate()
        .min_by(|a, b| a.1.total_cmp(b.1))
        .unwrap()
        .0;
    assert!(m[deepest] > 0.5);
}

#[test]
fn nothing_filled_gives_an_empty_sheet_and_empty_pictures() {
    let empty = Drawing::new();
    let sheet = Sheet::of(&empty, SheetOptions::default());
    assert!(sheet.is_empty() && sheet.cards().is_empty());
    let lone = Drawing::new()
        .with_piece(
            "Line",
            line(Vec2::splat(0.2), Vec2::splat(0.8)),
            PieceKind::Line,
        )
        .unwrap();
    let sheet = Sheet::of(&lone, SheetOptions::default());
    assert!(sheet.is_empty());
    assert!(raster::mask_on(&lone, Some(&sheet), 32).iter().all(|v| *v == 0.0));
    assert!(raster::veins_on(&lone, Some(&sheet), 32).is_empty());
    // Odd gaps are brought into range.
    let d = two_leaves();
    for gap in [f32::NAN, -1.0, 5.0, f32::INFINITY] {
        let s = Sheet::of(&d, SheetOptions { gap, turn: true });
        assert_eq!(s.cards().len(), 2);
    }
}

#[test]
fn random_drawings_always_lay_out_inside_the_sheet() {
    let mut state = 29u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (state >> 33) as u32
    };
    let mut unit = move || (next() % 10_000) as f32 / 10_000.0;
    for round in 0..24 {
        let mut d = Drawing::new();
        let count = 1 + (round % 7);
        for k in 0..count {
            let middle = Vec2::new(unit() * 1.4 - 0.2, unit() * 1.4 - 0.2);
            let path = match k % 3 {
                0 => Basic::Circle.path(middle, 0.02 + unit() * 0.3),
                1 => Basic::Star(5).path(middle, 0.02 + unit() * 0.3),
                _ => line(middle, middle + Vec2::new(unit() - 0.5, unit() - 0.5)),
            };
            let kind = match k % 4 {
                1 => PieceKind::Cut,
                _ if k % 3 == 2 => PieceKind::Line,
                _ => PieceKind::Add,
            };
            if let Ok(next) = d.with_piece("Piece", path, kind) {
                d = next;
            }
        }
        let gap = unit() * 0.08;
        let sheet = Sheet::of(&d, SheetOptions { gap, turn: round % 2 == 0 });
        for p in d.pieces() {
            for point in sheet
                .drawn_paths(&d, p)
                .iter()
                .flat_map(|path| path.flattened(0.002))
            {
                assert!(
                    point.cmpge(Vec2::splat(-2e-3)).all() && point.cmple(Vec2::splat(1.002)).all(),
                    "round {round}: {point}"
                );
            }
        }
        let mask = raster::mask_on(&d, Some(&sheet), 32);
        assert!(mask.iter().all(|v| (0.0..=1.0).contains(v)));
    }
}
