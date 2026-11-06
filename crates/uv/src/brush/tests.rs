//! Tests for the UV brushes (decision 0238).

use super::*;
use mesh::{PlaneParams, Primitive};

fn grid() -> Mesh {
    Primitive::Plane(PlaneParams {
        segments: [4, 4],
        ..Default::default()
    })
    .mesh()
}

/// The middle of the grid's UVs.
fn middle(mesh: &Mesh) -> Vec2 {
    let (lo, hi) =
        mesh::uvs::uv_bounds(mesh, &mesh.faces().map(|(f, _)| f).collect::<Vec<_>>(), &[]).unwrap();
    (lo + hi) / 2.0
}

#[test]
fn a_grid_has_one_spot_per_point_with_its_neighbours() {
    let mesh = grid();
    let spots = Spots::of(&mesh);
    assert_eq!(spots.len(), 25);
    // An inner point has four neighbours, a corner two.
    let counts: Vec<usize> = spots.spots.iter().map(|s| s.neighbours.len()).collect();
    assert_eq!(counts.iter().filter(|c| **c == 4).count(), 9);
    assert_eq!(counts.iter().filter(|c| **c == 2).count(), 4);
}

#[test]
fn falloff_is_full_in_the_middle_and_none_at_the_rim() {
    assert_eq!(falloff(0.0, 1.0), 1.0);
    assert_eq!(falloff(1.0, 1.0), 0.0);
    assert!(falloff(0.5, 1.0) > 0.0 && falloff(0.5, 1.0) < 1.0);
    assert_eq!(falloff(0.1, 0.0), 0.0);
    assert_eq!(falloff(f32::NAN, 1.0), 0.0);
}

#[test]
fn grab_moves_the_middle_most_and_leaves_far_spots() {
    let mesh = grid();
    let c = middle(&mesh);
    let mut spots = Spots::of(&mesh);
    let moved = spots.dab(UvBrushKind::Grab, c, 0.3, Vec2::new(0.05, 0.0), 1.0, false);
    assert!(moved > 0);
    let corners = spots.moved_corners();
    // The spot at the middle moved the whole way; none moved backwards.
    let most = corners
        .iter()
        .map(|(_, _, uv)| *uv)
        .fold(f32::MIN, |m, uv| m.max(uv.x - c.x));
    assert!(most > 0.04);
    // Corners at the same point move together (a piece never tears).
    let mut by_point: HashMap<VertexId, Vec2> = HashMap::new();
    for (_, v, uv) in &corners {
        if let Some(was) = by_point.insert(*v, *uv) {
            assert_eq!(was, *uv);
        }
    }
}

#[test]
fn relax_evens_a_bent_spot_out() {
    let mut mesh = grid();
    let c = middle(&mesh);
    // Push the middle point's corners off.
    let mut spots = Spots::of(&mesh);
    spots.dab(
        UvBrushKind::Grab,
        c,
        0.05,
        Vec2::new(0.08, 0.06),
        1.0,
        false,
    );
    mesh::uvs::set_corner_uvs(&mut mesh, &spots.moved_corners());
    let mut spots = Spots::of(&mesh);
    let bent = c + Vec2::new(0.08, 0.06);
    for _ in 0..30 {
        spots.dab(UvBrushKind::Relax, bent, 0.2, Vec2::ZERO, 0.5, false);
    }
    let corners = spots.moved_corners();
    let back = corners
        .iter()
        .map(|(_, _, uv)| uv.distance(c))
        .fold(f32::MAX, f32::min);
    assert!(back < 0.02, "{back}");
}

#[test]
fn pinch_pulls_in_and_the_other_way_pushes_out() {
    let mesh = grid();
    let c = middle(&mesh);
    let spread = |s: &Spots| -> f32 {
        s.spots
            .iter()
            .map(|p| p.at.distance(c))
            .filter(|d| *d < 0.3)
            .sum()
    };
    let mut pinched = Spots::of(&mesh);
    let before = spread(&pinched);
    pinched.dab(UvBrushKind::Pinch, c, 0.4, Vec2::ZERO, 1.0, false);
    assert!(spread(&pinched) < before);
    let mut pushed = Spots::of(&mesh);
    pushed.dab(UvBrushKind::Pinch, c, 0.4, Vec2::ZERO, 1.0, true);
    assert!(spread(&pushed) > before);
}

#[test]
fn pinned_and_locked_uvs_stay_and_broken_dabs_do_nothing() {
    let mut mesh = grid();
    let faces: Vec<FaceId> = mesh.faces().map(|(f, _)| f).collect();
    let c = middle(&mesh);
    mesh::uvs::set_locked(&mut mesh, &faces, true);
    let mut spots = Spots::of(&mesh);
    assert_eq!(spots.dab(UvBrushKind::Grab, c, 1.0, Vec2::X, 1.0, false), 0);
    let mut free = Spots::of(&grid());
    assert_eq!(
        free.dab(UvBrushKind::Grab, Vec2::NAN, 1.0, Vec2::X, 1.0, false),
        0
    );
    assert_eq!(free.dab(UvBrushKind::Grab, c, 0.0, Vec2::X, 1.0, false), 0);
    assert!(free.moved_corners().is_empty());
    // A shape without UVs has no spots.
    let mut bare = grid();
    bare.remove_attribute(mesh::attributes::UV);
    assert!(Spots::of(&bare).is_empty());
}
