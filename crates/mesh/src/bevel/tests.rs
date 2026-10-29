use super::*;
use crate::{CylinderParams, MeshReport, PlaneParams, Primitive};

fn cube() -> Mesh {
    Primitive::Cube(Default::default()).mesh()
}

fn report(mesh: &Mesh) -> MeshReport {
    MeshReport::new(mesh)
}

fn every_point_is_a_number(mesh: &Mesh) {
    for (v, p) in mesh.vertices() {
        assert!(p.is_finite(), "point {v:?} is {p}");
    }
}

/// A bevel as every bevel was before decision 0085 (none of the options),
/// which the earlier tests check.
fn shape(width: f32, segments: u32, profile: f32) -> BevelShape {
    BevelShape {
        width,
        segments,
        profile,
        options: BevelOptions::EARLIER,
    }
}

/// The cube's edge along its top front, at y = -50, z = 100.
fn top_front(mesh: &Mesh) -> EdgeKey {
    *mesh
        .edges()
        .iter()
        .find(|e| {
            let (a, b) = e.ends();
            let (pa, pb) = (mesh.position(a).unwrap(), mesh.position(b).unwrap());
            pa.y < -49.0 && pb.y < -49.0 && pa.z > 99.0 && pb.z > 99.0
        })
        .unwrap()
}

/// The edges around a cube's top polygon.
fn top_ring(mesh: &Mesh) -> Vec<EdgeKey> {
    mesh.edges()
        .iter()
        .filter(|e| {
            let (a, b) = e.ends();
            mesh.position(a).unwrap().z > 99.0 && mesh.position(b).unwrap().z > 99.0
        })
        .copied()
        .collect()
}

fn closed_and_clean(mesh: &Mesh) {
    let r = report(mesh);
    assert!(r.is_closed(), "{r:?}");
    assert!(!r.has_problems(), "{r:?}");
    assert_eq!(r.pieces, 1, "{r:?}");
    every_point_is_a_number(mesh);
}

/// How many polygons have `n` corners.
fn with_corners(mesh: &Mesh, n: usize) -> usize {
    mesh.faces().filter(|(_, c)| c.len() == n).count()
}

// ------------------------------------------------------------ edges

#[test]
fn one_cube_edge_becomes_a_chamfer() {
    let mut mesh = cube();
    let edge = top_front(&mesh);
    let outcome = bevel_edges(&mut mesh, &[edge], shape(10.0, 1, 1.0));
    assert_eq!(
        outcome,
        Outcome {
            matched: 1,
            missing: 0,
            skipped: 0
        }
    );
    // The edge's two points become four; one new polygon; the two end
    // polygons gain a corner.
    assert_eq!(mesh.vertex_count(), 10);
    assert_eq!(mesh.face_count(), 7);
    assert_eq!(with_corners(&mesh, 5), 2);
    closed_and_clean(&mesh);
    // The chamfer runs 10 in from the edge on both sides.
    let chamfer = FaceId::from_raw(6);
    for v in mesh.face(chamfer).unwrap() {
        let p = mesh.position(*v).unwrap();
        let on_top = (p.z - 100.0).abs() < 1e-3 && (p.y + 40.0).abs() < 1e-3;
        let on_front = (p.y + 50.0).abs() < 1e-3 && (p.z - 90.0).abs() < 1e-3;
        assert!(on_top || on_front, "{p}");
    }
    // Facing out at 45° between top and front.
    let n = mesh.face_normal(chamfer);
    assert!(
        n.distance(Vec3::new(0.0, -1.0, 1.0).normalize()) < 1e-3,
        "{n}"
    );
}

#[test]
fn more_segments_round_the_edge() {
    let mut round = cube();
    let edge = top_front(&round);
    bevel_edges(&mut round, &[edge], shape(10.0, 4, 1.0));
    assert_eq!(round.face_count(), 6 + 4);
    assert_eq!(round.vertex_count(), 8 - 2 + 2 * 5);
    closed_and_clean(&round);
    // The end polygons take the round end: 4 corners + 5 along the curve.
    assert_eq!(with_corners(&round, 4 + 4), 2);
    // A round profile bulges out past a flat one.
    let mut flat = cube();
    bevel_edges(&mut flat, &[edge], shape(10.0, 4, 0.0));
    let reach = |m: &Mesh| {
        m.vertices()
            .map(|(_, p)| Vec3::new(0.0, -1.0, 1.0).normalize().dot(p))
            .fold(f32::MIN, f32::max)
    };
    assert!(reach(&round) > reach(&flat) + 1.0);
    // A hollow profile curves the other way, inward past the flat one.
    let mut hollow = cube();
    bevel_edges(&mut hollow, &[edge], shape(10.0, 4, -1.0));
    closed_and_clean(&hollow);
    let inner = |m: &Mesh| {
        // The deepest point across the strip, at the edge's ends: on the
        // flat chamfer every point lies on the straight line.
        m.vertices()
            .filter(|(_, p)| p.x.abs() > 49.0)
            .map(|(_, p)| Vec3::new(0.0, -1.0, 1.0).normalize().dot(p))
            .filter(|d| *d > 80.0)
            .fold(f32::MAX, f32::min)
    };
    assert!(inner(&hollow) < inner(&flat) - 1.0);
}

#[test]
fn a_cubes_corner_where_three_edges_meet_gets_a_corner_polygon() {
    let mut mesh = cube();
    let all: Vec<EdgeKey> = mesh.edges().to_vec();
    assert_eq!(all.len(), 12);
    bevel_edges(&mut mesh, &all, shape(10.0, 1, 1.0));
    // 6 faces, 12 strips, 8 corner triangles; 3 points per corner.
    assert_eq!(mesh.face_count(), 26);
    assert_eq!(mesh.vertex_count(), 24);
    assert_eq!(with_corners(&mesh, 3), 8);
    closed_and_clean(&mesh);
    // Every edge stays crisp, like the cube's.
    for e in mesh.edges() {
        let (a, b) = e.ends();
        assert!(mesh.is_hard(a, b));
    }
    // Rounded too.
    let mut round = cube();
    bevel_edges(&mut round, &all, shape(10.0, 3, 1.0));
    assert_eq!(round.face_count(), 6 + 12 * 3 + 8);
    closed_and_clean(&round);
}

#[test]
fn a_ring_of_edges_bevels_together() {
    let mut mesh = cube();
    let ring = top_ring(&mesh);
    assert_eq!(ring.len(), 4);
    bevel_edges(&mut mesh, &ring, shape(10.0, 2, 1.0));
    // Each top corner: the top polygon's corner meets both offsets, and
    // the side edge slides down.
    // Where two strips meet at a corner they share their round ends, so
    // no corner polygons are needed.
    assert_eq!(mesh.face_count(), 6 + 4 * 2);
    closed_and_clean(&mesh);
    let top = mesh
        .faces()
        .map(|(f, _)| f)
        .find(|f| mesh.face_normal(*f).z > 0.999)
        .unwrap();
    for v in mesh.face(top).unwrap() {
        let p = mesh.position(*v).unwrap();
        assert!((p.x.abs() - 40.0).abs() < 1e-3 && (p.y.abs() - 40.0).abs() < 1e-3);
    }
}

#[test]
fn a_cylinders_rim_chamfers_round() {
    let mut mesh = Primitive::Cylinder(CylinderParams::default()).mesh();
    let rim: Vec<EdgeKey> = mesh
        .edges()
        .iter()
        .filter(|e| {
            let (a, b) = e.ends();
            mesh.position(a).unwrap().z > 99.0 && mesh.position(b).unwrap().z > 99.0
        })
        .copied()
        .collect();
    let (points, faces) = (mesh.vertex_count(), mesh.face_count());
    bevel_edges(&mut mesh, &rim, shape(5.0, 1, 1.0));
    assert_eq!(mesh.face_count(), faces + rim.len());
    assert_eq!(mesh.vertex_count(), points + rim.len());
    closed_and_clean(&mesh);
}

#[test]
fn an_edge_in_the_middle_of_a_grid_gets_corner_polygons_at_its_ends() {
    let mut mesh = Primitive::Plane(PlaneParams {
        segments: [4, 4],
        ..Default::default()
    })
    .mesh();
    // An edge between two inner points (four edges meet at each).
    let edge = *mesh
        .edges()
        .iter()
        .find(|e| {
            let (a, b) = e.ends();
            let (pa, pb) = (mesh.position(a).unwrap(), mesh.position(b).unwrap());
            pa.x.abs() < 1e-3 && pb.x.abs() < 1e-3 && pa.y.abs() < 30.0 && pb.y.abs() < 30.0
        })
        .unwrap();
    let before = report(&mesh);
    bevel_edges(&mut mesh, &[edge], shape(5.0, 2, 1.0));
    let r = report(&mesh);
    assert_eq!(r.border_edges, before.border_edges, "no holes: {r:?}");
    assert_eq!(r.non_manifold_edges, 0);
    assert_eq!(r.flipped_edges, 0);
    every_point_is_a_number(&mesh);
}

#[test]
fn an_edge_reaching_the_border_bevels_without_holes() {
    let mesh = Primitive::Plane(PlaneParams {
        segments: [2, 1],
        ..Default::default()
    })
    .mesh();
    let middle = *mesh
        .edges()
        .iter()
        .find(|e| {
            let (a, b) = e.ends();
            mesh.position(a).unwrap().x.abs() < 1e-3 && mesh.position(b).unwrap().x.abs() < 1e-3
        })
        .unwrap();
    let before = report(&mesh);
    for segments in [1, 3] {
        let mut m = mesh.clone();
        let outcome = bevel_edges(&mut m, &[middle], shape(5.0, segments, 1.0));
        assert_eq!(outcome.matched, 1);
        let r = report(&m);
        assert_eq!(r.non_manifold_edges, 0, "{r:?}");
        assert_eq!(r.flipped_edges, 0, "{r:?}");
        assert_eq!(r.pieces, 1);
        assert_eq!(m.face_count(), before.faces + segments as usize);
        every_point_is_a_number(&m);
    }
    // Border edges have nothing to bevel between: skipped.
    let border = mesh
        .edges()
        .iter()
        .copied()
        .find(|e| mesh.topology().is_border(*e))
        .unwrap();
    let mut m = mesh.clone();
    let outcome = bevel_edges(&mut m, &[border], shape(5.0, 1, 1.0));
    assert_eq!(outcome.skipped, 1);
    assert_eq!(m, mesh);
}

#[test]
fn polygon_ids_stay_and_the_same_bevel_gives_the_same_ids() {
    let start = cube();
    let edge = top_front(&start);
    let ids: Vec<FaceId> = start.faces().map(|(f, _)| f).collect();
    let mut a = start.clone();
    let mut b = start.clone();
    bevel_edges(&mut a, &[edge], shape(12.5, 3, 0.7));
    bevel_edges(&mut b, &[edge], shape(12.5, 3, 0.7));
    assert_eq!(a, b);
    for id in ids {
        assert!(a.face(id).is_some(), "{id:?} kept");
    }
}

#[test]
fn missing_repeated_and_broken_input() {
    let mut mesh = cube();
    let edge = top_front(&mesh);
    let unknown = EdgeKey::new(VertexId::from_raw(0), VertexId::from_raw(99));
    let outcome = bevel_edges(&mut mesh, &[edge, edge, unknown], shape(5.0, 1, 1.0));
    assert_eq!(outcome.matched, 1);
    assert_eq!(outcome.missing, 1);
    assert_eq!(mesh.face_count(), 7);
    for bad in [f32::NAN, f32::INFINITY] {
        let mut m = cube();
        bevel_edges(&mut m, &[edge], shape(bad, 1, 1.0));
        assert_eq!(m, cube());
        bevel_edges(&mut m, &[edge], shape(5.0, 1, bad));
        assert_eq!(m, cube());
        bevel_points(&mut m, &[VertexId::from_raw(0)], bad);
        assert_eq!(m, cube());
    }
    let mut m = cube();
    assert_eq!(
        bevel_edges(&mut m, &[], shape(5.0, 1, 1.0)),
        Outcome::default()
    );
    assert_eq!(m, cube());
    // Silly settings are brought within limits.
    let s = shape(-4.0, 0, 7.0).clamped().unwrap();
    assert_eq!((s.width, s.segments, s.profile), (0.0, 1, 1.0));
    assert_eq!(shape(1.0, 1, -9.0).clamped().unwrap().profile, -1.0);
    assert_eq!(
        shape(1.0, 1000, 0.5).clamped().unwrap().segments,
        MAX_SEGMENTS
    );
}

#[test]
fn zero_and_huge_widths_never_make_broken_points() {
    for width in [0.0, 1e-6, 49.0, 80.0, 1e6] {
        let mut mesh = cube();
        let all: Vec<EdgeKey> = mesh.edges().to_vec();
        bevel_edges(&mut mesh, &all, shape(width, 2, 1.0));
        every_point_is_a_number(&mesh);
        let mut mesh = cube();
        bevel_points(&mut mesh, &[VertexId::from_raw(0)], width);
        every_point_is_a_number(&mesh);
    }
}

#[test]
fn non_manifold_edges_are_skipped() {
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
    mesh.add_face(&[v[0], v[1], v[2]]).unwrap();
    mesh.add_face(&[v[1], v[0], v[3]]).unwrap();
    mesh.add_face(&[v[0], v[1], v[4]]).unwrap();
    let before = mesh.clone();
    let outcome = bevel_edges(&mut mesh, &[EdgeKey::new(v[0], v[1])], shape(1.0, 1, 1.0));
    assert_eq!(outcome.skipped, 1);
    assert_eq!(mesh, before);
    // Its points don't form one fan either.
    let outcome = bevel_points(&mut mesh, &[v[0]], 1.0);
    assert_eq!(outcome.skipped, 1);
    assert_eq!(mesh, before);
}

#[test]
fn n_gons_and_triangles_bevel() {
    // A triangular prism: two triangles and three quads.
    let mut mesh = Mesh::new();
    let p = |m: &mut Mesh, x: f32, y: f32, z: f32| m.add_vertex(Vec3::new(x, y, z)).unwrap();
    let a = p(&mut mesh, 0.0, 0.0, 0.0);
    let b = p(&mut mesh, 40.0, 0.0, 0.0);
    let c = p(&mut mesh, 20.0, 30.0, 0.0);
    let d = p(&mut mesh, 0.0, 0.0, 50.0);
    let e = p(&mut mesh, 40.0, 0.0, 50.0);
    let f = p(&mut mesh, 20.0, 30.0, 50.0);
    mesh.add_face(&[a, c, b]).unwrap();
    mesh.add_face(&[d, e, f]).unwrap();
    mesh.add_face(&[a, b, e, d]).unwrap();
    mesh.add_face(&[b, c, f, e]).unwrap();
    mesh.add_face(&[c, a, d, f]).unwrap();
    assert!(report(&mesh).is_closed());
    let all: Vec<EdgeKey> = mesh.edges().to_vec();
    for segments in [1, 2] {
        let mut m = mesh.clone();
        bevel_edges(&mut m, &all, shape(3.0, segments, 1.0));
        closed_and_clean(&m);
    }
}

// ------------------------------------------------------------ points

#[test]
fn a_cube_corner_is_cut_off() {
    let mut mesh = cube();
    let corner = mesh
        .vertices()
        .find(|(_, p)| p.x > 49.0 && p.y > 49.0 && p.z > 99.0)
        .unwrap()
        .0;
    let outcome = bevel_points(&mut mesh, &[corner], 10.0);
    assert_eq!(outcome.matched, 1);
    assert_eq!(mesh.vertex_count(), 8 - 1 + 3);
    assert_eq!(mesh.face_count(), 7);
    assert!(mesh.position(corner).is_none(), "the corner is gone");
    closed_and_clean(&mesh);
    let cut = FaceId::from_raw(6);
    let n = mesh.face_normal(cut);
    assert!(n.distance(Vec3::ONE.normalize()) < 1e-3, "{n}");
}

#[test]
fn points_on_a_border_and_in_a_grid() {
    let mesh = Primitive::Plane(PlaneParams {
        segments: [2, 2],
        ..Default::default()
    })
    .mesh();
    let before = report(&mesh);
    for (_, p) in mesh.vertices().collect::<Vec<_>>() {
        let mut m = mesh.clone();
        let id = m.vertices().find(|(_, q)| *q == p).unwrap().0;
        bevel_points(&mut m, &[id], 5.0);
        let r = report(&m);
        assert_eq!(r.non_manifold_edges, 0, "{p}: {r:?}");
        assert_eq!(r.flipped_edges, 0, "{p}: {r:?}");
        assert_eq!(r.pieces, before.pieces);
        every_point_is_a_number(&m);
    }
}

#[test]
fn neighbouring_points_bevel_together() {
    let mut mesh = cube();
    let top: Vec<VertexId> = mesh
        .vertices()
        .filter(|(_, p)| p.z > 99.0)
        .map(|(v, _)| v)
        .collect();
    bevel_points(&mut mesh, &top, 10.0);
    assert_eq!(mesh.face_count(), 6 + 4);
    assert_eq!(mesh.vertex_count(), 4 + 4 * 3);
    closed_and_clean(&mesh);
}

#[test]
fn the_topology_is_fresh_after_adding_polygons() {
    // A regression test: a topology worked out mid-edit must not be kept
    // once more polygons are added.
    let mut mesh = cube();
    let before = mesh.topology().edge_count();
    let mut m = Mesh::new();
    std::mem::swap(&mut m, &mut mesh);
    let a = m.add_vertex(Vec3::new(0.0, 0.0, 500.0)).unwrap();
    let b = m.add_vertex(Vec3::new(10.0, 0.0, 500.0)).unwrap();
    let c = m.add_vertex(Vec3::new(0.0, 10.0, 500.0)).unwrap();
    m.add_face(&[a, b, c]).unwrap();
    assert_eq!(m.topology().edge_count(), before + 3);
}

#[test]
fn back_to_back_polygons_never_make_a_hard_edge_from_a_point_to_itself() {
    // A regression test from the whole-app review (L1): this bevel used
    // to record hard edges like (3, 3), which made the project impossible
    // to open after Make editable.
    let mut mesh = Mesh::new();
    let v: Vec<VertexId> = [(0.0, 0.0), (10.0, 0.0), (0.0, 10.0)]
        .into_iter()
        .map(|(x, y)| mesh.add_vertex(Vec3::new(x, y, 0.0)).unwrap())
        .collect();
    mesh.add_face(&[v[0], v[1], v[2]]).unwrap();
    mesh.add_face(&[v[2], v[1], v[0]]).unwrap();
    mesh.set_hard(v[0], v[1], true);
    bevel_edges(&mut mesh, &[EdgeKey::new(v[0], v[1])], shape(5.0, 1, 1.0));
    for edge in mesh.hard_edges() {
        let (a, b) = edge.ends();
        assert_ne!(a, b);
    }
    // Whatever came out can be rebuilt from its slots, as opening a file
    // does.
    let slots: Vec<Option<Vec3>> = mesh.point_slots().collect();
    let faces: Vec<Option<Vec<VertexId>>> = mesh
        .face_slots()
        .map(|f| f.map(<[VertexId]>::to_vec))
        .collect();
    let rebuilt = Mesh::from_slots(slots, faces, mesh.hard_edges()).unwrap();
    assert_eq!(rebuilt.hard_edge_count(), mesh.hard_edge_count());
}

// ------------------------------------------- more control (decision 0085)

fn with_options(width: f32, options: BevelOptions) -> BevelShape {
    BevelShape {
        width,
        segments: 1,
        profile: 0.0,
        options,
    }
}

/// The bevel's new polygons (those not in `before`), and how wide each is
/// across: from the middle of one long side to the middle of the other.
fn strip_widths(before: &Mesh, after: &Mesh) -> Vec<f32> {
    after
        .faces()
        .filter(|(f, _)| before.face(*f).is_none())
        .filter(|(_, c)| c.len() == 4)
        .map(|(_, c)| {
            let p: Vec<Vec3> = c.iter().map(|v| after.position(*v).unwrap()).collect();
            ((p[0] + p[1]) * 0.5).distance((p[2] + p[3]) * 0.5)
        })
        .collect()
}

/// An 8-sided cylinder: its rim edges are right angles, its upright edges
/// much flatter (135° between their polygons).
fn octagon() -> Mesh {
    Primitive::Cylinder(CylinderParams {
        sides: 8,
        ..Default::default()
    })
    .mesh()
}

fn rim_edge(mesh: &Mesh) -> EdgeKey {
    *mesh
        .edges()
        .iter()
        .find(|e| {
            let (a, b) = e.ends();
            mesh.position(a).unwrap().z > 99.0 && mesh.position(b).unwrap().z > 99.0
        })
        .unwrap()
}

fn upright_edge(mesh: &Mesh) -> EdgeKey {
    *mesh
        .edges()
        .iter()
        .find(|e| {
            let (a, b) = e.ends();
            (mesh.position(a).unwrap().z - mesh.position(b).unwrap().z).abs() > 99.0
        })
        .unwrap()
}

#[test]
fn even_width_makes_flat_and_square_edges_the_same_width() {
    let only_even = BevelOptions {
        even: true,
        ..BevelOptions::EARLIER
    };
    let width_of = |options: BevelOptions, pick: fn(&Mesh) -> EdgeKey| {
        let before = octagon();
        let mut mesh = before.clone();
        bevel_edges(&mut mesh, &[pick(&before)], with_options(10.0, options));
        closed_and_clean(&mesh);
        let widths = strip_widths(&before, &mesh);
        assert_eq!(widths.len(), 1, "{widths:?}");
        widths[0]
    };
    // As before: the flatter edge's chamfer comes out much wider.
    let (rim, upright) = (
        width_of(BevelOptions::EARLIER, rim_edge),
        width_of(BevelOptions::EARLIER, upright_edge),
    );
    assert!(upright > rim * 1.2, "{rim} {upright}");
    // Even: the same, a right angle's width (10 √2).
    let (rim, upright) = (
        width_of(only_even, rim_edge),
        width_of(only_even, upright_edge),
    );
    assert!((rim - upright).abs() < 0.05, "{rim} {upright}");
    assert!(
        (rim - 10.0 * std::f32::consts::SQRT_2).abs() < 0.05,
        "{rim}"
    );
}

#[test]
fn skew_puts_more_on_one_side_and_flip_side_swaps_it() {
    let reach = |options: BevelOptions| {
        let mut mesh = cube();
        let edge = top_front(&mesh);
        bevel_edges(&mut mesh, &[edge], with_options(10.0, options));
        closed_and_clean(&mesh);
        // How far the bevel reaches down the front and back across the top.
        let down = mesh
            .vertices()
            .filter(|(_, p)| (p.y + 50.0).abs() < 1e-3 && p.z < 99.9 && p.z > 1.0)
            .map(|(_, p)| 100.0 - p.z)
            .fold(0.0_f32, f32::max);
        let back = mesh
            .vertices()
            .filter(|(_, p)| (p.z - 100.0).abs() < 1e-3 && p.y > -49.9 && p.y < 49.0)
            .map(|(_, p)| p.y + 50.0)
            .fold(0.0_f32, f32::max);
        (down, back)
    };
    let even = reach(BevelOptions::EARLIER);
    assert!(
        (even.0 - 10.0).abs() < 1e-3 && (even.1 - 10.0).abs() < 1e-3,
        "{even:?}"
    );
    let skewed = BevelOptions {
        skew: 0.5,
        ..BevelOptions::EARLIER
    };
    let (a, b) = reach(skewed);
    let lean = 0.5 * SKEW_REACH;
    let mut got = [a, b];
    got.sort_by(f32::total_cmp);
    assert!((got[0] - 10.0 * (1.0 - lean)).abs() < 1e-3, "{a} {b}");
    assert!((got[1] - 10.0 * (1.0 + lean)).abs() < 1e-3, "{a} {b}");
    let flipped = reach(BevelOptions {
        flip_side: true,
        ..skewed
    });
    assert!((flipped.0 - b).abs() < 1e-3 && (flipped.1 - a).abs() < 1e-3);
    // Full skew either way never takes a side to nothing.
    for skew in [-1.0, 1.0] {
        let (a, b) = reach(BevelOptions {
            skew,
            ..BevelOptions::EARLIER
        });
        assert!(a > 0.5 && b > 0.5, "{a} {b}");
    }
}

#[test]
fn skew_stays_on_the_same_side_all_the_way_round_a_loop() {
    let mut mesh = cube();
    let ring = top_ring(&mesh);
    let options = BevelOptions {
        skew: 0.6,
        ..BevelOptions::EARLIER
    };
    bevel_edges(&mut mesh, &ring, with_options(10.0, options));
    closed_and_clean(&mesh);
    // The top polygon is an even square inside, the same way in from all
    // four sides.
    let top: Vec<Vec3> = mesh
        .vertices()
        .filter(|(_, p)| (p.z - 100.0).abs() < 1e-3)
        .map(|(_, p)| p)
        .collect();
    assert_eq!(top.len(), 4);
    let inset = 50.0 - top[0].x.abs();
    for p in &top {
        assert!((50.0 - p.x.abs() - inset).abs() < 1e-3, "{p}");
        assert!((50.0 - p.y.abs() - inset).abs() < 1e-3, "{p}");
    }
    let lean = 0.6 * SKEW_REACH;
    assert!(
        (inset - 10.0 * (1.0 + lean)).abs() < 1e-3 || (inset - 10.0 * (1.0 - lean)).abs() < 1e-3,
        "{inset}"
    );
}

#[test]
fn stopping_at_overlaps_keeps_a_wide_bevel_from_crossing_over() {
    // The top's four edges bevelled 60 wide on a 100 cube: the four
    // corners would cross past the middle.
    let before = cube();
    let ring = top_ring(&before);
    let crossed = {
        let mut mesh = before.clone();
        bevel_edges(&mut mesh, &ring, with_options(60.0, BevelOptions::EARLIER));
        mesh
    };
    // Before: the top polygon turned inside out (facing down).
    let top_facing = |m: &Mesh| {
        m.faces()
            .filter(|(f, _)| before.face(*f).is_some())
            .map(|(f, _)| m.face_normal(f))
            .find(|n| n.z.abs() > 0.9)
    };
    assert!(
        top_facing(&crossed).is_some_and(|n| n.z < 0.0),
        "crossed over"
    );
    // Stopping at overlaps, with welding: the corners meet in the middle
    // and become one point, the top a neat point where four slopes meet.
    let mut mesh = before.clone();
    let options = BevelOptions {
        clamp: true,
        weld: true,
        ..BevelOptions::EARLIER
    };
    bevel_edges(&mut mesh, &ring, with_options(60.0, options));
    closed_and_clean(&mesh);
    for (_, p) in mesh.vertices() {
        assert!(p.x.abs() <= 50.0 + 1e-3 && p.y.abs() <= 50.0 + 1e-3, "{p}");
    }
    let apex: Vec<Vec3> = mesh
        .vertices()
        .filter(|(_, p)| (p.z - 100.0).abs() < 1e-3)
        .map(|(_, p)| p)
        .collect();
    assert_eq!(apex.len(), 1, "{apex:?}");
    assert!(apex[0].truncate().length() < 1e-3);
    // Stopping without welding leaves the meeting points on top of each
    // other.
    let mut unwelded = before.clone();
    bevel_edges(
        &mut unwelded,
        &ring,
        with_options(
            60.0,
            BevelOptions {
                clamp: true,
                ..BevelOptions::EARLIER
            },
        ),
    );
    assert!(report(&unwelded).zero_length_edges > 0);
    // A bevel that fits is not narrowed.
    let mut small = before.clone();
    bevel_edges(
        &mut small,
        &ring,
        with_options(
            10.0,
            BevelOptions {
                clamp: true,
                ..BevelOptions::EARLIER
            },
        ),
    );
    let mut plain = before.clone();
    bevel_edges(&mut plain, &ring, with_options(10.0, BevelOptions::EARLIER));
    assert_eq!(small, plain);
}

#[test]
fn the_new_standard_bevel_is_clean_where_strips_meet() {
    // Every edge of a cube, wide: with the standard options the bevel
    // fits, welds where it meets and leaves a clean shape.
    let before = cube();
    let all: Vec<EdgeKey> = before.edges().to_vec();
    for width in [10.0, 49.0, 80.0] {
        let mut mesh = before.clone();
        let shape = BevelShape {
            width,
            ..BevelShape::default()
        };
        bevel_edges(&mut mesh, &all, shape);
        closed_and_clean(&mesh);
        for (_, p) in mesh.vertices() {
            assert!(p.x.abs() <= 50.0 + 1e-3 && p.y.abs() <= 50.0 + 1e-3, "{p}");
            assert!(p.z >= -1e-3 && p.z <= 100.0 + 1e-3, "{p}");
        }
    }
}

#[test]
fn options_are_kept_in_range() {
    let wild = BevelOptions {
        skew: 7.0,
        weld_distance: -3.0,
        ..BevelOptions::default()
    }
    .clamped();
    assert_eq!((wild.skew, wild.weld_distance), (1.0, 0.0));
    let broken = BevelOptions {
        skew: f32::NAN,
        weld_distance: f32::INFINITY,
        ..BevelOptions::default()
    }
    .clamped();
    assert_eq!(broken.skew, 0.0);
    assert_eq!(broken.weld_distance, WELD_DISTANCE);
    let big = BevelOptions {
        weld_distance: 1e9,
        ..BevelOptions::default()
    }
    .clamped();
    assert_eq!(big.weld_distance, MAX_WELD_DISTANCE);
    // New bevels get the options; the earlier ones none.
    let standard = BevelOptions::default();
    assert!(standard.even && standard.clamp && standard.weld);
    assert_ne!(standard, BevelOptions::EARLIER);
}

#[test]
fn skewed_and_even_bevels_on_every_primitive_stay_clean() {
    for kind in crate::PrimitiveKind::ALL {
        let before = kind.standard().mesh();
        let edges: Vec<EdgeKey> = before.edges().iter().copied().take(12).collect();
        for skew in [-0.8, 0.0, 0.7] {
            let mut mesh = before.clone();
            let shape = BevelShape {
                width: 3.0,
                options: BevelOptions {
                    skew,
                    ..BevelOptions::default()
                },
                ..BevelShape::default()
            };
            bevel_edges(&mut mesh, &edges, shape);
            every_point_is_a_number(&mesh);
            let r = report(&mesh);
            assert_eq!(r.non_manifold_edges, 0, "{kind:?} {skew} {r:?}");
        }
    }
}

/// How far apart a polygon's texture corners lie (the widest of its
/// texture bounds' sides).
fn uv_extent(mesh: &Mesh, face: crate::FaceId) -> f32 {
    let n = mesh.face(face).unwrap().len();
    let uvs: Vec<_> = (0..n)
        .filter_map(|k| crate::uvs::corner_uv(mesh, face, k))
        .collect();
    let lo = uvs
        .iter()
        .fold(glam::Vec2::splat(f32::MAX), |a, b| a.min(*b));
    let hi = uvs
        .iter()
        .fold(glam::Vec2::splat(f32::MIN), |a, b| a.max(*b));
    (hi - lo).max_element()
}

#[test]
fn a_strip_across_a_seam_stays_in_one_piece_of_the_texture() {
    // Regression (second review): the cube's seams split its sides into
    // pieces far apart in the texture; strips over those edges blended the
    // two sides' corners and stretched across the whole texture.
    let mesh = cube();
    assert!(!crate::uvs::seams(&mesh).is_empty());
    let widest = mesh
        .faces()
        .map(|(f, _)| uv_extent(&mesh, f))
        .fold(0.0, f32::max);
    for segments in [1, 3] {
        let mut out = mesh.clone();
        let edges = out.edges().to_vec();
        bevel_edges(&mut out, &edges, shape(10.0, segments, 0.5));
        for (f, _) in out.faces() {
            let e = uv_extent(&out, f);
            assert!(
                e <= widest * 1.001,
                "{segments} rows: a polygon spans {e} of the texture (sides span {widest})"
            );
        }
    }
}
