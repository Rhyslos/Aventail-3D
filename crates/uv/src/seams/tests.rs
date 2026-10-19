//! Tests for seams: disks, pieces and suggested seams (decision 0098).

use super::*;
use mesh::{CubeParams, CylinderParams, PlaneParams, Primitive, SphereParams, TubeParams};

fn all() -> Vec<Primitive> {
    vec![
        Primitive::Cube(CubeParams::default()),
        Primitive::Cube(CubeParams {
            segments: [3, 2, 4],
            ..Default::default()
        }),
        Primitive::Plane(PlaneParams::default()),
        Primitive::Disc(Default::default()),
        Primitive::Cylinder(CylinderParams::default()),
        Primitive::Cone(Default::default()),
        Primitive::Tube(TubeParams::default()),
        Primitive::Sphere(SphereParams::default()),
    ]
}

fn hard(mesh: &Mesh) -> Vec<EdgeKey> {
    mesh.hard_edges().collect()
}

fn every_piece_is_a_disk(mesh: &Mesh, cuts: &BTreeSet<EdgeKey>) -> bool {
    pieces(mesh, cuts).iter().all(|p| is_disk(mesh, p, cuts))
}

#[test]
fn a_polygon_and_a_plane_are_disks_a_closed_or_tubular_piece_is_not() {
    let plane = Primitive::Plane(PlaneParams {
        segments: [3, 2],
        ..Default::default()
    })
    .mesh();
    let none = BTreeSet::new();
    let all_faces: Vec<FaceId> = plane.faces().map(|(f, _)| f).collect();
    assert!(is_disk(&plane, &all_faces, &none));
    assert!(is_disk(&plane, &all_faces[..1], &none));
    // A whole sphere has no border; a cylinder's side has two.
    let sphere = Primitive::Sphere(SphereParams::default()).mesh();
    let faces: Vec<FaceId> = sphere.faces().map(|(f, _)| f).collect();
    assert!(!is_disk(&sphere, &faces, &none));
    let cylinder = Primitive::Cylinder(CylinderParams::default()).mesh();
    let cuts: BTreeSet<EdgeKey> = hard(&cylinder).into_iter().collect();
    let side = pieces(&cylinder, &cuts)
        .into_iter()
        .max_by_key(Vec::len)
        .unwrap();
    assert!(!is_disk(&cylinder, &side, &cuts), "a tube needs a cut");
    assert!(!is_disk(&cylinder, &[], &cuts));
}

#[test]
fn a_cube_is_cut_along_its_twelve_edges() {
    let cube = Primitive::Cube(CubeParams::default()).mesh();
    // With or without its hard edges: its corners are sharp anyway.
    for given in [hard(&cube), Vec::new()] {
        let seams = suggest(&cube, &given);
        assert_eq!(seams.len(), 12);
        let cuts: BTreeSet<EdgeKey> = seams.into_iter().collect();
        assert_eq!(pieces(&cube, &cuts).len(), 6);
        assert!(every_piece_is_a_disk(&cube, &cuts));
    }
}

#[test]
fn a_cylinders_side_is_opened_with_one_line() {
    let params = CylinderParams::default();
    let cylinder = Primitive::Cylinder(params).mesh();
    let given = hard(&cylinder);
    let seams = suggest(&cylinder, &given);
    let cuts: BTreeSet<EdgeKey> = seams.iter().copied().collect();
    assert!(every_piece_is_a_disk(&cylinder, &cuts));
    // The rims, plus one line up the side (one edge per row).
    let extra: Vec<&EdgeKey> = seams.iter().filter(|e| !given.contains(e)).collect();
    assert!(!extra.is_empty());
    // The line runs up: every extra edge is upright.
    for e in extra {
        let (a, b) = e.ends();
        let (p, q) = (cylinder.position(a).unwrap(), cylinder.position(b).unwrap());
        assert!((p.z - q.z).abs() > 1e-4, "{p} {q}");
    }
}

#[test]
fn a_sphere_is_opened_from_bottom_to_top() {
    let sphere = Primitive::Sphere(SphereParams::default()).mesh();
    let seams = suggest(&sphere, &[]);
    assert!(!seams.is_empty());
    let cuts: BTreeSet<EdgeKey> = seams.iter().copied().collect();
    assert_eq!(pieces(&sphere, &cuts).len(), 1);
    assert!(every_piece_is_a_disk(&sphere, &cuts));
    // The cut reaches the bottom and the top.
    let heights: Vec<f32> = seams
        .iter()
        .flat_map(|e| {
            let (a, b) = e.ends();
            [a, b]
        })
        .map(|v| sphere.position(v).unwrap().z)
        .collect();
    let top = sphere
        .vertices()
        .map(|(_, p)| p.z)
        .fold(f32::NEG_INFINITY, f32::max);
    let low = sphere
        .vertices()
        .map(|(_, p)| p.z)
        .fold(f32::INFINITY, f32::min);
    assert!(heights.iter().any(|z| (z - top).abs() < 1e-4));
    assert!(heights.iter().any(|z| (z - low).abs() < 1e-4));
    // At the back, away from the Front view (which looks along +Y).
    let middle_y: f32 = seams
        .iter()
        .map(|e| {
            let (a, b) = e.ends();
            sphere.position(a).unwrap().y + sphere.position(b).unwrap().y
        })
        .sum::<f32>()
        / seams.len() as f32;
    assert!(middle_y > 0.1, "{middle_y}");
}

#[test]
fn every_primitive_lies_flat_in_pieces_after_suggesting() {
    for p in all() {
        let mesh = p.mesh();
        let seams = suggest(&mesh, &hard(&mesh));
        let cuts: BTreeSet<EdgeKey> = seams.iter().copied().collect();
        assert!(every_piece_is_a_disk(&mesh, &cuts), "{:?}", p.kind());
        // Inner edges only, each once, in order.
        let mut sorted = seams.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted, seams);
        for e in &seams {
            assert_eq!(mesh.topology().edge_uses(*e).len(), 2);
        }
        // The same every time.
        assert_eq!(suggest(&mesh, &hard(&mesh)), seams);
    }
}

#[test]
fn a_flat_plane_needs_no_seams() {
    let plane = Primitive::Plane(PlaneParams {
        segments: [4, 4],
        ..Default::default()
    })
    .mesh();
    assert!(suggest(&plane, &[]).is_empty());
}

#[test]
fn cuts_prefer_hidden_places() {
    // A cube's bottom edges face down; its top edges face up.
    let cube = Primitive::Cube(CubeParams::default()).mesh();
    let edges = inner_edges(&cube);
    let height = |e: &EdgeKey| {
        let (a, b) = e.ends();
        cube.position(a).unwrap().z + cube.position(b).unwrap().z
    };
    let bottom = edges
        .iter()
        .min_by(|a, b| height(a).total_cmp(&height(b)))
        .unwrap();
    let top = edges
        .iter()
        .max_by(|a, b| height(a).total_cmp(&height(b)))
        .unwrap();
    let hide = HideSeams::BelowAndBehind;
    assert!(visibility(&cube, *bottom, hide) < visibility(&cube, *top, hide));
}

#[test]
fn nothing_and_odd_shapes_are_handled() {
    let empty = Mesh::new();
    assert!(suggest(&empty, &[]).is_empty());
    assert!(pieces(&empty, &BTreeSet::new()).is_empty());
    // A lone triangle, and edges that aren't in the shape.
    let mut tri = Mesh::new();
    let a = tri.add_vertex(Vec3::ZERO).unwrap();
    let b = tri.add_vertex(Vec3::X).unwrap();
    let c = tri.add_vertex(Vec3::Y).unwrap();
    tri.add_face(&[a, b, c]).unwrap();
    let stray = EdgeKey::new(VertexId::from_raw(70), VertexId::from_raw(71));
    assert!(suggest(&tri, &[stray]).is_empty());
    let faces: Vec<FaceId> = tri.faces().map(|(f, _)| f).collect();
    assert!(is_disk(&tri, &faces, &BTreeSet::new()));
}

#[test]
fn any_starting_cuts_end_in_pieces_that_lie_flat() {
    // Random edges given as hard, on shapes with and without borders.
    let mut seed: u64 = 0x5eed;
    let mut next = move || {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (seed >> 33) as usize
    };
    let shapes = [
        Primitive::Cube(CubeParams {
            segments: [3, 3, 3],
            ..Default::default()
        })
        .mesh(),
        Primitive::Tube(TubeParams::default()).mesh(),
        Primitive::Sphere(SphereParams::default()).mesh(),
    ];
    for mesh in &shapes {
        let edges = inner_edges(mesh);
        for _ in 0..12 {
            let given: Vec<EdgeKey> = edges.iter().copied().filter(|_| next() % 7 == 0).collect();
            let cuts: BTreeSet<EdgeKey> = suggest(mesh, &given).into_iter().collect();
            assert!(every_piece_is_a_disk(mesh, &cuts));
            for e in &given {
                assert!(cuts.contains(e), "what was given stays cut");
            }
        }
    }
}
