//! Tests for measuring on the surface (decision 0115).

use super::*;
use mesh::{PlaneParams, Primitive, PrimitiveKind};

fn plane() -> Mesh {
    Primitive::Plane(PlaneParams {
        segments: [3, 3],
        ..Default::default()
    })
    .mesh()
}

/// The plane's UV box.
fn bounds(mesh: &Mesh) -> (Vec2, Vec2) {
    let all: Vec<_> = mesh.faces().map(|(f, _)| f).collect();
    mesh::uvs::uv_bounds(mesh, &all, &[]).unwrap()
}

#[test]
fn a_line_across_a_plane_is_as_long_as_the_plane() {
    let mesh = plane();
    let surface = Surface::new(&mesh, Mat4::IDENTITY).unwrap();
    let (min, max) = bounds(&mesh);
    let mid = (min.y + max.y) / 2.0;
    let across = surface_length(
        std::slice::from_ref(&surface),
        Vec2::new(min.x, mid),
        Vec2::new(max.x, mid),
    );
    assert!((across - 100.0).abs() < 1.0, "{across}");
    // Half way across, half as long.
    let half = surface_length(
        std::slice::from_ref(&surface),
        Vec2::new(min.x, mid),
        Vec2::new((min.x + max.x) / 2.0, mid),
    );
    assert!((half - 50.0).abs() < 1.0, "{half}");
}

#[test]
fn the_objects_size_in_the_world_counts() {
    let mesh = plane();
    let surface = Surface::new(&mesh, Mat4::from_scale(Vec3::splat(2.0))).unwrap();
    let (min, max) = bounds(&mesh);
    let mid = (min.y + max.y) / 2.0;
    let across = surface_length(&[surface], Vec2::new(min.x, mid), Vec2::new(max.x, mid));
    assert!((across - 200.0).abs() < 2.0, "{across}");
}

#[test]
fn empty_texture_and_jumps_between_pieces_count_nothing() {
    let mesh = plane();
    let surface = Surface::new(&mesh, Mat4::IDENTITY).unwrap();
    let (_, max) = bounds(&mesh);
    let outside = max + Vec2::splat(0.1);
    assert_eq!(
        surface_length(std::slice::from_ref(&surface), outside, outside + Vec2::X),
        0.0
    );
    assert!(surface.at(outside).is_none());
    // A cube: a line from one side's middle to another's counts only the
    // parts on each side, never the jump between them.
    let cube = PrimitiveKind::Cube.standard().mesh();
    let islands = mesh::uvs::islands(&cube);
    let a = mesh::uvs::uv_centre(&cube, &islands[0], &[]).unwrap();
    let b = mesh::uvs::uv_centre(&cube, &islands[1], &[]).unwrap();
    let s = Surface::new(&cube, Mat4::IDENTITY).unwrap();
    let length = surface_length(std::slice::from_ref(&s), a, b);
    let (pa, pb) = (s.at(a).unwrap().1, s.at(b).unwrap().1);
    assert!(length > 0.0);
    assert!(length < 2.0 * pa.distance(pb) + 100.0, "{length}");
    assert_ne!(s.at(a).unwrap().0, s.at(b).unwrap().0, "two pieces");
}

#[test]
fn a_line_of_no_length_measures_nothing() {
    let mesh = plane();
    let surface = Surface::new(&mesh, Mat4::IDENTITY).unwrap();
    let (min, max) = bounds(&mesh);
    let c = (min + max) / 2.0;
    assert_eq!(surface_length(&[surface], c, c), 0.0);
}

#[test]
fn a_shape_without_uvs_or_polygons_has_no_surface() {
    let mut mesh = Mesh::new();
    assert!(Surface::new(&mesh, Mat4::IDENTITY).is_none());
    let a = mesh.add_vertex(Vec3::ZERO).unwrap();
    let b = mesh.add_vertex(Vec3::X).unwrap();
    let c = mesh.add_vertex(Vec3::Y).unwrap();
    mesh.add_face(&[a, b, c]).unwrap();
    assert!(Surface::new(&mesh, Mat4::IDENTITY).is_none());
    assert_eq!(surface_length(&[], Vec2::ZERO, Vec2::ONE), 0.0);
}
