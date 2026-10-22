//! Tests for topology: adjacency, borders, non-manifold edges, loops,
//! rings and pieces, on primitives and on deliberately awkward meshes.

use super::*;
use crate::{CubeParams, CylinderParams, PlaneParams, Primitive, PrimitiveKind, SphereParams};
use glam::Vec3;

fn v(n: u32) -> VertexId {
    VertexId::from_raw(n)
}

fn e(a: u32, b: u32) -> EdgeKey {
    EdgeKey::new(v(a), v(b))
}

/// A flat grid of `n` × `n` quads, points numbered row by row.
fn grid(n: u32) -> Mesh {
    Primitive::Plane(PlaneParams {
        width: 100.0,
        depth: 100.0,
        segments: [n, n],
    })
    .mesh()
}

fn cube(segments: u32) -> Mesh {
    Primitive::Cube(CubeParams {
        segments: [segments; 3],
        ..CubeParams::default()
    })
    .mesh()
}

/// The edge between the points at `a` and `b` (looked up by position).
fn edge_at(mesh: &Mesh, a: Vec3, b: Vec3) -> EdgeKey {
    let find = |p: Vec3| {
        mesh.vertices()
            .find(|(_, q)| q.distance(p) < 1e-3)
            .map(|(id, _)| id)
            .unwrap_or_else(|| panic!("no point at {p}"))
    };
    EdgeKey::new(find(a), find(b))
}

/// A mesh from points and faces given as point numbers.
fn build(points: &[[f32; 3]], faces: &[&[u32]]) -> Mesh {
    let mut mesh = Mesh::new();
    for p in points {
        mesh.add_vertex(Vec3::from_array(*p)).unwrap();
    }
    for f in faces {
        let corners: Vec<VertexId> = f.iter().map(|n| v(*n)).collect();
        mesh.add_face(&corners).unwrap();
    }
    mesh
}

// ----------------------------------------------------------- adjacency ---

#[test]
fn a_cube_has_twelve_edges_each_between_two_faces() {
    let mesh = cube(1);
    let topology = Topology::new(&mesh);
    assert_eq!(topology.edge_count(), 12);
    assert_eq!(topology.edges().count(), mesh.edges().len());
    for edge in topology.edges() {
        assert_eq!(topology.edge_uses(edge).len(), 2, "{edge:?}");
        let uses = topology.edge_uses(edge);
        assert_ne!(uses[0].forward, uses[1].forward, "consistent winding");
    }
    for (vertex, _) in mesh.vertices() {
        assert_eq!(topology.vertex_edges(vertex).len(), 3);
        assert_eq!(topology.vertex_faces(vertex).len(), 3);
        assert_eq!(topology.vertex_neighbours(vertex).len(), 3);
    }
    for (face, _) in mesh.faces() {
        assert_eq!(topology.face_edges(face).len(), 4);
        assert_eq!(topology.face_neighbours(face).len(), 4);
    }
    assert!(topology.border_edges().is_empty());
    assert!(topology.non_manifold_edges().is_empty());
    assert!(topology.flipped_edges().is_empty());
}

#[test]
fn every_primitive_is_consistent_with_its_mesh() {
    for primitive in Primitive::all() {
        let mesh = primitive.mesh();
        let topology = Topology::new(&mesh);
        assert_eq!(topology.edge_count(), mesh.edges().len(), "{primitive:?}");
        // Euler's rule for one closed or open piece without holes in
        // its surface: points - edges + faces = 2 for a closed shape, 1 for
        // an open disc-like one, 0 for a tube.
        let euler =
            mesh.vertex_count() as i64 - topology.edge_count() as i64 + mesh.face_count() as i64;
        let expected = match primitive.kind() {
            PrimitiveKind::Plane | PrimitiveKind::Disc => 1,
            // Closed, with a hole through it, like a doughnut.
            PrimitiveKind::Tube => 0,
            _ => 2,
        };
        assert_eq!(euler, expected, "{primitive:?}");
        assert!(topology.non_manifold_edges().is_empty(), "{primitive:?}");
        assert!(topology.flipped_edges().is_empty(), "{primitive:?}");
        assert_eq!(topology.pieces().len(), 1, "{primitive:?}");
        // Face edges go round each face in corner order.
        for (face, corners) in mesh.faces() {
            let edges = topology.face_edges(face);
            assert_eq!(edges.len(), corners.len());
            assert_eq!(edges[0], EdgeKey::new(corners[0], corners[1]));
        }
    }
}

#[test]
fn open_shapes_have_borders() {
    let plane = grid(2);
    let topology = Topology::new(&plane);
    assert_eq!(topology.border_edges().len(), 8);
    let tube = PrimitiveKind::Tube.standard().mesh();
    assert!(
        Topology::new(&tube).border_edges().is_empty(),
        "a tube is closed"
    );
    let open = CylinderParams {
        caps: false,
        ..CylinderParams::default()
    };
    let pipe = Primitive::Cylinder(open).mesh();
    assert_eq!(
        Topology::new(&pipe).border_edges().len(),
        2 * open.sides as usize,
        "both open ends"
    );
}

#[test]
fn unknown_elements_have_nothing_attached() {
    let topology = Topology::new(&cube(1));
    assert!(topology.edge_uses(e(0, 999)).is_empty());
    assert!(topology.vertex_edges(v(999)).is_empty());
    assert!(topology.face_edges(FaceId::from_raw(999)).is_empty());
    assert!(topology.face_neighbours(FaceId::from_raw(999)).is_empty());
    assert!(topology.edge_loop(e(0, 999)).is_empty());
    assert!(topology.edge_ring(e(0, 999)).is_empty());
    let empty = Topology::new(&Mesh::new());
    assert_eq!(empty.edge_count(), 0);
    assert!(empty.pieces().is_empty());
}

// ------------------------------------------------------ awkward meshes ---

#[test]
fn an_edge_shared_by_three_faces_is_non_manifold() {
    // Three triangles fanning off one edge (0-1), like pages of a book.
    let mesh = build(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.5, 1.0, 0.0],
            [0.5, -1.0, 0.0],
            [0.5, 0.0, 1.0],
        ],
        &[&[0, 1, 2], &[1, 0, 3], &[0, 1, 4]],
    );
    let topology = Topology::new(&mesh);
    assert_eq!(topology.non_manifold_edges(), [e(0, 1)]);
    assert_eq!(topology.edge_faces(e(0, 1)).count(), 3);
    assert!(!topology.is_border(e(0, 1)));
    assert!(topology.flipped_edges().is_empty(), "only pairs count");
    // Loops and rings stop rather than guess.
    assert_eq!(topology.edge_loop(e(0, 1)), [e(0, 1)]);
    assert_eq!(topology.edge_ring(e(0, 1)), [e(0, 1)]);
}

#[test]
fn faces_running_the_same_way_along_an_edge_are_flipped() {
    // Two quads sharing edge 1-2, both going 1 -> 2.
    let mesh = build(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [2.0, 0.0, 0.0],
            [2.0, 1.0, 0.0],
        ],
        &[&[0, 1, 2, 3], &[4, 5, 2, 1]],
    );
    // In the second face, 2 -> 1 runs the other way: consistent.
    assert!(Topology::new(&mesh).flipped_edges().is_empty());
    let flipped = build(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [2.0, 0.0, 0.0],
            [2.0, 1.0, 0.0],
        ],
        &[&[0, 1, 2, 3], &[1, 2, 5, 4]],
    );
    assert_eq!(Topology::new(&flipped).flipped_edges(), [e(1, 2)]);
}

#[test]
fn loose_points_and_zero_length_edges_are_handled() {
    // A triangle whose two corners sit on the same spot, and a loose
    // point no face uses.
    let mesh = build(
        &[[0.0; 3], [0.0; 3], [0.0, 1.0, 0.0], [5.0, 5.0, 5.0]],
        &[&[0, 1, 2]],
    );
    let topology = Topology::new(&mesh);
    assert_eq!(topology.edge_count(), 3);
    assert!(topology.vertex_edges(v(3)).is_empty());
    assert!(topology.vertex_faces(v(3)).is_empty());
    assert_eq!(topology.border_edges().len(), 3);
    assert_eq!(topology.edge_loop(e(0, 1)).len(), 3, "round the border");
}

#[test]
fn separate_and_touching_pieces() {
    // Two triangles apart, then a third touching the second at a point.
    let mesh = build(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [5.0, 0.0, 0.0],
            [6.0, 0.0, 0.0],
            [5.0, 1.0, 0.0],
            [7.0, 1.0, 0.0],
            [6.0, 2.0, 0.0],
        ],
        &[&[0, 1, 2], &[3, 4, 5], &[4, 6, 7]],
    );
    let pieces = Topology::new(&mesh).pieces();
    let f = FaceId::from_raw;
    assert_eq!(pieces, [vec![f(0)], vec![f(1), f(2)]]);
}

#[test]
fn an_n_gon_is_one_face_with_many_edges() {
    let disc = PrimitiveKind::Disc.standard().mesh();
    let topology = Topology::new(&disc);
    let (face, corners) = disc.faces().next().unwrap();
    assert_eq!(disc.face_count(), 1);
    assert_eq!(topology.face_edges(face).len(), corners.len());
    assert!(topology.face_neighbours(face).is_empty());
    // Its rim is one closed border loop.
    let rim = topology.edge_loop(topology.face_edges(face)[0]);
    assert_eq!(rim.len(), corners.len());
}

// --------------------------------------------------------------- loops ---

#[test]
fn a_loop_on_a_grid_runs_from_border_to_border() {
    let mesh = grid(3);
    let topology = Topology::new(&mesh);
    // An inside edge running along X, a third of the way across.
    let start = edge_at(
        &mesh,
        Vec3::new(-50.0 + 100.0 / 3.0, -50.0 + 100.0 / 3.0, 0.0),
        Vec3::new(-50.0 + 200.0 / 3.0, -50.0 + 100.0 / 3.0, 0.0),
    );
    let found = topology.edge_loop(start);
    assert_eq!(found.len(), 3, "the whole row of edges");
    assert_eq!(found[0], start);
    let mut ends: Vec<f32> = found
        .iter()
        .flat_map(|e| {
            let (a, b) = e.ends();
            [a, b]
        })
        .map(|p| mesh.position(p).unwrap().y)
        .collect();
    ends.dedup();
    assert_eq!(ends.len(), 1, "a straight line along X");
}

#[test]
fn a_border_edge_loops_round_the_whole_border() {
    let mesh = grid(3);
    let topology = Topology::new(&mesh);
    let border = topology.border_edges()[0];
    let found = topology.edge_loop(border);
    assert_eq!(found.len(), 12);
    assert!(found.iter().all(|e| topology.is_border(*e)));
}

#[test]
fn a_loop_round_a_cut_cube_closes() {
    let mesh = cube(2);
    let topology = Topology::new(&mesh);
    // Round the middle: the edge along X at the front, half way up.
    let start = edge_at(
        &mesh,
        Vec3::new(-50.0, -50.0, 50.0),
        Vec3::new(0.0, -50.0, 50.0),
    );
    let found = topology.edge_loop(start);
    assert_eq!(found.len(), 8, "two edges on each of four sides");
    let unique: BTreeSet<EdgeKey> = found.iter().copied().collect();
    assert_eq!(unique.len(), 8);
    for edge in &found {
        let (a, b) = edge.ends();
        assert_eq!(mesh.position(a).unwrap().z, 50.0);
        assert_eq!(mesh.position(b).unwrap().z, 50.0);
    }
}

#[test]
fn loops_stop_at_poles() {
    // A plain cube: every corner has three edges, so loops go nowhere.
    let topology = Topology::new(&cube(1));
    for edge in topology.edges() {
        assert_eq!(topology.edge_loop(edge), [edge]);
    }
}

#[test]
fn sphere_loops_go_round_and_stop_at_the_poles() {
    let params = SphereParams {
        sides: 12,
        rings: 6,
        ..SphereParams::default()
    };
    let mesh = Primitive::Sphere(params).mesh();
    let topology = Topology::new(&mesh);
    // Round the equator.
    let equator: Vec<EdgeKey> = topology
        .edges()
        .filter(|e| {
            let (a, b) = e.ends();
            let (pa, pb) = (mesh.position(a).unwrap(), mesh.position(b).unwrap());
            (pa.z - 50.0).abs() < 1e-3 && (pb.z - 50.0).abs() < 1e-3
        })
        .collect();
    assert_eq!(equator.len(), 12);
    assert_eq!(topology.edge_loop(equator[0]).len(), 12);
    // From pole to pole: stops at the triangles round each pole.
    let upright = topology
        .vertex_edges(equator[0].ends().0)
        .iter()
        .copied()
        .find(|e| !equator.contains(e))
        .unwrap();
    let meridian = topology.edge_loop(upright);
    assert!(meridian.len() > 1 && meridian.len() < params.rings as usize);
}

#[test]
fn a_cylinder_side_loop_and_ring() {
    let params = CylinderParams {
        sides: 8,
        height_segments: 2,
        ..CylinderParams::default()
    };
    let mesh = Primitive::Cylinder(params).mesh();
    let topology = Topology::new(&mesh);
    let middle = edge_at(
        &mesh,
        Vec3::new(50.0, 0.0, 50.0),
        Vec3::new(
            50.0 * (std::f32::consts::TAU / 8.0).cos(),
            50.0 * (std::f32::consts::TAU / 8.0).sin(),
            50.0,
        ),
    );
    assert_eq!(topology.edge_loop(middle).len(), 8, "round the middle");
    // An upright edge: its ring goes all the way round.
    let upright = edge_at(&mesh, Vec3::new(50.0, 0.0, 0.0), Vec3::new(50.0, 0.0, 50.0));
    assert_eq!(topology.edge_ring(upright).len(), 8);
    // The upright loop stops at the n-gon ends: two edges, bottom to top.
    assert_eq!(topology.edge_loop(upright).len(), 2);
}

// --------------------------------------------------------------- rings ---

#[test]
fn a_ring_on_a_grid_crosses_from_border_to_border() {
    let mesh = grid(3);
    let topology = Topology::new(&mesh);
    let start = edge_at(
        &mesh,
        Vec3::new(-50.0, -50.0 + 100.0 / 3.0, 0.0),
        Vec3::new(-50.0 + 100.0 / 3.0, -50.0 + 100.0 / 3.0, 0.0),
    );
    let found = topology.edge_ring(start);
    assert_eq!(found.len(), 4, "the rungs across one column of quads");
    assert_eq!(found[0], start);
}

#[test]
fn a_ring_round_a_cut_cube_closes() {
    let mesh = cube(2);
    let topology = Topology::new(&mesh);
    let start = edge_at(
        &mesh,
        Vec3::new(-50.0, -50.0, 50.0),
        Vec3::new(-50.0, -50.0, 100.0),
    );
    let found = topology.edge_ring(start);
    assert_eq!(found.len(), 8);
    let unique: BTreeSet<EdgeKey> = found.iter().copied().collect();
    assert_eq!(unique.len(), 8);
}

#[test]
fn rings_stop_at_triangles_and_n_gons() {
    // A quad beside a triangle: the ring crosses the quad, then stops.
    let mesh = build(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [2.0, 0.5, 0.0],
        ],
        &[&[0, 1, 2, 3], &[1, 4, 2]],
    );
    let topology = Topology::new(&mesh);
    assert_eq!(topology.edge_ring(e(1, 2)), [e(1, 2), e(3, 0)]);
    assert_eq!(topology.edge_ring(e(1, 4)), [e(1, 4)]);
}

#[test]
fn the_other_end_of_an_edge() {
    assert_eq!(other_end(e(3, 7), v(3)), v(7));
    assert_eq!(other_end(e(3, 7), v(7)), v(3));
}

#[test]
fn a_loop_on_a_caps_rim_runs_round_the_cap() {
    // The project lead's find: a double-click on a cylinder's top edge
    // selected one edge, as the rim's points aren't where four quads meet.
    let params = CylinderParams {
        sides: 12,
        height_segments: 2,
        ..CylinderParams::default()
    };
    let mesh = Primitive::Cylinder(params).mesh();
    let topology = Topology::new(&mesh);
    let cap = mesh
        .faces()
        .map(|(f, _)| f)
        .find(|f| topology.face_edges(*f).len() == 12)
        .expect("a 12-sided cap");
    let rim: BTreeSet<EdgeKey> = topology.face_edges(cap).iter().copied().collect();
    for start in &rim {
        let found: BTreeSet<EdgeKey> = topology.edge_loop(*start).into_iter().collect();
        assert_eq!(found, rim, "the whole rim, from any of its edges");
    }
    // An edge up the side still stops at the rims (poles of three edges):
    // the side's two rows, nothing round the cap.
    let up = topology
        .edges()
        .find(|e| {
            !rim.contains(e)
                && topology
                    .edge_faces(*e)
                    .all(|f| topology.face_edges(f).len() == 4)
        })
        .unwrap();
    let side = topology.edge_loop(up);
    assert!(side.iter().all(|e| !rim.contains(e)));
    assert!(side.len() <= 2);
}

#[test]
fn an_edge_between_two_n_gons_or_beside_none_stays_alone() {
    // Two pentagons sharing an edge: the shared edge borders two n-gons,
    // so there's no one rim to follow.
    let mesh = build(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.5, 1.0, 0.0],
            [0.5, 1.5, 0.0],
            [-0.5, 1.0, 0.0],
            [1.0, -1.0, 0.0],
            [0.0, -1.0, 0.0],
            [-0.5, -0.5, 0.0],
        ],
        &[&[0, 1, 2, 3, 4], &[1, 0, 7, 6, 5]],
    );
    let topology = Topology::new(&mesh);
    assert_eq!(topology.edge_loop(e(0, 1)), vec![e(0, 1)]);
    // A triangle's inner edge beside an n-gon-free fan: unchanged.
    let fan = build(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [-1.0, 0.0, 0.0],
        ],
        &[&[0, 1, 2], &[0, 2, 3]],
    );
    let topology = Topology::new(&fan);
    assert_eq!(topology.edge_loop(e(0, 2)), vec![e(0, 2)]);
}
