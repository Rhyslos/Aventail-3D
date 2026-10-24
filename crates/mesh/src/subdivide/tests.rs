use super::*;
use crate::crease::set_creases;
use crate::{MeshReport, Primitive};

/// A cube 100 cm across, centred on the origin (its 12 edges marked hard,
/// with UVs and seams, as the primitive makes it).
fn cube() -> Mesh {
    let mut cube = Primitive::Cube(Default::default()).mesh();
    // The primitive stands on the ground: centre it.
    let ids: Vec<VertexId> = cube.vertices().map(|(v, _)| v).collect();
    for v in ids {
        let p = cube.position(v).unwrap();
        cube.set_position(v, p - Vec3::new(0.0, 0.0, 50.0));
    }
    cube
}

/// A mesh of the given points and polygons.
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

fn assert_clean_solid(mesh: &Mesh) {
    let report = MeshReport::new(mesh);
    assert_eq!(report.border_edges, 0, "{report:?}");
    assert_eq!(report.non_manifold_edges, 0, "{report:?}");
    assert_eq!(report.flipped_edges, 0, "{report:?}");
    assert_eq!(report.loose_points, 0, "{report:?}");
}

fn all_quads(mesh: &Mesh) -> bool {
    mesh.faces().all(|(_, c)| c.len() == 4)
}

fn biggest(mesh: &Mesh) -> f32 {
    mesh.vertices()
        .map(|(_, p)| p.abs().max_element())
        .fold(0.0, f32::max)
}

#[test]
fn a_cube_becomes_a_rounded_closed_solid_of_quads() {
    let cube = cube();
    let one = subdivide(&cube, 1, false).unwrap();
    assert_eq!(one.face_count(), 24);
    assert_eq!(one.vertex_count(), 26);
    assert!(all_quads(&one));
    assert_clean_solid(&one);
    let two = subdivide(&cube, 2, false).unwrap();
    assert_eq!(two.face_count(), 96);
    assert_eq!(two.face_count() as u64, polygon_count(&cube, 2));
    assert_clean_solid(&two);
    // Rounded: nothing reaches the cube's corners any more.
    assert!(biggest(&two) <= 50.0 + 1e-3);
    let corner = two.vertices().map(|(_, p)| p).fold(Vec3::ZERO, |a, p| {
        if p.length() > a.length() { p } else { a }
    });
    assert!(corner.length() < Vec3::splat(50.0).length() * 0.8);
}

#[test]
fn a_corner_moves_where_catmull_clark_puts_it() {
    // A cube of half-size 1: its corner goes to 5/9 of the way out.
    let mut cube = cube();
    let ids: Vec<VertexId> = cube.vertices().map(|(v, _)| v).collect();
    for v in ids {
        let p = cube.position(v).unwrap();
        cube.set_position(v, p / 50.0);
    }
    let one = subdivide(&cube, 1, false).unwrap();
    let expected = 5.0 / 9.0;
    let corners = one
        .vertices()
        .filter(|(_, p)| (p.abs() - Vec3::splat(expected)).abs().max_element() < 1e-5)
        .count();
    assert_eq!(corners, 8);
    // Edge points: the middle of (two ends and two polygon middles).
    let edge_point = Vec3::new(0.75, 0.75, 0.0);
    assert!(
        one.vertices()
            .any(|(_, p)| (p - edge_point).length() < 1e-5)
    );
}

#[test]
fn hard_edges_stay_sharp_when_asked() {
    let cube = cube();
    let sharp = subdivide(&cube, 2, true).unwrap();
    assert_eq!(sharp.face_count(), 96);
    // Every edge of the cube is hard: it stays a cube, every point on
    // its surface.
    for (_, p) in sharp.vertices() {
        assert!((p.abs().max_element() - 50.0).abs() < 1e-3, "{p}");
    }
    assert!((biggest(&sharp) - 50.0).abs() < 1e-3);
    // Hard edges are carried to the halves of the edges.
    assert_eq!(sharp.hard_edge_count(), 12 * 4);
}

#[test]
fn creases_keep_edges_sharp() {
    let mut cube = cube();
    let edges: Vec<EdgeKey> = cube.topology().edges().collect();
    set_creases(&mut cube, &edges, 1.0);
    let creased = subdivide(&cube, 2, false).unwrap();
    for (_, p) in creased.vertices() {
        assert!((p.abs().max_element() - 50.0).abs() < 1e-3, "{p}");
    }
    // Full creases are carried, so baking keeps them.
    assert_eq!(crate::crease::creases(&creased).len(), 12 * 4);
}

#[test]
fn a_partial_crease_is_between_smooth_and_sharp() {
    let reach = |crease: f32| {
        let mut cube = cube();
        let edges: Vec<EdgeKey> = cube.topology().edges().collect();
        set_creases(&mut cube, &edges, crease);
        let out = subdivide(&cube, 2, false).unwrap();
        // How far out the furthest point is.
        out.vertices().map(|(_, p)| p.length()).fold(0.0, f32::max)
    };
    let (smooth, half, sharp) = (reach(0.0), reach(0.5), reach(1.0));
    assert!(smooth < half && half < sharp, "{smooth} {half} {sharp}");
    // Partial creases are used up: none is left on the result.
    let mut cube = cube();
    let edges: Vec<EdgeKey> = cube.topology().edges().collect();
    set_creases(&mut cube, &edges, 0.5);
    assert!(crate::crease::creases(&subdivide(&cube, 2, false).unwrap()).is_empty());
}

#[test]
fn zero_levels_give_the_shape_as_it_is() {
    let cube = cube();
    assert_eq!(subdivide(&cube, 0, true).unwrap(), cube);
    assert_eq!(polygon_count(&cube, 0), 6);
}

#[test]
fn an_n_gon_becomes_one_quad_per_corner_and_its_corners_stay() {
    // A flat pentagon, open all round.
    let points: Vec<[f32; 3]> = (0..5)
        .map(|i| {
            let a = i as f32 * std::f32::consts::TAU / 5.0;
            [a.cos() * 10.0, a.sin() * 10.0, 0.0]
        })
        .collect();
    let pentagon = mesh_of(&points, &[&[0, 1, 2, 3, 4]]);
    let out = subdivide(&pentagon, 1, false).unwrap();
    assert_eq!(out.face_count(), 5);
    assert!(all_quads(&out));
    // A sheet's corners (one polygon each) stay put; it stays flat.
    for p in &points {
        assert!(
            out.vertices()
                .any(|(_, q)| (q - Vec3::from(*p)).length() < 1e-5)
        );
    }
    assert!(out.vertices().all(|(_, p)| p.z.abs() < 1e-6));
    // Facing kept.
    let normal = out.faces().map(|(f, _)| out.face_normal(f)).next().unwrap();
    assert!(normal.z > 0.9);
}

#[test]
fn an_open_border_is_smoothed_along_itself() {
    let plane = Primitive::Plane(crate::PlaneParams {
        width: 100.0,
        depth: 100.0,
        segments: [4, 4],
    })
    .mesh();
    let out = subdivide(&plane, 2, false).unwrap();
    assert_eq!(out.face_count(), 16 * 16);
    let report = MeshReport::new(&out);
    assert_eq!(report.border_edges, 16 * 4);
    assert_eq!(report.non_manifold_edges, 0);
    // Flat stays flat, and the border points stay on the border lines.
    assert!(out.vertices().all(|(_, p)| p.z.abs() < 1e-4));
    assert!((biggest(&out) - 50.0).abs() < 1e-3);
}

#[test]
fn edges_shared_by_three_polygons_are_kept_sharp() {
    // Three quads fanned round one edge.
    let mesh = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 10.0],
            [10.0, 0.0, 0.0],
            [10.0, 0.0, 10.0],
            [-10.0, 0.0, 0.0],
            [-10.0, 0.0, 10.0],
            [0.0, 10.0, 0.0],
            [0.0, 10.0, 10.0],
        ],
        &[&[0, 2, 3, 1], &[1, 5, 4, 0], &[0, 1, 7, 6]],
    );
    let out = subdivide(&mesh, 2, false).unwrap();
    assert_eq!(out.face_count(), 48);
    assert!(out.vertices().all(|(_, p)| p.is_finite()));
    let report = MeshReport::new(&out);
    // The shared edge, in four pieces.
    assert_eq!(report.non_manifold_edges, 4);
    // It stays straight: its points stay on the line x = y = 0.
    let on_line = out
        .vertices()
        .filter(|(_, p)| p.x.abs() < 1e-5 && p.y.abs() < 1e-5)
        .count();
    assert_eq!(on_line, 5);
}

#[test]
fn zero_length_edges_and_flat_polygons_give_real_numbers() {
    // A quad with two corners on the same spot, and a triangle.
    let mesh = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [0.0, 10.0, 0.0],
            [5.0, 5.0, 5.0],
        ],
        &[&[0, 1, 2, 3], &[1, 4, 2]],
    );
    let out = subdivide(&mesh, 3, true).unwrap();
    assert!(out.vertices().all(|(_, p)| p.is_finite()));
    assert_eq!(out.face_count() as u64, polygon_count(&mesh, 3));
}

#[test]
fn empty_and_broken_shapes() {
    let empty = Mesh::new();
    assert_eq!(subdivide(&empty, 2, true).unwrap().face_count(), 0);
    let mut broken = cube();
    let v = broken.vertices().next().unwrap().0;
    broken.set_position(v, Vec3::new(f32::NAN, 0.0, 0.0));
    assert_eq!(
        subdivide(&broken, 1, false),
        Err(SubdivideProblem::NotANumber)
    );
    // Loose points are left out.
    let mut loose = cube();
    loose.add_vertex(Vec3::splat(500.0)).unwrap();
    let out = subdivide(&loose, 1, false).unwrap();
    assert_eq!(out.vertex_count(), 26);
}

#[test]
fn too_many_polygons_are_refused_with_a_reason() {
    let sphere = Primitive::Sphere(crate::SphereParams {
        sides: 128,
        rings: 64,
        ..Default::default()
    })
    .mesh();
    let polygons = polygon_count(&sphere, 5);
    assert!(polygons > SubdivideOptions::MAX_POLYGONS);
    let refused = subdivide(&sphere, 5, false).unwrap_err();
    assert!(matches!(refused, SubdivideProblem::TooMany { .. }));
    assert!(refused.to_string().contains("Lower the levels"));
    // Levels past the most are brought down to it.
    assert_eq!(
        SubdivideOptions {
            levels: 99,
            export_levels: u32::MAX,
            hard_edges_sharp: false,
        }
        .clamped(),
        SubdivideOptions {
            levels: SubdivideOptions::MAX_LEVELS,
            export_levels: SubdivideOptions::MAX_LEVELS,
            hard_edges_sharp: false,
        }
    );
    // Asking for more than the most counts as the most.
    let small = cube();
    assert_eq!(
        subdivide(&small, 99, false).unwrap().face_count() as u64,
        polygon_count(&small, SubdivideOptions::MAX_LEVELS)
    );
}

#[test]
fn materials_are_carried_to_the_new_polygons() {
    let mut cube = cube();
    let faces: Vec<crate::FaceId> = cube.faces().map(|(f, _)| f).collect();
    crate::material::set_material(&mut cube, &faces[..2], 3);
    let out = subdivide(&cube, 2, false).unwrap();
    let with_three = crate::material::faces_with_slot(&out, 3).len();
    assert_eq!(with_three, 2 * 16);
}

#[test]
fn uvs_are_smoothed_and_seams_carried() {
    let cube = cube();
    assert!(crate::uvs::has_uvs(&cube));
    let seams_before = crate::uvs::seams(&cube).len();
    assert!(seams_before > 0);
    let out = subdivide(&cube, 2, false).unwrap();
    assert!(crate::uvs::has_uvs(&out));
    // Each seam in four pieces.
    assert_eq!(crate::uvs::seams(&out).len(), seams_before * 4);
    // Inside the UV range the cube had, and real numbers.
    let range = |mesh: &Mesh| {
        let mut lo = Vec2::splat(f32::MAX);
        let mut hi = Vec2::splat(f32::MIN);
        for (f, c) in mesh.faces() {
            for k in 0..c.len() {
                let uv = crate::uvs::corner_uv(mesh, f, k).unwrap();
                assert!(uv.is_finite());
                lo = lo.min(uv);
                hi = hi.max(uv);
            }
        }
        (lo, hi)
    };
    let (lo, hi) = range(&cube);
    let (out_lo, out_hi) = range(&out);
    assert!(out_lo.cmpge(lo - 1e-5).all() && out_hi.cmple(hi + 1e-5).all());
    // The UVs still split exactly where the seams are.
    let mut again = out.clone();
    crate::uvs::seams_from_splits(&mut again);
    let mut a = crate::uvs::seams(&again);
    let mut b = crate::uvs::seams(&out);
    a.sort();
    b.sort();
    assert_eq!(a, b);
}

#[test]
fn a_uv_corner_with_one_polygon_stays_put() {
    // A single quad with UVs: its four UV corners stay at the corners.
    let mut quad = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [10.0, 10.0, 0.0],
            [0.0, 10.0, 0.0],
        ],
        &[&[0, 1, 2, 3]],
    );
    let uvs = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(1.0, 1.0),
        Vec2::new(0.0, 1.0),
    ];
    quad.set_attribute_values(
        UV,
        Domain::Corner,
        Value::Vec2(Vec2::ZERO),
        AttributeValues::Vec2(uvs.clone()),
    )
    .unwrap();
    let out = subdivide(&quad, 2, false).unwrap();
    let mut found = Vec::new();
    for (f, c) in out.faces() {
        for k in 0..c.len() {
            found.push(crate::uvs::corner_uv(&out, f, k).unwrap());
        }
    }
    for uv in uvs {
        assert!(found.iter().any(|f| (*f - uv).length() < 1e-6), "{uv}");
    }
    // The UVs follow the points: a flat square stays an even grid.
    for (f, c) in out.faces() {
        for (k, v) in c.iter().enumerate() {
            let p = out.position(*v).unwrap();
            let uv = crate::uvs::corner_uv(&out, f, k).unwrap();
            assert!((Vec2::new(p.x, p.y) / 10.0 - uv).length() < 1e-5);
        }
    }
}

#[test]
fn the_same_shape_always_gives_the_same_result() {
    let torus = Primitive::Torus(Default::default()).mesh();
    let a = subdivide(&torus, 2, true).unwrap();
    let b = subdivide(&torus, 2, true).unwrap();
    assert_eq!(a, b);
    assert_clean_solid(&a);
}

#[test]
fn every_primitive_subdivides_cleanly() {
    for kind in crate::PrimitiveKind::ALL {
        let mesh = kind.standard().mesh();
        let before = MeshReport::new(&mesh);
        let out = subdivide(&mesh, 1, true).unwrap();
        let after = MeshReport::new(&out);
        assert!(out.vertices().all(|(_, p)| p.is_finite()), "{kind:?}");
        assert_eq!(out.face_count() as u64, polygon_count(&mesh, 1), "{kind:?}");
        assert_eq!(after.flipped_edges, 0, "{kind:?}");
        assert_eq!(after.non_manifold_edges, 0, "{kind:?}");
        // Open borders double in edges; closed shapes stay closed.
        assert_eq!(after.border_edges, before.border_edges * 2, "{kind:?}");
    }
}

/// Timings, in a release build: `cargo test --release -p mesh
/// subdivide_timing -- --ignored --nocapture`.
#[test]
#[ignore = "a timing, for release builds"]
fn subdivide_timing() {
    // A sphere of about 10 000 polygons.
    let sphere = Primitive::Sphere(crate::SphereParams {
        sides: 128,
        rings: 80,
        ..Default::default()
    })
    .mesh();
    println!("input: {} polygons", sphere.face_count());
    for levels in 1..=3 {
        let start = std::time::Instant::now();
        let out = subdivide(&sphere, levels, true).unwrap();
        println!(
            "level {levels}: {} polygons in {:.1} ms",
            out.face_count(),
            start.elapsed().as_secs_f64() * 1e3
        );
    }
}
