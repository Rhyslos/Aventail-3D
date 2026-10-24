use super::*;
use crate::Primitive;
use crate::primitives::{CubeParams, SphereParams};
use glam::Mat4;

fn sphere(sides: u32, rings: u32) -> crate::Mesh {
    Primitive::Sphere(SphereParams {
        radius: 50.0,
        sides,
        rings,
    })
    .mesh()
}

/// A little random number stream (xorshift), the same every run.
fn stream(mut seed: u64) -> impl FnMut() -> f32 {
    move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 40) as f32 / (1u64 << 24) as f32
    }
}

fn triangles_of(mesh: &crate::Mesh) -> Vec<[Vec3; 3]> {
    let mut out = Vec::new();
    for (_, corners) in mesh.faces() {
        let points: Vec<Vec3> = corners.iter().map(|v| mesh.position(*v).unwrap()).collect();
        crate::face_triangles(mesh, corners, |[a, b, c]| {
            out.push([points[a], points[b], points[c]]);
        });
    }
    out
}

#[test]
fn the_nearest_place_matches_looking_at_every_triangle() {
    let mesh = sphere(24, 12);
    let triangles = triangles_of(&mesh);
    let tree = TriangleTree::new(&triangles);
    assert_eq!(tree.len(), triangles.len());
    let mut random = stream(7);
    for _ in 0..300 {
        let p = Vec3::new(random(), random(), random()) * 300.0 - Vec3::splat(150.0);
        let hit = tree.nearest(p).unwrap();
        let best = triangles
            .iter()
            .map(|t| closest_on_triangle(p, t[0], t[1], t[2]).distance(p))
            .fold(f32::INFINITY, f32::min);
        assert!(
            (hit.distance - best).abs() < 1e-3,
            "{p}: {} {best}",
            hit.distance
        );
        assert!((hit.point.distance(p) - hit.distance).abs() < 1e-3);
        assert!((hit.normal.length() - 1.0).abs() < 1e-4);
    }
}

#[test]
fn a_line_meets_the_surface_nearest_its_start_either_way() {
    let mesh = sphere(24, 12);
    let tree = TriangleTree::of_mesh(&mesh, Mat4::IDENTITY);
    // From the middle, up and down: the sphere's top and bottom.
    let centre = Vec3::new(0.0, 0.0, 50.0);
    let up = tree.along_line(centre, Vec3::Z).unwrap();
    assert!((up.distance.abs() - 50.0).abs() < 0.5, "{}", up.distance);
    // From above, looking up: the sphere is behind, so the distance is
    // negative, at the top.
    let above = tree
        .along_line(Vec3::new(0.0, 0.0, 130.0), Vec3::Z)
        .unwrap();
    assert!((above.distance + 30.0).abs() < 0.5, "{}", above.distance);
    assert!((above.point.z - 100.0).abs() < 0.5);
    assert!(above.normal.z > 0.9, "the top faces up");
    // Off to the side, a line along X misses it.
    assert!(
        tree.along_line(Vec3::new(0.0, 200.0, 50.0), Vec3::X)
            .is_none()
    );
    // No way at all.
    assert!(tree.along_line(centre, Vec3::ZERO).is_none());
}

#[test]
fn lines_match_looking_at_every_triangle() {
    let mesh = sphere(16, 8);
    let triangles = triangles_of(&mesh);
    let tree = TriangleTree::new(&triangles);
    let mut random = stream(99);
    for _ in 0..300 {
        let origin = Vec3::new(random(), random(), random()) * 200.0 - Vec3::splat(100.0);
        let way = (Vec3::new(random(), random(), random()) - Vec3::splat(0.5)).normalize();
        let best = triangles
            .iter()
            .filter_map(|t| line_meets_triangle(origin, way, t))
            .min_by(|a, b| a.abs().total_cmp(&b.abs()));
        let hit = tree.along_line(origin, way);
        match (best, hit) {
            (None, None) => {}
            (Some(t), Some(hit)) => assert!((t.abs() - hit.distance.abs()).abs() < 1e-3),
            other => panic!("{other:?}"),
        }
    }
}

#[test]
fn triangles_without_a_surface_are_left_out() {
    let good = [Vec3::ZERO, Vec3::X, Vec3::Y];
    let tree = TriangleTree::new(&[
        [Vec3::ZERO, Vec3::ZERO, Vec3::ZERO],
        [Vec3::NAN, Vec3::X, Vec3::Y],
        [Vec3::ZERO, Vec3::X, Vec3::X * 2.0],
        good,
        [Vec3::INFINITY, Vec3::X, Vec3::Y],
    ]);
    assert_eq!(tree.len(), 1);
    let hit = tree.nearest(Vec3::new(0.2, 0.2, 5.0)).unwrap();
    assert_eq!(hit.triangle, 3, "the place in the list given");
    assert!((hit.distance - 5.0).abs() < 1e-5);
}

#[test]
fn an_empty_tree_finds_nothing() {
    let tree = TriangleTree::new(&[]);
    assert!(tree.is_empty());
    assert!(tree.nearest(Vec3::ZERO).is_none());
    assert!(tree.along_line(Vec3::ZERO, Vec3::Z).is_none());
    let empty = TriangleTree::of_mesh(&crate::Mesh::new(), Mat4::IDENTITY);
    assert!(empty.is_empty());
    let one = TriangleTree::new(&[[Vec3::ZERO, Vec3::X, Vec3::Y]]);
    assert!(one.nearest(Vec3::NAN).is_none());
    assert!(one.along_line(Vec3::NAN, Vec3::Z).is_none());
}

#[test]
fn many_triangles_in_one_spot_never_loop() {
    // Every triangle the same: splitting can't separate them by place.
    let t = [Vec3::ZERO, Vec3::X, Vec3::Y];
    let tree = TriangleTree::new(&vec![t; 1000]);
    assert_eq!(tree.len(), 1000);
    let hit = tree.nearest(Vec3::new(0.1, 0.1, 1.0)).unwrap();
    assert!((hit.distance - 1.0).abs() < 1e-5);
    assert!(
        tree.along_line(Vec3::new(0.1, 0.1, 1.0), -Vec3::Z)
            .is_some()
    );
}

#[test]
fn the_same_triangles_give_the_same_answers() {
    let mesh = Primitive::Cube(CubeParams {
        size: Vec3::splat(100.0),
        segments: [3, 3, 3],
    })
    .mesh();
    let a = TriangleTree::of_mesh(&mesh, Mat4::IDENTITY);
    let b = TriangleTree::of_mesh(&mesh, Mat4::IDENTITY);
    // A point equally near several triangles: always the same one.
    for p in [Vec3::new(0.0, 0.0, 200.0), Vec3::new(80.0, 80.0, 120.0)] {
        assert_eq!(a.nearest(p), b.nearest(p));
        assert_eq!(a.along_line(p, -Vec3::Z), b.along_line(p, -Vec3::Z));
    }
}

#[test]
fn a_matrix_moves_the_surface() {
    let mesh = sphere(16, 8);
    let tree = TriangleTree::of_mesh(&mesh, Mat4::from_translation(Vec3::new(1000.0, 0.0, 0.0)));
    let hit = tree.nearest(Vec3::new(1000.0, 0.0, 200.0)).unwrap();
    assert!((hit.point.z - 100.0).abs() < 0.5);
}
