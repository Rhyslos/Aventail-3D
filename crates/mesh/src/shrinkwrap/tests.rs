use super::*;
use crate::Primitive;
use crate::primitives::{CubeParams, PlaneParams, SphereParams};
use glam::Mat4;

fn sphere(radius: f32, sides: u32) -> Mesh {
    Primitive::Sphere(SphereParams {
        radius,
        sides,
        rings: sides / 2,
    })
    .mesh()
}

fn cube(size: f32, segments: u32) -> Mesh {
    Primitive::Cube(CubeParams {
        size: Vec3::splat(size),
        segments: [segments; 3],
    })
    .mesh()
}

fn grid(size: f32, segments: u32) -> Mesh {
    Primitive::Plane(PlaneParams {
        width: size,
        depth: size,
        segments: [segments, segments],
    })
    .mesh()
}

/// `mesh` with its points moved by `matrix` (ids kept).
fn moved(mesh: &Mesh, matrix: Mat4) -> Mesh {
    let mut out = mesh.clone();
    for (id, p) in mesh.vertices() {
        out.set_position(id, matrix.transform_point3(p));
    }
    out
}

fn tree(mesh: &Mesh) -> TriangleTree {
    TriangleTree::of_mesh(mesh, Mat4::IDENTITY)
}

fn centre(mesh: &Mesh) -> Vec3 {
    mesh.bounds().unwrap().centre()
}

fn mode(mode: WrapMode) -> ShrinkwrapOptions {
    ShrinkwrapOptions {
        mode,
        ..Default::default()
    }
}

#[test]
fn nearest_puts_every_point_on_the_target() {
    let target = sphere(100.0, 64);
    let middle = centre(&target);
    // A cube round the sphere's middle, wrapped onto it.
    let shape = moved(
        &cube(60.0, 4),
        Mat4::from_translation(middle - Vec3::Z * 30.0),
    );
    let out = shrinkwrap(&shape, &tree(&target), None, &mode(WrapMode::Nearest)).unwrap();
    for (_, p) in out.vertices() {
        let r = p.distance(middle);
        assert!((r - 100.0).abs() < 1.0, "{p} is {r} from the middle");
    }
    assert!(out.same_structure(&shape));
}

#[test]
fn offset_lifts_the_points_off_the_surface() {
    let target = sphere(100.0, 64);
    let middle = centre(&target);
    let shape = moved(
        &cube(60.0, 2),
        Mat4::from_translation(middle - Vec3::Z * 30.0),
    );
    for offset in [5.0, -5.0] {
        let out = shrinkwrap(
            &shape,
            &tree(&target),
            None,
            &ShrinkwrapOptions {
                offset,
                ..mode(WrapMode::Nearest)
            },
        )
        .unwrap();
        for (_, p) in out.vertices() {
            let r = p.distance(middle);
            assert!((r - (100.0 + offset)).abs() < 1.0, "{offset}: {r}");
        }
    }
}

#[test]
fn along_z_drops_points_onto_the_surface_below_or_above() {
    // A sheet high above a sphere: the points over it drop onto its top;
    // those beyond its edge meet nothing and stay.
    let target = sphere(50.0, 32);
    let middle = centre(&target);
    let sheet = moved(
        &grid(300.0, 30),
        Mat4::from_translation(Vec3::new(middle.x, middle.y, 500.0)),
    );
    let out = shrinkwrap(&sheet, &tree(&target), None, &mode(WrapMode::AxisZ)).unwrap();
    let mut dropped = 0;
    for (id, p) in out.vertices() {
        let before = sheet.position(id).unwrap();
        let flat = (before - middle).truncate().length();
        if flat < 45.0 {
            dropped += 1;
            assert!(p.z < 500.0 && p.z > middle.z, "{p}: onto the top half");
            assert!((p.distance(middle) - 50.0).abs() < 1.0);
            assert_eq!(p.truncate(), before.truncate(), "straight down");
        } else if flat > 55.0 {
            assert_eq!(p, before, "nothing below: it stays");
        }
    }
    assert!(dropped > 10);
    // From below, the sphere's underside is met (behind the line's start
    // counts as well).
    let low = moved(&sheet, Mat4::from_translation(Vec3::Z * -1000.0));
    let out = shrinkwrap(&low, &tree(&target), None, &mode(WrapMode::AxisZ)).unwrap();
    assert!(out.vertices().any(|(_, p)| p.z > -400.0 && p.z < middle.z));
}

#[test]
fn along_x_and_y_go_their_way() {
    let wall = moved(
        &grid(400.0, 1),
        Mat4::from_translation(Vec3::new(0.0, 0.0, 0.0))
            * Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2),
    );
    // The wall is the plane x = 0 now; a cube off to the side goes onto it.
    let shape = moved(&cube(20.0, 1), Mat4::from_translation(Vec3::X * 100.0));
    let out = shrinkwrap(&shape, &tree(&wall), None, &mode(WrapMode::AxisX)).unwrap();
    assert!(out.vertices().all(|(_, p)| p.x.abs() < 1e-3));
    // Along Y the line runs along the wall and never meets it.
    let out = shrinkwrap(&shape, &tree(&wall), None, &mode(WrapMode::AxisY)).unwrap();
    assert_eq!(out, shape);
}

#[test]
fn along_normals_a_small_sphere_grows_onto_a_big_one() {
    let big = sphere(100.0, 48);
    let small = sphere(20.0, 16);
    // The small one with the big one's middle.
    let small = moved(
        &small,
        Mat4::from_translation(centre(&big) - centre(&small)),
    );
    let middle = centre(&big);
    let out = shrinkwrap(&small, &tree(&big), None, &mode(WrapMode::Normals)).unwrap();
    for (id, p) in out.vertices() {
        assert!((p.distance(middle) - 100.0).abs() < 1.5, "{p}");
        // Straight out from the middle, as the small sphere faced.
        let before = small.position(id).unwrap() - middle;
        assert!((p - middle).normalize().dot(before.normalize()) > 0.99);
    }
}

#[test]
fn only_the_chosen_points_move() {
    let target = grid(500.0, 1);
    let shape = moved(&cube(20.0, 1), Mat4::from_translation(Vec3::Z * 50.0));
    let chosen: BTreeSet<VertexId> = shape.vertices().map(|(id, _)| id).take(3).collect();
    let out = shrinkwrap(
        &shape,
        &tree(&target),
        Some(&chosen),
        &mode(WrapMode::Nearest),
    )
    .unwrap();
    for (id, p) in out.vertices() {
        if chosen.contains(&id) {
            assert!(p.z.abs() < 1e-3);
        } else {
            assert_eq!(Some(p), shape.position(id));
        }
    }
    // Chosen ids that aren't in the shape do nothing.
    let gone: BTreeSet<VertexId> = [VertexId::from_raw(9999)].into();
    let out = shrinkwrap(
        &shape,
        &tree(&target),
        Some(&gone),
        &mode(WrapMode::Nearest),
    )
    .unwrap();
    assert_eq!(out, shape);
}

#[test]
fn a_target_with_no_surface_is_refused() {
    let shape = cube(10.0, 1);
    assert_eq!(
        shrinkwrap(&shape, &tree(&Mesh::new()), None, &mode(WrapMode::Nearest)),
        Err(ShrinkwrapProblem::NoSurface)
    );
    assert!(!ShrinkwrapProblem::NoSurface.to_string().is_empty());
    // An empty shape stays empty.
    let out = shrinkwrap(&Mesh::new(), &tree(&shape), None, &mode(WrapMode::Normals)).unwrap();
    assert_eq!(out, Mesh::new());
}

#[test]
fn degenerate_shapes_keep_their_polygons() {
    let mut mesh = Mesh::new();
    let p: Vec<VertexId> = [
        Vec3::new(0.0, 0.0, 10.0),
        Vec3::new(10.0, 0.0, 10.0),
        Vec3::new(10.0, 0.0, 10.0), // a zero-length edge
        Vec3::new(20.0, 5.0, 10.0),
        Vec3::new(5.0, 10.0, 10.0),
        Vec3::new(5.0, -10.0, 13.0),
        Vec3::new(5.0, 0.0, 19.0),
        Vec3::new(50.0, 50.0, 50.0), // a loose point
    ]
    .iter()
    .map(|p| mesh.add_vertex(*p).unwrap())
    .collect();
    mesh.add_face(&[p[0], p[1], p[2], p[3], p[4]]).unwrap();
    mesh.add_face(&[p[0], p[1], p[5]]).unwrap();
    mesh.add_face(&[p[1], p[0], p[6]]).unwrap();
    let target = sphere(30.0, 16);
    for wrap in WrapMode::ALL {
        let out = shrinkwrap(&mesh, &tree(&target), None, &mode(wrap)).unwrap();
        assert!(out.same_structure(&mesh), "{wrap:?}");
        assert!(out.vertices().all(|(_, p)| p.is_finite()), "{wrap:?}");
    }
    // The loose point has no normal, so along normals it stays.
    let out = shrinkwrap(&mesh, &tree(&target), None, &mode(WrapMode::Normals)).unwrap();
    assert_eq!(out.position(p[7]), mesh.position(p[7]));
}

#[test]
fn point_normals_follow_the_polygons_round_each_point() {
    let shape = cube(10.0, 1);
    let normals = point_normals(&shape);
    let middle = centre(&shape);
    for (id, p) in shape.vertices() {
        // A cube's corner faces out along its diagonal.
        let n = normals[id.get() as usize];
        assert!(n.dot((p - middle).normalize()) > 0.99, "{p} {n}");
    }
    let mut lone = Mesh::new();
    lone.add_vertex(Vec3::ONE).unwrap();
    assert_eq!(point_normals(&lone), vec![Vec3::ZERO]);
}

#[test]
fn settings_out_of_range_are_brought_within_limits() {
    let wild = ShrinkwrapOptions {
        mode: WrapMode::AxisY,
        offset: f32::NAN,
    }
    .clamped();
    assert_eq!(wild.offset, 0.0);
    assert_eq!(wild.mode, WrapMode::AxisY);
    let far = ShrinkwrapOptions {
        offset: 1e20,
        ..Default::default()
    }
    .clamped();
    assert_eq!(far.offset, ShrinkwrapOptions::MAX_OFFSET);
    for wrap in WrapMode::ALL {
        assert!(!wrap.name().is_empty());
    }
}

#[test]
fn the_same_settings_give_the_same_shape() {
    let target = cube(100.0, 3);
    let shape = sphere(80.0, 24);
    for wrap in WrapMode::ALL {
        let a = shrinkwrap(&shape, &tree(&target), None, &mode(wrap)).unwrap();
        let b = shrinkwrap(&shape, &tree(&target), None, &mode(wrap)).unwrap();
        assert_eq!(a, b, "{wrap:?}");
    }
}

#[test]
fn random_settings_never_give_a_point_that_isnt_a_number() {
    let shape = sphere(30.0, 12);
    let targets = [
        tree(&cube(50.0, 1)),
        tree(&grid(1e6, 1)),
        TriangleTree::new(&[[Vec3::ZERO, Vec3::X * 1e-3, Vec3::Y * 1e-3]]),
    ];
    let values = [0.0, -1.0, 1.0, 1e9, -1e9, f32::NAN, f32::INFINITY, 1e-9];
    let mut seed = 0x2545_f491_4f6c_dd1d_u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for _ in 0..100 {
        let options = ShrinkwrapOptions {
            mode: WrapMode::ALL[(next() % 5) as usize],
            offset: values[(next() % values.len() as u64) as usize],
        };
        let target = &targets[(next() % targets.len() as u64) as usize];
        let out = shrinkwrap(&shape, target, None, &options).unwrap();
        assert!(out.vertices().all(|(_, p)| p.is_finite()), "{options:?}");
    }
}

#[test]
#[ignore = "timing: run with --release -- --ignored --nocapture"]
fn timings() {
    let target = sphere(500.0, 512);
    let start = std::time::Instant::now();
    let tree = tree(&target);
    let built = start.elapsed();
    // A shape close inside the target (20 cm off, as a shape wrapped onto
    // another usually is), and one far inside (200 cm off: the nearest
    // search then looks at many more triangles).
    for (radius, wrap) in [
        (480.0, WrapMode::Nearest),
        (480.0, WrapMode::Normals),
        (480.0, WrapMode::AxisZ),
        (300.0, WrapMode::Nearest),
    ] {
        let shape = moved(
            &sphere(radius, 256),
            Mat4::from_translation(Vec3::Z * (500.0 - radius)),
        );
        let start = std::time::Instant::now();
        let out = shrinkwrap(&shape, &tree, None, &mode(wrap)).unwrap();
        eprintln!(
            "{} points {} cm inside onto {} triangles ({:.1} ms to build the tree), {wrap:?}: \
             {:.1} ms",
            out.vertex_count(),
            500.0 - radius,
            tree.len(),
            built.as_secs_f64() * 1000.0,
            start.elapsed().as_secs_f64() * 1000.0
        );
    }
}
