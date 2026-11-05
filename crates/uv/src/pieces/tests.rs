//! Tests for editing pieces by hand (decision 0236).

use super::*;
use mesh::{CubeParams, PlaneParams, Primitive};

/// A plane two polygons across, with its UVs laid flat (its own).
fn strip() -> Mesh {
    Primitive::Plane(PlaneParams {
        segments: [2, 1],
        ..Default::default()
    })
    .mesh()
}

fn faces(mesh: &Mesh) -> Vec<FaceId> {
    mesh.faces().map(|(f, _)| f).collect()
}

/// The edge the two polygons share.
fn middle_edge(mesh: &Mesh) -> EdgeKey {
    let t = mesh.topology();
    t.edges().find(|e| t.edge_faces(*e).count() == 2).unwrap()
}

/// Moves polygon `f`'s UVs by `by` (cutting it off its neighbour in the
/// texture).
fn shift(mesh: &mut Mesh, f: FaceId, by: Vec2) {
    let layout = Layout::of(mesh).unwrap();
    let corners: Vec<Corner> = mesh
        .face(f)
        .unwrap()
        .iter()
        .map(|v| (f, *v, layout.at(mesh, f, *v).unwrap() + by))
        .collect();
    assert_eq!(mesh::uvs::set_corner_uvs(mesh, &corners).matched, corners.len());
}

fn apply(mesh: &mut Mesh, corners: &[Corner]) {
    mesh::uvs::set_corner_uvs(mesh, corners);
}

#[test]
fn stitching_brings_a_cut_off_piece_back_and_joins_the_edge() {
    let mut mesh = strip();
    let fs = faces(&mesh);
    let edge = middle_edge(&mesh);
    let before = mesh.clone();
    shift(&mut mesh, fs[1], Vec2::new(0.3, 0.2));
    assert_eq!(mesh::uvs::islands(&mesh).len(), 2, "cut apart");
    let done = stitch(&mesh, &[edge]);
    assert_eq!(done.joined, vec![edge]);
    apply(&mut mesh, &done.corners);
    assert_eq!(mesh::uvs::islands(&mesh).len(), 1, "one piece again");
    // Back where it was, since only a move separated them.
    let a = Layout::of(&mesh).unwrap().values;
    let b = Layout::of(&before).unwrap().values;
    for (p, q) in a.iter().zip(&b) {
        assert!(p.distance(*q) < 1e-4, "{p} {q}");
    }
}

#[test]
fn stitching_turns_and_sizes_the_moved_piece_to_fit() {
    let mut mesh = strip();
    let fs = faces(&mesh);
    let edge = middle_edge(&mesh);
    // Turn and shrink the second polygon's UVs about a far spot.
    let layout = Layout::of(&mesh).unwrap();
    let m = Affine2::from_scale_angle_translation(Vec2::splat(0.5), 1.0, Vec2::new(0.7, 0.1));
    let corners: Vec<Corner> = mesh
        .face(fs[1])
        .unwrap()
        .iter()
        .map(|v| (fs[1], *v, m.transform_point2(layout.at(&mesh, fs[1], *v).unwrap())))
        .collect();
    apply(&mut mesh, &corners);
    let done = stitch(&mesh, &[edge]);
    apply(&mut mesh, &done.corners);
    assert_eq!(mesh::uvs::islands(&mesh).len(), 1);
    // Its UV area is back to its neighbour's (the plane's polygons match).
    let area = |f: FaceId| {
        let l = Layout::of(&mesh).unwrap();
        let c: Vec<Vec2> = mesh.face(f).unwrap().iter().map(|v| l.at(&mesh, f, *v).unwrap()).collect();
        let mut a = 0.0;
        for k in 0..c.len() {
            a += c[k].perp_dot(c[(k + 1) % c.len()]);
        }
        a / 2.0
    };
    assert!((area(fs[0]) - area(fs[1])).abs() < 1e-4);
}

#[test]
fn stitching_skips_joined_open_and_locked_edges() {
    let mut mesh = strip();
    let fs = faces(&mesh);
    let edge = middle_edge(&mesh);
    // Joined already: nothing to do.
    assert_eq!(stitch(&mesh, &[edge]), Stitched::default());
    // An open edge.
    let open = mesh
        .topology()
        .edges()
        .find(|e| mesh.topology().edge_faces(*e).count() == 1)
        .unwrap();
    shift(&mut mesh, fs[1], Vec2::new(0.4, 0.0));
    assert!(stitch(&mesh, &[open]).joined.is_empty());
    // Both sides locked: nothing moves.
    mesh::uvs::set_locked(&mut mesh, &fs, true);
    assert!(stitch(&mesh, &[edge]).corners.is_empty());
    // Without UVs at all.
    let mut bare = strip();
    bare.remove_attribute(mesh::attributes::UV);
    assert_eq!(stitch(&bare, &[edge]), Stitched::default());
}

#[test]
fn a_locked_piece_stays_and_the_other_comes_to_it() {
    let mut mesh = strip();
    let fs = faces(&mesh);
    let edge = middle_edge(&mesh);
    shift(&mut mesh, fs[1], Vec2::new(0.3, 0.0));
    // The bigger-or-equal choice would move fs[1]; lock it so fs[0] moves.
    mesh::uvs::set_locked(&mut mesh, &fs[1..], true);
    let done = stitch(&mesh, &[edge]);
    assert!(done.corners.iter().all(|(f, _, _)| *f == fs[0]));
}

#[test]
fn cutting_round_a_part_nudges_it_away_and_a_slit_moves_nothing() {
    let mut cube = Primitive::Cube(CubeParams::default()).mesh();
    // Join every side into one piece first: clear its seams... instead,
    // use a strip: cut its middle edge.
    let strip = strip();
    let edge = middle_edge(&strip);
    let moved = cut(&strip, &[edge], 0.01);
    assert!(!moved.is_empty(), "the strip parts in two");
    let mut parted = strip.clone();
    apply(&mut parted, &moved);
    assert_eq!(mesh::uvs::islands(&parted).len(), 2);
    // A slit in a bigger piece that doesn't part it moves nothing.
    let big = Primitive::Plane(PlaneParams {
        segments: [3, 3],
        ..Default::default()
    })
    .mesh();
    let t = big.topology();
    let inner = t
        .edges()
        .find(|e| t.edge_faces(*e).count() == 2)
        .unwrap();
    assert!(cut(&big, &[inner], 0.01).is_empty());
    // A broken gap moves nothing.
    assert!(cut(&strip, &[edge], f32::NAN).is_empty());
    assert!(cut(&strip, &[edge], 0.0).is_empty());
    let _ = &mut cube;
}

#[test]
fn aligning_lines_pieces_up_and_spreading_evens_them() {
    let mut mesh = Primitive::Plane(PlaneParams {
        segments: [3, 1],
        ..Default::default()
    })
    .mesh();
    let fs = faces(&mesh);
    // Three pieces at different heights.
    shift(&mut mesh, fs[1], Vec2::new(0.0, 0.3));
    shift(&mut mesh, fs[2], Vec2::new(0.5, -0.2));
    let pieces = mesh::uvs::islands(&mesh);
    assert_eq!(pieces.len(), 3);
    let lows = |m: &Mesh| -> Vec<f32> {
        let l = Layout::of(m).unwrap();
        mesh::uvs::islands(m)
            .iter()
            .map(|p| bounds(m, &l, p).unwrap().0.y)
            .collect()
    };
    let mut bottom = mesh.clone();
    apply(&mut bottom, &align(&mesh, &pieces, Align::Bottom));
    let l = lows(&bottom);
    assert!(l.iter().all(|y| (y - l[0]).abs() < 1e-5), "{l:?}");
    // Spread across: equal steps between the middles.
    let mut spread = mesh.clone();
    apply(&mut spread, &align(&mesh, &pieces, Align::SpreadAcross));
    let lay = Layout::of(&spread).unwrap();
    let mut mids: Vec<f32> = mesh::uvs::islands(&spread)
        .iter()
        .map(|p| {
            let (a, b) = bounds(&spread, &lay, p).unwrap();
            (a.x + b.x) / 2.0
        })
        .collect();
    mids.sort_by(f32::total_cmp);
    assert!(((mids[1] - mids[0]) - (mids[2] - mids[1])).abs() < 1e-5, "{mids:?}");
    // One piece can't be aligned with anything; two can't be spread.
    assert!(align(&mesh, &pieces[..1], Align::Left).is_empty());
    assert!(align(&mesh, &pieces[..2], Align::SpreadUp).is_empty());
    // Every way gives real UVs.
    for how in Align::ALL {
        assert!(align(&mesh, &pieces, how).iter().all(|c| c.2.is_finite()));
    }
}

#[test]
fn orienting_turns_a_piece_square_to_its_edge() {
    let mut mesh = strip();
    let fs = faces(&mesh);
    let edge = middle_edge(&mesh);
    let all = fs.clone();
    // Turn the whole piece by 20°.
    let layout = Layout::of(&mesh).unwrap();
    let m = Affine2::from_angle(0.35);
    let turned: Vec<Corner> = corners_of(&mesh, &layout, &all)
        .into_iter()
        .map(|(f, v, uv)| (f, v, m.transform_point2(uv)))
        .collect();
    apply(&mut mesh, &turned);
    let done = orient(&mesh, &all, edge);
    assert!(!done.is_empty());
    apply(&mut mesh, &done);
    let l = Layout::of(&mesh).unwrap();
    let (a, b) = edge.ends();
    let d = l.at(&mesh, fs[0], b).unwrap() - l.at(&mesh, fs[0], a).unwrap();
    assert!(d.x.abs() < 1e-4 || d.y.abs() < 1e-4, "{d}");
    // Already square: nothing to do.
    assert!(orient(&mesh, &all, edge).is_empty());
}

#[test]
fn the_lattice_starts_even_and_warps_by_its_points() {
    let lo = Vec2::ZERO;
    let hi = Vec2::ONE;
    let grid = lattice_grid(lo, hi, 3);
    assert_eq!(grid.len(), 9);
    // Untouched, nothing moves.
    for uv in [Vec2::new(0.2, 0.7), Vec2::new(0.5, 0.5), Vec2::ONE] {
        assert!(warp_point(uv, lo, hi, 3, &grid).distance(uv) < 1e-6);
    }
    // Pull the middle point right: the middle moves with it, the corners
    // stay.
    let mut pulled = grid.clone();
    pulled[4] += Vec2::new(0.2, 0.0);
    assert!(warp_point(Vec2::splat(0.5), lo, hi, 3, &pulled).distance(Vec2::new(0.7, 0.5)) < 1e-6);
    assert!(warp_point(Vec2::ZERO, lo, hi, 3, &pulled).distance(Vec2::ZERO) < 1e-6);
    // A wrong number of points changes nothing; sizes are kept in range.
    assert_eq!(warp_point(Vec2::splat(0.3), lo, hi, 3, &grid[..4]), Vec2::splat(0.3));
    assert_eq!(lattice_grid(lo, hi, 1).len(), 4);
    assert_eq!(lattice_grid(lo, hi, 99).len(), 64);
}

#[test]
fn pieces_holding_and_corners_now_find_what_was_asked() {
    let mut mesh = strip();
    let fs = faces(&mesh);
    shift(&mut mesh, fs[1], Vec2::new(0.3, 0.0));
    assert_eq!(pieces_holding(&mesh, &fs[..1]), vec![vec![fs[0]]]);
    assert_eq!(pieces_holding(&mesh, &fs).len(), 2);
    assert_eq!(corners_now(&mesh, &fs).len(), 8);
    mesh::uvs::set_locked(&mut mesh, &fs[..1], true);
    assert_eq!(corners_now(&mesh, &fs).len(), 4, "locked left out");
}
