use super::*;
use crate::PrimitiveKind;
use crate::deform::deform;

fn cage(divisions: [u32; 3], interpolation: Interpolation) -> Lattice {
    Lattice::new(Vec3::splat(200.0), divisions, interpolation)
}

/// `lattice` with the point at `i` moved by `by`.
fn pulled(lattice: &Lattice, i: usize, by: Vec3) -> Lattice {
    lattice
        .edited(&LatticeEdit::Transform {
            points: vec![i as u32],
            matrix: Mat4::from_translation(by),
        })
        .unwrap()
}

fn near(a: Vec3, b: Vec3) -> bool {
    a.distance(b) < 1e-3
}

#[test]
fn a_new_cage_is_at_rest_and_changes_nothing() {
    for interpolation in Interpolation::ALL {
        for divisions in [[2, 2, 2], [3, 4, 5], [16, 2, 7]] {
            let lattice = cage(divisions, interpolation);
            assert!(lattice.is_at_rest());
            assert_eq!(
                lattice.point_count(),
                divisions.iter().product::<u32>() as usize
            );
            let d = lattice.deformer();
            for p in [Vec3::ZERO, Vec3::splat(99.0), Vec3::new(-30.0, 70.0, 5.0)] {
                assert!(near(d.moved(p), p), "{p} {divisions:?} {interpolation:?}");
            }
        }
    }
}

#[test]
fn the_rest_places_fill_the_box_evenly() {
    let lattice = cage([3, 2, 2], Interpolation::Smooth);
    assert_eq!(lattice.rest(0), Vec3::splat(-100.0));
    assert_eq!(lattice.rest(1), Vec3::new(0.0, -100.0, -100.0));
    assert_eq!(lattice.rest(lattice.index(2, 1, 1)), Vec3::splat(100.0));
    assert_eq!(lattice.lines().len(), 2 * 2 * 2 + 3 * 2 + 3 * 2);
    assert_eq!(cage([2, 2, 2], Interpolation::Smooth).lines().len(), 12);
}

#[test]
fn settings_are_brought_within_limits() {
    let lattice = Lattice::new(
        Vec3::new(f32::NAN, -5.0, 1e12),
        [0, 1, 99],
        Interpolation::Linear,
    );
    assert_eq!(lattice.divisions(), [2, 2, 16]);
    assert_eq!(
        lattice.size(),
        Vec3::new(Lattice::MIN_SIZE, 5.0, Lattice::MAX_DISTANCE)
    );
    assert!(lattice.points().iter().all(|p| p.is_finite()));
}

#[test]
fn a_fitted_cage_has_room_inside_a_flat_shape() {
    let plane = Lattice::fitted(Vec3::new(200.0, 100.0, 0.0), Lattice::STANDARD_DIVISIONS);
    assert_eq!(plane.size(), Vec3::new(200.0, 100.0, 10.0));
    let dot = Lattice::fitted(Vec3::ZERO, [2, 2, 2]);
    assert_eq!(dot.size(), Vec3::splat(0.1));
    let lost = Lattice::fitted(Vec3::NAN, [2, 2, 2]);
    assert!(lost.size().is_finite());
}

#[test]
fn moving_a_corner_moves_the_shape_s_corner_and_fades_away() {
    for interpolation in Interpolation::ALL {
        let lattice = cage([2, 2, 2], interpolation);
        let corner = lattice.index(1, 1, 1);
        let d = pulled(&lattice, corner, Vec3::new(0.0, 0.0, 50.0)).deformer();
        assert!(near(
            d.moved(Vec3::splat(100.0)),
            Vec3::new(100.0, 100.0, 150.0)
        ));
        // Halfway along each side: an eighth of the pull (trilinear).
        assert!(near(d.moved(Vec3::ZERO), Vec3::new(0.0, 0.0, 50.0 / 8.0)));
        // The opposite corner doesn't move.
        assert!(near(d.moved(Vec3::splat(-100.0)), Vec3::splat(-100.0)));
    }
}

#[test]
fn smooth_blends_more_widely_than_linear() {
    let at = |interpolation| {
        let lattice = cage([5, 5, 5], interpolation);
        let middle = lattice.index(2, 2, 2);
        pulled(&lattice, middle, Vec3::Z * 40.0).deformer()
    };
    let (linear, smooth) = (at(Interpolation::Linear), at(Interpolation::Smooth));
    // Linear moves the place of the point fully, and nothing a cell away.
    assert!(near(linear.moved(Vec3::ZERO), Vec3::Z * 40.0));
    assert!(near(
        linear.moved(Vec3::new(50.0, 0.0, 0.0)),
        Vec3::new(50.0, 0.0, 0.0)
    ));
    // Smooth moves less at the point, but still a cell away.
    let mid = smooth.moved(Vec3::ZERO).z;
    assert!(mid > 1.0 && mid < 40.0, "{mid}");
    assert!(smooth.moved(Vec3::new(50.0, 0.0, 0.0)).z > 0.1);
    // And it changes smoothly: no jumps between near places.
    let mut last = smooth.moved(Vec3::new(-100.0, 0.0, 0.0));
    for step in 1..=400 {
        let x = -100.0 + step as f32 * 0.5;
        let now = smooth.moved(Vec3::new(x, 0.0, 0.0));
        assert!((now.z - last.z).abs() < 1.0, "{x}");
        last = now;
    }
}

#[test]
fn places_outside_follow_the_nearest_side() {
    let lattice = cage([2, 2, 2], Interpolation::Smooth);
    let top: Vec<u32> = (0..8)
        .filter(|i| lattice.rest(*i).z > 0.0)
        .map(|i| i as u32)
        .collect();
    let raised = lattice
        .edited(&LatticeEdit::Transform {
            points: top,
            matrix: Mat4::from_translation(Vec3::Z * 30.0),
        })
        .unwrap()
        .deformer();
    // Above the box: moved as its top is.
    assert!(near(
        raised.moved(Vec3::new(0.0, 0.0, 300.0)),
        Vec3::new(0.0, 0.0, 330.0)
    ));
    // Below: as its bottom, which stayed.
    assert!(near(
        raised.moved(Vec3::new(0.0, 0.0, -300.0)),
        Vec3::new(0.0, 0.0, -300.0)
    ));
}

#[test]
fn changing_divisions_keeps_the_cage_s_shape() {
    let lattice = cage([2, 2, 2], Interpolation::Linear);
    let pulled = pulled(&lattice, 7, Vec3::new(20.0, -10.0, 60.0));
    let finer = pulled.edited(&LatticeEdit::Divisions([3, 3, 3])).unwrap();
    assert_eq!(finer.divisions(), [3, 3, 3]);
    let (a, b) = (pulled.deformer(), finer.deformer());
    for p in [Vec3::ZERO, Vec3::splat(70.0), Vec3::new(-20.0, 90.0, 10.0)] {
        assert!(near(a.moved(p), b.moved(p)), "{p}");
    }
    let same = pulled.edited(&LatticeEdit::Divisions([2, 2, 2])).unwrap();
    assert_eq!(same, pulled);
    let clamped = pulled.edited(&LatticeEdit::Divisions([0, 40, 2])).unwrap();
    assert_eq!(clamped.divisions(), [2, 16, 2]);
}

#[test]
fn reset_and_interpolation_edits() {
    let lattice = pulled(&cage([3, 3, 3], Interpolation::Smooth), 4, Vec3::X * 9.0);
    assert!(!lattice.is_at_rest());
    let reset = lattice.edited(&LatticeEdit::Reset).unwrap();
    assert!(reset.is_at_rest());
    assert_eq!(reset.divisions(), [3, 3, 3]);
    let linear = lattice
        .edited(&LatticeEdit::Interpolation(Interpolation::Linear))
        .unwrap();
    assert_eq!(linear.interpolation(), Interpolation::Linear);
    assert_eq!(linear.points(), lattice.points());
    assert!(!Interpolation::Linear.name().is_empty());
}

#[test]
fn moves_to_no_place_or_too_far_are_refused() {
    let lattice = cage([2, 2, 2], Interpolation::Smooth);
    let to = |matrix| {
        lattice.edited(&LatticeEdit::Transform {
            points: vec![0],
            matrix,
        })
    };
    assert_eq!(
        to(Mat4::from_translation(Vec3::NAN)),
        Err(LatticeProblem::NotANumber)
    );
    assert_eq!(
        to(Mat4::from_translation(Vec3::X * 2e6)),
        Err(LatticeProblem::TooFar)
    );
    // Places that aren't there are left alone.
    let none = lattice
        .edited(&LatticeEdit::Transform {
            points: vec![99, 8],
            matrix: Mat4::from_translation(Vec3::X),
        })
        .unwrap();
    assert_eq!(none, lattice);
    for problem in [
        LatticeProblem::NotANumber,
        LatticeProblem::TooFar,
        LatticeProblem::Damaged,
    ] {
        assert!(!problem.to_string().is_empty());
    }
}

#[test]
fn saved_parts_are_checked() {
    let lattice = pulled(&cage([2, 3, 2], Interpolation::Linear), 3, Vec3::Y);
    let back = Lattice::from_parts(
        lattice.divisions(),
        lattice.size(),
        lattice.points().to_vec(),
        lattice.interpolation(),
    );
    assert_eq!(back, Ok(lattice.clone()));
    let points = lattice.points().to_vec();
    let bad = |divisions, size, points: Vec<Vec3>| {
        Lattice::from_parts(divisions, size, points, Interpolation::Smooth).is_err()
    };
    assert!(bad([2, 3, 2], lattice.size(), points[1..].to_vec()));
    assert!(bad([1, 3, 2], lattice.size(), points.clone()));
    assert!(bad([2, 3, 17], lattice.size(), points.clone()));
    assert!(bad([2, 3, 2], Vec3::new(0.0, 1.0, 1.0), points.clone()));
    assert!(bad([2, 3, 2], Vec3::NAN, points.clone()));
    let mut lost = points.clone();
    lost[2] = Vec3::INFINITY;
    assert!(bad([2, 3, 2], lattice.size(), lost));
    let mut far = points;
    far[0] = Vec3::Z * 1e7;
    assert!(bad([2, 3, 2], lattice.size(), far));
}

#[test]
fn the_points_mesh_names_each_point_by_its_place() {
    let lattice = pulled(&cage([2, 2, 3], Interpolation::Smooth), 5, Vec3::Z);
    let mesh = lattice.points_mesh();
    assert_eq!(mesh.vertex_count(), 12);
    assert_eq!(mesh.face_count(), 0);
    for (id, p) in mesh.vertices() {
        assert_eq!(lattice.points()[Lattice::point_of(id) as usize], p);
    }
}

#[test]
fn deforming_shapes_of_every_sort_keeps_their_polygons() {
    let lattice = cage([3, 3, 3], Interpolation::Smooth);
    let top = lattice.index(1, 1, 2);
    let d = pulled(&lattice, top, Vec3::Z * 50.0).deformer();
    let mut shapes: Vec<Mesh> = [
        PrimitiveKind::Cube,
        PrimitiveKind::Cylinder,
        PrimitiveKind::Plane,
        PrimitiveKind::Sphere,
    ]
    .into_iter()
    .map(|k| k.standard().mesh())
    .collect();
    // An n-gon with a zero-length edge, and a fin of three polygons on one
    // edge (not manifold).
    let mut odd = Mesh::new();
    let v: Vec<VertexId> = [
        Vec3::ZERO,
        Vec3::X * 50.0,
        Vec3::X * 50.0,
        Vec3::new(50.0, 50.0, 0.0),
        Vec3::new(0.0, 50.0, 0.0),
        Vec3::Z * 50.0,
        Vec3::new(0.0, -50.0, 0.0),
    ]
    .into_iter()
    .map(|p| odd.add_vertex(p).unwrap())
    .collect();
    odd.add_face(&[v[0], v[1], v[2], v[3], v[4]]).unwrap();
    odd.add_face(&[v[0], v[4], v[5]]).unwrap();
    odd.add_face(&[v[4], v[0], v[6]]).unwrap();
    shapes.push(odd);
    shapes.push(Mesh::default());
    for shape in shapes {
        let out = deform(&shape, Mat4::IDENTITY, &d).unwrap();
        assert!(out.same_structure(&shape));
        assert!(out.vertices().all(|(_, p)| p.is_finite()));
        if let Some(b) = shape.bounds()
            && b.size().z > 1.0
        {
            assert!(
                out.bounds().unwrap().size().z > b.size().z,
                "the top is raised"
            );
        }
    }
}

#[test]
fn ten_thousand_cage_moves_stay_real() {
    // A small fuzz: random-ish moves and divisions never give a place
    // that isn't a number.
    let mut lattice = cage([4, 3, 5], Interpolation::Smooth);
    let mut seed = 12345u32;
    let mut next = || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (seed >> 8) as f32 / (1u32 << 24) as f32
    };
    for round in 0..10_000 {
        let i = (next() * lattice.point_count() as f32) as u32;
        let by = Vec3::new(next(), next(), next()) * 400.0 - 200.0;
        let edit = if round % 997 == 0 {
            LatticeEdit::Divisions([2 + (next() * 6.0) as u32, 3, 2 + (next() * 4.0) as u32])
        } else {
            LatticeEdit::Transform {
                points: vec![i],
                matrix: Mat4::from_translation(by),
            }
        };
        if let Ok(next_lattice) = lattice.edited(&edit) {
            lattice = next_lattice;
        }
    }
    let d = lattice.deformer();
    for step in 0..100 {
        let p = Vec3::splat(step as f32 * 3.0 - 150.0);
        assert!(d.moved(p).is_finite());
    }
}

#[test]
#[ignore = "timing: run with --release -- --ignored --nocapture"]
fn timing() {
    use crate::primitives::SphereParams;
    let sphere = crate::Primitive::Sphere(SphereParams {
        radius: 90.0,
        sides: 1024,
        rings: 512,
    })
    .mesh();
    for (divisions, interpolation) in [
        ([2, 2, 2], Interpolation::Linear),
        ([4, 4, 4], Interpolation::Smooth),
        ([16, 16, 16], Interpolation::Smooth),
    ] {
        let lattice = Lattice::new(Vec3::splat(200.0), divisions, interpolation);
        let d = pulled(&lattice, 1, Vec3::Z * 20.0).deformer();
        let start = std::time::Instant::now();
        let out = deform(&sphere, Mat4::IDENTITY, &d).unwrap();
        println!(
            "{divisions:?} {interpolation:?}: {} points in {:.1} ms",
            out.vertex_count(),
            start.elapsed().as_secs_f64() * 1000.0
        );
    }
}
