//! Tests for the mesh builder, and for the tools carrying attribute values
//! through it.

use super::*;
use crate::attributes::{AttributeElement, HARD};
use crate::edit::{
    Bend, BevelShape, Eye, Grouping, SlitLine, bevel_edges, bevel_points, extrude_bent,
    extrude_edges, extrude_polygons, inset_polygons, loop_cut, slit,
};
use crate::{PlaneParams, Primitive, PrimitiveKind};
use glam::{Vec2, Vec3};

fn uv(x: f32, y: f32) -> Value {
    Value::Vec2(Vec2::new(x, y))
}

/// A square with a point layer `w`, a polygon layer `material`, a corner
/// layer `uv` (each corner's UV is its position), and an edge layer `seam`.
fn square() -> (Mesh, [VertexId; 4], FaceId) {
    let mut mesh = Mesh::new();
    let v = [
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ]
    .map(|p| mesh.add_vertex(p).unwrap());
    let f = mesh.add_face(&v).unwrap();
    with_layers(&mut mesh);
    (mesh, v, f)
}

/// Adds the test layers to `mesh`: every point's `w` is its height plus
/// one, every polygon's `material` is 7, every corner's `uv` its point's X
/// and Y.
fn with_layers(mesh: &mut Mesh) {
    // The test's own UVs and seams, in place of a primitive's.
    mesh.remove_attribute("uv");
    mesh.remove_attribute("seam");
    mesh.add_attribute("w", Domain::Point, Value::Float(0.0))
        .unwrap();
    mesh.add_attribute("material", Domain::Face, Value::Int(0))
        .unwrap();
    mesh.add_attribute("uv", Domain::Corner, uv(-1.0, -1.0))
        .unwrap();
    mesh.add_attribute("seam", Domain::Edge, Value::Bool(false))
        .unwrap();
    let points: Vec<(VertexId, Vec3)> = mesh.vertices().collect();
    for (v, p) in points {
        mesh.set_attribute("w", AttributeElement::Point(v), Value::Float(p.z + 1.0))
            .unwrap();
    }
    let faces: Vec<(FaceId, Vec<VertexId>)> = mesh.faces().map(|(f, c)| (f, c.to_vec())).collect();
    for (f, corners) in faces {
        mesh.set_attribute("material", AttributeElement::Face(f), Value::Int(7))
            .unwrap();
        for (i, c) in corners.iter().enumerate() {
            let p = mesh.position(*c).unwrap();
            mesh.set_attribute("uv", AttributeElement::Corner(f, i), uv(p.x, p.y))
                .unwrap();
        }
    }
}

fn value(mesh: &Mesh, name: &str, element: AttributeElement) -> Value {
    mesh.attribute(name, element).unwrap()
}

#[test]
fn new_points_copy_or_blend_their_sources() {
    let (original, v, _) = square();
    let mut mesh = original.clone();
    mesh.set_attribute("w", AttributeElement::Point(v[1]), Value::Float(3.0))
        .unwrap();
    let mut build = Builder::new(&original, &mut mesh);
    let copy = build.point(Vec3::ZERO, PointFrom::Copy(v[1]));
    let half = build.point(Vec3::ZERO, PointFrom::Between(v[0], v[1], 0.5));
    // A new point can lie between other new ones.
    let quarter = build.point(Vec3::ZERO, PointFrom::Between(half, v[0], 0.5));
    let none = build.point(Vec3::ZERO, PointFrom::Nothing);
    let w = |p| value(&mesh, "w", AttributeElement::Point(p));
    assert_eq!(w(copy), Value::Float(3.0), "from the mesh being built");
    assert_eq!(w(half), Value::Float(2.0));
    assert_eq!(w(quarter), Value::Float(1.5));
    assert_eq!(w(none), Value::Float(0.0));
}

#[test]
fn new_polygons_copy_their_model_and_their_corners_sources() {
    let (original, v, f) = square();
    let mut mesh = original.clone();
    let mut build = Builder::new(&original, &mut mesh);
    let far = build.point(Vec3::new(0.0, 0.0, 1.0), PointFrom::Copy(v[0]));
    let face = build
        .face(
            &[
                (v[0], CornerFrom::Copy(f, v[0])),
                (v[1], CornerFrom::Between(f, v[1], f, v[2], 0.5)),
                (far, CornerFrom::Nothing),
            ],
            Some(f),
        )
        .unwrap();
    // A polygon the mesh would refuse isn't made.
    assert_eq!(build.face(&[(v[0], CornerFrom::Nothing)], Some(f)), None);
    assert_eq!(
        value(&mesh, "material", AttributeElement::Face(face)),
        Value::Int(7)
    );
    let corner = |i| value(&mesh, "uv", AttributeElement::Corner(face, i));
    assert_eq!(corner(0), uv(0.0, 0.0));
    assert_eq!(corner(1), uv(1.0, 0.5));
    assert_eq!(corner(2), uv(-1.0, -1.0), "the default");
    // The original polygon is untouched.
    assert_eq!(
        value(&mesh, "uv", AttributeElement::Corner(f, 2)),
        uv(1.0, 1.0)
    );
}

#[test]
fn rewritten_polygons_take_their_corners_from_where_they_say() {
    let (original, v, f) = square();
    let mut mesh = original.clone();
    let mut build = Builder::new(&original, &mut mesh);
    let cut = build.point(
        Vec3::new(0.5, 0.0, 0.0),
        PointFrom::Between(v[0], v[1], 0.5),
    );
    let mut changes = BTreeMap::new();
    changes.insert(
        f,
        vec![
            (v[0], CornerFrom::Copy(f, v[0])),
            (cut, CornerFrom::Between(f, v[0], f, v[1], 0.5)),
            (v[1], CornerFrom::Copy(f, v[1])),
            (v[2], CornerFrom::Copy(f, v[2])),
            (v[3], CornerFrom::Copy(f, v[3])),
        ],
    );
    build.rewrite_faces(&changes).unwrap();
    let corner = |i| value(&mesh, "uv", AttributeElement::Corner(f, i));
    assert_eq!(corner(1), uv(0.5, 0.0));
    assert_eq!(corner(4), uv(0.0, 1.0));
    // Refused rewrites change nothing.
    let before = mesh.clone();
    let mut build = Builder::new(&original, &mut mesh);
    let mut bad = BTreeMap::new();
    bad.insert(f, vec![(v[0], CornerFrom::Nothing)]);
    assert!(build.rewrite_faces(&bad).is_err());
    assert_eq!(mesh, before);
}

#[test]
fn edges_copy_move_and_clear_their_values() {
    let (mut original, v, _) = square();
    let edge = EdgeKey::new(v[0], v[1]);
    original
        .set_attribute("seam", AttributeElement::Edge(edge), Value::Bool(true))
        .unwrap();
    original.set_hard(v[0], v[1], true);
    let mut mesh = original.clone();
    let mut build = Builder::new(&original, &mut mesh);
    let (x, y) = (
        build.point(Vec3::ZERO, PointFrom::Copy(v[0])),
        build.point(Vec3::X, PointFrom::Copy(v[1])),
    );
    build.copy_edge(edge, x, y);
    // An edge that can't hold values is skipped.
    build.copy_edge(edge, x, x);
    let seam = |m: &Mesh, a, b| value(m, "seam", AttributeElement::Edge(EdgeKey::new(a, b)));
    assert_eq!(seam(&mesh, x, y), Value::Bool(true));
    assert!(mesh.is_hard(x, y));
    assert_eq!(seam(&mesh, v[0], v[1]), Value::Bool(true), "a copy");
    let mut build = Builder::new(&original, &mut mesh);
    build.move_edge(edge, v[2], v[3]);
    assert_eq!(seam(&mesh, v[0], v[1]), Value::Bool(false), "moved away");
    assert!(!mesh.is_hard(v[0], v[1]));
    assert!(mesh.is_hard(v[2], v[3]));
    let mut build = Builder::new(&original, &mut mesh);
    build.clear_edge(EdgeKey::new(v[2], v[3]));
    build.clear_edge(EdgeKey::new(x, y));
    assert_eq!(mesh.hard_edge_count(), 0);
    assert!(
        mesh.attributes().all(|(n, _, _)| n != HARD),
        "no hard edges: no hard layer"
    );
}

/// A cube with the test layers, and its top polygon.
fn cube() -> (Mesh, FaceId) {
    let mut mesh = PrimitiveKind::Cube.standard().mesh();
    with_layers(&mut mesh);
    let top = mesh
        .faces()
        .find(|(f, _)| mesh.face_normal(*f).z > 0.9)
        .unwrap()
        .0;
    (mesh, top)
}

/// Every polygon's corners hold a value set from somewhere (none is left at
/// the default), and every polygon its material.
fn nothing_left_at_the_default(mesh: &Mesh) {
    for (f, corners) in mesh.faces() {
        assert_eq!(
            value(mesh, "material", AttributeElement::Face(f)),
            Value::Int(7),
            "polygon {f:?}"
        );
        for i in 0..corners.len() {
            assert_ne!(
                value(mesh, "uv", AttributeElement::Corner(f, i)),
                uv(-1.0, -1.0),
                "polygon {f:?} corner {i}"
            );
        }
    }
    for (v, _) in mesh.vertices() {
        assert_ne!(
            value(mesh, "w", AttributeElement::Point(v)),
            Value::Float(0.0)
        );
    }
}

#[test]
fn extruding_carries_every_value_over() {
    let (mut mesh, top) = cube();
    let corners_before: Vec<Value> = (0..4)
        .map(|i| value(&mesh, "uv", AttributeElement::Corner(top, i)))
        .collect();
    let rim = mesh.face(top).unwrap().to_vec();
    let border = EdgeKey::new(rim[0], rim[1]);
    mesh.set_attribute("seam", AttributeElement::Edge(border), Value::Bool(true))
        .unwrap();
    extrude_polygons(&mut mesh, &[top], 50.0, Grouping::Together);
    nothing_left_at_the_default(&mesh);
    // The top keeps its UVs; its new corners are copies of the old ones.
    for (i, before) in corners_before.iter().enumerate() {
        assert_eq!(
            value(&mesh, "uv", AttributeElement::Corner(top, i)),
            *before
        );
    }
    // The seam is on the old border and the new one.
    let new_rim = mesh.face(top).unwrap().to_vec();
    let seam = |a, b| value(&mesh, "seam", AttributeElement::Edge(EdgeKey::new(a, b)));
    assert_eq!(seam(rim[0], rim[1]), Value::Bool(true));
    assert_eq!(seam(new_rim[0], new_rim[1]), Value::Bool(true));
}

#[test]
fn insetting_and_extruding_edges_carry_every_value_over() {
    let (mut mesh, top) = cube();
    inset_polygons(&mut mesh, &[top], 10.0, Grouping::Together);
    nothing_left_at_the_default(&mesh);
    let mut plane = Primitive::Plane(PlaneParams::default()).mesh();
    with_layers(&mut plane);
    let edges = plane.topology().border_edges();
    extrude_edges(&mut plane, &edges, 10.0, Grouping::Together);
    nothing_left_at_the_default(&plane);
}

#[test]
fn bevelling_blends_corners_across_the_strip() {
    let (mut mesh, top) = cube();
    let rim = mesh.face(top).unwrap().to_vec();
    let edge = EdgeKey::new(rim[0], rim[1]);
    mesh.set_attribute("seam", AttributeElement::Edge(edge), Value::Bool(true))
        .unwrap();
    let shape = BevelShape {
        width: 10.0,
        segments: 2,
        profile: 0.0,
        options: crate::bevel::BevelOptions::EARLIER,
    };
    let before = mesh.clone();
    bevel_edges(&mut mesh, &[edge], shape);
    nothing_left_at_the_default(&mesh);
    // The bevelled edge is gone; its seam moved to one border of the
    // strip (decision 0104), so the pieces either side stay apart.
    let edges: Vec<EdgeKey> = mesh.topology().edges().collect();
    let seams: Vec<&EdgeKey> = edges
        .iter()
        .filter(|e| value(&mesh, "seam", AttributeElement::Edge(**e)) == Value::Bool(true))
        .collect();
    assert_eq!(seams.len(), 1);
    assert!(!seams.contains(&&edge));
    assert_eq!(mesh.topology().edge_uses(*seams[0]).len(), 2);
    assert_eq!(mesh.face_count() - before.face_count(), 2, "two rows");
}

#[test]
fn bevelling_points_carries_every_value_over() {
    let (mut mesh, top) = cube();
    let corner = mesh.face(top).unwrap()[0];
    bevel_points(&mut mesh, &[corner], 10.0);
    nothing_left_at_the_default(&mesh);
}

#[test]
fn every_tool_keeps_values_on_every_primitive() {
    for kind in PrimitiveKind::ALL {
        let mut mesh = kind.standard().mesh();
        with_layers(&mut mesh);
        let faces: Vec<FaceId> = mesh.faces().map(|(f, _)| f).take(3).collect();
        let edges: Vec<EdgeKey> = mesh.topology().edges().take(3).collect();
        let points: Vec<VertexId> = mesh.vertices().map(|(v, _)| v).take(2).collect();
        let mut m = mesh.clone();
        extrude_polygons(&mut m, &faces, 5.0, Grouping::Separate);
        nothing_left_at_the_default(&m);
        for radius in [0.0, 30.0] {
            let mut m = mesh.clone();
            let bend = Bend {
                angle: 60.0,
                direction: 30.0,
                radius,
                segments: 3,
                ..Bend::default()
            };
            extrude_bent(&mut m, &faces, 10.0, Grouping::Together, bend);
            nothing_left_at_the_default(&m);
        }
        let mut m = mesh.clone();
        inset_polygons(&mut m, &faces, 1.0, Grouping::Together);
        nothing_left_at_the_default(&m);
        let mut m = mesh.clone();
        bevel_edges(&mut m, &edges, BevelShape::default());
        nothing_left_at_the_default(&m);
        let mut m = mesh.clone();
        bevel_points(&mut m, &points, 1.0);
        nothing_left_at_the_default(&m);
        for edge in &edges {
            let mut m = mesh.clone();
            loop_cut(&mut m, *edge, 2, 0.3);
            nothing_left_at_the_default(&m);
        }
        let mut m = mesh.clone();
        crate::join::connect_edges(&mut m, &edges, 2);
        nothing_left_at_the_default(&m);
        let mut m = mesh.clone();
        crate::join::connect_points(&mut m, &points);
        nothing_left_at_the_default(&m);
        let mut m = mesh.clone();
        crate::join::weld(&mut m, &points, None);
        nothing_left_at_the_default(&m);
        let middle = mesh.bounds().unwrap().centre();
        let mut m = mesh.clone();
        slit(
            &mut m,
            &SlitLine {
                points: vec![
                    middle + Vec3::new(-200.0, -10.0, -30.0),
                    middle + Vec3::new(3.0, 2.0, 1.0),
                    middle + Vec3::new(200.0, 15.0, 40.0),
                ],
                eye: Eye::At(middle + Vec3::new(40.0, -600.0, 300.0)),
                through: true,
            },
        );
        nothing_left_at_the_default(&m);
    }
}

#[test]
fn points_and_corners_inside_a_polygon_blend_its_corners() {
    let (mut mesh, v, f) = square();
    let original = mesh.clone();
    let spot = Vec3::new(0.25, 0.5, 0.0);
    let weights = weights_inside(&original, f, spot);
    let sum: f32 = weights.iter().map(|(.., w)| w).sum();
    assert!((sum - 1.0).abs() < 1e-5);
    // On a corner: all of it.
    let at_corner = weights_inside(&original, f, Vec3::new(1.0, 1.0, 0.0));
    assert_eq!(at_corner[2].2, 1.0);
    let mut build = Builder::new(&original, &mut mesh);
    let p = build.point(spot, PointFrom::Inside(f, spot));
    let g = build
        .face(
            &[
                (v[0], CornerFrom::Copy(f, v[0])),
                (v[1], CornerFrom::Copy(f, v[1])),
                (p, CornerFrom::Inside(f, spot)),
            ],
            Some(f),
        )
        .unwrap();
    // w is height plus one; uv is the point's X and Y.
    assert_eq!(
        value(&mesh, "w", AttributeElement::Point(p)),
        Value::Float(1.0)
    );
    let Value::Vec2(got) = value(&mesh, "uv", AttributeElement::Corner(g, 2)) else {
        panic!("a uv");
    };
    assert!(got.distance(glam::Vec2::new(0.25, 0.5)) < 1e-5, "{got}");
    assert!(weights_inside(&original, FaceId::from_raw(99), spot).is_empty());
}
