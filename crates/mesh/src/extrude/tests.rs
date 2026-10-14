use super::*;
use crate::{MeshReport, PlaneParams, Primitive};

fn cube() -> Mesh {
    Primitive::Cube(Default::default()).mesh()
}

/// A flat grid of `n` × `n` squares, 100 across, facing up.
fn grid(n: u32) -> Mesh {
    Primitive::Plane(PlaneParams {
        segments: [n, n],
        ..Default::default()
    })
    .mesh()
}

/// The polygon facing most along `direction`.
fn facing(mesh: &Mesh, direction: Vec3) -> FaceId {
    mesh.faces()
        .map(|(f, _)| f)
        .max_by(|a, b| {
            mesh.face_normal(*a)
                .dot(direction)
                .total_cmp(&mesh.face_normal(*b).dot(direction))
        })
        .unwrap()
}

fn centre(mesh: &Mesh, face: FaceId) -> Vec3 {
    let corners = mesh.face(face).unwrap();
    corners
        .iter()
        .map(|v| mesh.position(*v).unwrap())
        .sum::<Vec3>()
        / corners.len() as f32
}

fn area(mesh: &Mesh, face: FaceId) -> f32 {
    let corners: Vec<Vec3> = mesh
        .face(face)
        .unwrap()
        .iter()
        .map(|v| mesh.position(*v).unwrap())
        .collect();
    let mut sum = Vec3::ZERO;
    for i in 0..corners.len() {
        sum += corners[i].cross(corners[(i + 1) % corners.len()]);
    }
    sum.length() / 2.0
}

fn report(mesh: &Mesh) -> MeshReport {
    MeshReport::new(mesh)
}

fn every_point_is_a_number(mesh: &Mesh) {
    for (v, p) in mesh.vertices() {
        assert!(p.is_finite(), "point {v:?} is {p}");
    }
}

/// The polygons inside a grid's middle, as far as `reach` from its centre.
fn middle(mesh: &Mesh, reach: f32) -> Vec<FaceId> {
    mesh.faces()
        .map(|(f, _)| f)
        .filter(|f| {
            let c = centre(mesh, *f);
            c.x.abs() < reach && c.y.abs() < reach
        })
        .collect()
}

// ---------------------------------------------------------------- extrude

#[test]
fn extruding_a_cube_face_builds_four_walls_and_keeps_the_polygon() {
    let mut mesh = cube();
    let top = facing(&mesh, Vec3::Z);
    let before = centre(&mesh, top);
    let outcome = extrude_polygons(&mut mesh, &[top], 25.0, Grouping::Together);
    assert_eq!(
        outcome,
        Outcome {
            matched: 1,
            missing: 0,
            skipped: 0
        }
    );
    assert_eq!(mesh.vertex_count(), 12);
    assert_eq!(mesh.face_count(), 10);
    // The same polygon, 25 further up, the same size.
    assert!(centre(&mesh, top).distance(before + Vec3::Z * 25.0) < 1e-3);
    assert!((area(&mesh, top) - area(&cube(), top)).abs() < 1e-2);
    let r = report(&mesh);
    assert!(r.is_closed(), "{r:?}");
    assert!(!r.has_problems(), "{r:?}");
    assert_eq!(r.pieces, 1);
}

#[test]
fn an_extruded_cube_stays_crisp() {
    let mut mesh = cube();
    let top = facing(&mesh, Vec3::Z);
    extrude_polygons(&mut mesh, &[top], 25.0, Grouping::Together);
    // Every edge of the new block is hard, like the cube's own.
    for edge in mesh.edges() {
        let (a, b) = edge.ends();
        let pa = mesh.position(a).unwrap();
        let pb = mesh.position(b).unwrap();
        // Edges where the walls meet the old top are flat inside the
        // block's sides, but they were the cube's hard edges and stay so.
        assert!(mesh.is_hard(a, b), "edge {pa} – {pb} should be hard");
    }
}

#[test]
fn an_extruded_sphere_stays_smooth() {
    let mut mesh = Primitive::Sphere(Default::default()).mesh();
    let top = facing(&mesh, Vec3::new(0.3, 0.2, 1.0).normalize());
    extrude_polygons(&mut mesh, &[top], 10.0, Grouping::Together);
    assert_eq!(mesh.hard_edge_count(), 0);
}

#[test]
fn pushing_in_goes_the_other_way() {
    let mut mesh = cube();
    let top = facing(&mesh, Vec3::Z);
    let before = centre(&mesh, top);
    extrude_polygons(&mut mesh, &[top], -30.0, Grouping::Together);
    assert!(centre(&mesh, top).distance(before - Vec3::Z * 30.0) < 1e-3);
    assert!(report(&mesh).is_closed());
}

#[test]
fn neighbours_extrude_together_or_each_on_their_own() {
    let start = grid(2);
    let all: Vec<FaceId> = start.faces().map(|(f, _)| f).collect();

    // Together: one block. The middle point moves up instead of being
    // copied; the border's 8 points are copied, 8 walls.
    let mut together = start.clone();
    extrude_polygons(&mut together, &all, 10.0, Grouping::Together);
    assert_eq!(together.vertex_count(), 9 + 8);
    assert_eq!(together.face_count(), 4 + 8);
    let r = report(&together);
    assert_eq!(r.non_manifold_edges, 0, "{r:?}");
    assert_eq!(r.pieces, 1);
    for face in &all {
        assert!((centre(&together, *face).z - 10.0).abs() < 1e-3);
    }

    // Each on its own: four separate blocks, every corner copied.
    let mut each = start.clone();
    extrude_polygons(&mut each, &all, 10.0, Grouping::Separate);
    assert_eq!(each.vertex_count(), 9 + 16);
    assert_eq!(each.face_count(), 4 + 16);
    let r = report(&each);
    assert_eq!(r.non_manifold_edges, 0, "{r:?}");
    for face in &all {
        assert!((centre(&each, *face).z - 10.0).abs() < 1e-3);
    }
}

#[test]
fn polygons_around_a_corner_keep_an_even_thickness() {
    // The top and a side of a cube, together: each moves the full
    // distance along its own facing.
    let mut mesh = cube();
    let top = facing(&mesh, Vec3::Z);
    let side = facing(&mesh, Vec3::X);
    let (top_before, side_before) = (centre(&mesh, top), centre(&mesh, side));
    extrude_polygons(&mut mesh, &[top, side], 20.0, Grouping::Together);
    let top_moved = (centre(&mesh, top) - top_before).dot(Vec3::Z);
    let side_moved = (centre(&mesh, side) - side_before).dot(Vec3::X);
    assert!((top_moved - 20.0).abs() < 1e-2, "{top_moved}");
    assert!((side_moved - 20.0).abs() < 1e-2, "{side_moved}");
    assert!(report(&mesh).is_closed());
    // Six border points copied, six walls.
    assert_eq!(mesh.vertex_count(), 8 + 6);
    assert_eq!(mesh.face_count(), 6 + 6);
}

#[test]
fn a_whole_closed_shape_extruded_together_grows() {
    let mut mesh = cube();
    let all: Vec<FaceId> = mesh.faces().map(|(f, _)| f).collect();
    let size = mesh.bounds().unwrap().size();
    extrude_polygons(&mut mesh, &all, 10.0, Grouping::Together);
    assert_eq!(mesh.vertex_count(), 8, "no border: nothing new");
    assert_eq!(mesh.face_count(), 6);
    let grown = mesh.bounds().unwrap().size();
    assert!(grown.abs_diff_eq(size + Vec3::splat(20.0), 1e-2), "{grown}");
}

#[test]
fn an_open_border_gets_walls_too() {
    // A plane's only polygon: its border is the mesh's open border.
    let mut mesh = grid(1);
    let face = mesh.faces().next().unwrap().0;
    extrude_polygons(&mut mesh, &[face], 10.0, Grouping::Together);
    assert_eq!(mesh.face_count(), 5, "the lid and four walls: an open box");
    let r = report(&mesh);
    assert_eq!(r.border_edges, 4);
    assert_eq!(r.non_manifold_edges, 0);
    assert!(!r.has_problems(), "{r:?}");
}

#[test]
fn an_n_gon_extrudes_like_any_polygon() {
    let mut mesh = Primitive::Cylinder(Default::default()).mesh();
    let cap = facing(&mesh, Vec3::Z);
    let corners = mesh.face(cap).unwrap().len();
    assert!(corners > 4, "the cap is an n-gon");
    let (points, faces) = (mesh.vertex_count(), mesh.face_count());
    extrude_polygons(&mut mesh, &[cap], 15.0, Grouping::Together);
    assert_eq!(mesh.vertex_count(), points + corners);
    assert_eq!(mesh.face_count(), faces + corners);
    assert_eq!(mesh.face(cap).unwrap().len(), corners);
    assert!(report(&mesh).is_closed());
}

#[test]
fn extruding_again_carries_on_from_the_new_place() {
    let mut mesh = cube();
    let top = facing(&mesh, Vec3::Z);
    let before = centre(&mesh, top);
    extrude_polygons(&mut mesh, &[top], 10.0, Grouping::Together);
    extrude_polygons(&mut mesh, &[top], 10.0, Grouping::Together);
    assert!(centre(&mesh, top).distance(before + Vec3::Z * 20.0) < 1e-3);
    assert_eq!(mesh.face_count(), 14);
    let r = report(&mesh);
    assert!(r.is_closed() && !r.has_problems(), "{r:?}");
}

#[test]
fn missing_polygons_are_counted_and_duplicates_used_once() {
    let mut mesh = cube();
    let top = facing(&mesh, Vec3::Z);
    let outcome = extrude_polygons(
        &mut mesh,
        &[top, top, FaceId::from_raw(500)],
        10.0,
        Grouping::Together,
    );
    assert_eq!(outcome.matched, 1);
    assert_eq!(outcome.missing, 1);
    assert_eq!(mesh.face_count(), 10);
}

#[test]
fn nothing_or_a_broken_distance_changes_nothing() {
    for grouping in [Grouping::Together, Grouping::Separate] {
        let mut mesh = cube();
        assert_eq!(
            extrude_polygons(&mut mesh, &[], 10.0, grouping),
            Outcome::default()
        );
        assert_eq!(mesh, cube());
        let top = facing(&mesh, Vec3::Z);
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let outcome = extrude_polygons(&mut mesh, &[top], bad, grouping);
            assert_eq!(outcome.matched, 1);
            assert_eq!(mesh, cube());
            inset_polygons(&mut mesh, &[top], bad, grouping);
            assert_eq!(mesh, cube());
        }
        let mut empty = Mesh::new();
        let outcome = extrude_polygons(&mut empty, &[FaceId::from_raw(0)], 1.0, grouping);
        assert_eq!(outcome.missing, 1);
    }
}

#[test]
fn a_zero_distance_still_builds_the_walls() {
    // As in other programs: the walls are there, flat, ready to move.
    let mut mesh = cube();
    let top = facing(&mesh, Vec3::Z);
    extrude_polygons(&mut mesh, &[top], 0.0, Grouping::Together);
    assert_eq!(mesh.face_count(), 10);
    every_point_is_a_number(&mesh);
}

#[test]
fn awkward_polygons_never_make_broken_points() {
    // A polygon with no area (every corner in one place), one with a
    // zero-length side, and a triangle.
    let mut mesh = Mesh::new();
    let p = |m: &mut Mesh, x: f32, y: f32| m.add_vertex(Vec3::new(x, y, 0.0)).unwrap();
    let a = p(&mut mesh, 0.0, 0.0);
    let b = p(&mut mesh, 0.0, 0.0);
    let c = p(&mut mesh, 0.0, 0.0);
    let flat = mesh.add_face(&[a, b, c]).unwrap();
    let d = p(&mut mesh, 10.0, 0.0);
    let e = p(&mut mesh, 10.0, 0.0);
    let f = p(&mut mesh, 10.0, 10.0);
    let g = p(&mut mesh, 0.0, 10.0);
    let sliver = mesh.add_face(&[d, e, f, g]).unwrap();
    let h = p(&mut mesh, 20.0, 0.0);
    let i = p(&mut mesh, 30.0, 0.0);
    let j = p(&mut mesh, 20.0, 10.0);
    let triangle = mesh.add_face(&[h, i, j]).unwrap();
    let start = mesh.clone();
    for grouping in [Grouping::Together, Grouping::Separate] {
        for distance in [5.0, -5.0, 0.0, 1e6] {
            let mut m = start.clone();
            extrude_polygons(&mut m, &[flat, sliver, triangle], distance, grouping);
            every_point_is_a_number(&m);
            let mut m = start.clone();
            inset_polygons(&mut m, &[flat, sliver, triangle], distance, grouping);
            every_point_is_a_number(&m);
        }
    }
}

#[test]
fn non_manifold_input_is_handled() {
    // Three polygons on one edge.
    let mut mesh = Mesh::new();
    let v: Vec<VertexId> = [
        (0.0, 0.0, 0.0),
        (10.0, 0.0, 0.0),
        (5.0, 10.0, 0.0),
        (5.0, -10.0, 0.0),
        (5.0, 0.0, 10.0),
    ]
    .into_iter()
    .map(|(x, y, z)| mesh.add_vertex(Vec3::new(x, y, z)).unwrap())
    .collect();
    let f1 = mesh.add_face(&[v[0], v[1], v[2]]).unwrap();
    let f2 = mesh.add_face(&[v[1], v[0], v[3]]).unwrap();
    let f3 = mesh.add_face(&[v[0], v[1], v[4]]).unwrap();
    let start = mesh.clone();
    for faces in [vec![f1], vec![f1, f2], vec![f1, f2, f3]] {
        for grouping in [Grouping::Together, Grouping::Separate] {
            let mut m = start.clone();
            let outcome = extrude_polygons(&mut m, &faces, 5.0, grouping);
            assert_eq!(outcome.matched, faces.len());
            every_point_is_a_number(&m);
            let mut m = start.clone();
            inset_polygons(&mut m, &faces, 1.0, grouping);
            every_point_is_a_number(&m);
        }
    }
}

#[test]
fn the_same_edit_always_gives_the_same_ids() {
    let start = grid(3);
    let faces = middle(&start, 40.0);
    for grouping in [Grouping::Together, Grouping::Separate] {
        let mut a = start.clone();
        let mut b = start.clone();
        extrude_polygons(&mut a, &faces, 12.5, grouping);
        extrude_polygons(&mut b, &faces, 12.5, grouping);
        assert_eq!(a, b);
        inset_polygons(&mut a, &faces, 3.0, grouping);
        inset_polygons(&mut b, &faces, 3.0, grouping);
        assert_eq!(a, b);
    }
}

// ------------------------------------------------------------------ inset

#[test]
fn insetting_a_cube_face_leaves_a_ring() {
    let mut mesh = cube();
    let top = facing(&mesh, Vec3::Z);
    let (before_area, before_centre) = (area(&mesh, top), centre(&mesh, top));
    let outcome = inset_polygons(&mut mesh, &[top], 10.0, Grouping::Together);
    assert_eq!(outcome.matched, 1);
    assert_eq!(mesh.vertex_count(), 12);
    assert_eq!(mesh.face_count(), 10);
    // Same place and plane, 10 in from each side: 80 × 80 of 100 × 100.
    assert!(centre(&mesh, top).distance(before_centre) < 1e-3);
    assert!((before_area - 100.0 * 100.0).abs() < 1e-1);
    assert!((area(&mesh, top) - 80.0 * 80.0).abs() < 1e-1);
    for v in mesh.face(top).unwrap() {
        assert!((mesh.position(*v).unwrap().z - before_centre.z).abs() < 1e-3);
    }
    let r = report(&mesh);
    assert!(r.is_closed() && !r.has_problems(), "{r:?}");
}

#[test]
fn inset_then_extrude_makes_a_panel() {
    let mut mesh = cube();
    let top = facing(&mesh, Vec3::Z);
    inset_polygons(&mut mesh, &[top], 10.0, Grouping::Together);
    extrude_polygons(&mut mesh, &[top], -5.0, Grouping::Together);
    assert_eq!(mesh.face_count(), 14);
    let r = report(&mesh);
    assert!(r.is_closed() && !r.has_problems(), "{r:?}");
    // The panel's walls are crisp, because the inset copied the cube's
    // hard edges to the new border.
    let corners = mesh.face(top).unwrap().to_vec();
    for (a, b) in sides(&corners) {
        assert!(mesh.is_hard(a, b));
    }
}

#[test]
fn neighbours_inset_together_or_each_on_their_own() {
    let start = grid(2);
    let all: Vec<FaceId> = start.faces().map(|(f, _)| f).collect();

    // Together: only the outer border moves in; the middle point and the
    // inner edges stay. 8 border points, 8 ring polygons.
    let mut together = start.clone();
    inset_polygons(&mut together, &all, 10.0, Grouping::Together);
    assert_eq!(together.vertex_count(), 9 + 8);
    assert_eq!(together.face_count(), 4 + 8);
    let centre_point = together
        .vertices()
        .find(|(_, p)| p.x.abs() < 1e-3 && p.y.abs() < 1e-3)
        .is_some();
    assert!(centre_point, "the middle point stays");
    let r = report(&together);
    assert_eq!(r.non_manifold_edges, 0, "{r:?}");
    assert!(!r.has_problems(), "{r:?}");

    // Each on its own: every square gets its own ring.
    let mut each = start.clone();
    inset_polygons(&mut each, &all, 10.0, Grouping::Separate);
    assert_eq!(each.vertex_count(), 9 + 16);
    assert_eq!(each.face_count(), 4 + 16);
    for face in &all {
        assert!((area(&each, *face) - 30.0 * 30.0).abs() < 1e-1);
    }
    let r = report(&each);
    assert_eq!(r.non_manifold_edges, 0, "{r:?}");
}

#[test]
fn n_gons_and_triangles_inset() {
    let mut mesh = Primitive::Cylinder(Default::default()).mesh();
    let cap = facing(&mesh, Vec3::Z);
    let corners = mesh.face(cap).unwrap().len();
    let before = area(&mesh, cap);
    inset_polygons(&mut mesh, &[cap], 5.0, Grouping::Together);
    assert_eq!(mesh.face(cap).unwrap().len(), corners);
    assert!(area(&mesh, cap) < before);
    assert!(report(&mesh).is_closed());

    let mut mesh = Mesh::new();
    let v: Vec<VertexId> = [(0.0, 0.0), (30.0, 0.0), (0.0, 30.0)]
        .into_iter()
        .map(|(x, y)| mesh.add_vertex(Vec3::new(x, y, 0.0)).unwrap())
        .collect();
    let triangle = mesh.add_face(&v).unwrap();
    inset_polygons(&mut mesh, &[triangle], 2.0, Grouping::Together);
    assert_eq!(mesh.face_count(), 4);
    assert!(area(&mesh, triangle) < 450.0);
    every_point_is_a_number(&mesh);
}

// ------------------------------------------------------------ edge extrude

#[test]
fn a_border_edge_grows_flat_and_outward() {
    let mut mesh = grid(1);
    let edge = *mesh
        .edges()
        .iter()
        .find(|e| {
            let (a, b) = e.ends();
            let (pa, pb) = (mesh.position(a).unwrap(), mesh.position(b).unwrap());
            pa.x > 49.0 && pb.x > 49.0
        })
        .unwrap();
    let outcome = extrude_edges(&mut mesh, &[edge], 20.0, Grouping::Together);
    assert_eq!(outcome.matched, 1);
    assert_eq!(mesh.vertex_count(), 6);
    assert_eq!(mesh.face_count(), 2);
    let bounds = mesh.bounds().unwrap();
    assert!((bounds.max.x - 70.0).abs() < 1e-3, "grew along +X");
    assert!(bounds.size().z.abs() < 1e-3, "and stayed flat");
    // The new polygon faces the same way as the old one.
    let new = FaceId::from_raw(1);
    assert!(mesh.face_normal(new).dot(Vec3::Z) > 0.99);
    let r = report(&mesh);
    assert_eq!(r.flipped_edges, 0);
    assert!(!r.has_problems(), "{r:?}");
}

#[test]
fn joined_border_edges_grow_together_or_each_on_their_own() {
    let start = grid(1);
    let border: Vec<EdgeKey> = start.edges().to_vec();
    assert_eq!(border.len(), 4);
    let mut together = start.clone();
    extrude_edges(&mut together, &border, 10.0, Grouping::Together);
    assert_eq!(together.vertex_count(), 4 + 4);
    assert_eq!(together.face_count(), 1 + 4);
    // The corners move out diagonally, so the frame is 10 wide all round.
    let size = together.bounds().unwrap().size();
    assert!((size.x - 120.0).abs() < 1e-2, "{size}");
    assert_eq!(report(&together).non_manifold_edges, 0);

    let mut each = start.clone();
    extrude_edges(&mut each, &border, 10.0, Grouping::Separate);
    assert_eq!(each.vertex_count(), 4 + 8);
    assert_eq!(each.face_count(), 1 + 4);
}

#[test]
fn edges_without_an_open_side_are_skipped() {
    let mut mesh = cube();
    let edges: Vec<EdgeKey> = mesh.edges().iter().take(3).copied().collect();
    let outcome = extrude_edges(&mut mesh, &edges, 10.0, Grouping::Together);
    assert_eq!(outcome.matched, 0);
    assert_eq!(outcome.skipped, 3);
    assert_eq!(mesh, cube());
    let unknown = EdgeKey::new(VertexId::from_raw(0), VertexId::from_raw(900));
    let outcome = extrude_edges(&mut mesh, &[unknown], 10.0, Grouping::Together);
    assert_eq!(outcome.missing, 1);
}

#[test]
fn an_extruded_border_edge_can_grow_again() {
    let mut mesh = grid(1);
    let edge = mesh.edges()[0];
    extrude_edges(&mut mesh, &[edge], 10.0, Grouping::Together);
    // The new outer edge is the newest polygon's third and fourth corner.
    let newest = mesh.faces().last().unwrap().1.to_vec();
    let outer = EdgeKey::new(newest[2], newest[3]);
    let outcome = extrude_edges(&mut mesh, &[outer], 10.0, Grouping::Together);
    assert_eq!(outcome.matched, 1);
    assert_eq!(mesh.face_count(), 3);
    assert_eq!(report(&mesh).non_manifold_edges, 0);
}

#[test]
fn replacing_corners_keeps_the_id_and_refuses_bad_corners() {
    let mut mesh = cube();
    let top = facing(&mesh, Vec3::Z);
    let corners = mesh.face(top).unwrap().to_vec();
    let mut turned = corners.clone();
    turned.rotate_left(1);
    mesh.replace_corners(top, &turned).unwrap();
    assert_eq!(mesh.face(top).unwrap(), turned.as_slice());
    assert!(matches!(
        mesh.replace_corners(top, &corners[..3]),
        Err(crate::MeshError::CornerCountChanged(_))
    ));
    assert!(matches!(
        mesh.replace_corners(FaceId::from_raw(99), &corners),
        Err(crate::MeshError::UnknownFace(_))
    ));
    let twice = [corners[0], corners[0], corners[1], corners[2]];
    assert!(mesh.replace_corners(top, &twice).is_err());
    assert_eq!(mesh.face(top).unwrap(), turned.as_slice());
}

// ------------------------------------------------------------ bending ---

/// An upright cylinder of 16 sides, radius 50, 100 tall, with flat caps,
/// and its top cap.
fn pipe() -> (Mesh, FaceId) {
    let mesh = Primitive::Cylinder(crate::CylinderParams {
        sides: 16,
        height_segments: 1,
        caps: true,
        cap_segments: 1,
        cap_centre: false,
        ..Default::default()
    })
    .mesh();
    let top = facing(&mesh, Vec3::Z);
    (mesh, top)
}

fn bend(angle: f32, radius: f32, segments: u32) -> Bend {
    Bend {
        angle,
        direction: 0.0,
        radius,
        segments,
    }
}

/// The ring of points made by an extrude, by ring: `per` points each, in
/// id order after the first `before`.
fn rings(mesh: &Mesh, before: usize, per: usize) -> Vec<Vec<Vec3>> {
    let points: Vec<Vec3> = mesh.vertices().skip(before).map(|(_, p)| p).collect();
    points.chunks(per).map(<[Vec3]>::to_vec).collect()
}

/// How far each point is from the ring's middle.
fn spread(ring: &[Vec3]) -> (f32, f32) {
    let middle = ring.iter().copied().sum::<Vec3>() / ring.len() as f32;
    ring.iter()
        .map(|p| p.distance(middle))
        .fold((f32::MAX, f32::MIN), |(lo, hi), d| (lo.min(d), hi.max(d)))
}

#[test]
fn a_rounded_bend_keeps_the_pipe_full_size() {
    let (before, top) = pipe();
    let radius = spread(
        &before
            .face(top)
            .unwrap()
            .iter()
            .map(|v| before.position(*v).unwrap())
            .collect::<Vec<_>>(),
    )
    .0;
    let mut mesh = before.clone();
    let out = extrude_bent(
        &mut mesh,
        &[top],
        50.0,
        Grouping::Together,
        bend(90.0, 100.0, 8),
    );
    assert_eq!(out.matched, 1);
    // Eight rows round the turn and one straight on: nine rings of 16.
    assert_eq!(mesh.vertex_count(), before.vertex_count() + 9 * 16);
    assert_eq!(mesh.face_count(), before.face_count() + 9 * 16);
    let r = report(&mesh);
    assert!(r.is_closed() && !r.has_problems(), "{r:?}");
    every_point_is_a_number(&mesh);
    for ring in rings(&mesh, before.vertex_count(), 16) {
        let (lo, hi) = spread(&ring);
        assert!(
            (lo - radius).abs() < 1e-2 && (hi - radius).abs() < 1e-2,
            "{lo} {hi}"
        );
    }
    // The end faces +X, beyond the turn: up 100 (the radius) and across
    // 100 plus 50 straight on.
    assert!(mesh.face_normal(top).dot(Vec3::X) > 0.999);
    let end = centre(&mesh, top);
    assert!(end.distance(Vec3::new(150.0, 0.0, 200.0)) < 1e-2, "{end}");
}

#[test]
fn a_sharp_bend_is_a_mitre_joint_that_stays_round() {
    let (before, top) = pipe();
    let old: Vec<VertexId> = before.face(top).unwrap().to_vec();
    let mut mesh = before.clone();
    extrude_bent(
        &mut mesh,
        &[top],
        80.0,
        Grouping::Together,
        bend(90.0, 0.0, 8),
    );
    // One ring: the cap's copy, round the corner.
    assert_eq!(mesh.vertex_count(), before.vertex_count() + 16);
    let r = report(&mesh);
    assert!(r.is_closed() && !r.has_problems(), "{r:?}");
    // The old top ring now lies on the 45° plane through the corner, still
    // on the pipe's side: as far from the upright axis as before.
    for v in &old {
        let p = mesh.position(*v).unwrap();
        assert!((p.z - (100.0 - p.x)).abs() < 1e-2, "{p}");
        assert!((p.truncate().length() - 50.0).abs() < 0.5, "{p}");
    }
    // The end is a full circle facing +X, 80 along.
    let end = centre(&mesh, top);
    assert!(end.distance(Vec3::new(80.0, 0.0, 100.0)) < 1e-2, "{end}");
    let (lo, hi) = spread(&rings(&mesh, before.vertex_count(), 16)[0]);
    assert!(lo > 49.0 && hi < 51.0, "{lo} {hi}");
}

#[test]
fn the_direction_turns_the_bend_round_the_pipe() {
    let (before, top) = pipe();
    let mut mesh = before.clone();
    let mut b = bend(90.0, 100.0, 4);
    b.direction = 90.0;
    extrude_bent(&mut mesh, &[top], 0.0, Grouping::Together, b);
    // A quarter turn round from +X is +Y.
    assert!(mesh.face_normal(top).dot(Vec3::Y) > 0.999);
    // No straight part: just the four rows of the turn.
    assert_eq!(mesh.vertex_count(), before.vertex_count() + 4 * 16);
}

#[test]
fn no_angle_is_a_plain_extrude() {
    let (before, top) = pipe();
    let mut bent = before.clone();
    extrude_bent(
        &mut bent,
        &[top],
        40.0,
        Grouping::Together,
        bend(0.0, 30.0, 8),
    );
    let mut plain = before.clone();
    extrude_polygons(&mut plain, &[top], 40.0, Grouping::Together);
    assert_eq!(bent, plain);
}

#[test]
fn bends_keep_their_numbers_in_range() {
    let b = Bend {
        angle: 500.0,
        direction: -90.0,
        radius: -3.0,
        segments: 0,
    }
    .clamped();
    assert_eq!(
        (b.angle, b.direction, b.radius, b.segments),
        (MAX_MITRE_ANGLE, 270.0, 0.0, 1)
    );
    let b = Bend {
        angle: f32::NAN,
        direction: f32::INFINITY,
        radius: f32::NAN,
        segments: 9999,
    }
    .clamped();
    assert_eq!(
        (b.angle, b.direction, b.radius, b.segments),
        (0.0, 0.0, 0.0, MAX_BEND_SEGMENTS)
    );
    assert!(!b.bends());
    assert_eq!(bend(200.0, 10.0, 3).clamped().angle, 180.0);
    assert_eq!(bend(90.0, 10.0, 3).rings(0.0), 3);
    assert_eq!(bend(90.0, 10.0, 3).rings(5.0), 4);
    assert_eq!(bend(90.0, 0.0, 3).rings(0.0), 1);
}

#[test]
fn every_shape_bends_cleanly() {
    for primitive in Primitive::all() {
        let before = primitive.mesh();
        let faces = middle(&before, 1e9);
        let some: Vec<FaceId> = faces.iter().copied().take(3).collect();
        for grouping in [Grouping::Together, Grouping::Separate] {
            for b in [
                bend(90.0, 60.0, 4),
                bend(45.0, 0.0, 1),
                bend(180.0, 200.0, 6),
            ] {
                let mut mesh = before.clone();
                extrude_bent(&mut mesh, &some, 20.0, grouping, b);
                let r = report(&mesh);
                let what = format!("{} {grouping:?} {b:?}: {r:?}", primitive.name());
                assert_eq!(r.non_manifold_edges, 0, "{what}");
                assert_eq!(r.loose_points, 0, "{what}");
                assert_eq!(r.is_closed(), report(&before).is_closed(), "{what}");
                every_point_is_a_number(&mesh);
            }
        }
    }
}

#[test]
fn a_bend_direction_and_its_way_round_turn_into_each_other() {
    for facing in [
        Vec3::X,
        Vec3::Z,
        -Vec3::Z,
        Vec3::new(1.0, 1.0, 0.3).normalize(),
    ] {
        for direction in [0.0, 45.0, 90.0, 200.0, 359.0] {
            let towards = bend_towards(facing, direction).unwrap();
            assert!(towards.dot(facing).abs() < 1e-4);
            let back = bend_direction(facing, towards);
            let apart = (back - direction).rem_euclid(360.0);
            assert!(
                apart.min(360.0 - apart) < 1e-2,
                "{facing} {direction} → {back}"
            );
        }
    }
}
