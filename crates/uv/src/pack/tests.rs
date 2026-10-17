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
