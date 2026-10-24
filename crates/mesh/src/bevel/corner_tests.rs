//! Tests for the improved bevel (decision 0141): rounded corners where
//! bevelled edges meet, and stopping at neighbours point by point.

use super::*;
use crate::{ConeParams, CubeParams, CylinderParams, MeshReport, PlaneParams, Primitive};

fn cube() -> Mesh {
    Primitive::Cube(Default::default()).mesh()
}

/// A rounded bevel with the standard options (improved).
fn round(width: f32, segments: u32) -> BevelShape {
    BevelShape {
        width,
        segments,
        profile: 1.0,
        options: BevelOptions::default(),
    }
}

fn all_edges(mesh: &Mesh) -> Vec<EdgeKey> {
    mesh.edges().to_vec()
}

fn bevelled(before: &Mesh, edges: &[EdgeKey], shape: BevelShape) -> Mesh {
    let mut mesh = before.clone();
    bevel_edges(&mut mesh, edges, shape);
    mesh
}

fn centroid(mesh: &Mesh, face: FaceId) -> Vec3 {
    let c = mesh.face(face).unwrap();
    c.iter().map(|v| mesh.position(*v).unwrap()).sum::<Vec3>() / c.len() as f32
}

/// Closed, one piece, no problems, every point a number.
fn sound(mesh: &Mesh) {
    let r = MeshReport::new(mesh);
    assert!(r.is_closed(), "{r:?}");
    assert!(!r.has_problems(), "{r:?}");
    assert_eq!(r.pieces, 1, "{r:?}");
    for (v, p) in mesh.vertices() {
        assert!(p.is_finite(), "{v:?} {p}");
    }
}

/// For a shape that stays convex: every polygon faces away from `inside`
/// (so none is folded over or turned in).
fn faces_out(mesh: &Mesh, inside: Vec3) {
    for (f, _) in mesh.faces() {
        let n = mesh.face_normal(f);
        let d = n.dot(centroid(mesh, f) - inside);
        assert!(d > 0.0, "{f:?} faces in: {n} at {}", centroid(mesh, f));
    }
}

fn with_corners(mesh: &Mesh, n: usize) -> usize {
    mesh.faces().filter(|(_, c)| c.len() == n).count()
}

// ------------------------------------------------------ rounded corners

#[test]
fn a_cube_with_every_edge_bevelled_gets_round_corners() {
    let before = cube();
    for segments in [1, 2, 3, 8] {
        let mesh = bevelled(&before, &all_edges(&before), round(10.0, segments));
        sound(&mesh);
        faces_out(&mesh, Vec3::new(0.0, 0.0, 50.0));
        let s = segments as usize;
        let m = s / 2;
        // Each corner: m² quads per region (three regions), and with an
        // odd count a strip of m quads between regions and a triangle.
        let corner_quads = if s.is_multiple_of(2) {
            3 * m * m
        } else {
            3 * m * m + 3 * m
        };
        let corner_triangles = s % 2;
        let quads = 6 + 12 * s + 8 * corner_quads;
        assert_eq!(with_corners(&mesh, 4), quads, "{segments} segments");
        assert_eq!(with_corners(&mesh, 3), 8 * corner_triangles, "{segments}");
        assert_eq!(mesh.face_count(), quads + 8 * corner_triangles);
    }
}

#[test]
fn a_cubes_round_corner_lies_close_to_a_ball() {
    let before = cube();
    for segments in [2, 3, 4, 8] {
        let mesh = bevelled(&before, &all_edges(&before), round(10.0, segments));
        // The top corner at (50, 50, 100): its ball has its middle 10 in
        // from each side.
        let middle = Vec3::new(40.0, 40.0, 90.0);
        let near: Vec<Vec3> = mesh
            .vertices()
            .map(|(_, p)| p)
            .filter(|p| p.x > 39.99 && p.y > 39.99 && p.z > 89.99)
            .collect();
        assert!(near.len() > 3, "{near:?}");
        for p in near {
            let r = p.distance(middle);
            assert!(
                (r - 10.0).abs() < 0.8,
                "{segments}: {p} is {r} from the middle"
            );
        }
    }
}

#[test]
fn the_corner_patch_is_symmetric_and_the_same_every_time() {
    let before = cube();
    let a = bevelled(&before, &all_edges(&before), round(10.0, 4));
    let b = bevelled(&before, &all_edges(&before), round(10.0, 4));
    assert_eq!(a, b);
    // The cube's top four corners look alike: the points near each,
    // turned to the first, match.
    let at = |sx: f32, sy: f32| {
        let mut points: Vec<[i64; 3]> = a
            .vertices()
            .map(|(_, p)| p)
            .filter(|p| p.x * sx > 39.99 && p.y * sy > 39.99 && p.z > 89.99)
            .map(|p| {
                let q = Vec3::new(p.x * sx, p.y * sy, p.z);
                // Mirroring swaps nothing round the diagonal, so sort.
                [
                    (q.x * 100.0) as i64,
                    (q.y * 100.0) as i64,
                    (q.z * 100.0) as i64,
                ]
            })
            .collect();
        points.sort();
        points
    };
    let first = at(1.0, 1.0);
    for (sx, sy) in [(-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
        let other = at(sx, sy);
        assert_eq!(first.len(), other.len());
        for (p, q) in first.iter().zip(&other) {
            for k in 0..3 {
                assert!((p[k] - q[k]).abs() <= 1, "{p:?} {q:?}");
            }
        }
    }
}

#[test]
fn every_profile_stays_sound() {
    let before = cube();
    for profile in [-1.0, -0.5, 0.0, 0.5, 1.0] {
        for segments in [2, 3, 6] {
            let shape = BevelShape {
                profile,
                ..round(12.0, segments)
            };
            let mesh = bevelled(&before, &all_edges(&before), shape);
            sound(&mesh);
            if profile >= 0.0 {
                faces_out(&mesh, Vec3::new(0.0, 0.0, 50.0));
            }
            // A flat profile keeps the corner flat: its points lie on the
            // plane through the three cut corners.
            if profile == 0.0 {
                let n = Vec3::ONE.normalize();
                let plane = Vec3::new(38.0, 38.0, 100.0).dot(n);
                for (_, p) in mesh.vertices() {
                    if p.x > 37.99 && p.y > 37.99 && p.z > 87.99 {
                        assert!((p.dot(n) - plane).abs() < 0.05, "{p}");
                    }
                }
            }
        }
    }
}

#[test]
fn improved_off_keeps_one_corner_polygon_as_before() {
    let before = cube();
    let shape = BevelShape {
        options: BevelOptions {
            improved: false,
            ..BevelOptions::default()
        },
        ..round(10.0, 3)
    };
    let mesh = bevelled(&before, &all_edges(&before), shape);
    sound(&mesh);
    // Eight corner polygons of 3 × 3 corners.
    assert_eq!(with_corners(&mesh, 9), 8);
}

#[test]
fn one_segment_corners_are_one_flat_triangle() {
    let before = cube();
    let mesh = bevelled(&before, &all_edges(&before), round(10.0, 1));
    assert_eq!(with_corners(&mesh, 3), 8);
    let old = bevelled(
        &before,
        &all_edges(&before),
        BevelShape {
            options: BevelOptions {
                improved: false,
                ..BevelOptions::default()
            },
            ..round(10.0, 1)
        },
    );
    assert_eq!(mesh, old);
}

fn cone(sides: u32) -> Mesh {
    Primitive::Cone(ConeParams {
        sides,
        ..Default::default()
    })
    .mesh()
}

#[test]
fn a_pyramids_tip_where_four_or_five_edges_meet_is_rounded() {
    for sides in [4, 5] {
        let before = cone(sides);
        for segments in [2, 3, 4] {
            let mesh = bevelled(&before, &all_edges(&before), round(6.0, segments));
            sound(&mesh);
            faces_out(&mesh, Vec3::new(0.0, 0.0, 30.0));
            // The tip is gone and the shape is lower, by less than a bevel
            // can take off.
            let top = mesh.vertices().map(|(_, p)| p.z).fold(f32::MIN, f32::max);
            assert!(top < 100.0 && top > 80.0, "{sides} {segments}: {top}");
            // Every polygon at the tip is a quad (or, with an odd count,
            // the middle polygon has a corner for each edge).
            let tip: Vec<usize> = mesh
                .faces()
                .filter(|(f, _)| centroid(&mesh, *f).z > top - 4.0)
                .map(|(_, c)| c.len())
                .collect();
            assert!(!tip.is_empty());
            let odd = segments % 2 == 1;
            assert!(
                tip.iter().all(|n| *n == 4 || (odd && *n == sides as usize)),
                "{sides} {segments}: {tip:?}"
            );
        }
    }
}

#[test]
fn a_cylinder_with_n_gon_caps_bevels_round() {
    let before = Primitive::Cylinder(CylinderParams {
        sides: 8,
        ..Default::default()
    })
    .mesh();
    for segments in [1, 2, 3, 5] {
        let mesh = bevelled(&before, &all_edges(&before), round(5.0, segments));
        sound(&mesh);
        faces_out(&mesh, Vec3::new(0.0, 0.0, 50.0));
        // The caps are still one polygon of eight corners each.
        assert_eq!(with_corners(&mesh, 8), 2, "{segments}");
    }
}

/// A cube's top front right corner, `which` of its three edges bevelled.
fn corner_edges(mesh: &Mesh, count: usize) -> Vec<EdgeKey> {
    let corner = mesh
        .vertices()
        .find(|(_, p)| p.x > 49.0 && p.y > 49.0 && p.z > 99.0)
        .map(|(v, _)| v)
        .unwrap();
    let mut edges: Vec<EdgeKey> = mesh
        .edges()
        .iter()
        .copied()
        .filter(|e| {
            let (a, b) = e.ends();
            a == corner || b == corner
        })
        .collect();
    edges.sort();
    edges.truncate(count);
    edges
}

#[test]
fn bevelled_and_plain_edges_meeting_at_a_point_stay_sound() {
    let before = cube();
    for count in [1, 2, 3] {
        for segments in [1, 2, 3, 4] {
            let mesh = bevelled(
                &before,
                &corner_edges(&before, count),
                round(10.0, segments),
            );
            sound(&mesh);
            faces_out(&mesh, Vec3::new(0.0, 0.0, 50.0));
        }
    }
    // Two edges turning a corner along a loop (the top ring): the strips
    // meet in a mitre, all quads.
    let ring: Vec<EdgeKey> = before
        .edges()
        .iter()
        .copied()
        .filter(|e| {
            let (a, b) = e.ends();
            before.position(a).unwrap().z > 99.0 && before.position(b).unwrap().z > 99.0
        })
        .collect();
    for profile in [-1.0, 0.0, 1.0] {
        let mesh = bevelled(
            &before,
            &ring,
            BevelShape {
                profile,
                ..round(10.0, 4)
            },
        );
        sound(&mesh);
        assert_eq!(with_corners(&mesh, 4), mesh.face_count(), "{profile}");
        assert_eq!(mesh.face_count(), 6 + 4 * 4);
    }
}

#[test]
fn a_point_in_a_grid_with_three_of_four_edges_bevelled_is_rounded() {
    let before = Primitive::Cube(CubeParams {
        segments: [2, 2, 2],
        ..Default::default()
    })
    .mesh();
    // The point in the middle of the top: four edges.
    let middle = before
        .vertices()
        .find(|(_, p)| p.x.abs() < 1e-3 && p.y.abs() < 1e-3 && p.z > 99.0)
        .map(|(v, _)| v)
        .unwrap();
    let mut edges: Vec<EdgeKey> = before
        .edges()
        .iter()
        .copied()
        .filter(|e| {
            let (a, b) = e.ends();
            a == middle || b == middle
        })
        .collect();
    assert_eq!(edges.len(), 4);
    edges.sort();
    edges.truncate(3);
    for segments in [2, 3] {
        let mesh = bevelled(&before, &edges, round(8.0, segments));
        // The strips' far ends, where they slide along the cube's edges
        // in a straight line, leave flat slivers, as they always have; the
        // rest is sound.
        let r = MeshReport::new(&mesh);
        assert!(r.is_closed(), "{r:?}");
        assert_eq!(r.non_manifold_edges + r.flipped_edges, 0, "{r:?}");
        // A flat top bevelled stays flat: every point stays on it.
        for (_, p) in mesh.vertices() {
            assert!(p.z <= 100.0 + 1e-3, "{p}");
        }
        // The corner in the middle is quads (and with an odd count, a
        // middle triangle), all facing up.
        for (f, c) in mesh.faces() {
            let middle = centroid(&mesh, f);
            if middle.z > 99.0 && middle.truncate().length() < 4.0 {
                assert!(c.len() == 4 || (segments == 3 && c.len() == 3), "{c:?}");
                assert!(mesh.face_normal(f).z > 0.99);
            }
        }
    }
}

// --------------------------------------------------- stop at neighbours

/// A box whose top narrows to 4 wide at one end (x = 50) and stays 100
/// wide at the other.
fn wedge_box() -> Mesh {
    let mut mesh = cube();
    let moved: Vec<(VertexId, Vec3)> = mesh
        .vertices()
        .filter(|(_, p)| p.x > 49.0)
        .map(|(v, p)| (v, Vec3::new(p.x, p.y.signum() * 2.0, p.z)))
        .collect();
    for (v, p) in moved {
        mesh.set_position(v, p);
    }
    mesh
}

fn top_ring(mesh: &Mesh) -> Vec<EdgeKey> {
    mesh.edges()
        .iter()
        .copied()
        .filter(|e| {
            let (a, b) = e.ends();
            mesh.position(a).unwrap().z > 99.0 && mesh.position(b).unwrap().z > 99.0
        })
        .collect()
}

#[test]
fn stopping_at_neighbours_narrows_only_where_it_must() {
    let before = wedge_box();
    let ring = top_ring(&before);
    let shape = |improved: bool| BevelShape {
        options: BevelOptions {
            improved,
            ..BevelOptions::default()
        },
        ..round(10.0, 1)
    };
    // How far in from the wide end (x = -50) the top reaches.
    let wide_end = |mesh: &Mesh| {
        mesh.vertices()
            .map(|(_, p)| p)
            .filter(|p| p.z > 99.99)
            .map(|p| p.x + 50.0)
            .fold(f32::MAX, f32::min)
    };
    let each = bevelled(&before, &ring, shape(true));
    sound(&each);
    let whole = bevelled(&before, &ring, shape(false));
    sound(&whole);
    // Point by point: the wide end keeps its full 10.
    assert!((wide_end(&each) - 10.0).abs() < 0.2, "{}", wide_end(&each));
    // The whole bevel narrowed by the tight end: much less.
    assert!(wide_end(&whole) < 4.0, "{}", wide_end(&whole));
    // At the tight end neither crosses the middle line or the edge.
    for mesh in [&each, &whole] {
        for (_, p) in mesh.vertices() {
            assert!(p.y.abs() <= 50.0 + 1e-3 && p.x <= 50.0 + 1e-3, "{p}");
        }
        for (f, _) in mesh.faces() {
            if mesh.face_normal(f).z > 0.9 {
                assert!(centroid(mesh, f).z > 99.0);
            }
        }
    }
}

#[test]
fn widths_larger_than_the_shape_stay_inside_it() {
    let before = cube();
    for width in [60.0, 1000.0, 1e7] {
        for segments in [1, 2, 3] {
            let mesh = bevelled(&before, &all_edges(&before), round(width, segments));
            sound(&mesh);
            for (_, p) in mesh.vertices() {
                assert!(
                    p.x.abs() <= 50.0 + 1e-2 && p.y.abs() <= 50.0 + 1e-2,
                    "{width} {segments}: {p}"
                );
                assert!(
                    p.z >= -1e-2 && p.z <= 100.0 + 1e-2,
                    "{width} {segments}: {p}"
                );
            }
        }
    }
}

#[test]
fn a_narrow_face_cuts_the_width_down() {
    // A plank 100 × 4 × 100: its narrow faces are 4 wide, so a 10-wide
    // bevel must stop at 2 from each edge there.
    let mut before = cube();
    let moved: Vec<(VertexId, Vec3)> = before
        .vertices()
        .map(|(v, p)| (v, Vec3::new(p.x, p.y * 0.04, p.z)))
        .collect();
    for (v, p) in moved {
        before.set_position(v, p);
    }
    for segments in [1, 2, 3, 4] {
        let mesh = bevelled(&before, &all_edges(&before), round(10.0, segments));
        sound(&mesh);
        faces_out(&mesh, Vec3::new(0.0, 0.0, 50.0));
        for (_, p) in mesh.vertices() {
            assert!(p.y.abs() <= 2.0 + 1e-3, "{p}");
        }
    }
    // Without stopping, the corners cross and polygons fold over.
    let free = bevelled(
        &before,
        &all_edges(&before),
        BevelShape {
            options: BevelOptions {
                clamp: false,
                weld: false,
                ..BevelOptions::default()
            },
            ..round(10.0, 1)
        },
    );
    assert!(free.vertices().any(|(_, p)| p.y.abs() > 2.0 + 1e-3));
}

// ------------------------------------------------------ edges and values

#[test]
fn open_borders_are_skipped_and_the_rest_bevels() {
    let before = Primitive::Plane(PlaneParams {
        segments: [3, 3],
        ..Default::default()
    })
    .mesh();
    for segments in [1, 2, 3] {
        let mut mesh = before.clone();
        let outcome = bevel_edges(&mut mesh, &all_edges(&before), round(5.0, segments));
        assert!(outcome.skipped > 0 && outcome.matched > 0, "{outcome:?}");
        let r = MeshReport::new(&mesh);
        assert_eq!(r.non_manifold_edges, 0, "{r:?}");
        assert_eq!(r.flipped_edges, 0, "{r:?}");
        for (_, p) in mesh.vertices() {
            assert!(p.is_finite());
        }
    }
}

/// The polygons of the round patch at the cube's top corner.
fn top_corner_faces(mesh: &Mesh) -> Vec<FaceId> {
    mesh.faces()
        .map(|(f, _)| f)
        .filter(|f| {
            mesh.face(*f).unwrap().iter().all(|v| {
                let p = mesh.position(*v).unwrap();
                p.x > 39.99 && p.y > 39.99 && p.z > 89.99
            })
        })
        .collect()
}

#[test]
fn round_corners_carry_uvs_materials_and_soft_edges() {
    let mut before = cube();
    // The top a material of its own, every edge hard.
    let top: Vec<FaceId> = before
        .faces()
        .map(|(f, _)| f)
        .filter(|f| before.face_normal(*f).z > 0.9)
        .collect();
    crate::material::set_material(&mut before, &top, 1);
    let edges = all_edges(&before);
    for e in &edges {
        let (a, b) = e.ends();
        before.set_hard(a, b, true);
    }
    let mesh = bevelled(&before, &edges, round(10.0, 3));
    let corner = top_corner_faces(&mesh);
    // Three regions of one quad, three strips of one, and a triangle,
    // plus the strip ends next to them.
    assert!(corner.len() >= 7, "{corner:?}");
    for f in &corner {
        let n = mesh.face(*f).unwrap().len();
        for k in 0..n {
            let uv = crate::uvs::corner_uv(&mesh, *f, k);
            assert!(uv.is_some_and(|uv| uv.is_finite()), "{f:?} {k}");
        }
        // A rounded corner is smooth inside.
        let c = mesh.face(*f).unwrap();
        for k in 0..n {
            let (a, b) = (c[k], c[(k + 1) % n]);
            let p = (mesh.position(a).unwrap() + mesh.position(b).unwrap()) * 0.5;
            // Edges inside the patch (not on the cube's original faces).
            if p.x < 49.99 && p.y < 49.99 && p.z < 99.99 {
                assert!(!mesh.is_hard(a, b), "{a:?} {b:?} at {p}");
            }
        }
    }
    // The region next to the top takes the top's material; the others
    // the sides'.
    let slots: Vec<u32> = corner
        .iter()
        .map(|f| crate::material::slot_of(&mesh, *f))
        .collect();
    assert!(slots.contains(&1) && slots.contains(&0), "{slots:?}");
    // With one segment, the corner triangle stays crisp, as before.
    let chamfered = bevelled(&before, &edges, round(10.0, 1));
    let triangle = chamfered
        .faces()
        .find(|(f, c)| c.len() == 3 && centroid(&chamfered, *f).z > 90.0)
        .map(|(_, c)| c.to_vec())
        .unwrap();
    for k in 0..3 {
        assert!(chamfered.is_hard(triangle[k], triangle[(k + 1) % 3]));
    }
}

#[test]
fn ids_follow_a_fixed_order() {
    let before = cone(5);
    let a = bevelled(&before, &all_edges(&before), round(4.0, 5));
    let b = bevelled(&before, &all_edges(&before), round(4.0, 5));
    assert_eq!(a, b);
    let ids: Vec<VertexId> = a.vertices().map(|(v, _)| v).collect();
    let again: Vec<VertexId> = b.vertices().map(|(v, _)| v).collect();
    assert_eq!(ids, again);
}

#[test]
fn bad_settings_never_break_the_corner() {
    let before = cube();
    for width in [0.0, -5.0, f32::NAN, f32::INFINITY, 1e30] {
        for profile in [f32::NAN, -7.0, 7.0] {
            let mut mesh = before.clone();
            bevel_edges(
                &mut mesh,
                &all_edges(&before),
                BevelShape {
                    width,
                    segments: 1000,
                    profile,
                    options: BevelOptions::default(),
                },
            );
            for (_, p) in mesh.vertices() {
                assert!(p.is_finite(), "{width} {profile}: {p}");
            }
        }
    }
}

#[test]
fn random_bevels_never_panic() {
    let mut seed = 0x51ed_270b_a3c4_9f1d_u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let kinds = crate::PrimitiveKind::ALL;
    for round in 0..120 {
        let kind = kinds[(next() as usize) % kinds.len()];
        let before = kind.standard().mesh();
        let edges: Vec<EdgeKey> = before
            .edges()
            .iter()
            .copied()
            .filter(|_| next() % 3 == 0)
            .collect();
        let widths = [0.0, 0.5, 3.0, 20.0, 500.0, -1.0];
        let shape = BevelShape {
            width: widths[(next() as usize) % widths.len()],
            segments: 1 + (next() % 9) as u32,
            profile: ((next() % 201) as f32 - 100.0) / 100.0,
            options: BevelOptions {
                skew: ((next() % 3) as f32 - 1.0) * 0.5,
                even: next() % 2 == 0,
                clamp: next() % 4 != 0,
                weld: next() % 2 == 0,
                ..BevelOptions::default()
            },
        };
        let mut mesh = before.clone();
        bevel_edges(&mut mesh, &edges, shape);
        for (_, p) in mesh.vertices() {
            assert!(p.is_finite(), "{round} {kind:?} {shape:?}");
        }
        // No worse than the earlier bevel (which, for a width of 0 with
        // welding, can leave a doubled edge where everything meets).
        let mut old = before.clone();
        let earlier = BevelShape {
            options: BevelOptions {
                improved: false,
                ..shape.options
            },
            ..shape
        };
        bevel_edges(&mut old, &edges, earlier);
        let (now, then) = (MeshReport::new(&mesh), MeshReport::new(&old));
        assert!(
            now.non_manifold_edges <= then.non_manifold_edges
                && now.flipped_edges <= then.flipped_edges
                && now.border_edges <= then.border_edges.max(before_borders(&before)),
            "{round} {kind:?} {shape:?}: {now:?} {then:?}"
        );
    }
}

fn before_borders(mesh: &Mesh) -> usize {
    MeshReport::new(mesh).border_edges
}

/// Timing on a heavy shape: every edge of a ball of about 32,000 polygons,
/// rounded with 3 segments (each point a four-way corner). Run by hand:
/// `cargo test --release -p mesh heavy_round_bevel -- --ignored --nocapture`.
#[test]
#[ignore = "timing: run by hand in release mode"]
fn heavy_round_bevel() {
    let before = crate::samples::heavy_ball(32_000, 100.0);
    let edges = all_edges(&before);
    for segments in [1, 3] {
        for improved in [false, true] {
            let shape = BevelShape {
                options: BevelOptions {
                    improved,
                    ..BevelOptions::default()
                },
                ..round(0.6, segments)
            };
            let start = std::time::Instant::now();
            let mesh = bevelled(&before, &edges, shape);
            println!(
                "Bevel of {} edges, {segments} segments, improved {improved}: {} ms, {} polygons",
                edges.len(),
                start.elapsed().as_millis(),
                mesh.face_count()
            );
        }
    }
}
