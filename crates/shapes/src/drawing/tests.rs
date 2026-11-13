//! Tests for drawings: adding pieces, every refusal, changes, order, the
//! mirror line and rebuilding from parts.

use super::*;
use crate::path::Knot;
use glam::Vec2;

fn square(at: f32, side: f32) -> Path {
    Path::polygon([
        Vec2::new(at, at),
        Vec2::new(at + side, at),
        Vec2::new(at + side, at + side),
        Vec2::new(at, at + side),
    ])
}

fn one() -> Drawing {
    Drawing::new()
        .with_piece("Leaf", square(0.25, 0.5), PieceKind::Add)
        .unwrap()
}

#[test]
fn pieces_are_added_on_top_with_ids_never_reused() {
    let d = one();
    assert_eq!(d.pieces().len(), 1);
    assert_eq!(d.pieces()[0].id, PieceId(1));
    let d = d
        .with_piece("Bite", square(0.6, 0.2), PieceKind::Cut)
        .unwrap();
    assert_eq!(d.pieces()[1].id, PieceId(2));
    let d = d.without(&[PieceId(2)]).unwrap();
    let d = d
        .with_piece("Again", square(0.1, 0.1), PieceKind::Add)
        .unwrap();
    assert_eq!(d.pieces()[1].id, PieceId(3));
    assert!(d.piece(PieceId(2)).is_none());
    assert!(d.without(&[PieceId(2)]).is_err());
}

#[test]
fn open_paths_are_lines_and_closed_paths_need_room_inside() {
    let open = Path::new(square(0.2, 0.3).knots().to_vec(), false);
    assert_eq!(
        Drawing::new().with_piece("Open", open.clone(), PieceKind::Add),
        Err(Problem::OpenPath)
    );
    assert_eq!(
        Drawing::new().with_piece("Open", open.clone(), PieceKind::Cut),
        Err(Problem::OpenPath)
    );
    assert!(
        Drawing::new()
            .with_piece("Vein", open, PieceKind::Line)
            .is_ok()
    );
    // One point; points on one line; two on one spot.
    let dot = Path::polygon([Vec2::splat(0.5)]);
    assert_eq!(
        Drawing::new().with_piece("Dot", dot.clone(), PieceKind::Add),
        Err(Problem::NoInside)
    );
    let flat = Path::polygon([Vec2::ZERO, Vec2::splat(0.5), Vec2::ONE]);
    assert_eq!(
        Drawing::new().with_piece("Flat", flat, PieceKind::Add),
        Err(Problem::NoInside)
    );
    assert_eq!(
        Drawing::new().with_piece(
            "Short",
            Path::new(dot.knots().to_vec(), false),
            PieceKind::Line
        ),
        Err(Problem::ShortLine)
    );
    // Two points with curved sides have room inside; a figure of eight
    // (signed area nothing) too.
    let lens = Path::new(
        vec![
            Knot::smooth(Vec2::new(0.3, 0.5), Vec2::new(0.0, -0.2)),
            Knot::smooth(Vec2::new(0.7, 0.5), Vec2::new(0.0, 0.2)),
        ],
        true,
    );
    assert!(
        Drawing::new()
            .with_piece("Lens", lens, PieceKind::Add)
            .is_ok()
    );
    let eight = Path::polygon([
        Vec2::new(0.1, 0.1),
        Vec2::new(0.9, 0.9),
        Vec2::new(0.9, 0.1),
        Vec2::new(0.1, 0.9),
    ]);
    assert!(
        Drawing::new()
            .with_piece("Eight", eight, PieceKind::Add)
            .is_ok()
    );
    // Names.
    assert_eq!(
        Drawing::new().with_piece(" ", square(0.1, 0.1), PieceKind::Add),
        Err(Problem::BadName)
    );
    for p in [
        Problem::OpenPath,
        Problem::NoInside,
        Problem::ShortLine,
        Problem::TooMany,
        Problem::OutOfPlace,
        Problem::BadName,
        Problem::Gone,
    ] {
        assert!(p.describe().ends_with('.'));
    }
}

#[test]
fn kinds_names_mirroring_and_paths_change_with_checks() {
    let d = one();
    let id = PieceId(1);
    let line = d.with_kind(id, PieceKind::Line).unwrap();
    assert!(!line.piece(id).unwrap().path.is_closed());
    // A line can't become a fill again until it is closed.
    assert_eq!(line.with_kind(id, PieceKind::Add), Err(Problem::OpenPath));
    let closed = line
        .with_path(id, line.piece(id).unwrap().path.clone().closed())
        .unwrap()
        .with_kind(id, PieceKind::Cut)
        .unwrap();
    assert_eq!(closed.piece(id).unwrap().kind, PieceKind::Cut);
    assert_eq!(
        d.with_name(id, "Big leaf").unwrap().piece(id).unwrap().name,
        "Big leaf"
    );
    assert!(
        !d.with_mirrored(id, false)
            .unwrap()
            .piece(id)
            .unwrap()
            .mirrored
    );
    assert_eq!(d.with_name(PieceId(9), "x"), Err(Problem::Gone));
    let far = d.with_change(&[id], &Change::moved(Vec2::splat(100.0)));
    assert_eq!(far, Err(Problem::OutOfPlace));
    let moved = d
        .with_change(&[id], &Change::moved(Vec2::new(0.1, 0.0)))
        .unwrap();
    assert_eq!(moved.piece(id).unwrap().path.middle(), Vec2::new(0.6, 0.5));
}

#[test]
fn order_moves_within_the_list() {
    let d = one()
        .with_piece("B", square(0.1, 0.1), PieceKind::Add)
        .unwrap()
        .with_piece("C", square(0.1, 0.1), PieceKind::Add)
        .unwrap();
    let ids = |d: &Drawing| d.pieces().iter().map(|p| p.id.0).collect::<Vec<_>>();
    assert_eq!(ids(&d.with_order(PieceId(1), 1).unwrap()), vec![2, 1, 3]);
    assert_eq!(ids(&d.with_order(PieceId(1), 9).unwrap()), vec![2, 3, 1]);
    assert_eq!(ids(&d.with_order(PieceId(3), -9).unwrap()), vec![3, 1, 2]);
    assert!(d.with_order(PieceId(7), 1).is_err());
}

#[test]
fn the_mirror_line_draws_pieces_twice_and_stays_on_the_texture() {
    let d = one();
    let p = d.pieces()[0].clone();
    assert_eq!(d.drawn_paths(&p).len(), 1);
    let m = d.with_mirror(Mirror { on: true, x: 0.3 }).unwrap();
    let drawn = m.drawn_paths(&p);
    assert_eq!(drawn.len(), 2);
    assert!(drawn[1].middle().distance(Vec2::new(0.1, 0.5)) < 1e-6);
    let unmirrored = m.with_mirrored(p.id, false).unwrap();
    assert_eq!(unmirrored.drawn_paths(&unmirrored.pieces()[0]).len(), 1);
    assert_eq!(
        d.with_mirror(Mirror { on: true, x: 3.0 })
            .unwrap()
            .mirror()
            .x,
        1.0
    );
    assert!(
        d.with_mirror(Mirror {
            on: true,
            x: f32::NAN
        })
        .is_err()
    );
    assert_ne!(d.key(), m.key());
}

#[test]
fn the_reference_image_never_changes_the_key() {
    let d = one();
    let r = crate::Reference::new(1, 1, vec![9; 4], "ref").unwrap();
    let with = d.with_reference(Some(std::sync::Arc::new(r)));
    assert_eq!(d.key(), with.key());
    assert!(with.reference().is_some());
    assert!(with.memory_size() > d.memory_size());
    assert_ne!(d.key(), Drawing::new().key());
}

#[test]
fn parts_round_trip_and_damage_is_refused() {
    let d = one()
        .with_piece(
            "Vein",
            Path::new(square(0.3, 0.1).knots().to_vec(), false),
            PieceKind::Line,
        )
        .unwrap()
        .with_mirror(Mirror { on: true, x: 0.5 })
        .unwrap();
    assert_eq!(Drawing::from_parts(d.to_parts()), Ok(d.clone()));
    let mut twice = d.to_parts();
    twice.pieces[1].id = PieceId(1);
    assert!(Drawing::from_parts(twice).is_err());
    let mut ahead = d.to_parts();
    ahead.next_id = 1;
    assert!(Drawing::from_parts(ahead).is_err());
    let mut open = d.to_parts();
    open.pieces[0].path = Path::new(open.pieces[0].path.knots().to_vec(), false);
    assert_eq!(Drawing::from_parts(open), Err(Problem::OpenPath));
    let mut many = d.to_parts();
    many.pieces = vec![many.pieces[0].clone(); MAX_PIECES + 1];
    assert_eq!(Drawing::from_parts(many), Err(Problem::TooMany));
    let mut wild = d.to_parts();
    wild.mirror.x = 7.0;
    assert_eq!(Drawing::from_parts(wild).unwrap().mirror().x, 1.0);
}

#[test]
fn too_many_pieces_or_points_are_refused() {
    let mut d = Drawing::new();
    for _ in 0..MAX_PIECES {
        d = d.with_piece("P", square(0.1, 0.1), PieceKind::Add).unwrap();
    }
    assert_eq!(
        d.with_piece("P", square(0.1, 0.1), PieceKind::Add),
        Err(Problem::TooMany)
    );
    let crowded = crate::Basic::Polygon(32).path(Vec2::splat(0.5), 0.3);
    let many: Vec<Knot> = (0..crate::path::MAX_KNOTS)
        .map(|i| crowded.knots()[i % 32])
        .collect();
    let mut d = Drawing::new();
    let mut refused = false;
    for _ in 0..(MAX_ALL_KNOTS / crate::path::MAX_KNOTS + 1) {
        match d.with_piece("Many", Path::new(many.clone(), true), PieceKind::Add) {
            Ok(next) => d = next,
            Err(why) => {
                assert_eq!(why, Problem::TooMany);
                refused = true;
            }
        }
    }
    assert!(refused);
}

/// A fuzz run: random changes never leave a drawing that its own parts
/// can't rebuild.
#[test]
fn random_changes_always_leave_a_drawing_that_rebuilds() {
    let mut state = 7u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (state >> 33) as u32
    };
    let mut d = Drawing::new();
    for _ in 0..600 {
        let r = next();
        let pick = |d: &Drawing, r: u32| {
            d.pieces()
                .get(r as usize % d.pieces().len().max(1))
                .map_or(PieceId(99), |p| p.id)
        };
        let v = (next() % 2000) as f32 / 1000.0 - 0.5;
        let result = match r % 8 {
            0 | 1 => {
                let n = next() % 6;
                let pts: Vec<Vec2> = (0..n)
                    .map(|_| Vec2::new((next() % 100) as f32 / 99.0, (next() % 100) as f32 / 99.0))
                    .collect();
                let closed = next() % 3 > 0;
                let kind = [PieceKind::Add, PieceKind::Cut, PieceKind::Line][next() as usize % 3];
                let path = Path::new(pts.into_iter().map(Knot::corner).collect(), closed);
                d.with_piece("P", path, kind)
            }
            2 => d.with_kind(
                pick(&d, next()),
                [PieceKind::Add, PieceKind::Cut, PieceKind::Line][next() as usize % 3],
            ),
            3 => d.with_change(&[pick(&d, next())], &Change::turned(Vec2::splat(v), v)),
            4 => d.with_order(pick(&d, next()), next() as i32 % 5 - 2),
            5 => d.with_mirror(Mirror {
                on: next() % 2 == 0,
                x: v,
            }),
            6 => d.without(&[pick(&d, next())]),
            _ => d.with_change(
                &[pick(&d, next())],
                &Change::scaled(Vec2::splat(0.5), Vec2::splat(v)),
            ),
        };
        if let Ok(changed) = result {
            d = changed;
        }
        assert_eq!(Drawing::from_parts(d.to_parts()).as_ref(), Ok(&d));
        assert_eq!(crate::raster::mask(&d, 16).len(), 256);
    }
}
