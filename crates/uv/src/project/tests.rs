//! Tests for projections (decision 0235).

use super::*;
use mesh::{CubeParams, CylinderParams, PlaneParams, Primitive, SphereParams};

fn all_faces(mesh: &Mesh) -> Vec<FaceId> {
    mesh.faces().map(|(f, _)| f).collect()
}

fn uvs(mesh: &Mesh) -> Vec<Vec2> {
    let uvs = Uvs::of(mesh).unwrap();
    (0..mesh.face_layout().1.len()).map(|i| uvs.at(i)).collect()
}

fn bounds(mesh: &Mesh) -> (Vec3, Vec3) {
    mesh.vertices().fold(
        (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
        |(lo, hi), (_, p)| (lo.min(p), hi.max(p)),
    )
}

/// Each polygon's signed UV area (positive: not mirrored).
fn uv_areas(mesh: &Mesh) -> Vec<f32> {
    let uvs = Uvs::of(mesh).unwrap();
    let (starts, _) = mesh.face_layout();
    mesh.faces()
        .map(|(f, list)| {
            let s = starts[f.get() as usize] as usize;
            let mut a = 0.0;
            for k in 0..list.len() {
                let p = uvs.at(s + k);
                let q = uvs.at(s + (k + 1) % list.len());
                a += p.perp_dot(q);
            }
            a / 2.0
        })
        .collect()
}

#[test]
fn a_flat_projection_of_a_plane_fills_the_square_evenly() {
    let mut plane = Primitive::Plane(PlaneParams {
        segments: [2, 2],
        ..Default::default()
    })
    .mesh();
    let (lo, hi) = bounds(&plane);
    // The plane lies in X and Y in its own space? Whichever way, fit and
    // look along its flat axis.
    let thin = (hi - lo).min_position();
    let turn = match thin {
        0 => glam::Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2),
        1 => glam::Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2),
        _ => glam::Mat4::IDENTITY,
    };
    let options = ProjectOptions {
        kind: Projection::Planar,
        placement: turn * fit_box(lo, hi),
        world: None,
    };
    let faces = all_faces(&plane);
    let done = project(&mut plane, &faces, &options);
    assert_eq!(done.matched, faces.len());
    let laid = uvs(&plane);
    for uv in &laid {
        assert!(
            (0.0..=1.0).contains(&uv.x) && (0.0..=1.0).contains(&uv.y),
            "{uv}"
        );
    }
    // Every polygon gets the same area: the projection is even.
    let areas: Vec<f32> = uv_areas(&plane).iter().map(|a| a.abs()).collect();
    for a in &areas {
        assert!((a - areas[0]).abs() < 1e-5, "{areas:?}");
    }
    assert!(areas[0] > 0.0);
}

#[test]
fn a_box_projection_lays_each_side_of_a_cube_unmirrored() {
    let mut cube = Primitive::Cube(CubeParams::default()).mesh();
    let (lo, hi) = bounds(&cube);
    let options = ProjectOptions {
        kind: Projection::Box,
        placement: fit_box(lo, hi),
        world: None,
    };
    let faces = all_faces(&cube);
    project(&mut cube, &faces, &options);
    let areas = uv_areas(&cube);
    // Every side has the same size, none mirrored (the polygons wind the
    // same way in the texture as on the outside of the cube).
    let first = areas[0];
    for a in &areas {
        assert!((a - first).abs() < 1e-4, "{areas:?}");
    }
    assert!(first.abs() > 0.1);
    // The sides part in the texture, so the cube's edges become seams.
    assert_eq!(mesh::uvs::seams(&cube).len(), 12);
}

#[test]
fn a_cylinder_projection_goes_round_once_with_one_seam_line() {
    let mut cylinder = Primitive::Cylinder(CylinderParams::default()).mesh();
    let (lo, hi) = bounds(&cylinder);
    // Stand the projection along the cylinder's axis: the one whose
    // other two sizes are equal.
    let size = hi - lo;
    let along = if (size.x - size.y).abs() < 1e-3 {
        2
    } else if (size.x - size.z).abs() < 1e-3 {
        1
    } else {
        0
    };
    let turn = match along {
        1 => glam::Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        0 => glam::Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2),
        _ => glam::Mat4::IDENTITY,
    };
    let topology = cylinder.topology().clone();
    let ready_made: Vec<EdgeKey> = mesh::uvs::seams(&cylinder);
    // The side polygons: four corners each.
    let side: Vec<FaceId> = cylinder
        .faces()
        .filter(|(_, l)| l.len() == 4)
        .map(|(f, _)| f)
        .collect();
    let options = ProjectOptions {
        kind: Projection::Cylinder,
        placement: turn * fit_box(lo, hi),
        world: None,
    };
    let done = project(&mut cylinder, &side, &options);
    assert_eq!(done.matched, side.len());
    let laid = uvs(&cylinder);
    let (starts, _) = cylinder.face_layout();
    let mut total = 0.0;
    for f in &side {
        let s = starts[f.get() as usize] as usize;
        let us: Vec<f32> = (0..4).map(|k| laid[s + k].x).collect();
        let spread = us.iter().copied().fold(f32::MIN, f32::max)
            - us.iter().copied().fold(f32::MAX, f32::min);
        assert!(spread < 0.5, "no polygon wraps round the texture: {us:?}");
        for u in us {
            assert!((-1e-4..=1.0 + 1e-4).contains(&u), "{u}");
        }
        total += spread;
    }
    assert!((total - 1.0).abs() < 0.05, "once round: {total}");
    // One new line of seams down the back, between side polygons (the
    // ready-made seams stay).
    let seams = mesh::uvs::seams(&cylinder);
    let down: Vec<_> = seams
        .iter()
        .filter(|e| !ready_made.contains(e))
        .filter(|e| topology.edge_faces(**e).all(|f| side.contains(&f)))
        .collect();
    assert!(down.len() <= 1, "{down:?}");
    let side_seams = seams
        .iter()
        .filter(|e| topology.edge_faces(**e).all(|f| side.contains(&f)))
        .count();
    assert!(side_seams >= 1);
}

#[test]
fn a_ball_projection_runs_from_bottom_to_top_without_folding_at_the_poles() {
    let mut ball = Primitive::Sphere(SphereParams::default()).mesh();
    let (lo, hi) = bounds(&ball);
    let faces = all_faces(&ball);
    let options = ProjectOptions {
        kind: Projection::Sphere,
        placement: fit_box(lo, hi),
        world: None,
    };
    project(&mut ball, &faces, &options);
    let laid = uvs(&ball);
    let vs: Vec<f32> = laid.iter().map(|u| u.y).collect();
    let low = vs.iter().copied().fold(f32::MAX, f32::min);
    let high = vs.iter().copied().fold(f32::MIN, f32::max);
    assert!(low < 0.05 && high > 0.95, "{low} {high}");
    // No polygon is mirrored (or squashed to nothing) round the poles:
    // all wind the same way.
    let areas = uv_areas(&ball);
    let positive = areas.iter().filter(|a| **a > 0.0).count();
    let negative = areas.iter().filter(|a| **a < 0.0).count();
    assert!(positive == 0 || negative == 0, "{positive} {negative}");
}

#[test]
fn world_aligned_uvs_line_up_between_neighbours() {
    // Two cubes side by side in the world: projected in the world with
    // the same scale, the texture runs on across the join.
    let mut a = Primitive::Cube(CubeParams::default()).mesh();
    let mut b = a.clone();
    let (lo, hi) = bounds(&a);
    let width = hi.x - lo.x;
    let scale = 0.5;
    let at = |offset: f32| ProjectOptions {
        kind: Projection::Box,
        placement: glam::Mat4::from_translation(Vec3::new(offset, 0.0, 0.0)),
        world: Some(scale),
    };
    let fa = all_faces(&a);
    let fb = all_faces(&b);
    project(&mut a, &fa, &at(0.0));
    project(&mut b, &fb, &at(width));
    // The point at the right of a and the left of b, on the front side,
    // get the same texture place.
    let front_u = |m: &Mesh, x: f32| -> Option<f32> {
        let uvs = Uvs::of(m)?;
        let (starts, _) = m.face_layout();
        for (f, list) in m.faces() {
            let ps: Vec<Vec3> = list.iter().map(|v| m.position(*v).unwrap()).collect();
            if ps.iter().all(|p| (p.y - lo.y).abs() < 1e-4) {
                for (k, p) in ps.iter().enumerate() {
                    if (p.x - x).abs() < 1e-4 {
                        return Some(uvs.at(starts[f.get() as usize] as usize + k).x);
                    }
                }
            }
        }
        None
    };
    let right_of_a = front_u(&a, hi.x).unwrap();
    let left_of_b = front_u(&b, lo.x).unwrap();
    assert!(
        (right_of_a - left_of_b).abs() < 1e-4,
        "{right_of_a} {left_of_b}"
    );
}

#[test]
fn broken_options_and_odd_input_change_nothing() {
    let mut cube = Primitive::Cube(CubeParams::default()).mesh();
    let before = cube.clone();
    let faces = all_faces(&cube);
    let squashed = ProjectOptions {
        placement: glam::Mat4::from_scale(Vec3::new(1.0, 0.0, 1.0)),
        ..Default::default()
    };
    let done = project(&mut cube, &faces, &squashed);
    assert_eq!(done.matched, 0);
    assert_eq!(done.skipped, faces.len());
    let nan = ProjectOptions {
        world: Some(f32::NAN),
        ..Default::default()
    };
    assert_eq!(project(&mut cube, &faces, &nan).matched, 0);
    assert_eq!(cube, before, "nothing changed");
    // Polygons that aren't there are counted missing.
    let done = project(
        &mut cube,
        &[FaceId::from_raw(999)],
        &ProjectOptions::default(),
    );
    assert_eq!((done.matched, done.missing), (0, 1));
    // Nothing asked: nothing done.
    assert_eq!(
        project(&mut cube, &[], &ProjectOptions::default()).matched,
        0
    );
}

#[test]
fn locked_pieces_keep_their_uvs() {
    let mut cube = Primitive::Cube(CubeParams::default()).mesh();
    let faces = all_faces(&cube);
    mesh::uvs::set_locked(&mut cube, &faces[..1], true);
    let before = uvs(&cube);
    let (lo, hi) = bounds(&cube);
    let done = project(
        &mut cube,
        &faces,
        &ProjectOptions {
            kind: Projection::Planar,
            placement: fit_box(lo, hi),
            world: None,
        },
    );
    assert_eq!(done.skipped, 1);
    let after = uvs(&cube);
    let (starts, _) = cube.face_layout();
    let s = starts[faces[0].get() as usize] as usize;
    for k in 0..4 {
        assert_eq!(before[s + k], after[s + k]);
    }
}

#[test]
fn the_same_projection_gives_the_same_uvs() {
    let mut a = Primitive::Sphere(SphereParams::default()).mesh();
    let mut b = a.clone();
    let faces = all_faces(&a);
    let options = ProjectOptions {
        kind: Projection::Cylinder,
        placement: glam::Mat4::from_rotation_z(0.3),
        world: None,
    };
    project(&mut a, &faces, &options);
    project(&mut b, &faces, &options);
    assert_eq!(uvs(&a), uvs(&b));
}

#[test]
fn fit_box_holds_the_shape_inside_the_box() {
    let lo = Vec3::new(-1.0, 2.0, 0.0);
    let hi = Vec3::new(3.0, 4.0, 0.0);
    let m = fit_box(lo, hi);
    for p in [lo, hi, Vec3::new(3.0, 2.0, 0.0)] {
        let q = m.transform_point3(p);
        assert!(q.abs().max_element() <= 1.0 + 1e-5, "{q}");
    }
    assert!(m.is_finite());
    // A point (no size at all) still gives a usable placement.
    assert!(fit_box(Vec3::ONE, Vec3::ONE).determinant().abs() > 1e-12);
}
