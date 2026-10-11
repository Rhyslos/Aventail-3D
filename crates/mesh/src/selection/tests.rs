//! Tests for element selections: combining, select all and invert, grow
//! and shrink, loops and rings, on grids, a cube and awkward meshes.

use super::*;
use crate::{CubeParams, PlaneParams, Primitive, PrimitiveKind};
use glam::Vec3;

/// A flat grid of `n` × `n` quads, 100 cm across, centred on the origin.
fn grid(n: u32) -> Mesh {
    Primitive::Plane(PlaneParams {
        width: 100.0,
        depth: 100.0,
        segments: [n, n],
    })
    .mesh()
}

fn cube() -> Mesh {
    Primitive::Cube(CubeParams::default()).mesh()
}

/// The point nearest `p`.
fn point(mesh: &Mesh, p: Vec3) -> VertexId {
    mesh.vertices()
        .min_by(|a, b| a.1.distance(p).total_cmp(&b.1.distance(p)))
        .map(|(v, _)| v)
        .unwrap()
}

/// The edge between the points nearest `a` and `b`.
fn edge(mesh: &Mesh, a: Vec3, b: Vec3) -> EdgeKey {
    EdgeKey::new(point(mesh, a), point(mesh, b))
}

/// The polygon whose middle is nearest `p`.
fn polygon(mesh: &Mesh, p: Vec3) -> FaceId {
    let middle = |corners: &[VertexId]| {
        corners
            .iter()
            .map(|v| mesh.position(*v).unwrap())
            .sum::<Vec3>()
            / corners.len() as f32
    };
    mesh.faces()
        .min_by(|a, b| middle(a.1).distance(p).total_cmp(&middle(b.1).distance(p)))
        .map(|(f, _)| f)
        .unwrap()
}

fn selection_of(elements: &[Element]) -> ElementSelection {
    ElementSelection::of(elements.iter().copied())
}

const fn xy(x: f32, y: f32) -> Vec3 {
    Vec3::new(x, y, 0.0)
}

// ------------------------------------------------------------ combine ---

#[test]
fn combining_replaces_adds_removes_and_toggles() {
    let p = |n| Element::Point(VertexId::from_raw(n));
    let mut s = selection_of(&[p(1), p(2), Element::Polygon(FaceId::from_raw(0))]);

    s.combine(ElementKind::Points, [p(3)], Combine::Add);
    assert_eq!(s.of_kind(ElementKind::Points), vec![p(1), p(2), p(3)]);

    s.combine(ElementKind::Points, [p(2)], Combine::Remove);
    assert_eq!(s.of_kind(ElementKind::Points), vec![p(1), p(3)]);

    s.combine(ElementKind::Points, [p(1), p(4)], Combine::Toggle);
    assert_eq!(s.of_kind(ElementKind::Points), vec![p(3), p(4)]);

    s.combine(ElementKind::Points, [p(7)], Combine::Replace);
    assert_eq!(s.of_kind(ElementKind::Points), vec![p(7)]);

    // The polygon selection was never touched.
    assert_eq!(s.count(ElementKind::Polygons), 1);
    // Elements of another kind are ignored.
    s.combine(
        ElementKind::Points,
        [Element::Polygon(FaceId::from_raw(5))],
        Combine::Add,
    );
    assert_eq!(s.count(ElementKind::Polygons), 1);
    assert_eq!(s.len(), 2);

    // Removing what isn't selected, and replacing with nothing.
    s.combine(ElementKind::Edges, [], Combine::Remove);
    s.combine(ElementKind::Points, [], Combine::Replace);
    assert_eq!(s.count(ElementKind::Points), 0);
    assert!(!s.is_empty(), "the polygon is still selected");
}

#[test]
fn elements_know_their_kind_and_names() {
    let e = Element::Edge(EdgeKey::new(VertexId::from_raw(0), VertexId::from_raw(1)));
    assert_eq!(e.kind(), ElementKind::Edges);
    assert_eq!(ElementKind::Polygons.plural(), "polygons");
    assert_eq!(ElementKind::Points.singular(), "point");
}

// --------------------------------------------------- all and inverted ---

#[test]
fn select_all_and_invert_on_a_cube() {
    let mesh = cube();
    let topology = Topology::new(&mesh);
    assert_eq!(all(&mesh, &topology, ElementKind::Points).len(), 8);
    assert_eq!(all(&mesh, &topology, ElementKind::Edges).len(), 12);
    assert_eq!(all(&mesh, &topology, ElementKind::Polygons).len(), 6);

    let one = all(&mesh, &topology, ElementKind::Edges)[0];
    let selection = selection_of(&[one]);
    let rest = inverted(&mesh, &topology, &selection, ElementKind::Edges);
    assert_eq!(rest.len(), 11);
    assert!(!rest.contains(&one));
    // Inverting another kind ignores the edge.
    assert_eq!(
        inverted(&mesh, &topology, &selection, ElementKind::Points).len(),
        8
    );
}

#[test]
fn an_empty_mesh_has_nothing_to_select() {
    let mesh = Mesh::new();
    let topology = Topology::new(&mesh);
    for kind in [
        ElementKind::Points,
        ElementKind::Edges,
        ElementKind::Polygons,
    ] {
        assert!(all(&mesh, &topology, kind).is_empty());
        let empty = ElementSelection::new();
        assert!(grown(&mesh, &topology, &empty, kind).is_empty());
        assert!(shrunk(&mesh, &topology, &empty, kind).is_empty());
    }
}

// ----------------------------------------------------- grow and shrink ---

#[test]
fn points_grow_to_their_neighbours_and_shrink_back() {
    let mesh = grid(2); // 3 × 3 points
    let topology = Topology::new(&mesh);
    let middle = Element::Point(point(&mesh, Vec3::ZERO));
    let cross = grown(
        &mesh,
        &topology,
        &selection_of(&[middle]),
        ElementKind::Points,
    );
    assert_eq!(cross.len(), 5, "the middle and the four along its edges");
    let everything = grown(&mesh, &topology, &selection_of(&cross), ElementKind::Points);
    assert_eq!(everything.len(), 9);

    let back = shrunk(&mesh, &topology, &selection_of(&cross), ElementKind::Points);
    assert_eq!(back, vec![middle]);
    // With everything selected there is no rim: nothing goes.
    assert_eq!(
        shrunk(
            &mesh,
            &topology,
            &selection_of(&everything),
            ElementKind::Points
        )
        .len(),
        9
    );
    // A lone point shrinks away.
    let corner = Element::Point(point(&mesh, xy(-50.0, -50.0)));
    assert!(
        shrunk(
            &mesh,
            &topology,
            &selection_of(&[corner]),
            ElementKind::Points
        )
        .is_empty()
    );
}

#[test]
fn polygons_grow_through_shared_points_and_shrink_back() {
    let mesh = grid(5);
    let topology = Topology::new(&mesh);
    let middle = Element::Polygon(polygon(&mesh, Vec3::ZERO));
    let block = grown(
        &mesh,
        &topology,
        &selection_of(&[middle]),
        ElementKind::Polygons,
    );
    assert_eq!(block.len(), 9, "the 3 × 3 block around the middle");
    let more = grown(
        &mesh,
        &topology,
        &selection_of(&block),
        ElementKind::Polygons,
    );
    assert_eq!(more.len(), 25);
    let back = shrunk(
        &mesh,
        &topology,
        &selection_of(&block),
        ElementKind::Polygons,
    );
    assert_eq!(back, vec![middle]);
}

#[test]
fn edges_grow_to_edges_sharing_a_point_and_shrink_back() {
    let mesh = grid(2);
    let topology = Topology::new(&mesh);
    // The inner edge from the middle to the right.
    let start = Element::Edge(edge(&mesh, Vec3::ZERO, xy(50.0, 0.0)));
    let grown_edges = grown(
        &mesh,
        &topology,
        &selection_of(&[start]),
        ElementKind::Edges,
    );
    // 4 at the middle point, 3 at the border point, the start counted once.
    assert_eq!(grown_edges.len(), 4 + 3 - 1);
    assert!(grown_edges.contains(&start));
    let back = shrunk(
        &mesh,
        &topology,
        &selection_of(&grown_edges),
        ElementKind::Edges,
    );
    assert_eq!(
        back,
        vec![start],
        "only the start has no unselected edge at its ends"
    );
    let all_edges = all(&mesh, &topology, ElementKind::Edges);
    assert_eq!(
        shrunk(
            &mesh,
            &topology,
            &selection_of(&all_edges),
            ElementKind::Edges
        )
        .len(),
        all_edges.len()
    );
}

#[test]
fn elements_the_mesh_no_longer_has_are_ignored() {
    let mesh = grid(1);
    let topology = Topology::new(&mesh);
    let gone = [
        Element::Point(VertexId::from_raw(99)),
        Element::Edge(EdgeKey::new(VertexId::from_raw(0), VertexId::from_raw(3))),
        Element::Polygon(FaceId::from_raw(42)),
    ];
    let selection = selection_of(&gone);
    assert!(selection.valid_in(&mesh, &topology).is_empty());
    for kind in [
        ElementKind::Points,
        ElementKind::Edges,
        ElementKind::Polygons,
    ] {
        assert!(grown(&mesh, &topology, &selection, kind).is_empty());
    }
}

// ------------------------------------------------------ loops and rings ---

#[test]
fn loops_run_along_a_row_of_a_grid() {
    let mesh = grid(3); // lines at -50, -16.7, 16.7, 50
    let topology = Topology::new(&mesh);
    let t = 50.0 / 3.0;
    // An edge on the inner horizontal line y = -t.
    let along = edge(&mesh, xy(-t, -t), xy(t, -t));
    let edges = loop_through(&topology, along, ElementKind::Edges);
    assert_eq!(edges.len(), 3, "the whole line, border to border");
    let points = loop_through(&topology, along, ElementKind::Points);
    assert_eq!(points.len(), 4);

    // In Polygons mode the loop is the band the edge's ring crosses: a
    // vertical edge's ring runs left to right, so the band is a row.
    let upright = edge(&mesh, xy(-t, -t), xy(-t, t));
    let row = loop_through(&topology, upright, ElementKind::Polygons);
    assert_eq!(row.len(), 3);
    for p in [xy(-2.0 * t, 0.0), Vec3::ZERO, xy(2.0 * t, 0.0)] {
        assert!(row.contains(&Element::Polygon(polygon(&mesh, p))), "{p}");
    }
}

#[test]
fn rings_cross_a_grid_and_polygon_rings_turn_the_other_way() {
    let mesh = grid(3);
    let topology = Topology::new(&mesh);
    let t = 50.0 / 3.0;
    let upright = edge(&mesh, xy(-t, -t), xy(-t, t));
    let rungs = ring_through(&mesh, &topology, upright, None, ElementKind::Edges);
    assert_eq!(rungs.len(), 4, "every upright edge in the middle row");
    let points = ring_through(&mesh, &topology, upright, None, ElementKind::Points);
    assert_eq!(points.len(), 8);

    // The polygon ring runs along the edge, through the polygon given: a
    // column here, at right angles to the loop's row.
    let middle = polygon(&mesh, Vec3::ZERO);
    let column = ring_through(
        &mesh,
        &topology,
        upright,
        Some(middle),
        ElementKind::Polygons,
    );
    assert_eq!(column.len(), 3);
    for p in [xy(0.0, -2.0 * t), Vec3::ZERO, xy(0.0, 2.0 * t)] {
        assert!(column.contains(&Element::Polygon(polygon(&mesh, p))), "{p}");
    }
    // Without a polygon: the two beside the edge.
    let beside = ring_through(&mesh, &topology, upright, None, ElementKind::Polygons);
    assert_eq!(beside.len(), 2);
}

#[test]
fn loops_around_a_cube_close_on_themselves() {
    let mesh = Primitive::Cube(CubeParams {
        segments: [2, 2, 2],
        ..CubeParams::default()
    })
    .mesh();
    let topology = Topology::new(&mesh);
    // An edge across the middle of the front face, from its middle point
    // to the right-hand edge's middle.
    let bounds = mesh.bounds().unwrap();
    let c = bounds.centre();
    let front_middle = Vec3::new(c.x, bounds.min.y, c.z);
    let right_middle = Vec3::new(bounds.max.x, bounds.min.y, c.z);
    let start = edge(&mesh, front_middle, right_middle);
    let belt = loop_through(&topology, start, ElementKind::Edges);
    assert_eq!(belt.len(), 8, "two edges on each of the four sides");
    let band = loop_through(&topology, start, ElementKind::Polygons);
    assert_eq!(band.len(), 8, "a band of polygons across the edge");
}

#[test]
fn n_gons_triangles_and_unknown_edges_stop_loops() {
    // A disc is one n-gon: its border is a loop, and the polygon "band"
    // across a border edge is just the n-gon.
    let disc = PrimitiveKind::Disc.standard().mesh();
    let topology = Topology::new(&disc);
    let rim = topology.edges().next().unwrap();
    let border = loop_through(&topology, rim, ElementKind::Edges);
    assert_eq!(border.len(), topology.edge_count(), "all the way round");
    assert_eq!(
        loop_through(&topology, rim, ElementKind::Polygons),
        vec![Element::Polygon(disc.faces().next().unwrap().0)]
    );
    let face = disc.faces().next().unwrap().0;
    assert_eq!(
        ring_through(&disc, &topology, rim, Some(face), ElementKind::Polygons),
        vec![Element::Polygon(face)]
    );

    // Two triangles: no quads, so a ring is just the edge.
    let mut mesh = Mesh::new();
    let v: Vec<VertexId> = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]
        .iter()
        .map(|[x, y]| mesh.add_vertex(Vec3::new(*x, *y, 0.0)).unwrap())
        .collect();
    mesh.add_face(&[v[0], v[1], v[2]]).unwrap();
    mesh.add_face(&[v[0], v[2], v[3]]).unwrap();
    let topology = Topology::new(&mesh);
    let diagonal = EdgeKey::new(v[0], v[2]);
    assert_eq!(
        ring_through(&mesh, &topology, diagonal, None, ElementKind::Edges),
        vec![Element::Edge(diagonal)]
    );
    assert_eq!(
        loop_through(&topology, diagonal, ElementKind::Polygons).len(),
        2,
        "the triangles beside it"
    );

    let unknown = EdgeKey::new(v[1], v[3]);
    for kind in [
        ElementKind::Points,
        ElementKind::Edges,
        ElementKind::Polygons,
    ] {
        assert!(
            loop_through(&topology, unknown, kind).is_empty(),
            "{kind:?}"
        );
        assert!(ring_through(&mesh, &topology, unknown, None, kind).is_empty());
    }
}

#[test]
fn face_edges_go_round_the_face() {
    let mesh = grid(1);
    let face = mesh.faces().next().unwrap().0;
    let edges = face_edges(&mesh, face);
    assert_eq!(edges.len(), 4);
    let topology = Topology::new(&mesh);
    assert!(edges.iter().all(|e| topology.has_edge(*e)));
    assert!(face_edges(&mesh, FaceId::from_raw(9)).is_empty());
}

#[test]
fn memory_size_grows_with_the_selection() {
    let empty = ElementSelection::new();
    assert_eq!(empty.memory_size(), 0);
    let some = selection_of(&[
        Element::Point(VertexId::from_raw(1)),
        Element::Polygon(FaceId::from_raw(1)),
    ]);
    assert!(some.memory_size() > 0);
}
