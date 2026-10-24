//! Tests for struts (decision 0159).

use super::*;
use crate::material::{set_material, slot_of};
use crate::primitives::{CubeParams, PlaneParams, SphereParams};
use crate::uvs::{has_uvs, islands, seams};
use crate::{MeshReport, Primitive};
use glam::{Vec2, Vec3};

const MOST: usize = 30_000_000;

fn cube() -> Mesh {
    Primitive::Cube(CubeParams::default()).mesh()
}

fn plane(segments: u32) -> Mesh {
    Primitive::Plane(PlaneParams {
        width: 100.0,
        depth: 100.0,
        segments: [segments, segments],
    })
    .mesh()
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

fn sides(sides: u32) -> StrutsOptions {
    StrutsOptions {
        sides,
        ..StrutsOptions::default()
    }
}

/// The result's polygons, in pieces (each a list of polygons).
fn pieces_of(mesh: &Mesh) -> Vec<Vec<FaceId>> {
    mesh.topology().pieces()
}

/// Checks every piece of `mesh` is closed, two polygons to each edge,
/// and every polygon faces out of its piece (the piece's volume is
/// positive).
fn every_piece_closed_and_outward(mesh: &Mesh) {
    let report = MeshReport::new(mesh);
    assert_eq!(report.border_edges, 0, "{report:?}");
    assert_eq!(report.non_manifold_edges, 0, "{report:?}");
    assert_eq!(report.flipped_edges, 0, "{report:?}");
    for piece in pieces_of(mesh) {
        let mut volume = 0.0f32;
        for face in &piece {
            crate::triangulate::face_triangles(mesh, mesh.face(*face).unwrap(), |[a, b, c]| {
                let corners = mesh.face(*face).unwrap();
                let p = |i: usize| mesh.position(corners[i]).unwrap();
                volume += p(a).dot(p(b).cross(p(c))) / 6.0;
            });
        }
        assert!(volume > 0.0, "a piece faces inward: {volume}");
    }
}

#[test]
fn a_cube_becomes_twelve_struts_and_eight_joints() {
    let input = cube();
    let out = struts(&input, &StrutsOptions::default(), MOST).unwrap();
    assert_eq!(pieces_of(&out).len(), 12 + 8);
    // Square struts: 4 sides and 2 ends; joints: cubes.
    assert_eq!(out.face_count(), 12 * 6 + 8 * 6);
    assert_eq!(
        polygon_count(&input, &StrutsOptions::default()),
        12 * 6 + 8 * 6
    );
    every_piece_closed_and_outward(&out);
}

#[test]
fn a_square_strut_is_exactly_its_thickness_wide() {
    let edge = mesh_of(
        &[[0.0, 0.0, 0.0], [100.0, 0.0, 0.0], [0.0, 100.0, 0.0]],
        &[&[0, 1, 2]],
    );
    let out = struts(&edge, &StrutsOptions::default(), MOST).unwrap();
    // The strut along X (between the first two points) lies flat: its
    // points are 1 cm above and below the triangle, and 1 cm to each
    // side of the edge.
    let along_x: Vec<Vec3> = pieces_of(&out)
        .iter()
        .map(|piece| {
            let mut points: Vec<Vec3> = piece
                .iter()
                .flat_map(|f| out.face(*f).unwrap().iter())
                .map(|v| out.position(*v).unwrap())
                .collect();
            points.dedup();
            points
        })
        .find(|points| {
            points.iter().any(|p| p.x > 99.0)
                && points.iter().any(|p| p.x < 1.0)
                && points.iter().all(|p| p.y.abs() < 2.0)
        })
        .expect("the strut along X");
    for p in along_x {
        assert!((p.y.abs() - 1.0).abs() < 1e-4, "{p}");
        assert!((p.z.abs() - 1.0).abs() < 1e-4, "{p}");
    }
}

#[test]
fn even_thickness_off_measures_across_the_corners() {
    let input = cube();
    let off = StrutsOptions {
        even: false,
        ..StrutsOptions::default()
    };
    let on = struts(&input, &StrutsOptions::default(), MOST).unwrap();
    let off = struts(&input, &off, MOST).unwrap();
    let size = |m: &Mesh| m.bounds().unwrap().size().x;
    // The joints, cubes 2 × the corner radius wide, stick out furthest.
    let corner_on = 2.0_f32.sqrt();
    let corner_off = 1.0;
    assert!((size(&on) - (100.0 + 2.0 * corner_on)).abs() < 1e-3);
    assert!((size(&off) - (100.0 + 2.0 * corner_off)).abs() < 1e-3);
}

#[test]
fn joints_cover_the_struts_ends() {
    for n in [3, 4, 5, 6, 8, 12, 24] {
        let options = sides(n);
        let input = cube();
        let (_, corner) = options.radii();
        let corner_point = input.vertices().next().unwrap().0;
        let joint = pieces::joint(&input, corner_point, &options).unwrap();
        let centre = input.position(corner_point).unwrap();
        // Every face of the joint is at least the strut's corner radius
        // from the point (so the end, a polygon that size round the
        // point, is inside).
        let mut mesh = Mesh::new();
        let made = joint.add_to(&mut mesh).unwrap();
        for (face, _) in &made.faces {
            let corners = mesh.face(*face).unwrap();
            let normal = mesh.face_normal(*face);
            let distance = (mesh.position(corners[0]).unwrap() - centre).dot(normal);
            assert!(
                distance >= corner * 0.999,
                "{n} sides: {distance} < {corner}"
            );
        }
        every_piece_closed_and_outward(&mesh);
    }
}

#[test]
fn sides_change_the_profile_and_round_struts_shade_smooth() {
    let input = cube();
    let square = struts(&input, &sides(4), MOST).unwrap();
    let round = struts(&input, &sides(12), MOST).unwrap();
    let hexagon = struts(&input, &sides(6), MOST).unwrap();
    // A square strut's corners and ends, and a cube joint's every edge.
    assert_eq!(square.hard_edge_count(), 12 * 12 + 8 * 12);
    // A hexagonal one is hard all over too.
    assert_eq!(
        hexagon.hard_edge_count(),
        12 * (6 * 3) + 8 * (6 * 2 + 6 * 3)
    );
    // A round strut's ends only; round joints have none.
    assert_eq!(round.hard_edge_count(), 12 * 24);
    every_piece_closed_and_outward(&round);
    every_piece_closed_and_outward(&hexagon);
}

#[test]
fn n_gons_open_borders_and_non_manifold_edges_all_get_struts() {
    // A pentagon, an open plane and three triangles on one edge.
    let ngon = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [13.0, 9.0, 0.0],
            [5.0, 15.0, 0.0],
            [-3.0, 9.0, 0.0],
        ],
        &[&[0, 1, 2, 3, 4]],
    );
    let out = struts(&ngon, &StrutsOptions::default(), MOST).unwrap();
    assert_eq!(pieces_of(&out).len(), 5 + 5);
    every_piece_closed_and_outward(&out);
    let fan = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 10.0],
            [10.0, 0.0, 5.0],
            [-10.0, 3.0, 5.0],
            [0.0, -10.0, 5.0],
        ],
        &[&[0, 2, 1], &[0, 1, 3], &[1, 0, 4]],
    );
    let out = struts(&fan, &StrutsOptions::default(), MOST).unwrap();
    assert_eq!(pieces_of(&out).len(), fan.edges().len() + 5);
    every_piece_closed_and_outward(&out);
    let out = struts(&plane(3), &StrutsOptions::default(), MOST).unwrap();
    every_piece_closed_and_outward(&out);
}

#[test]
fn zero_length_edges_get_no_strut_but_their_points_get_joints() {
    let input = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [0.0, 10.0, 0.0],
        ],
        &[&[0, 1, 2, 3]],
    );
    let out = struts(&input, &StrutsOptions::default(), MOST).unwrap();
    assert_eq!(pieces_of(&out).len(), 3 + 4);
    assert!(out.vertices().all(|(_, p)| p.is_finite()));
    every_piece_closed_and_outward(&out);
}

#[test]
fn empty_shapes_loose_points_and_no_thickness_change_nothing() {
    let empty = Mesh::new();
    assert_eq!(
        struts(&empty, &StrutsOptions::default(), MOST).unwrap(),
        empty
    );
    let mut loose = Mesh::new();
    loose.add_vertex(Vec3::ONE).unwrap();
    assert_eq!(
        struts(&loose, &StrutsOptions::default(), MOST).unwrap(),
        loose
    );
    let input = cube();
    let none = StrutsOptions {
        thickness: 0.0,
        ..StrutsOptions::default()
    };
    assert_eq!(struts(&input, &none, MOST).unwrap(), input);
    // A loose point beside polygons gets no joint.
    let mut with_loose = cube();
    with_loose.add_vertex(Vec3::splat(500.0)).unwrap();
    let out = struts(&with_loose, &StrutsOptions::default(), MOST).unwrap();
    assert_eq!(pieces_of(&out).len(), 20);
    assert!(out.bounds().unwrap().max.x < 100.0);
}

#[test]
fn settings_out_of_range_are_brought_within_limits() {
    let odd = StrutsOptions {
        thickness: f32::NAN,
        sides: 0,
        ..StrutsOptions::default()
    }
    .clamped();
    assert_eq!(odd.thickness, StrutsOptions::STANDARD_THICKNESS);
    assert_eq!(odd.sides, StrutsOptions::MIN_SIDES);
    let huge = StrutsOptions {
        thickness: -1e30,
        sides: u32::MAX,
        ..StrutsOptions::default()
    }
    .clamped();
    assert_eq!(huge.thickness, StrutsOptions::MAX_THICKNESS);
    assert_eq!(huge.sides, StrutsOptions::MAX_SIDES);
    for options in [
        StrutsOptions {
            thickness: f32::INFINITY,
            sides: 1,
            ..StrutsOptions::default()
        },
        StrutsOptions {
            thickness: -5.0,
            sides: 1000,
            even: false,
            keep_faces: true,
        },
    ] {
        let out = struts(&cube(), &options, MOST).unwrap();
        assert!(out.vertices().all(|(_, p)| p.is_finite()));
        every_piece_closed_and_outward(&out);
    }
}

#[test]
fn too_many_polygons_are_refused() {
    let input = plane(10);
    let count = polygon_count(&input, &StrutsOptions::default());
    let refused = struts(&input, &StrutsOptions::default(), count as usize - 1);
    assert_eq!(
        refused,
        Err(StrutsProblem::TooManyPolygons {
            polygons: count,
            most: count as usize - 1
        })
    );
    assert!(refused.unwrap_err().to_string().contains("fewer sides"));
    assert!(struts(&input, &StrutsOptions::default(), count as usize).is_ok());
}

#[test]
fn keep_the_faces_keeps_the_shape_and_its_ids() {
    let input = cube();
    let kept = StrutsOptions {
        keep_faces: true,
        ..StrutsOptions::default()
    };
    let out = struts(&input, &kept, MOST).unwrap();
    assert_eq!(out.face_count(), input.face_count() + 12 * 6 + 8 * 6);
    for (face, corners) in input.faces() {
        assert_eq!(out.face(face), Some(corners));
    }
    for (v, p) in input.vertices() {
        assert_eq!(out.position(v), Some(p));
    }
    // The shape's own UVs stay; the struts' go in the square above.
    let before = Uvs::of(&input).unwrap();
    let after = Uvs::of(&out).unwrap();
    let corners = input.face_layout().1.len();
    for i in 0..corners {
        assert_eq!(before.at(i), after.at(i));
    }
    for i in corners..out.face_layout().1.len() {
        assert!(after.at(i).y >= 1.0 && after.at(i).y <= 2.0);
    }
}

#[test]
fn every_strut_and_joint_is_its_own_island_inside_the_square() {
    let input = Primitive::Sphere(SphereParams {
        sides: 8,
        rings: 6,
        ..SphereParams::default()
    })
    .mesh();
    let out = struts(&input, &sides(6), MOST).unwrap();
    assert!(has_uvs(&out));
    let pieces = pieces_of(&out).len();
    assert_eq!(islands(&out).len(), pieces);
    let uvs = Uvs::of(&out).unwrap();
    for i in 0..out.face_layout().1.len() {
        let uv = uvs.at(i);
        assert!(
            uv.cmpge(Vec2::ZERO).all() && uv.cmple(Vec2::ONE).all(),
            "{uv}"
        );
    }
    // Seams round each island, and nowhere inside one.
    assert!(!seams(&out).is_empty());
    assert_eq!(seams(&out).len(), uv_splits(&out).len());
}

#[test]
fn islands_dont_overlap_and_keep_the_surfaces_winding() {
    let out = struts(&cube(), &StrutsOptions::default(), MOST).unwrap();
    let uvs = Uvs::of(&out).unwrap();
    let starts = out.face_layout().0;
    let mut boxes: Vec<(Vec2, Vec2)> = Vec::new();
    for island in islands(&out) {
        let mut low = Vec2::splat(f32::MAX);
        let mut high = Vec2::splat(f32::MIN);
        for face in &island {
            let corners = out.face(*face).unwrap();
            let start = starts[face.get() as usize] as usize;
            let flat: Vec<Vec2> = (0..corners.len()).map(|k| uvs.at(start + k)).collect();
            let mut area = 0.0;
            for k in 0..flat.len() {
                area += flat[k].perp_dot(flat[(k + 1) % flat.len()]);
                low = low.min(flat[k]);
                high = high.max(flat[k]);
            }
            assert!(area > 0.0, "UVs turned over");
        }
        boxes.push((low, high));
    }
    for (i, a) in boxes.iter().enumerate() {
        for b in &boxes[i + 1..] {
            let apart = a.1.x <= b.0.x + 1e-6
                || b.1.x <= a.0.x + 1e-6
                || a.1.y <= b.0.y + 1e-6
                || b.1.y <= a.0.y + 1e-6;
            assert!(apart, "{a:?} {b:?}");
        }
    }
}

#[test]
fn a_struts_sides_unroll_without_stretching() {
    let out = struts(&cube(), &StrutsOptions::default(), MOST).unwrap();
    let uvs = Uvs::of(&out).unwrap();
    let starts = out.face_layout().0;
    // Every polygon's UV edges are its real edges times one scale.
    let mut scale: Option<f32> = None;
    for (face, corners) in out.faces() {
        let start = starts[face.get() as usize] as usize;
        for k in 0..corners.len() {
            let next = (k + 1) % corners.len();
            let real = out
                .position(corners[k])
                .unwrap()
                .distance(out.position(corners[next]).unwrap());
            let flat = uvs.at(start + k).distance(uvs.at(start + next));
            let ratio = flat / real;
            let s = *scale.get_or_insert(ratio);
            assert!((ratio - s).abs() < s * 1e-3, "{ratio} {s}");
        }
    }
}

#[test]
fn struts_and_joints_take_the_materials_round_them() {
    let mut input = cube();
    let top = input
        .faces()
        .map(|(f, _)| f)
        .find(|f| input.face_normal(*f).z > 0.9)
        .unwrap();
    // The first polygon of each edge decides; give them all slot 2.
    let all: Vec<FaceId> = input.faces().map(|(f, _)| f).collect();
    set_material(&mut input, &all, 2);
    set_material(&mut input, &[top], 1);
    let out = struts(&input, &StrutsOptions::default(), MOST).unwrap();
    let slots: std::collections::BTreeSet<u32> =
        out.faces().map(|(f, _)| slot_of(&out, f)).collect();
    assert!(slots.contains(&2));
    assert!(!slots.contains(&0));
}

#[test]
fn the_same_shape_always_gives_the_same_struts() {
    let input = plane(4);
    let a = struts(&input, &sides(5), MOST).unwrap();
    let b = struts(&input, &sides(5), MOST).unwrap();
    assert_eq!(a, b);
    assert_eq!(
        work(&input, &sides(5)),
        polygon_count(&input, &sides(5)) as usize / WORK_SHARE
    );
}

#[test]
fn random_shapes_never_panic() {
    let mut seed = 0x5a7c_0159_0000_0001_u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for _ in 0..60 {
        let count = 3 + (next() % 6) as usize;
        let points: Vec<[f32; 3]> = (0..count)
            .map(|_| {
                let mut c = || ((next() % 2000) as f32 - 1000.0) / 100.0;
                [c(), c(), c()]
            })
            .collect();
        let mut faces: Vec<Vec<u32>> = Vec::new();
        for _ in 0..3 {
            let a = (next() % count as u64) as u32;
            let b = (next() % count as u64) as u32;
            let c = (next() % count as u64) as u32;
            if a != b && b != c && a != c {
                faces.push(vec![a, b, c]);
            }
        }
        let face_refs: Vec<&[u32]> = faces.iter().map(Vec::as_slice).collect();
        let input = mesh_of(&points, &face_refs);
        let options = StrutsOptions {
            thickness: (next() % 500) as f32 / 100.0,
            sides: (next() % 30) as u32,
            even: next() % 2 == 0,
            keep_faces: next() % 2 == 0,
        };
        let out = struts(&input, &options, MOST).unwrap();
        assert!(out.vertices().all(|(_, p)| p.is_finite()));
    }
}

/// Timings (run with `--ignored --nocapture` in a release build).
#[test]
#[ignore = "a timing, not a test"]
fn timings() {
    let sphere = Primitive::Sphere(SphereParams {
        sides: 64,
        rings: 32,
        ..SphereParams::default()
    })
    .mesh();
    for n in [4, 8] {
        let start = std::time::Instant::now();
        let out = struts(&sphere, &sides(n), MOST).unwrap();
        println!(
            "{} edges, {n} sides: {} polygons in {:?}",
            sphere.edges().len(),
            out.face_count(),
            start.elapsed()
        );
    }
}
