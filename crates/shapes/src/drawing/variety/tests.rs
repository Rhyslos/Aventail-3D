//! Tests for varied copies, edges shared by linked copies, the mirror line
//! and veins.

use super::*;
use crate::basic::Basic;
use crate::drawing::{Mirror, PieceKind};
use crate::edge::EdgeKind;
use crate::path::Knot;
use crate::raster;

fn jagged() -> Edge {
    Edge {
        kind: EdgeKind::Serrated,
        amount: 0.8,
        size: 0.03,
        seed: 5,
    }
}

/// A leaf and two linked copies of it, moved right and down.
fn twig() -> (Drawing, Vec<PieceId>) {
    let d = Drawing::new()
        .with_piece(
            "Leaf",
            Basic::Circle.path(Vec2::new(0.2, 0.2), 0.12),
            PieceKind::Add,
        )
        .unwrap();
    let first = d.pieces()[0].id;
    let copies = d.copied(&[first]);
    let (d, a) = d
        .with_pasted(&copies, true, &Change::moved(Vec2::new(0.5, 0.0)))
        .unwrap();
    let (d, b) = d
        .with_pasted(&copies, true, &Change::moved(Vec2::new(0.0, 0.5)))
        .unwrap();
    (d, vec![first, a[0], b[0]])
}

fn drawn(d: &Drawing, id: PieceId) -> Path {
    d.drawn_paths(d.piece(id).unwrap())[0].clone()
}

/// `path` moved back by `by`, to compare shapes drawn in different places.
fn back(path: &Path, by: Vec2) -> Path {
    path.changed(&Change::moved(-by))
}

fn close(a: &Path, b: &Path) -> bool {
    a.knots().len() == b.knots().len()
        && a.knots()
            .iter()
            .zip(b.knots())
            .all(|(x, y)| x.at.distance(y.at) < 1e-4)
}

#[test]
fn an_edge_is_shared_by_linked_copies_and_they_match_until_varied() {
    let (d, ids) = twig();
    let d = d.with_edge(ids[1], jagged()).unwrap();
    for id in &ids {
        assert_eq!(d.piece(*id).unwrap().edge, jagged());
    }
    // Not varied: every copy has the same teeth.
    let by = Vec2::new(0.5, 0.0);
    assert!(close(&back(&drawn(&d, ids[1]), by), &drawn(&d, ids[0])));
    // Varied: each copy its own, though they still share one shape.
    let vary = Vary {
        on: true,
        ..Vary::OFF
    };
    let mut v = d.with_vary(ids[0], vary).unwrap();
    for id in &ids {
        v = v.with_colour(*id, [0.3, 0.5, 0.2]).unwrap();
    }
    assert!(!close(&back(&drawn(&v, ids[1]), by), &drawn(&v, ids[0])));
    assert_eq!(v.piece(ids[0]).unwrap().path, v.piece(ids[1]).unwrap().path);
    assert!(v.piece(ids[2]).unwrap().vary.on);
    // Colours shift too, each its own way.
    let colours: Vec<_> = ids
        .iter()
        .map(|id| v.drawn_colour(v.piece(*id).unwrap()))
        .collect();
    assert_ne!(colours[0], colours[1]);
    assert_ne!(colours[1], colours[2]);
    assert_eq!(d.drawn_colour(d.piece(ids[0]).unwrap()), [1.0; 3]);
    // The same drawing gives the same result: nothing is left to chance.
    assert_eq!(drawn(&v, ids[2]), drawn(&v.clone(), ids[2]));
    assert_ne!(d.key(), v.key());
}

#[test]
fn varied_copies_differ_in_size_and_turn_and_a_new_seed_changes_one() {
    let (d, ids) = twig();
    let d = d
        .with_vary(
            ids[0],
            Vary {
                on: true,
                size: 0.4,
                angle: 1.0,
                colour: 0.0,
            },
        )
        .unwrap();
    let areas: Vec<f32> = ids.iter().map(|id| drawn(&d, *id).area()).collect();
    assert!((areas[0] - areas[1]).abs() > 1e-4 || (areas[1] - areas[2]).abs() > 1e-4);
    let rolled = d.with_new_seeds(&[ids[1]]).unwrap();
    assert_ne!(drawn(&rolled, ids[1]), drawn(&d, ids[1]));
    assert_eq!(drawn(&rolled, ids[0]), drawn(&d, ids[0]));
    assert_ne!(rolled.key(), d.key());
    assert_eq!(d.with_new_seeds(&[PieceId(99)]), Err(Problem::Gone));
    // Pasted copies get seeds of their own.
    let seeds: Vec<u32> = ids.iter().map(|id| d.piece(*id).unwrap().seed).collect();
    assert!(seeds[0] != seeds[1] && seeds[1] != seeds[2]);
}

#[test]
fn a_mirror_image_mirrors_its_teeth_unless_copies_vary() {
    let d = Drawing::new()
        .with_piece(
            "Leaf",
            Basic::Circle.path(Vec2::new(0.25, 0.5), 0.1),
            PieceKind::Add,
        )
        .unwrap()
        .with_mirror(Mirror { on: true, x: 0.5 })
        .unwrap();
    let id = d.pieces()[0].id;
    let d = d.with_edge(id, jagged()).unwrap();
    let paths = d.drawn_paths(d.piece(id).unwrap());
    assert_eq!(paths.len(), 2);
    assert_eq!(paths[1], paths[0].mirrored(0.5));
    let v = d
        .with_vary(
            id,
            Vary {
                on: true,
                ..Vary::OFF
            },
        )
        .unwrap();
    let paths = v.drawn_paths(v.piece(id).unwrap());
    assert!(!close(&paths[1], &paths[0].mirrored(0.5)));
}

#[test]
fn a_varied_half_against_the_mirror_line_still_meets_its_other_half() {
    let half = Path::new(
        vec![
            Knot::corner(Vec2::new(0.5, 0.1)),
            Knot::smooth(Vec2::new(0.25, 0.5), Vec2::new(0.0, 0.2)),
            Knot::corner(Vec2::new(0.5, 0.9)),
        ],
        true,
    );
    let d = Drawing::new()
        .with_piece("Half", half, PieceKind::Add)
        .unwrap()
        .with_mirror(Mirror { on: true, x: 0.5 })
        .unwrap();
    let id = d.pieces()[0].id;
    let d = d
        .with_edge(id, jagged())
        .unwrap()
        .with_vary(
            id,
            Vary {
                on: true,
                size: 0.3,
                angle: 1.0,
                colour: 0.3,
            },
        )
        .unwrap();
    assert!(d.variation(d.piece(id).unwrap()).touches);
    let m = raster::mask(&d, 128);
    for y in 30..98 {
        assert!(m[y * 128 + 64] > 0.99 && m[y * 128 + 63] > 0.99, "row {y}");
    }
}

#[test]
fn a_group_varies_as_one_so_veins_stay_on_their_leaf() {
    let vein = Path::new(
        vec![
            Knot::corner(Vec2::new(0.2, 0.1)),
            Knot::corner(Vec2::new(0.2, 0.3)),
        ],
        false,
    );
    let d = Drawing::new()
        .with_piece(
            "Leaf",
            Basic::Circle.path(Vec2::new(0.2, 0.2), 0.12),
            PieceKind::Add,
        )
        .unwrap()
        .with_piece("Vein", vein, PieceKind::Line)
        .unwrap();
    let (leaf, line) = (d.pieces()[0].id, d.pieces()[1].id);
    let (d, _) = d.with_group(&[leaf, line], "Leaf").unwrap();
    let d = d
        .with_vary(
            leaf,
            Vary {
                on: true,
                size: 0.4,
                angle: 1.0,
                colour: 0.5,
            },
        )
        .unwrap();
    let a = d.variation(d.piece(leaf).unwrap());
    let b = d.variation(d.piece(line).unwrap());
    assert_eq!(a, b);
    assert!(a.on);
}

#[test]
fn bad_numbers_are_refused_and_out_of_range_ones_brought_in() {
    let (d, ids) = twig();
    let nan = Edge {
        size: f32::NAN,
        ..jagged()
    };
    assert_eq!(d.with_edge(ids[0], nan), Err(Problem::OutOfPlace));
    let wild = Vary {
        on: true,
        size: f32::INFINITY,
        ..Vary::OFF
    };
    assert_eq!(d.with_vary(ids[0], wild), Err(Problem::OutOfPlace));
    let big = d
        .with_edge(
            ids[0],
            Edge {
                amount: 9.0,
                size: 9.0,
                ..jagged()
            },
        )
        .unwrap();
    let e = big.piece(ids[0]).unwrap().edge;
    assert_eq!((e.amount, e.size), (1.0, 0.25));
    assert_eq!(d.with_edge(PieceId(77), jagged()), Err(Problem::Gone));
    // Linked pieces with different edges (a damaged file) are refused.
    let mut parts = big.to_parts();
    parts.pieces[1].edge = Edge::SMOOTH;
    assert_eq!(Drawing::from_parts(parts), Err(Problem::BadLink));
    let mut parts = big.to_parts();
    for p in &mut parts.pieces {
        p.vein.depth = f32::NAN;
    }
    assert_eq!(Drawing::from_parts(parts), Err(Problem::OutOfPlace));
    let mut parts = big.to_parts();
    for p in &mut parts.pieces {
        p.vary.angle = 40.0;
    }
    let fitted = Drawing::from_parts(parts).unwrap();
    assert_eq!(fitted.pieces()[0].vary.angle, *VARY_ANGLE.end());
}

#[test]
fn veins_sink_or_raise_the_height_and_shade_the_colour_along_lines() {
    let line = Path::new(
        vec![
            Knot::corner(Vec2::new(0.5, 0.2)),
            Knot::corner(Vec2::new(0.5, 0.8)),
        ],
        false,
    );
    let d = Drawing::new()
        .with_piece("Vein", line, PieceKind::Line)
        .unwrap();
    let id = d.pieces()[0].id;
    let v = raster::veins(&d, 256);
    assert!(!v.is_empty());
    let at = |x: usize, y: usize| 256 * y + x;
    assert!(v.raise[at(128, 128)] < -0.25 && v.shade[at(128, 128)] < -0.2);
    assert_eq!(v.raise[at(40, 128)], 0.0);
    assert_eq!(v.raise[at(128, 8)], 0.0);
    // A ridge, lighter.
    let ridge = d
        .with_vein(
            id,
            Vein {
                width: 0.05,
                depth: 0.6,
                shade: 0.4,
            },
        )
        .unwrap();
    let r = raster::veins(&ridge, 256);
    assert!(r.raise[at(128, 128)] > 0.5 && r.shade[at(128, 128)] > 0.3);
    // Wider: reaches further; rounded: highest in the middle.
    assert!(r.raise[at(133, 128)] > 0.0 && r.raise[at(133, 128)] < r.raise[at(128, 128)]);
    // No vein, or no lines: nothing.
    let none = d.with_vein(id, Vein::NONE).unwrap();
    assert!(raster::veins(&none, 64).is_empty());
    assert!(raster::veins(&Drawing::new(), 64).is_empty());
    assert!(raster::veins(&d, 0).is_empty());
}

#[test]
fn veins_wrap_round_the_edges() {
    let line = Path::new(
        vec![
            Knot::corner(Vec2::new(-0.2, 0.5)),
            Knot::corner(Vec2::new(0.3, 0.5)),
        ],
        false,
    );
    let d = Drawing::new()
        .with_piece("Vein", line, PieceKind::Line)
        .unwrap();
    let v = raster::veins(&d, 256);
    // In from the right edge as well as along the left.
    assert!(v.raise[256 * 128 + 240] < -0.2);
    assert!(v.raise[256 * 128 + 20] < -0.2);
    assert_eq!(v.raise[256 * 128 + 160], 0.0);
}

/// A fuzz run: random edges, varying, veins, seeds, copies and mirroring
/// always leave a drawing that rebuilds from its parts and fills at any
/// size without a panic, the same each time.
#[test]
fn random_variety_changes_always_leave_a_drawing_that_rebuilds_and_fills() {
    let mut state = 13u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (state >> 33) as u32
    };
    let (mut d, _) = twig();
    d = d
        .with_piece(
            "Vein",
            Path::new(
                vec![
                    Knot::corner(Vec2::splat(0.1)),
                    Knot::corner(Vec2::splat(0.9)),
                ],
                false,
            ),
            PieceKind::Line,
        )
        .unwrap();
    for round in 0..300 {
        let ids: Vec<PieceId> = d.pieces().iter().map(|p| p.id).collect();
        let pick = ids[next() as usize % ids.len()];
        let v = (next() % 3000) as f32 / 1000.0 - 1.0;
        let w = (next() % 3000) as f32 / 1000.0 - 1.0;
        let result = match next() % 7 {
            0 => d.with_edge(
                pick,
                Edge {
                    kind: EdgeKind::ALL[next() as usize % 5],
                    amount: v,
                    size: w * 0.1,
                    seed: next(),
                },
            ),
            1 => d.with_vary(
                pick,
                Vary {
                    on: next() % 2 == 0,
                    size: v,
                    angle: w * 4.0,
                    colour: v,
                },
            ),
            2 => d.with_vein(
                pick,
                Vein {
                    width: w * 0.2,
                    depth: v,
                    shade: w,
                },
            ),
            3 => d.with_new_seeds(&[pick]),
            4 => d.with_mirror(Mirror {
                on: next() % 2 == 0,
                x: v.abs(),
            }),
            5 if ids.len() < 12 => d
                .with_pasted(
                    &d.copied(&[pick]),
                    next() % 2 == 0,
                    &Change::moved(Vec2::splat(v)),
                )
                .map(|(d, _)| d),
            _ => d.with_change(&[pick], &Change::turned(Vec2::splat(0.5), v)),
        };
        if let Ok(changed) = result {
            d = changed;
        }
        assert_eq!(Drawing::from_parts(d.to_parts()).as_ref(), Ok(&d));
        if round % 10 == 0 {
            let size = [8, 33, 64][round % 3];
            let a = raster::layers(&d, size);
            assert_eq!(a, raster::layers(&d, size));
            assert!(a.mask.iter().all(|m| m.is_finite()));
            let veins = raster::veins(&d, size);
            assert!(veins.raise.iter().all(|r| r.abs() <= 1.0));
        }
    }
}
