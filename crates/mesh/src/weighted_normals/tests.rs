//! Tests for weighted normals (decision 0148).

use super::*;
use crate::bevel::{BevelOptions, BevelShape, bevel_edges};
use crate::custom_normals::{custom_normal_list, has_custom_normals};
use crate::{EdgeKey, Primitive};

/// A 100 cm cube with every edge bevelled 5 cm in `segments` rounded
/// segments, every edge soft: the usual shading smears across the big
/// faces.
fn bevelled_cube(segments: u32) -> Mesh {
    let mut mesh = Primitive::Cube(Default::default()).mesh();
    let edges: Vec<EdgeKey> = mesh.edges().to_vec();
    bevel_edges(
        &mut mesh,
        &edges,
        BevelShape {
            width: 5.0,
            segments,
            profile: 1.0,
            options: BevelOptions::default(),
        },
    );
    let all: Vec<EdgeKey> = mesh.edges().to_vec();
    for e in all {
        let (a, b) = e.ends();
        mesh.set_hard(a, b, false);
    }
    mesh
}

/// The polygons of `mesh` bigger than `area` (the cube's big faces).
fn big_faces(mesh: &Mesh, area: f32) -> Vec<FaceId> {
    mesh.faces()
        .filter(|(_, c)| {
            let points: Vec<Vec3> = c.iter().map(|v| mesh.position(*v).unwrap()).collect();
            newell(&points).length() / 2.0 > area
        })
        .map(|(f, _)| f)
        .collect()
}

/// The largest angle, in degrees, between a big face's corner normals
/// and the face's own facing.
fn worst_lean(mesh: &Mesh, normals: &[Vec3]) -> f32 {
    let (starts, _) = mesh.face_layout();
    let mut worst = 0.0f32;
    for f in big_faces(mesh, 5_000.0) {
        let facing = mesh.face_normal(f);
        let i = f.get() as usize;
        for n in &normals[starts[i] as usize..starts[i + 1] as usize] {
            worst = worst.max(n.angle_between(facing).to_degrees());
        }
    }
    worst
}

fn shading(mesh: &Mesh) -> Vec<Vec3> {
    crate::corner_shading_with(mesh, Hardness::Marked).1
}

#[test]
fn a_bevelled_cubes_big_faces_shade_flat() {
    for segments in [1, 3] {
        let cube = bevelled_cube(segments);
        assert_eq!(big_faces(&cube, 5_000.0).len(), 6);
        // The usual shading leans the big faces' corners into the bevel.
        let before = worst_lean(&cube, &shading(&cube));
        assert!(before > 5.0, "{segments}: {before}");
        let out = weighted_normals(&cube, &WeightedNormalsOptions::default(), Hardness::Marked);
        let after = worst_lean(&out, &shading(&out));
        assert!(after < 0.05, "{segments}: {after}");
        // The bevels still shade round: their corners aren't all their own
        // polygon's facing.
        let normals = shading(&out);
        let (starts, _) = out.face_layout();
        let mut round = 0;
        for (f, _) in out.faces() {
            let i = f.get() as usize;
            for n in &normals[starts[i] as usize..starts[i + 1] as usize] {
                if n.angle_between(out.face_normal(f)).to_degrees() > 5.0 {
                    round += 1;
                }
            }
        }
        assert!(round > 24, "{segments}: {round}");
    }
}

#[test]
fn only_the_custom_normals_change() {
    let cube = bevelled_cube(2);
    let out = weighted_normals(&cube, &WeightedNormalsOptions::default(), Hardness::Marked);
    assert!(has_custom_normals(&out));
    assert!(!has_custom_normals(&cube));
    let mut back = out.clone();
    crate::custom_normals::forget_custom_normals(&mut back);
    assert_eq!(back, cube);
    // Every corner has one, of unit length.
    let list = custom_normal_list(&out).unwrap();
    assert_eq!(list.len(), cube.face_layout().1.len());
    for n in list {
        assert!((n.unwrap().length() - 1.0).abs() < 1e-4);
    }
}

#[test]
fn it_is_deterministic() {
    let cube = bevelled_cube(3);
    for weight_by in [WeightBy::Area, WeightBy::Angle, WeightBy::AreaAndAngle] {
        let options = WeightedNormalsOptions {
            weight_by,
            ..Default::default()
        };
        let a = weighted_normals(&cube, &options, Hardness::Marked);
        let b = weighted_normals(&cube, &options, Hardness::Marked);
        assert_eq!(a, b);
    }
}

#[test]
fn no_strength_gives_the_usual_shading() {
    let cube = bevelled_cube(2);
    let options = WeightedNormalsOptions {
        strength: 0.0,
        ..Default::default()
    };
    let out = weighted_normals(&cube, &options, Hardness::Marked);
    for (a, b) in shading(&out).iter().zip(shading(&cube)) {
        assert!(a.distance(b) < 1e-5);
    }
    // Half strength lies between.
    let half = weighted_normals(
        &cube,
        &WeightedNormalsOptions {
            strength: 0.5,
            ..Default::default()
        },
        Hardness::Marked,
    );
    let full = weighted_normals(&cube, &Default::default(), Hardness::Marked);
    let lean = |m: &Mesh| worst_lean(m, &shading(m));
    assert!(lean(&half) < lean(&cube) && lean(&half) > lean(&full));
}

#[test]
fn angle_weighting_with_no_influence_is_the_usual_shading() {
    let cube = bevelled_cube(2);
    let options = WeightedNormalsOptions {
        weight_by: WeightBy::Angle,
        influence: 0.0,
        ..Default::default()
    };
    let out = weighted_normals(&cube, &options, Hardness::Marked);
    for (a, b) in shading(&out).iter().zip(shading(&cube)) {
        assert!(a.distance(b) < 1e-4, "{a} {b}");
    }
}

#[test]
fn more_influence_flattens_more() {
    let cube = bevelled_cube(1);
    let lean = |influence: f32| {
        let options = WeightedNormalsOptions {
            influence,
            ..Default::default()
        };
        let out = weighted_normals(&cube, &options, Hardness::Marked);
        worst_lean(&out, &shading(&out))
    };
    let (none, half, full) = (lean(0.0), lean(0.75), lean(1.0));
    assert!(none > half && half >= full, "{none} {half} {full}");
    assert!(full < 1e-3);
}

#[test]
fn hard_edges_are_kept_or_smoothed_over() {
    // A plain cube, every edge hard: kept, each face shades flat.
    let mut cube = Primitive::Cube(Default::default()).mesh();
    let edges: Vec<EdgeKey> = cube.edges().to_vec();
    for e in &edges {
        let (a, b) = e.ends();
        cube.set_hard(a, b, true);
    }
    let kept = weighted_normals(&cube, &Default::default(), Hardness::Marked);
    assert!(worst_lean(&kept, &shading(&kept)) < 1e-3);
    assert_eq!(
        crate::hardness::shading_hard_edges(&kept, Hardness::Marked).len(),
        12
    );
    // Off: one normal per point, the whole cube smooth (the three equal
    // faces at a corner share it: it points out of the corner).
    let smooth = weighted_normals(
        &cube,
        &WeightedNormalsOptions {
            keep_hard_edges: false,
            ..Default::default()
        },
        Hardness::Marked,
    );
    let normals = shading(&smooth);
    let (_, corners) = smooth.face_layout();
    let middle = smooth.bounds().unwrap().centre();
    for (k, n) in normals.iter().enumerate() {
        let p = smooth.position(corners[k]).unwrap() - middle;
        assert!(n.dot(p.normalize()) > 0.999, "{p} {n}");
    }
    assert!(crate::hardness::shading_hard_edges(&smooth, Hardness::Marked).is_empty());
    // By angle: a cube's 90° edges are hard at 30°.
    let soft = Primitive::Cube(Default::default()).mesh();
    let by_angle = weighted_normals(&soft, &Default::default(), Hardness::ByAngle(30.0));
    assert!(worst_lean(&by_angle, &shading(&by_angle)) < 1e-3);
}

#[test]
fn odd_shapes_never_fail() {
    // Empty.
    let empty = weighted_normals(&Mesh::new(), &Default::default(), Hardness::Marked);
    assert_eq!(empty, Mesh::new());
    // A loose point and an n-gon, a polygon with no area, an edge of no
    // length, an open border and an edge shared by three polygons.
    let mut mesh = Mesh::new();
    let p = [
        Vec3::ZERO,
        Vec3::X,
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.5, 1.5, 0.0),
        Vec3::Y,
        Vec3::new(0.0, -1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(3.0, 3.0, 3.0),
    ]
    .map(|q| mesh.add_vertex(q).unwrap());
    mesh.add_face(&[p[0], p[1], p[2], p[3], p[4]]).unwrap();
    mesh.add_face(&[p[1], p[0], p[5]]).unwrap();
    mesh.add_face(&[p[0], p[1], p[6]]).unwrap();
    mesh.add_face(&[p[0], p[1], p[7]]).unwrap();
    let same = mesh.add_vertex(Vec3::new(2.0, 0.0, 0.0)).unwrap();
    mesh.add_face(&[p[1], p[7], same]).unwrap();
    for weight_by in [WeightBy::Area, WeightBy::Angle, WeightBy::AreaAndAngle] {
        for keep_hard_edges in [false, true] {
            let options = WeightedNormalsOptions {
                weight_by,
                keep_hard_edges,
                ..Default::default()
            };
            let out = weighted_normals(&mesh, &options, Hardness::ByAngle(30.0));
            for n in shading(&out) {
                assert!(n.is_finite() && (n.length() - 1.0).abs() < 1e-3, "{n}");
            }
        }
    }
    // A point that isn't a number.
    let mut bad = mesh.clone();
    bad.set_position(p[8], Vec3::NAN);
    let out = weighted_normals(&bad, &Default::default(), Hardness::Marked);
    assert!(shading(&out).iter().all(|n| n.is_finite()));
    // A face with a NaN corner shades without one.
    bad.set_position(p[3], Vec3::NAN);
    let out = weighted_normals(&bad, &Default::default(), Hardness::Marked);
    assert!(shading(&out).iter().all(|n| n.is_finite()));
}

#[test]
fn settings_are_clamped() {
    for (strength, influence) in [
        (f32::NAN, f32::NAN),
        (f32::INFINITY, -1.0),
        (-5.0, 7.0),
        (1e30, f32::NEG_INFINITY),
    ] {
        let options = WeightedNormalsOptions {
            strength,
            influence,
            ..Default::default()
        }
        .clamped();
        assert!((0.0..=1.0).contains(&options.strength));
        assert!((0.0..=1.0).contains(&options.influence));
        assert!(options.power() >= 1.0 && options.power() <= 64.0);
        let out = weighted_normals(&bevelled_cube(1), &options, Hardness::Marked);
        assert!(shading(&out).iter().all(|n| n.is_finite()));
    }
    let standard = WeightedNormalsOptions {
        strength: f32::NAN,
        influence: f32::NAN,
        ..Default::default()
    }
    .clamped();
    assert_eq!(standard, WeightedNormalsOptions::default());
}

#[test]
fn custom_normals_given_are_replaced() {
    let cube = bevelled_cube(1);
    let mut given = cube.clone();
    let corners = given.face_layout().1.len();
    crate::custom_normals::set_custom_normals(&mut given, &vec![Vec3::X; corners]);
    let a = weighted_normals(&given, &Default::default(), Hardness::Marked);
    let b = weighted_normals(&cube, &Default::default(), Hardness::Marked);
    assert_eq!(a, b);
}

#[test]
fn a_heavy_shape_is_quick() {
    let ball = crate::samples::heavy_ball(200_000, 100.0);
    let start = std::time::Instant::now();
    let out = weighted_normals(&ball, &Default::default(), Hardness::Marked);
    let took = start.elapsed();
    assert!(has_custom_normals(&out));
    // Generous for a debug build on a slow machine; a release build takes
    // a small share of it.
    assert!(took.as_secs_f32() < 20.0, "{took:?}");
}
