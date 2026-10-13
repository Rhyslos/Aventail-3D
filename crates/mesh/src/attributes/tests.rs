//! Tests for attribute layers on a mesh.

use super::*;
use crate::{Mesh, PrimitiveKind};
use glam::{Vec2, Vec3, Vec4};

/// Two squares side by side, sharing the edge 1–2:
///
/// ```text
/// 3 --- 2 --- 5
/// |  0  |  1  |
/// 0 --- 1 --- 4
/// ```
fn two_squares() -> (Mesh, [VertexId; 6], [FaceId; 2]) {
    let mut mesh = Mesh::new();
    let v = [
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(2.0, 1.0, 0.0),
    ]
    .map(|p| mesh.add_vertex(p).unwrap());
    let a = mesh.add_face(&[v[0], v[1], v[2], v[3]]).unwrap();
    let b = mesh.add_face(&[v[1], v[4], v[5], v[2]]).unwrap();
    (mesh, v, [a, b])
}

fn uv(x: f32, y: f32) -> Value {
    Value::Vec2(Vec2::new(x, y))
}

#[test]
fn layers_are_added_once_and_names_belong_to_one_kind() {
    let (mut mesh, _, _) = two_squares();
    assert_eq!(mesh.attributes().count(), 0);
    mesh.add_attribute("uv", Domain::Corner, uv(0.0, 0.0))
        .unwrap();
    // The same again changes nothing.
    mesh.add_attribute("uv", Domain::Corner, uv(0.5, 0.5))
        .unwrap();
    assert_eq!(
        mesh.attributes().collect::<Vec<_>>(),
        vec![("uv", Domain::Corner, AttributeKind::Vec2)]
    );
    assert_eq!(
        mesh.add_attribute("uv", Domain::Point, uv(0.0, 0.0)),
        Err(AttributeError::NameTaken("uv".into()))
    );
    assert_eq!(
        mesh.add_attribute("uv", Domain::Corner, Value::Float(0.0)),
        Err(AttributeError::NameTaken("uv".into()))
    );
    assert_eq!(
        mesh.add_attribute("w", Domain::Point, Value::Float(f32::NAN)),
        Err(AttributeError::NotANumber)
    );
    assert!(mesh.remove_attribute("uv"));
    assert!(!mesh.remove_attribute("uv"));
    assert_eq!(
        AttributeError::NameTaken("uv".into()).to_string(),
        "an attribute called \"uv\" already exists"
    );
}

#[test]
fn values_on_every_domain_start_at_the_default() {
    let (mut mesh, v, f) = two_squares();
    mesh.add_attribute("colour", Domain::Point, Value::Vec4(Vec4::ONE))
        .unwrap();
    mesh.add_attribute("seam", Domain::Edge, Value::Bool(false))
        .unwrap();
    mesh.add_attribute("material", Domain::Face, Value::Int(0))
        .unwrap();
    mesh.add_attribute("uv", Domain::Corner, uv(0.0, 0.0))
        .unwrap();
    let red = Value::Vec4(Vec4::new(1.0, 0.0, 0.0, 1.0));
    let shared = AttributeElement::Edge(EdgeKey::new(v[1], v[2]));
    mesh.set_attribute("colour", AttributeElement::Point(v[4]), red)
        .unwrap();
    mesh.set_attribute("seam", shared, Value::Bool(true))
        .unwrap();
    mesh.set_attribute("material", AttributeElement::Face(f[1]), Value::Int(2))
        .unwrap();
    mesh.set_attribute("uv", AttributeElement::Corner(f[1], 3), uv(0.25, 1.0))
        .unwrap();
    let get = |name, element| mesh.attribute(name, element);
    assert_eq!(get("colour", AttributeElement::Point(v[4])), Some(red));
    assert_eq!(
        get("colour", AttributeElement::Point(v[0])),
        Some(Value::Vec4(Vec4::ONE))
    );
    assert_eq!(get("seam", shared), Some(Value::Bool(true)));
    // Edges are the same either way round.
    assert_eq!(
        get("seam", AttributeElement::Edge(EdgeKey::new(v[2], v[1]))),
        Some(Value::Bool(true))
    );
    assert_eq!(
        get("seam", AttributeElement::Edge(EdgeKey::new(v[0], v[1]))),
        Some(Value::Bool(false))
    );
    assert_eq!(
        get("material", AttributeElement::Face(f[1])),
        Some(Value::Int(2))
    );
    assert_eq!(
        get("material", AttributeElement::Face(f[0])),
        Some(Value::Int(0))
    );
    assert_eq!(
        get("uv", AttributeElement::Corner(f[1], 3)),
        Some(uv(0.25, 1.0))
    );
    assert_eq!(
        get("uv", AttributeElement::Corner(f[0], 3)),
        Some(uv(0.0, 0.0))
    );
    // Asking for the wrong thing gives nothing.
    assert_eq!(get("uv", AttributeElement::Face(f[0])), None);
    assert_eq!(get("nope", AttributeElement::Face(f[0])), None);
    assert_eq!(get("uv", AttributeElement::Corner(f[0], 4)), None);
}

#[test]
fn bad_values_are_refused_and_change_nothing() {
    let (mut mesh, v, f) = two_squares();
    mesh.add_attribute("weight", Domain::Point, Value::Float(0.0))
        .unwrap();
    let before = mesh.clone();
    let set = |mesh: &mut Mesh, element, value| mesh.set_attribute("weight", element, value);
    assert_eq!(
        set(&mut mesh, AttributeElement::Face(f[0]), Value::Float(1.0)),
        Err(AttributeError::WrongDomain {
            layer: Domain::Point,
            element: Domain::Face
        })
    );
    assert_eq!(
        set(&mut mesh, AttributeElement::Point(v[0]), Value::Int(1)),
        Err(AttributeError::WrongKind {
            layer: AttributeKind::Float,
            value: AttributeKind::Int
        })
    );
    assert_eq!(
        set(
            &mut mesh,
            AttributeElement::Point(v[0]),
            Value::Float(f32::INFINITY)
        ),
        Err(AttributeError::NotANumber)
    );
    assert_eq!(
        set(
            &mut mesh,
            AttributeElement::Point(VertexId::from_raw(99)),
            Value::Float(1.0)
        ),
        Err(AttributeError::NoSuchElement)
    );
    assert_eq!(
        mesh.set_attribute("nope", AttributeElement::Point(v[0]), Value::Float(1.0)),
        Err(AttributeError::Unknown("nope".into()))
    );
    assert_eq!(mesh, before);
    // An edge from a point to itself can't hold a value.
    mesh.add_attribute("seam", Domain::Edge, Value::Bool(false))
        .unwrap();
    assert_eq!(
        mesh.set_attribute(
            "seam",
            AttributeElement::Edge(EdgeKey::new(v[0], v[0])),
            Value::Bool(true)
        ),
        Err(AttributeError::NoSuchElement)
    );
}

#[test]
fn hard_edges_are_the_built_in_edge_layer() {
    let (mut mesh, v, _) = two_squares();
    let plain = mesh.clone();
    mesh.set_hard(v[1], v[2], true);
    assert_eq!(
        mesh.attributes().collect::<Vec<_>>(),
        vec![(HARD, Domain::Edge, AttributeKind::Bool)]
    );
    let edge = AttributeElement::Edge(EdgeKey::new(v[1], v[2]));
    assert_eq!(mesh.attribute(HARD, edge), Some(Value::Bool(true)));
    // Through the attribute: the same as set_hard.
    mesh.set_attribute(
        HARD,
        AttributeElement::Edge(EdgeKey::new(v[0], v[1])),
        Value::Bool(true),
    )
    .unwrap();
    assert_eq!(mesh.hard_edge_count(), 2);
    assert!(mesh.is_hard(v[1], v[0]));
    mesh.set_hard(v[0], v[1], false);
    mesh.set_attribute(HARD, edge, Value::Bool(false)).unwrap();
    // No hard edges left: no layer, and equal to a mesh that never had any.
    assert_eq!(mesh.attributes().count(), 0);
    assert_eq!(mesh, plain);
}

#[test]
fn rewriting_some_polygons_keeps_the_corner_values_of_the_others() {
    let (mut mesh, v, f) = two_squares();
    mesh.add_attribute("uv", Domain::Corner, uv(0.0, 0.0))
        .unwrap();
    for i in 0..4 {
        mesh.set_attribute("uv", AttributeElement::Corner(f[0], i), uv(i as f32, 0.0))
            .unwrap();
        mesh.set_attribute("uv", AttributeElement::Corner(f[1], i), uv(i as f32, 1.0))
            .unwrap();
    }
    // The first square gets a fifth corner; its corners go back to the
    // default (a tool fills them in through the builder). The second's
    // corners move along in the list, and keep their values.
    let middle = mesh.add_vertex(Vec3::new(0.5, -0.5, 0.0)).unwrap();
    let mut changes = std::collections::BTreeMap::new();
    changes.insert(f[0], vec![v[0], middle, v[1], v[2], v[3]]);
    mesh.rewrite_faces(&changes).unwrap();
    for i in 0..5 {
        assert_eq!(
            mesh.attribute("uv", AttributeElement::Corner(f[0], i)),
            Some(uv(0.0, 0.0))
        );
    }
    for i in 0..4 {
        assert_eq!(
            mesh.attribute("uv", AttributeElement::Corner(f[1], i)),
            Some(uv(i as f32, 1.0))
        );
    }
}

#[test]
fn deleting_polygons_keeps_ids_points_and_the_others_values() {
    let (mut mesh, v, f) = two_squares();
    mesh.add_attribute("uv", Domain::Corner, uv(0.0, 0.0))
        .unwrap();
    mesh.add_attribute("seam", Domain::Edge, Value::Bool(false))
        .unwrap();
    for i in 0..4 {
        mesh.set_attribute("uv", AttributeElement::Corner(f[1], i), uv(i as f32, 1.0))
            .unwrap();
    }
    let seam = |a: VertexId, b: VertexId| AttributeElement::Edge(EdgeKey::new(a, b));
    mesh.set_attribute("seam", seam(v[0], v[3]), Value::Bool(true))
        .unwrap();
    mesh.set_attribute("seam", seam(v[1], v[2]), Value::Bool(true))
        .unwrap();
    mesh.set_hard(v[0], v[1], true);
    let ghost = FaceId::from_raw(99);
    // Unknown and repeated ids are ignored.
    assert_eq!(mesh.delete_faces(&[f[0], f[0], ghost]), 1);
    assert_eq!(mesh.face(f[0]), None);
    assert_eq!(mesh.face_count(), 1);
    assert_eq!(mesh.vertex_count(), 6, "points stay");
    assert_eq!(mesh.topology().edge_count(), 4, "connections are fresh");
    // The other square kept its corner values and its id.
    for i in 0..4 {
        assert_eq!(
            mesh.attribute("uv", AttributeElement::Corner(f[1], i)),
            Some(uv(i as f32, 1.0))
        );
    }
    // Edge values: gone with the edges that went, kept on the shared one.
    assert!(!mesh.is_hard(v[0], v[1]));
    assert_eq!(
        mesh.attribute("seam", seam(v[0], v[3])),
        Some(Value::Bool(false))
    );
    assert_eq!(
        mesh.attribute("seam", seam(v[1], v[2])),
        Some(Value::Bool(true))
    );
    // New polygons never take a deleted polygon's id.
    let again = mesh.add_face(&[v[0], v[1], v[2], v[3]]).unwrap();
    assert_eq!(again, FaceId::from_raw(2));
    // The points the deleted polygon alone used can then go.
    assert_eq!(mesh.delete_faces(&[f[1], again]), 2);
    assert_eq!(mesh.face_count(), 0);
    assert_eq!(mesh.remove_unused_vertices(&v), 6);
    assert_eq!(mesh.delete_faces(&[]), 0);
}

#[test]
fn deleting_from_every_primitive_leaves_a_sound_mesh() {
    for kind in PrimitiveKind::ALL {
        let mut mesh = kind.standard().mesh();
        let half: Vec<FaceId> = mesh.faces().map(|(f, _)| f).step_by(2).collect();
        let n = mesh.face_count();
        assert_eq!(mesh.delete_faces(&half), half.len(), "{kind:?}");
        assert_eq!(mesh.face_count(), n - half.len());
        let report = crate::MeshReport::new(&mesh);
        assert_eq!(report.faces, n - half.len(), "{kind:?}");
        assert!(mesh.faces().all(|(f, _)| !half.contains(&f)));
    }
}

#[test]
fn a_move_shares_the_layers_and_a_value_change_does_not() {
    let (mut mesh, v, _) = two_squares();
    mesh.add_attribute("w", Domain::Point, Value::Float(0.0))
        .unwrap();
    let mut moved = mesh.clone();
    moved.set_position(v[0], Vec3::splat(5.0));
    assert!(moved.same_structure(&mesh), "only positions changed");
    let mut weighted = mesh.clone();
    weighted
        .set_attribute("w", AttributeElement::Point(v[0]), Value::Float(1.0))
        .unwrap();
    assert!(!weighted.same_structure(&mesh));
    assert_ne!(weighted, mesh);
    // (Compared with a plain copy: copies hold no spare room in their
    // lists, so they measure a little smaller than the original.)
    assert!(weighted.memory_size() > mesh.clone().memory_size());
}

#[test]
fn points_added_later_hold_the_default_and_removed_points_lose_edge_values() {
    let (mut mesh, v, _) = two_squares();
    mesh.add_attribute("w", Domain::Point, Value::Float(0.5))
        .unwrap();
    mesh.add_attribute("seam", Domain::Edge, Value::Bool(false))
        .unwrap();
    let loose = mesh.add_vertex(Vec3::splat(9.0)).unwrap();
    assert_eq!(
        mesh.attribute("w", AttributeElement::Point(loose)),
        Some(Value::Float(0.5))
    );
    let edge = AttributeElement::Edge(EdgeKey::new(v[0], loose));
    mesh.set_attribute("seam", edge, Value::Bool(true)).unwrap();
    assert_eq!(mesh.remove_unused_vertices(&[loose]), 1);
    let back = mesh.add_vertex(Vec3::splat(9.0)).unwrap();
    assert_ne!(back, loose, "ids are never reused");
    assert_eq!(mesh.attribute("seam", edge), None, "the point is gone");
}

#[test]
fn values_blend_by_kind() {
    let half = |a, b| Value::mix(a, b, 0.5);
    assert_eq!(
        half(Value::Float(0.0), Value::Float(2.0)),
        Value::Float(1.0)
    );
    assert_eq!(half(uv(0.0, 0.0), uv(1.0, 2.0)), uv(0.5, 1.0));
    assert_eq!(
        half(Value::Vec3(Vec3::ZERO), Value::Vec3(Vec3::ONE)),
        Value::Vec3(Vec3::splat(0.5))
    );
    // On/off and whole numbers take the nearer end.
    assert_eq!(Value::mix(Value::Int(1), Value::Int(5), 0.4), Value::Int(1));
    assert_eq!(
        Value::mix(Value::Bool(false), Value::Bool(true), 0.6),
        Value::Bool(true)
    );
    // Different kinds, or a blend that isn't a number, keep the first.
    assert_eq!(half(Value::Int(1), Value::Float(5.0)), Value::Int(1));
    assert_eq!(
        Value::mix(Value::Float(1.0), Value::Float(3.0), f32::NAN),
        Value::Float(1.0)
    );
    assert!(!Value::Vec2(Vec2::new(0.0, f32::NAN)).is_finite());
}

#[test]
fn whole_layers_go_out_and_come_back_checked() {
    let (mut mesh, v, f) = two_squares();
    mesh.add_attribute("uv", Domain::Corner, uv(0.0, 0.0))
        .unwrap();
    mesh.add_attribute("seam", Domain::Edge, Value::Bool(false))
        .unwrap();
    mesh.set_attribute("uv", AttributeElement::Corner(f[1], 2), uv(0.5, 0.5))
        .unwrap();
    let edge = EdgeKey::new(v[1], v[2]);
    mesh.set_attribute("seam", AttributeElement::Edge(edge), Value::Bool(true))
        .unwrap();
    // Out and back in: the same mesh.
    let mut copy = two_squares().0;
    for name in ["uv", "seam"] {
        let (domain, default, values) = mesh.attribute_values(name).unwrap();
        copy.set_attribute_values(name, domain, default, values)
            .unwrap();
    }
    assert_eq!(copy, mesh);
    assert_eq!(mesh.attribute_values("nope"), None);
    // Everything that doesn't fit is refused.
    let mut m = two_squares().0;
    let refused = |m: &mut Mesh, name, domain, default, values| {
        let before = m.clone();
        let result = m.set_attribute_values(name, domain, default, values);
        assert_eq!(*m, before, "nothing changed");
        result.unwrap_err()
    };
    let nine = AttributeValues::Vec2(vec![Vec2::ZERO; 9]);
    assert_eq!(
        refused(&mut m, "uv", Domain::Corner, uv(0.0, 0.0), nine),
        AttributeError::NoSuchElement,
        "two squares have eight corners"
    );
    assert_eq!(
        refused(
            &mut m,
            "uv",
            Domain::Corner,
            uv(0.0, 0.0),
            AttributeValues::Float(vec![1.0])
        ),
        AttributeError::WrongKind {
            layer: AttributeKind::Vec2,
            value: AttributeKind::Float
        }
    );
    assert_eq!(
        refused(
            &mut m,
            "w",
            Domain::Point,
            Value::Float(0.0),
            AttributeValues::Float(vec![f32::NAN])
        ),
        AttributeError::NotANumber
    );
    let far = EdgeKey::new(v[0], VertexId::from_raw(77));
    assert_eq!(
        refused(
            &mut m,
            "seam",
            Domain::Edge,
            Value::Bool(false),
            AttributeValues::Edges(vec![(far, Value::Bool(true))])
        ),
        AttributeError::NoSuchElement
    );
    assert_eq!(
        refused(
            &mut m,
            "seam",
            Domain::Edge,
            Value::Bool(false),
            AttributeValues::Edges(vec![(edge, Value::Int(1))])
        ),
        AttributeError::WrongKind {
            layer: AttributeKind::Bool,
            value: AttributeKind::Int
        }
    );
    assert!(matches!(
        refused(
            &mut m,
            "seam",
            Domain::Edge,
            Value::Bool(false),
            AttributeValues::Bool(vec![true])
        ),
        AttributeError::WrongDomain { .. }
    ));
    assert!(matches!(
        refused(
            &mut m,
            "w",
            Domain::Point,
            Value::Bool(false),
            AttributeValues::Edges(vec![])
        ),
        AttributeError::WrongDomain { .. }
    ));
    assert_eq!(
        refused(
            &mut m,
            HARD,
            Domain::Edge,
            Value::Bool(false),
            AttributeValues::Edges(vec![])
        ),
        AttributeError::NameTaken(HARD.into()),
        "hard edges are kept with the mesh"
    );
}
