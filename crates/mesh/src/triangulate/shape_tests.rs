//! Tests for the Triangulate modifier's work and Show triangles' lines
//! (decision 0161).

use super::*;
use crate::attributes::{AttributeElement, UV, Value};
use crate::custom_normals::{custom_normal_list, set_custom_normals};
use crate::material::{set_material, slot_of};
use crate::primitives::{CylinderParams, PlaneParams, SphereParams};
use crate::shading::corner_shading_with;
use crate::{FaceId, Hardness, Mesh, MeshReport, Primitive, VertexId};
use glam::{Vec2, Vec3};

fn mesh_of(points: &[[f32; 3]], faces: &[&[u32]]) -> Mesh {
    let mut mesh = Mesh::new();
    let ids: Vec<VertexId> = points
        .iter()
        .map(|p| mesh.add_vertex(Vec3::from(*p)).unwrap())
        .collect();
    for face in faces {
        let corners: Vec<VertexId> = face.iter().map(|i| ids[*i as usize]).collect();
        mesh.add_face(&corners).unwrap();
    }
    mesh
}

fn cube() -> Mesh {
    Primitive::Cube(Default::default()).mesh()
}

fn cylinder() -> Mesh {
    Primitive::Cylinder(CylinderParams::default()).mesh()
}

fn fixed() -> TriangulateOptions {
    TriangulateOptions {
        method: TriangulateMethod::Fixed,
        quads_only: false,
    }
}

fn beauty() -> TriangulateOptions {
    TriangulateOptions::default()
}

/// A flat circle of `n` corners as one polygon.
fn circle(n: u32) -> Mesh {
    let points: Vec<[f32; 3]> = (0..n)
        .map(|i| {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            [a.cos() * 50.0, a.sin() * 50.0, 0.0]
        })
        .collect();
    let face: Vec<u32> = (0..n).collect();
    mesh_of(&points, &[&face])
}

/// The smallest corner of any triangle of `mesh`, in degrees.
fn smallest_corner(mesh: &Mesh) -> f32 {
    let mut least = 180.0f32;
    for (_, c) in mesh.faces() {
        let p: Vec<Vec3> = c.iter().map(|v| mesh.position(*v).unwrap()).collect();
        for i in 0..p.len() {
            let n = p.len();
            let here = p[i];
            let angle = (p[(i + n - 1) % n] - here).angle_between(p[(i + 1) % n] - here);
            least = least.min(angle.to_degrees());
        }
    }
    least
}

fn all_triangles(mesh: &Mesh) -> bool {
    mesh.faces().all(|(_, c)| c.len() == 3)
}

/// Every triangle of `mesh` facing the same way as the original polygon
/// `like` it came from (all of `original`'s polygons are flat here).
fn faces_kept_their_way(original: &Mesh, mesh: &Mesh) {
    for (face, corners) in mesh.faces() {
        let normal = mesh.face_normal(face);
        let from = original
            .faces()
            .find(|(_, c)| corners.iter().all(|v| c.contains(v)))
            .map(|(f, _)| f)
            .expect("each triangle lies in a polygon of the original");
        assert!(
            normal.dot(original.face_normal(from)) > 0.99,
            "{face:?} faces another way"
        );
    }
}

#[test]
fn a_cube_becomes_twelve_triangles_keeping_its_ids_and_points() {
    let before = cube();
    let after = triangulated(&before, &fixed()).unwrap();
    assert_eq!(after.face_count(), 12);
    assert!(all_triangles(&after));
    // The points are the same points in the same places.
    assert_eq!(
        before.vertices().collect::<Vec<_>>(),
        after.vertices().collect::<Vec<_>>()
    );
    // Each polygon's first triangle keeps its id; the rest come after.
    for (face, _) in before.faces() {
        assert!(after.face(face).is_some());
    }
    let ids: Vec<u32> = after.faces().map(|(f, _)| f.get()).collect();
    assert_eq!(ids, (0..12).collect::<Vec<_>>());
    // Still a closed solid with nothing wrong.
    let report = MeshReport::new(&after);
    assert!(report.is_closed() && !report.has_problems(), "{report:?}");
    faces_kept_their_way(&before, &after);
}

#[test]
fn fixed_is_exactly_the_export_cut() {
    // A dented quad, a bent quad, a flat n-gon and a dented n-gon.
    let mesh = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [4.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 4.0, 0.0],
            [10.0, 0.0, 0.0],
            [12.0, 0.0, 1.0],
            [12.0, 2.0, 0.0],
            [10.0, 2.0, 1.0],
            [20.0, 0.0, 0.0],
            [24.0, 0.0, 0.0],
            [26.0, 2.0, 0.0],
            [24.0, 4.0, 0.0],
            [20.0, 4.0, 0.0],
            [22.0, 2.0, 0.0],
        ],
        &[
            &[0, 1, 2, 3],
            &[4, 5, 6, 7],
            &[8, 9, 10, 11, 12],
            &[8, 9, 10, 11, 12, 13],
        ],
    );
    // The last two share points but are separate polygons: still fine.
    let after = triangulated(&mesh, &fixed()).unwrap();
    let mut expected: Vec<Vec<VertexId>> = Vec::new();
    for (_, corners) in mesh.faces() {
        crate::face_triangles(&mesh, corners, |t| {
            let mut tri: Vec<VertexId> = t.iter().map(|i| corners[*i]).collect();
            let low = (0..3).min_by_key(|i| tri[*i]).unwrap();
            tri.rotate_left(low);
            expected.push(tri);
        });
    }
    let mut got: Vec<Vec<VertexId>> = after
        .faces()
        .map(|(_, c)| {
            let mut tri = c.to_vec();
            let low = (0..3).min_by_key(|i| tri[*i]).unwrap();
            tri.rotate_left(low);
            tri
        })
        .collect();
    expected.sort();
    got.sort();
    assert_eq!(got, expected);
}

#[test]
fn inner_lines_are_the_cuts_inside_polygons() {
    assert_eq!(inner_lines(&cube()).len(), 6);
    assert_eq!(inner_lines(&circle(12)).len(), 9);
    let triangles = triangulated(&cube(), &fixed()).unwrap();
    assert!(inner_lines(&triangles).is_empty());
    assert!(inner_lines(&Mesh::new()).is_empty());
    // A cube's lines are its faces' diagonals: corner to opposite corner.
    for [a, b] in inner_lines(&cube()) {
        let d = (a - b).abs();
        let zeros = d.to_array().iter().filter(|x| **x < 1e-4).count();
        assert_eq!(zeros, 1, "{a} to {b} isn't a face's diagonal");
    }
}

#[test]
fn show_triangles_lines_are_the_fixed_results_new_edges() {
    let mesh = cylinder();
    let after = triangulated(&mesh, &fixed()).unwrap();
    let old: std::collections::BTreeSet<_> = mesh.edges().iter().copied().collect();
    let new: Vec<_> = after
        .edges()
        .iter()
        .filter(|e| !old.contains(e))
        .map(|e| {
            let (a, b) = e.ends();
            (after.position(a).unwrap(), after.position(b).unwrap())
        })
        .collect();
    let lines = inner_lines(&mesh);
    assert_eq!(new.len(), lines.len());
    for [p, q] in lines {
        assert!(
            new.iter()
                .any(|(a, b)| (*a == p && *b == q) || (*a == q && *b == p))
        );
    }
}

#[test]
fn beauty_evens_out_a_flat_ngon() {
    // A long strip with points along both sides: the export's fan from
    // its first corner makes slivers.
    let mut points = Vec::new();
    for i in 0..=10 {
        points.push([i as f32 * 10.0, 0.0, 0.0]);
    }
    for i in (0..=10).rev() {
        points.push([i as f32 * 10.0, 10.0, 0.0]);
    }
    let face: Vec<u32> = (0..points.len() as u32).collect();
    let mesh = mesh_of(&points, &[&face]);
    let fan = triangulated(&mesh, &fixed()).unwrap();
    let even = triangulated(&mesh, &beauty()).unwrap();
    assert_eq!(even.face_count(), 20);
    assert!(all_triangles(&even));
    assert!(
        smallest_corner(&even) > smallest_corner(&fan) + 20.0,
        "{} against {}",
        smallest_corner(&even),
        smallest_corner(&fan)
    );
    faces_kept_their_way(&mesh, &even);
    // Covers the circle exactly: the same area.
    let area = |m: &Mesh| -> f32 {
        m.faces()
            .map(|(_, c)| {
                let p: Vec<Vec3> = c.iter().map(|v| m.position(*v).unwrap()).collect();
                (p[1] - p[0]).cross(p[2] - p[0]).z / 2.0
            })
            .sum()
    };
    assert!((area(&even) - area(&fan)).abs() < 1e-2);
}

#[test]
fn beauty_turns_a_quads_cut_to_the_better_diagonal() {
    // A long thin quad whose 0–2 line makes a sliver: 1–3 is better.
    let mesh = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [5.0, -1.0, 0.0],
            [10.0, 0.0, 0.0],
            [5.0, 1.0, 0.0],
        ],
        &[&[0, 1, 2, 3]],
    );
    let fixed_cut = triangulated(&mesh, &fixed()).unwrap();
    let even = triangulated(&mesh, &beauty()).unwrap();
    assert!(smallest_corner(&even) > smallest_corner(&fixed_cut));
    // A square has two equal choices: the export's stays.
    let square = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ],
        &[&[0, 1, 2, 3]],
    );
    assert_eq!(
        triangulated(&square, &beauty())
            .unwrap()
            .faces()
            .collect::<Vec<_>>(),
        triangulated(&square, &fixed())
            .unwrap()
            .faces()
            .collect::<Vec<_>>()
    );
}

#[test]
fn beauty_keeps_a_bent_quads_fold() {
    // Bent along 0–2 by a lot: turning the line would change its shape.
    let mesh = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [5.0, -1.0, 3.0],
            [10.0, 0.0, 0.0],
            [5.0, 1.0, 3.0],
        ],
        &[&[0, 1, 2, 3]],
    );
    assert_eq!(
        triangulated(&mesh, &beauty())
            .unwrap()
            .faces()
            .collect::<Vec<_>>(),
        triangulated(&mesh, &fixed())
            .unwrap()
            .faces()
            .collect::<Vec<_>>()
    );
}

#[test]
fn beauty_never_does_worse_than_fixed() {
    for mesh in [
        cylinder(),
        cube(),
        circle(5),
        circle(40),
        Primitive::Sphere(SphereParams::default()).mesh(),
    ] {
        let a = triangulated(&mesh, &fixed()).unwrap();
        let b = triangulated(&mesh, &beauty()).unwrap();
        assert_eq!(a.face_count(), b.face_count());
        assert!(smallest_corner(&b) >= smallest_corner(&a) - 1e-3);
        let report = MeshReport::new(&b);
        assert_eq!(report.is_closed(), MeshReport::new(&mesh).is_closed());
    }
}

#[test]
fn quads_only_leaves_bigger_polygons() {
    let mesh = cylinder();
    let options = TriangulateOptions {
        quads_only: true,
        ..fixed()
    };
    let after = triangulated(&mesh, &options).unwrap();
    let sizes: Vec<usize> = after.faces().map(|(_, c)| c.len()).collect();
    assert_eq!(sizes.iter().filter(|n| **n == 16).count(), 2);
    assert_eq!(sizes.iter().filter(|n| **n == 3).count(), 32);
    assert_eq!(sizes.len(), 34);
    assert_eq!(corners_to_cut(&mesh, &options), 16 * 4);
    assert_eq!(corners_to_cut(&mesh, &fixed()), 16 * 4 + 32);
}

#[test]
fn nothing_to_cut_is_refused_with_a_reason() {
    assert_eq!(
        triangulated(&Mesh::new(), &fixed()).unwrap_err(),
        TriangulateProblem::Empty
    );
    let triangles = triangulated(&cube(), &fixed()).unwrap();
    assert_eq!(
        triangulated(&triangles, &fixed()).unwrap_err(),
        TriangulateProblem::AllTriangles
    );
    let quads_only = TriangulateOptions {
        quads_only: true,
        ..fixed()
    };
    assert_eq!(
        triangulated(&circle(6), &quads_only).unwrap_err(),
        TriangulateProblem::NoQuads
    );
    for problem in [
        TriangulateProblem::Empty,
        TriangulateProblem::AllTriangles,
        TriangulateProblem::NoQuads,
    ] {
        let text = problem.to_string();
        assert!(text.starts_with(char::is_lowercase), "{text}");
    }
}

#[test]
fn uvs_materials_and_edge_marks_come_along() {
    let mut mesh = cube();
    let top = mesh.faces().map(|(f, _)| f).nth(1).unwrap();
    set_material(&mut mesh, &[top], 2);
    let (a, b) = mesh.edges()[0].ends();
    mesh.set_hard(a, b, true);
    let after = triangulated(&mesh, &beauty()).unwrap();
    assert!(after.is_hard(a, b));
    assert_eq!(after.hard_edge_count(), mesh.hard_edge_count());
    let uv_of =
        |m: &Mesh, f: FaceId, i: usize| match m.attribute(UV, AttributeElement::Corner(f, i)) {
            Some(Value::Vec2(uv)) => uv,
            other => panic!("{other:?}"),
        };
    // Every triangle's corner has the UV its polygon had at that point.
    for (face, corners) in after.faces() {
        let (from, from_corners) = mesh
            .faces()
            .find(|(_, c)| corners.iter().all(|v| c.contains(v)))
            .unwrap();
        for (i, v) in corners.iter().enumerate() {
            let j = from_corners.iter().position(|w| w == v).unwrap();
            assert_eq!(uv_of(&after, face, i), uv_of(&mesh, from, j));
        }
        assert_eq!(slot_of(&after, face), slot_of(&mesh, from));
    }
    let uvs: Vec<Vec2> = (0..3).map(|i| uv_of(&after, top, i)).collect();
    assert_eq!(uvs.len(), 3);
}

#[test]
fn custom_normals_come_along() {
    let mut mesh = cylinder();
    let (_, corners) = mesh.face_layout();
    let normals: Vec<Vec3> = (0..corners.len())
        .map(|k| Vec3::new(k as f32, 1.0, 2.0).normalize())
        .collect();
    set_custom_normals(&mut mesh, &normals);
    let after = triangulated(&mesh, &beauty()).unwrap();
    let list = custom_normal_list(&after).expect("still has custom normals");
    let (starts, corners) = after.face_layout();
    let (old_starts, old_corners) = mesh.face_layout();
    for (f, pair) in starts.windows(2).enumerate() {
        let tri = &corners[pair[0] as usize..pair[1] as usize];
        // The polygon it came from, and each corner's normal there.
        let (g, _) = old_starts
            .windows(2)
            .enumerate()
            .find(|(_, w)| {
                let c = &old_corners[w[0] as usize..w[1] as usize];
                tri.iter().all(|v| c.contains(v))
            })
            .unwrap();
        for (i, v) in tri.iter().enumerate() {
            let k = (old_starts[g]..old_starts[g + 1])
                .find(|k| old_corners[*k as usize] == *v)
                .unwrap();
            let got = list[pair[0] as usize + i].unwrap();
            assert!(
                got.distance(normals[k as usize]) < 1e-5,
                "face {f} corner {i}"
            );
        }
    }
}

#[test]
fn the_shading_stays_the_same_on_flat_polygons() {
    for hardness in [Hardness::Marked, Hardness::ByAngle(30.0)] {
        let mesh = cylinder();
        let after = triangulated(&mesh, &beauty()).unwrap();
        let normals_at = |m: &Mesh| {
            let (_, normals) = corner_shading_with(m, hardness);
            let (_, corners) = m.face_layout();
            let mut by_point: std::collections::BTreeMap<VertexId, Vec<Vec3>> = Default::default();
            for (k, v) in corners.iter().enumerate() {
                by_point.entry(*v).or_default().push(normals[k]);
            }
            by_point
        };
        let (before, now) = (normals_at(&mesh), normals_at(&after));
        for (v, list) in &now {
            for n in list {
                assert!(
                    before[v].iter().any(|m| m.distance(*n) < 1e-4),
                    "{v:?}: {n} is new"
                );
            }
        }
    }
}

#[test]
fn open_borders_non_manifold_and_degenerate_polygons_are_fine() {
    // An open grid.
    let plane = Primitive::Plane(PlaneParams {
        width: 100.0,
        depth: 100.0,
        segments: [3, 3],
    })
    .mesh();
    let after = triangulated(&plane, &beauty()).unwrap();
    assert_eq!(after.face_count(), 18);
    assert!(!MeshReport::new(&after).is_closed());
    // Three quads on one edge.
    let fan = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 1.0],
            [-1.0, -1.0, 0.0],
            [-1.0, -1.0, 1.0],
        ],
        &[&[0, 2, 3, 1], &[0, 1, 5, 4], &[0, 6, 7, 1]],
    );
    assert_eq!(triangulated(&fan, &beauty()).unwrap().face_count(), 6);
    // A zero-length edge, and a polygon with every point in one place.
    let squashed = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [5.0, 5.0, 5.0],
            [5.0, 5.0, 5.0],
            [5.0, 5.0, 5.0],
            [5.0, 5.0, 5.0],
            [5.0, 5.0, 5.0],
        ],
        &[&[0, 1, 2, 3, 4], &[5, 6, 7, 8, 9]],
    );
    for options in [fixed(), beauty()] {
        let after = triangulated(&squashed, &options).unwrap();
        assert_eq!(after.face_count(), 6);
        assert!(all_triangles(&after));
    }
    // Huge and tiny coordinates.
    let far = mesh_of(
        &[
            [1e30, 0.0, 0.0],
            [1e30, 1.0, 0.0],
            [-1e30, 1.0, 0.0],
            [-1e30, 0.0, 0.0],
            [0.0, 1e-30, 0.0],
        ],
        &[&[0, 1, 2, 4, 3]],
    );
    assert_eq!(triangulated(&far, &beauty()).unwrap().face_count(), 3);
}

#[test]
fn the_same_shape_always_gives_the_same_result() {
    let mesh = Primitive::Sphere(SphereParams::default()).mesh();
    for options in [fixed(), beauty()] {
        let a = triangulated(&mesh, &options).unwrap();
        let b = triangulated(&mesh, &options).unwrap();
        assert_eq!(a.faces().collect::<Vec<_>>(), b.faces().collect::<Vec<_>>());
    }
}

#[test]
fn methods_have_names_and_explanations() {
    for method in TriangulateMethod::ALL {
        assert!(!method.name().is_empty() && !method.explanation().is_empty());
    }
    assert_eq!(
        TriangulateOptions::default().method,
        TriangulateMethod::Beauty
    );
    assert!(!TriangulateOptions::default().quads_only);
    assert_eq!(fixed().clamped(), fixed());
}

/// Timings, in a release build: `cargo test -p mesh --release
/// triangulate_timing -- --ignored --nocapture`.
#[test]
#[ignore = "a timing, run by hand"]
fn triangulate_timing() {
    let mesh = Primitive::Sphere(SphereParams {
        radius: 50.0,
        sides: 400,
        rings: 250,
    })
    .mesh();
    for options in [fixed(), beauty()] {
        let start = std::time::Instant::now();
        let after = triangulated(&mesh, &options).unwrap();
        println!(
            "{:?}: {} polygons into {} triangles in {:?}",
            options.method,
            mesh.face_count(),
            after.face_count(),
            start.elapsed()
        );
    }
    let start = std::time::Instant::now();
    let lines = inner_lines(&mesh);
    println!("{} inner lines in {:?}", lines.len(), start.elapsed());
}
