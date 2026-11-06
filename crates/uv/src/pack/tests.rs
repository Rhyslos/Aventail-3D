//! Tests for packing (decision 0101).

use super::*;

fn square(at: Vec2, size: f32) -> Piece {
    Piece {
        polygons: vec![vec![
            at,
            at + Vec2::new(size, 0.0),
            at + Vec2::new(size, size),
            at + Vec2::new(0.0, size),
        ]],
    }
}

/// A square frame with a hole in the middle: four strips.
fn frame(size: f32, side: f32) -> Piece {
    let q = |a: Vec2, b: Vec2| vec![a, Vec2::new(b.x, a.y), b, Vec2::new(a.x, b.y)];
    Piece {
        polygons: vec![
            q(Vec2::ZERO, Vec2::new(size, side)),
            q(Vec2::new(0.0, size - side), Vec2::new(size, size)),
            q(Vec2::new(0.0, side), Vec2::new(side, size - side)),
            q(Vec2::new(size - side, side), Vec2::new(size, size - side)),
        ],
    }
}

fn boxes(pieces: &[Piece], moves: &[Affine2]) -> Vec<(Vec2, Vec2)> {
    pieces
        .iter()
        .zip(moves)
        .map(|(p, t)| bounds(&moved(p, *t)).unwrap())
        .collect()
}

fn gap(a: (Vec2, Vec2), b: (Vec2, Vec2)) -> f32 {
    let dx = (b.0.x - a.1.x).max(a.0.x - b.1.x);
    let dy = (b.0.y - a.1.y).max(a.0.y - b.1.y);
    dx.max(dy)
}

fn inside_square(b: (Vec2, Vec2)) -> bool {
    b.0.x >= -1e-4 && b.0.y >= -1e-4 && b.1.x <= 1.0 + 1e-4 && b.1.y <= 1.0 + 1e-4
}

#[test]
fn pieces_fill_the_square_apart_by_the_padding() {
    let pieces: Vec<Piece> = (0..6)
        .map(|i| square(Vec2::new(i as f32 * 3.0, 0.0), 1.0))
        .collect();
    let options = PackOptions::default();
    let moves = pack(&pieces, &options).unwrap();
    let b = boxes(&pieces, &moves);
    for (i, x) in b.iter().enumerate() {
        assert!(inside_square(*x), "{x:?}");
        for y in &b[i + 1..] {
            assert!(gap(*x, *y) >= options.padding * 0.99, "{x:?} {y:?}");
        }
    }
    // Same scale for all, and most of the square used.
    let sizes: Vec<f32> = b.iter().map(|(lo, hi)| (*hi - *lo).x).collect();
    for s in &sizes {
        assert!((s - sizes[0]).abs() < 1e-4);
    }
    let used: f32 = sizes.iter().map(|s| s * s).sum();
    assert!(used > 0.5, "{used}");
}

#[test]
fn a_small_piece_nests_in_a_big_ones_hole() {
    let pieces = vec![frame(10.0, 2.0), square(Vec2::new(20.0, 20.0), 3.0)];
    let options = PackOptions {
        padding: 0.01,
        cells: 256,
        ..PackOptions::default()
    };
    let moves = pack(&pieces, &options).unwrap();
    let b = boxes(&pieces, &moves);
    // The small square sits inside the frame's box.
    let (frame_box, small) = (b[0], b[1]);
    assert!(
        small.0.x > frame_box.0.x && small.1.x < frame_box.1.x,
        "{b:?}"
    );
    assert!(
        small.0.y > frame_box.0.y && small.1.y < frame_box.1.y,
        "{b:?}"
    );
}

#[test]
fn identical_pieces_stack_when_asked() {
    let pieces = vec![
        square(Vec2::ZERO, 1.0),
        square(Vec2::new(5.0, 5.0), 1.0),
        square(Vec2::new(9.0, 0.0), 2.0),
    ];
    let stacked = pack(
        &pieces,
        &PackOptions {
            stack: true,
            ..PackOptions::default()
        },
    )
    .unwrap();
    let b = boxes(&pieces, &stacked);
    assert!((b[0].0 - b[1].0).length() < 1e-4 && (b[0].1 - b[1].1).length() < 1e-4);
    let apart = pack(&pieces, &PackOptions::default()).unwrap();
    let b = boxes(&pieces, &apart);
    assert!(gap(b[0], b[1]) > 0.0);
}

#[test]
fn a_slanted_piece_is_turned_square_unless_turning_is_off() {
    let slanted = Piece {
        polygons: vec![vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(4.0, 4.0),
            Vec2::new(3.5, 4.5),
            Vec2::new(-0.5, 0.5),
        ]],
    };
    let pieces = vec![slanted];
    let moves = pack(&pieces, &PackOptions::default()).unwrap();
    let corners = moved(&pieces[0], moves[0]);
    let side = corners[0][1] - corners[0][0];
    assert!(side.x.abs() < 1e-3 || side.y.abs() < 1e-3, "{side}");
    let kept = pack(
        &pieces,
        &PackOptions {
            rotate: false,
            ..PackOptions::default()
        },
    )
    .unwrap();
    let corners = moved(&pieces[0], kept[0]);
    let side = corners[0][1] - corners[0][0];
    assert!((side.x - side.y).abs() < 1e-3, "kept at 45°: {side}");
}

#[test]
fn nothing_to_pack_gives_nothing() {
    assert!(pack(&[], &PackOptions::default()).is_none());
    let flat = Piece {
        polygons: vec![vec![Vec2::ZERO, Vec2::X, Vec2::new(2.0, 0.0)]],
    };
    assert!(pack(&[flat], &PackOptions::default()).is_none());
}

#[test]
fn placing_moves_exactly_the_islands_corners() {
    let mut mesh = mesh::Primitive::Cube(mesh::CubeParams::default()).mesh();
    let islands = islands_of(&mesh, &[]);
    assert_eq!(islands.len(), 6);
    let before = Uvs::of(&mesh).unwrap().into_values(0);
    let shift = Affine2::from_translation(Vec2::new(0.25, 0.0));
    let done = place(&mut mesh, &[(islands[0].clone(), shift)]);
    assert_eq!(done.matched, islands[0].len());
    let after = Uvs::of(&mesh).unwrap().into_values(0);
    let (starts, _) = mesh.face_layout();
    let moved_corners: BTreeSet<usize> = islands[0]
        .iter()
        .flat_map(|f| {
            let s = starts[f.get() as usize] as usize;
            s..s + 4
        })
        .collect();
    for (c, (a, b)) in before.iter().zip(&after).enumerate() {
        if moved_corners.contains(&c) {
            assert!((*b - *a - Vec2::new(0.25, 0.0)).length() < 1e-6);
        } else {
            assert_eq!(a, b);
        }
    }
    // A whole cube packs into the square.
    let pieces: Vec<Piece> = islands.iter().map(|i| piece_of(&mesh, i)).collect();
    let moves = pack(&pieces, &PackOptions::default()).unwrap();
    let list: Vec<(Vec<FaceId>, Affine2)> = islands.into_iter().zip(moves).collect();
    place(&mut mesh, &list);
    for uv in Uvs::of(&mesh).unwrap().into_values(0) {
        assert!(
            (0.0..=1.0).contains(&uv.x) && (0.0..=1.0).contains(&uv.y),
            "{uv}"
        );
    }
}

/// How long packing takes for many pieces: run with
/// `cargo test --release -p uv pack_benchmark -- --ignored --nocapture`.
#[test]
#[ignore = "benchmark"]
fn pack_benchmark() {
    for count in [20usize, 100, 300] {
        let pieces: Vec<Piece> = (0..count)
            .map(|i| {
                let w = 1.0 + (i * 7 % 5) as f32;
                let h = 1.0 + (i * 3 % 4) as f32;
                Piece {
                    polygons: vec![vec![
                        Vec2::ZERO,
                        Vec2::new(w, 0.0),
                        Vec2::new(w, h),
                        Vec2::new(0.0, h),
                    ]],
                }
            })
            .collect();
        let t = std::time::Instant::now();
        let moves = pack(&pieces, &PackOptions::default()).unwrap();
        let b = boxes(&pieces, &moves);
        let used: f32 = b.iter().map(|(lo, hi)| (hi.x - lo.x) * (hi.y - lo.y)).sum();
        eprintln!(
            "{count} pieces: {:?}, {:.0}% used",
            t.elapsed(),
            used * 100.0
        );
    }
}

fn rect(at: Vec2, size: Vec2) -> Piece {
    Piece {
        polygons: vec![vec![
            at,
            at + Vec2::new(size.x, 0.0),
            at + size,
            at + Vec2::new(0.0, size.y),
        ]],
    }
}

#[test]
fn pieces_pack_into_a_region_only() {
    let pieces: Vec<Piece> = (0..4).map(|i| square(Vec2::splat(i as f32), 0.3)).collect();
    let options = PackOptions {
        region: Some((Vec2::new(0.5, 0.5), Vec2::ONE)),
        ..PackOptions::default()
    };
    let moves = pack(&pieces, &options).unwrap();
    for (lo, hi) in boxes(&pieces, &moves) {
        assert!(lo.x >= 0.5 - 1e-4 && lo.y >= 0.5 - 1e-4, "{lo}");
        assert!(hi.x <= 1.0 + 1e-4 && hi.y <= 1.0 + 1e-4, "{hi}");
    }
}

#[test]
fn locked_pieces_keep_their_space() {
    // A locked piece filling the left half: the rest go right.
    let filled = vec![rect(Vec2::ZERO, Vec2::new(0.5, 1.0))];
    let pieces: Vec<Piece> = (0..3).map(|i| square(Vec2::splat(i as f32), 0.2)).collect();
    let moves = pack_with(&pieces, &[], &filled, &PackOptions::default()).unwrap();
    for (lo, _) in boxes(&pieces, &moves) {
        assert!(lo.x >= 0.5 - 0.01, "{lo}");
    }
}

#[test]
fn a_piece_keeping_its_grain_is_never_turned() {
    // A tall thin piece would be turned to lie flat; with its grain kept
    // it stays upright.
    let tall = rect(Vec2::ZERO, Vec2::new(0.1, 0.8));
    let turned = |asks: &[PieceAsks]| {
        let moves = pack_with(
            std::slice::from_ref(&tall),
            asks,
            &[],
            &PackOptions::default(),
        )
        .unwrap();
        let x = moves[0].transform_vector2(Vec2::X);
        x.y.abs() > 1e-4 || x.x < 0.0
    };
    let kept = PieceAsks {
        grain: true,
        scale: 1.0,
    };
    assert!(!turned(&[kept]));
}

#[test]
fn a_bigger_scale_gives_a_piece_more_texture() {
    let pieces = vec![square(Vec2::ZERO, 0.2), square(Vec2::ONE, 0.2)];
    let asks = [
        PieceAsks {
            grain: false,
            scale: 2.0,
        },
        PieceAsks::default(),
    ];
    let moves = pack_with(&pieces, &asks, &[], &PackOptions::default()).unwrap();
    let b = boxes(&pieces, &moves);
    let size = |(lo, hi): (Vec2, Vec2)| (hi - lo).x;
    assert!((size(b[0]) / size(b[1]) - 2.0).abs() < 0.05, "{b:?}");
}

#[test]
fn udim_tiles_hold_pieces_without_crossing_a_tiles_side() {
    let pieces: Vec<Piece> = (0..6).map(|i| square(Vec2::splat(i as f32), 0.3)).collect();
    let options = PackOptions {
        tiles: 2,
        ..PackOptions::default()
    };
    let moves = pack(&pieces, &options).unwrap();
    let b = boxes(&pieces, &moves);
    assert!(b.iter().any(|(lo, _)| lo.x >= 1.0), "some in tile 1002");
    for (lo, hi) in &b {
        assert!(hi.x <= 2.0 + 1e-4 && hi.y <= 1.0 + 1e-4);
        assert!(!(lo.x < 1.0 && hi.x > 1.0), "across the side: {lo} {hi}");
    }
}

// ------------------------------------------------ found by the test pass

/// `n` unit squares far apart.
fn squares(n: usize) -> Vec<Piece> {
    (0..n)
        .map(|i| square(Vec2::new(i as f32 * 3.0, 0.0), 1.0))
        .collect()
}

/// The smallest gap between any two placed pieces' boxes.
fn least_gap(b: &[(Vec2, Vec2)]) -> f32 {
    let mut least = f32::MAX;
    for i in 0..b.len() {
        for j in i + 1..b.len() {
            least = least.min(gap(b[i], b[j]));
        }
    }
    least
}

#[test]
fn packing_into_a_region_keeps_the_padding_in_texture_pixels() {
    let pieces = squares(4);
    let padding = 32.0 / 1024.0;
    for region in [
        (Vec2::ZERO, Vec2::splat(0.25)),
        (Vec2::new(0.5, 0.0), Vec2::new(1.0, 0.5)),
    ] {
        let options = PackOptions {
            padding,
            region: Some(region),
            ..PackOptions::default()
        };
        let moves = pack(&pieces, &options).unwrap();
        let b = boxes(&pieces, &moves);
        let least = least_gap(&b);
        assert!(
            least >= padding * 0.99,
            "{region:?}: pieces {} px apart in a 1024 texture, {} px asked",
            least * 1024.0,
            padding * 1024.0
        );
        // Half the padding to the region's edges.
        for (lo, hi) in b {
            assert!(
                (lo - region.0).min_element() >= padding / 2.0 * 0.99,
                "{lo}"
            );
            assert!(
                (region.1 - hi).min_element() >= padding / 2.0 * 0.99,
                "{hi}"
            );
        }
    }
}

#[test]
fn packing_into_a_wide_region_uses_its_width() {
    let pieces = squares(4);
    for (region, across) in [
        ((Vec2::ZERO, Vec2::new(1.0, 0.25)), true),
        ((Vec2::ZERO, Vec2::new(0.25, 1.0)), false),
    ] {
        let options = PackOptions {
            padding: 0.0,
            region: Some(region),
            ..PackOptions::default()
        };
        let moves = pack(&pieces, &options).unwrap();
        let b = boxes(&pieces, &moves);
        let reach = b
            .iter()
            .map(|(_, hi)| if across { hi.x } else { hi.y })
            .fold(0.0f32, f32::max);
        assert!(reach > 0.9, "{region:?}: the pieces reach only {reach}");
        for (lo, hi) in b {
            assert!(lo.x >= -1e-4 && lo.y >= -1e-4, "{lo}");
            assert!(
                hi.x <= region.1.x + 1e-4 && hi.y <= region.1.y + 1e-4,
                "{hi}"
            );
        }
    }
}

#[test]
fn a_backwards_region_is_the_same_as_the_right_way_round() {
    let pieces = squares(3);
    let at = |region| {
        let options = PackOptions {
            region: Some(region),
            ..PackOptions::default()
        };
        pack(&pieces, &options).unwrap()
    };
    let right = at((Vec2::splat(0.2), Vec2::splat(0.8)));
    let backwards = at((Vec2::splat(0.8), Vec2::splat(0.2)));
    let mixed = at((Vec2::new(0.8, 0.2), Vec2::new(0.2, 0.8)));
    for (a, b) in right.iter().zip(&backwards).chain(right.iter().zip(&mixed)) {
        assert!(a.abs_diff_eq(*b, 1e-5));
    }
    for (lo, hi) in boxes(&pieces, &backwards) {
        assert!(lo.min_element() >= 0.2 - 1e-4 && hi.max_element() <= 0.8 + 1e-4);
    }
    // One with no size is the whole square.
    assert_eq!(
        region_of(&PackOptions {
            region: Some((Vec2::splat(0.3), Vec2::new(0.3, 0.9))),
            ..PackOptions::default()
        }),
        (Vec2::ZERO, Vec2::ONE)
    );
}

#[test]
fn udim_tiles_with_a_region_use_the_region_in_each_tile() {
    let pieces: Vec<Piece> = (0..8).map(|i| square(Vec2::splat(i as f32), 0.3)).collect();
    let options = PackOptions {
        tiles: 2,
        region: Some((Vec2::new(0.5, 0.0), Vec2::ONE)),
        ..PackOptions::default()
    };
    let b = boxes(&pieces, &pack(&pieces, &options).unwrap());
    assert!(
        b.iter().any(|(lo, _)| lo.x >= 1.5 - 1e-4),
        "some in tile 1002"
    );
    for (lo, hi) in &b {
        let local = lo.x.fract();
        assert!(local >= 0.5 - 1e-4, "outside the region: {lo}");
        assert!(
            hi.x <= lo.x.floor() + 1.0 + 1e-4,
            "across a side: {lo} {hi}"
        );
    }
}

/// Two identical L shapes, the second lying a half turn round.
fn two_ls() -> Vec<Piece> {
    let l = |turn: f32, at: Vec2| {
        let t = Affine2::from_angle(turn);
        let poly = |corners: [(f32, f32); 4]| {
            corners
                .iter()
                .map(|(x, y)| at + t.transform_point2(Vec2::new(*x, *y)))
                .collect()
        };
        Piece {
            polygons: vec![
                poly([(0.0, 0.0), (2.0, 0.0), (2.0, 1.0), (0.0, 1.0)]),
                poly([(0.0, 1.0), (1.0, 1.0), (1.0, 3.0), (0.0, 3.0)]),
            ],
        }
    };
    vec![
        l(0.0, Vec2::ZERO),
        l(std::f32::consts::PI, Vec2::new(10.0, 0.0)),
    ]
}

fn angle(t: &Affine2) -> f32 {
    t.matrix2.x_axis.y.atan2(t.matrix2.x_axis.x)
}

#[test]
fn a_grain_piece_is_never_turned_even_when_stacked() {
    let pieces = two_ls();
    let grain = PieceAsks {
        grain: true,
        scale: 1.0,
    };
    let options = PackOptions {
        stack: true,
        ..PackOptions::default()
    };
    // Both keeping their grain, one keeping it, and turning off.
    for (asks, rotate) in [
        (vec![grain, grain], true),
        (vec![PieceAsks::default(), grain], true),
        (vec![], false),
    ] {
        let options = PackOptions { rotate, ..options };
        let moves = pack_with(&pieces, &asks, &[], &options).unwrap();
        for (i, t) in moves.iter().enumerate() {
            if asks.get(i).is_some_and(|a| a.grain) || !rotate {
                assert!(angle(t).abs() < 1e-3, "{asks:?}: turned {}", angle(t));
            }
        }
        // Not stacked (they lie different ways), so not in one place (an
        // L can tuck into the other's corner, so their boxes may overlap).
        let b = boxes(&pieces, &moves);
        assert!(b[0].0.distance(b[1].0) > 1e-3, "{b:?}");
    }
    // Free to turn, they still stack.
    let moves = pack_with(&pieces, &[], &[], &options).unwrap();
    let b = boxes(&pieces, &moves);
    assert!(b[0].0.distance(b[1].0) < 1e-4, "{b:?}");
}

#[test]
fn identical_grain_pieces_lying_the_same_way_still_stack() {
    let pieces = vec![two_ls()[0].clone(), {
        let mut p = two_ls()[0].clone();
        for v in p.polygons.iter_mut().flatten() {
            *v += Vec2::new(20.0, 0.0);
        }
        p
    }];
    let grain = PieceAsks {
        grain: true,
        scale: 1.0,
    };
    let options = PackOptions {
        stack: true,
        ..PackOptions::default()
    };
    let moves = pack_with(&pieces, &[grain, grain], &[], &options).unwrap();
    let b = boxes(&pieces, &moves);
    assert!(b[0].0.distance(b[1].0) < 1e-4, "{b:?}");
    assert!(moves.iter().all(|t| angle(t).abs() < 1e-3));
}

#[test]
fn growing_a_grid_matches_growing_each_cell() {
    // The quick way against the plain way, on an odd shape and sizes
    // across word boundaries.
    for (w, h, by) in [(5, 3, 0), (70, 9, 1), (63, 2, 7), (130, 40, 33), (1, 1, 64)] {
        let mut g = Grid::new(w, h);
        for y in 0..h {
            for x in 0..w {
                if (x * 7 + y * 13) % 11 == 0 || x == w - 1 {
                    g.set(x, y);
                }
            }
        }
        let quick = g.grown(by);
        let mut plain = Grid::new(w + 2 * by, h + 2 * by);
        for y in 0..h {
            for x in 0..w {
                if g.get(x, y) {
                    for dy in 0..=2 * by {
                        for dx in 0..=2 * by {
                            plain.set(x + dx, y + dy);
                        }
                    }
                }
            }
        }
        assert!(quick.rows == plain.rows, "{w} x {h} by {by}");
    }
}

#[test]
fn locked_pieces_outside_the_region_take_nothing_inside_it() {
    let filled = vec![rect(Vec2::ZERO, Vec2::new(0.4, 1.0))];
    let pieces = squares(2);
    let options = PackOptions {
        region: Some((Vec2::new(0.5, 0.0), Vec2::ONE)),
        ..PackOptions::default()
    };
    let with = pack_with(&pieces, &[], &filled, &options).unwrap();
    let without = pack(&pieces, &options).unwrap();
    assert_eq!(with, without);
}

#[test]
fn a_big_lightmap_with_lots_of_room_is_quick() {
    let cube = mesh::Primitive::Cube(mesh::CubeParams::default()).mesh();
    let hard: Vec<mesh::EdgeKey> = cube.hard_edges().collect();
    let t = std::time::Instant::now();
    assert!(crate::lightmap::lightmap(&cube, &hard, 2048, 64).is_some());
    // Was 204 s in a debug build (each set cell grown by a square of
    // cells); now a few seconds, more on a busy CI machine. The limit
    // only has to catch the old slowness coming back, not time it
    // closely.
    assert!(t.elapsed().as_secs_f32() < 30.0, "{:?}", t.elapsed());
}

#[test]
fn packing_twice_gives_the_same_layout() {
    let mut shapes: Vec<(String, mesh::Mesh)> = mesh::Primitive::all()
        .into_iter()
        .map(|p| (p.name().to_string(), p.mesh()))
        .collect();
    shapes.push((
        "tall thin cube".into(),
        mesh::Primitive::Cube(mesh::CubeParams {
            size: glam::Vec3::new(1.0, 1.0, 1000.0),
            segments: [1, 1, 8],
        })
        .mesh(),
    ));
    let pack_all = |mesh: &mut mesh::Mesh| {
        let islands = islands_of(mesh, &[]);
        let pieces: Vec<Piece> = islands.iter().map(|i| piece_of(mesh, i)).collect();
        let moves = pack(&pieces, &PackOptions::default()).unwrap();
        let placed: Vec<_> = islands.into_iter().zip(moves).collect();
        place(mesh, &placed);
    };
    let values = |mesh: &mesh::Mesh| {
        Uvs::of(mesh)
            .unwrap()
            .into_values(mesh.face_layout().1.len())
    };
    for (name, mut mesh) in shapes {
        let hard: Vec<mesh::EdgeKey> = mesh.hard_edges().collect();
        let seams = crate::seams::suggest(&mesh, &hard);
        let old = mesh::uvs::seams(&mesh);
        mesh::uvs::set_seams(&mut mesh, &old, false);
        mesh::uvs::set_seams(&mut mesh, &seams, true);
        let faces: Vec<FaceId> = mesh.faces().map(|(f, _)| f).collect();
        crate::unwrap(&mut mesh, &faces, true);
        pack_all(&mut mesh);
        let once = values(&mesh);
        pack_all(&mut mesh);
        let worst = once
            .iter()
            .zip(&values(&mesh))
            .map(|(a, b)| a.distance(*b))
            .fold(0.0f32, f32::max);
        assert!(worst < 0.02, "{name}: moved {worst} on a second pack");
    }
}
