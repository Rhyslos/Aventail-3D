//! Tests for unwrapping, straightening and relaxing (decision 0099).

use super::*;
use mesh::{CubeParams, CylinderParams, PlaneParams, Primitive, SphereParams};

fn plane(n: u32) -> Mesh {
    Primitive::Plane(PlaneParams {
        segments: [n, n],
        ..Default::default()
    })
    .mesh()
}

fn all_faces(mesh: &Mesh) -> Vec<FaceId> {
    mesh.faces().map(|(f, _)| f).collect()
}

/// Polygon `f`'s corner UVs.
fn uvs_of(mesh: &Mesh, f: FaceId) -> Vec<Vec2> {
    let uvs = Uvs::of(mesh).unwrap();
    let n = mesh.face(f).unwrap().len();
    (0..n).map(|k| uvs.corner(mesh, f, k).unwrap()).collect()
}

fn uv_area(mesh: &Mesh, f: FaceId) -> f32 {
    let c = uvs_of(mesh, f);
    let mut sum = 0.0;
    for k in 1..c.len() - 1 {
        sum += (c[k] - c[0]).perp_dot(c[k + 1] - c[0]) / 2.0;
    }
    sum
}

/// Every triangle of every polygon faces the right way in the texture.
fn none_flipped(mesh: &Mesh) -> bool {
    let uvs = Uvs::of(mesh).unwrap();
    let (starts, _) = mesh.face_layout();
    mesh.faces().all(|(f, list)| {
        let start = starts[f.get() as usize] as usize;
        let mut ok = true;
        mesh::triangulate::face_triangles(mesh, list, |[a, b, c]| {
            let (ua, ub, uc) = (uvs.at(start + a), uvs.at(start + b), uvs.at(start + c));
            let p = |i: usize| mesh.position(list[i]).unwrap();
            let real = (p(b) - p(a)).cross(p(c) - p(a)).length();
            if real > 1e-9 {
                ok &= (ub - ua).perp_dot(uc - ua) > 0.0;
            }
        });
        ok
    })
}

/// How far each quad's UV corners are from square (the largest cosine).
fn least_square(mesh: &Mesh) -> f32 {
    mesh.faces()
        .filter(|(_, l)| l.len() == 4)
        .map(|(f, _)| {
            let c = uvs_of(mesh, f);
            (0..4)
                .map(|k| {
                    let a = (c[(k + 3) % 4] - c[k]).normalize();
                    let b = (c[(k + 1) % 4] - c[k]).normalize();
                    a.dot(b).abs()
                })
                .fold(0.0f32, f32::max)
        })
        .fold(0.0, f32::max)
}

fn middle_of_box(mesh: &Mesh, faces: &[FaceId]) -> Vec2 {
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);
    for f in faces {
        for v in uvs_of(mesh, *f) {
            min = min.min(v);
            max = max.max(v);
        }
    }
    (min + max) / 2.0
}

#[test]
fn a_flat_grid_comes_out_as_an_even_grid_where_it_was() {
    for strips in [true, false] {
        let mut mesh = plane(3);
        let faces = all_faces(&mesh);
        let before = middle_of_box(&mesh, &faces);
        let area_before: f32 = faces.iter().map(|f| uv_area(&mesh, *f)).sum();
        let done = unwrap(&mut mesh, &faces, strips);
        assert_eq!(done.matched, faces.len());
        assert!(none_flipped(&mesh));
        assert!(
            least_square(&mesh) < 1e-3,
            "{strips}: {}",
            least_square(&mesh)
        );
        // Every square the same size, the whole the same size and place.
        let areas: Vec<f32> = faces.iter().map(|f| uv_area(&mesh, *f)).collect();
        for a in &areas {
            assert!((a - areas[0]).abs() < 1e-4, "{areas:?}");
        }
        let area_after: f32 = areas.iter().sum();
        assert!((area_after - area_before).abs() < 1e-3 * area_before.max(1.0));
        assert!((middle_of_box(&mesh, &faces) - before).length() < 1e-4);
    }
}

#[test]
fn a_box_keeps_its_sides_real_sizes_against_each_other() {
    let mut mesh = Primitive::Cube(CubeParams {
        size: Vec3::new(2.0, 1.0, 0.5),
        ..Default::default()
    })
    .mesh();
    let faces = all_faces(&mesh);
    unwrap(&mut mesh, &faces, true);
    assert!(none_flipped(&mesh));
    let real = |f: FaceId| {
        let l = mesh.face(f).unwrap();
        let p = |i: usize| mesh.position(l[i]).unwrap();
        (p(1) - p(0)).cross(p(3) - p(0)).length()
    };
    let ratio: Vec<f32> = faces
        .iter()
        .map(|f| uv_area(&mesh, *f) / real(*f))
        .collect();
    for r in &ratio {
        assert!((r - ratio[0]).abs() < 1e-3 * ratio[0], "{ratio:?}");
    }
    assert!(least_square(&mesh) < 1e-3);
}

#[test]
fn a_cylinders_side_comes_out_as_a_straight_strip() {
    let mut mesh = Primitive::Cylinder(CylinderParams {
        sides: 16,
        height_segments: 3,
        ..Default::default()
    })
    .mesh();
    let side: Vec<FaceId> = all_faces(&mesh)
        .into_iter()
        .filter(|f| mesh.face_normal(*f).z.abs() < 0.1)
        .collect();
    unwrap(&mut mesh, &side, true);
    assert!(none_flipped(&mesh));
    // Square corners, and every quad the same size.
    assert!(least_square(&mesh) < 1e-3);
    let areas: Vec<f32> = side.iter().map(|f| uv_area(&mesh, *f)).collect();
    for a in &areas {
        assert!((a - areas[0]).abs() < 1e-3 * areas[0]);
    }
}

#[test]
fn a_ball_unwraps_without_folds_and_a_closed_piece_gets_a_seam() {
    let mut mesh = Primitive::Sphere(SphereParams::default()).mesh();
    let seams = crate::seams::suggest(&mesh, &[]);
    let old = mesh::uvs::seams(&mesh);
    mesh::uvs::set_seams(&mut mesh, &old, false);
    mesh::uvs::set_seams(&mut mesh, &seams, true);
    let faces = all_faces(&mesh);
    unwrap(&mut mesh, &faces, true);
    assert!(none_flipped(&mesh));
    // With no seams at all, unwrapping cuts it open itself.
    let mut closed = Primitive::Sphere(SphereParams::default()).mesh();
    let all_seams = mesh::uvs::seams(&closed);
    mesh::uvs::set_seams(&mut closed, &all_seams, false);
    assert!(mesh::uvs::seams(&closed).is_empty());
    unwrap(&mut closed, &faces, true);
    assert!(!mesh::uvs::seams(&closed).is_empty(), "the cut is a seam");
    assert!(none_flipped(&closed));
}

#[test]
fn a_piece_keeps_the_way_it_lay_a_quarter_turn_at_a_time() {
    let mut mesh = plane(2);
    let faces = all_faces(&mesh);
    // Turn the UVs a quarter, then unwrap: it stays turned.
    let middle = middle_of_box(&mesh, &faces);
    let turn = glam::Affine2::from_translation(middle)
        * glam::Affine2::from_angle(std::f32::consts::FRAC_PI_2)
        * glam::Affine2::from_translation(-middle);
    mesh::uvs::transform_uvs(&mut mesh, &faces, &[], turn);
    let before: Vec<Vec<Vec2>> = faces.iter().map(|f| uvs_of(&mesh, *f)).collect();
    unwrap(&mut mesh, &faces, true);
    for (f, was) in faces.iter().zip(&before) {
        for (now, then) in uvs_of(&mesh, *f).iter().zip(was) {
            assert!(now.distance(*then) < 1e-3, "{now} {then}");
        }
    }
}

#[test]
fn only_the_pieces_asked_for_change() {
    let mut mesh = Primitive::Cube(CubeParams::default()).mesh();
    let faces = all_faces(&mesh);
    let top = *faces
        .iter()
        .find(|f| mesh.face_normal(**f).z > 0.9)
        .unwrap();
    let others: Vec<Vec<Vec2>> = faces
        .iter()
        .filter(|f| **f != top)
        .map(|f| uvs_of(&mesh, *f))
        .collect();
    // Squash the top's UVs, then unwrap just it: it comes back at the
    // others' density, and they don't move.
    mesh::uvs::transform_uvs(
        &mut mesh,
        &[top],
        &[],
        glam::Affine2::from_scale(Vec2::new(0.1, 0.1)),
    );
    unwrap(&mut mesh, &[top], true);
    let after: Vec<Vec<Vec2>> = faces
        .iter()
        .filter(|f| **f != top)
        .map(|f| uvs_of(&mesh, *f))
        .collect();
    assert_eq!(after, others);
    let side = faces.iter().find(|f| **f != top).unwrap();
    assert!((uv_area(&mesh, top) - uv_area(&mesh, *side)).abs() < 1e-4);
}

#[test]
fn straighten_makes_a_bent_grid_straight_and_skips_other_pieces() {
    let mut mesh = plane(3);
    let faces = all_faces(&mesh);
    // Bend the UVs: one inner corner off.
    let inner = mesh
        .vertices()
        .min_by(|a, b| a.1.length().total_cmp(&b.1.length()))
        .map(|(v, _)| v)
        .unwrap();
    mesh::uvs::transform_uvs(
        &mut mesh,
        &[],
        &[inner],
        glam::Affine2::from_translation(Vec2::new(0.05, 0.03)),
    );
    assert!(least_square(&mesh) > 0.01);
    let done = straighten(&mut mesh, &faces);
    assert_eq!(done.matched, faces.len());
    assert!(least_square(&mesh) < 1e-3);
    // A ball's pieces have triangles: nothing to straighten.
    let mut ball = Primitive::Sphere(SphereParams::default()).mesh();
    let ball_faces = all_faces(&ball);
    let done = straighten(&mut ball, &ball_faces);
    assert_eq!(done.matched, 0);
}

#[test]
fn relax_pulls_a_bent_corner_back_and_leaves_the_rest() {
    let mut mesh = plane(4);
    let faces = all_faces(&mesh);
    let grid_before: Vec<Vec<Vec2>> = faces.iter().map(|f| uvs_of(&mesh, *f)).collect();
    let inner = mesh
        .vertices()
        .find(|(_, p)| p.x.abs() < 0.1 && p.y.abs() < 0.1)
        .map(|(v, _)| v)
        .unwrap();
    let shove = glam::Affine2::from_translation(Vec2::new(0.06, 0.04));
    mesh::uvs::transform_uvs(&mut mesh, &[], &[inner], shove);
    let off = |m: &Mesh| -> f32 {
        faces
            .iter()
            .zip(&grid_before)
            .flat_map(|(f, was)| {
                uvs_of(m, *f)
                    .into_iter()
                    .zip(was.iter().copied())
                    .map(|(a, b)| a.distance(b))
                    .collect::<Vec<_>>()
            })
            .fold(0.0, f32::max)
    };
    let bent = off(&mesh);
    // The polygons around the corner only.
    let around: Vec<FaceId> = faces
        .iter()
        .copied()
        .filter(|f| mesh.face(*f).unwrap().contains(&inner))
        .collect();
    let done = relax(&mut mesh, &around, RELAX_STEPS);
    assert_eq!(done.matched, around.len());
    assert!(off(&mesh) < bent * 0.5, "{} vs {bent}", off(&mesh));
    assert!(none_flipped(&mesh));
}

#[test]
fn odd_input_changes_nothing_or_gives_real_uvs() {
    // Missing polygons are counted.
    let mut mesh = plane(2);
    let ghost = FaceId::from_raw(999);
    let done = unwrap(&mut mesh, &[ghost], true);
    assert_eq!((done.matched, done.missing), (0, 1));
    // A shape without UVs gets them; relaxing it does nothing.
    let mut bare = plane(2);
    bare.remove_attribute(mesh::attributes::UV);
    let faces = all_faces(&bare);
    assert_eq!(relax(&mut bare, &faces, 5).matched, 0);
    assert_eq!(unwrap(&mut bare, &faces, false).matched, faces.len());
    assert!(mesh::uvs::has_uvs(&bare));
    // Polygons with no area at all.
    let mut flat = Mesh::new();
    let a = flat.add_vertex(Vec3::ZERO).unwrap();
    let b = flat.add_vertex(Vec3::ZERO).unwrap();
    let c = flat.add_vertex(Vec3::ZERO).unwrap();
    let f = flat.add_face(&[a, b, c]).unwrap();
    unwrap(&mut flat, &[f], true);
    if let Some(uvs) = Uvs::of(&flat) {
        for k in 0..3 {
            assert!(uvs.corner(&flat, f, k).unwrap().is_finite());
        }
    }
    // The same every time.
    let mut one = Primitive::Sphere(SphereParams::default()).mesh();
    let mut two = one.clone();
    let faces = all_faces(&one);
    unwrap(&mut one, &faces, true);
    unwrap(&mut two, &faces, true);
    assert_eq!(
        Uvs::of(&one).unwrap().into_values(0),
        Uvs::of(&two).unwrap().into_values(0)
    );
}

/// How long unwrapping takes on balls of growing size: run with
/// `cargo test --release -p uv unwrap_benchmark -- --ignored --nocapture`.
#[test]
#[ignore = "benchmark"]
fn unwrap_benchmark() {
    for (sides, rings) in [(64u32, 32u32), (128, 64), (256, 128)] {
        let mut mesh = Primitive::Sphere(SphereParams {
            sides,
            rings,
            ..Default::default()
        })
        .mesh();
        let seams = crate::seams::suggest(&mesh, &[]);
        let old = mesh::uvs::seams(&mesh);
        mesh::uvs::set_seams(&mut mesh, &old, false);
        mesh::uvs::set_seams(&mut mesh, &seams, true);
        let faces = all_faces(&mesh);
        let t = std::time::Instant::now();
        unwrap(&mut mesh, &faces, true);
        eprintln!(
            "{} faces: {:?} flipped-free {}",
            faces.len(),
            t.elapsed(),
            none_flipped(&mesh)
        );
    }
}

/// A flat ring of `n` quads round a hole: inner radius `inner`, outer 1.
fn ring(n: usize, inner: f32) -> Mesh {
    let mut mesh = Mesh::new();
    let mut ids = Vec::new();
    for r in [inner, 1.0] {
        for k in 0..n {
            let a = std::f32::consts::TAU * k as f32 / n as f32;
            ids.push(
                mesh.add_vertex(Vec3::new(r * a.cos(), r * a.sin(), 0.0))
                    .unwrap(),
            );
        }
    }
    for k in 0..n {
        let k1 = (k + 1) % n;
        mesh.add_face(&[ids[k], ids[k1], ids[n + k1], ids[n + k]])
            .unwrap();
    }
    mesh
}

#[test]
fn a_flat_ring_round_a_small_hole_is_unwrapped_not_straightened() {
    // Its quads have square corners but taper five times over: laid out
    // as a straight strip they'd be stretched (a review found a ring
    // stretched 11 times). Unwrapped, it lies flat as the ring it is.
    let mut mesh = ring(32, 0.2);
    let faces = all_faces(&mesh);
    unwrap(&mut mesh, &faces, true);
    assert!(none_flipped(&mesh));
    let ratios: Vec<f32> = faces
        .iter()
        .map(|f| {
            let list = mesh.face(*f).unwrap();
            let p = |i: usize| mesh.position(list[i]).unwrap();
            let real = ((p(1) - p(0)).cross(p(2) - p(0)).length()
                + (p(2) - p(0)).cross(p(3) - p(0)).length())
                / 2.0;
            uv_area(&mesh, *f) / real
        })
        .collect();
    let most = ratios.iter().copied().fold(0.0f32, f32::max);
    let least = ratios.iter().copied().fold(f32::MAX, f32::min);
    assert!(
        most / least < 1.5,
        "every quad keeps its size: {least} to {most}"
    );
    // A ring that barely tapers (a wide band) is still straightened.
    let mut band = ring(32, 0.85);
    let faces = all_faces(&band);
    unwrap(&mut band, &faces, true);
    assert!(least_square(&band) < 0.05, "laid out as a straight grid");
}
