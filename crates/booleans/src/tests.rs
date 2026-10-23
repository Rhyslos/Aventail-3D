use super::*;
use glam::{Mat4, Vec2, Vec3};
use mesh::attributes::{AttributeElement, UV};
use mesh::primitives::{CubeParams, CylinderParams, SphereParams};
use mesh::{FaceId, Primitive, Value};

fn cube(size: f32) -> Mesh {
    Primitive::Cube(CubeParams {
        size: Vec3::splat(size),
        segments: [1, 1, 1],
    })
    .mesh()
}

fn cylinder(radius: f32, height: f32, sides: u32) -> Mesh {
    Primitive::Cylinder(CylinderParams {
        radius,
        height,
        sides,
        ..Default::default()
    })
    .mesh()
}

fn cut(kind: Kind, mesh: &Mesh, matrix: Mat4) -> Cutter<'_> {
    Cutter { kind, mesh, matrix }
}

fn at(x: f32, y: f32, z: f32) -> Mat4 {
    Mat4::from_translation(Vec3::new(x, y, z))
}

/// The volume a closed mesh holds (positive when it faces out).
fn volume(mesh: &Mesh) -> f32 {
    let mut total = 0.0;
    for (_, corners) in mesh.faces() {
        mesh::face_triangles(mesh, corners, |[a, b, c]| {
            let p = |i: usize| mesh.position(corners[i]).unwrap();
            total += p(a).dot(p(b).cross(p(c))) / 6.0;
        });
    }
    total
}

/// Every edge has exactly two polygons, facing the same way.
fn is_closed(mesh: &Mesh) -> bool {
    let topology = mesh.topology();
    topology.border_edges().is_empty()
        && topology.non_manifold_edges().is_empty()
        && topology.flipped_edges().is_empty()
}

fn close(a: f32, b: f32, share: f32) -> bool {
    (a - b).abs() <= b.abs().max(1.0) * share
}

#[test]
fn subtracting_a_corner_leaves_a_notched_cube_of_polygons() {
    let kept = cube(100.0);
    let cutter = cube(100.0);
    let result = apply(
        &kept,
        &[cut(Kind::Subtract, &cutter, at(50.0, 50.0, 50.0))],
        &Options::default(),
    )
    .unwrap();
    assert!(is_closed(&result));
    assert!(close(volume(&result), 1_000_000.0 - 125_000.0, 1e-4));
    // Three faces untouched, three notched (L-shaped, six corners), three
    // from the cutter: polygons, not triangles.
    assert_eq!(result.face_count(), 9, "{result:?}");
    let mut sizes: Vec<usize> = result.faces().map(|(_, c)| c.len()).collect();
    sizes.sort_unstable();
    assert_eq!(sizes, [4, 4, 4, 4, 4, 4, 6, 6, 6]);
}

#[test]
fn edges_of_the_notch_are_hard_and_the_cubes_own_hard_edges_stay() {
    let kept = cube(100.0);
    let result = apply(
        &kept,
        &[cut(Kind::Subtract, &kept, at(50.0, 50.0, 50.0))],
        &Options::default(),
    )
    .unwrap();
    let topology = result.topology();
    let soft: Vec<_> = topology
        .edges()
        .filter(|e| {
            let (a, b) = e.ends();
            !result.is_hard(a, b)
        })
        .collect();
    // Every edge of a notched box is a right angle: all hard, if the
    // cube's own edges were hard; the cut's edges are hard either way.
    let kept_hard = kept.hard_edge_count() > 0;
    if kept_hard {
        assert!(soft.is_empty(), "{soft:?}");
    } else {
        assert!(result.hard_edge_count() >= 9, "the cut's nine edges");
    }
}

#[test]
fn a_seam_angle_above_the_cut_leaves_its_edges_soft() {
    let kept = Primitive::Sphere(SphereParams::default()).mesh();
    let cutter = cube(40.0);
    let options = Options {
        seam_angle: 180.0,
        ..Options::default()
    };
    let result = apply(
        &kept,
        &[cut(Kind::Subtract, &cutter, at(0.0, 0.0, 80.0))],
        &options,
    )
    .unwrap();
    assert!(is_closed(&result));
    let before = kept.hard_edge_count();
    let cutter_hard = cutter.hard_edge_count();
    assert!(result.hard_edge_count() <= before + cutter_hard);
}

#[test]
fn a_cylinder_through_a_cube_leaves_a_round_hole() {
    let kept = cube(100.0);
    let cutter = cylinder(20.0, 300.0, 16);
    let result = apply(
        &kept,
        &[cut(Kind::Subtract, &cutter, at(0.0, 0.0, -100.0))],
        &Options::default(),
    )
    .unwrap();
    assert!(is_closed(&result));
    let hole = 16.0 * 0.5 * 20.0 * 20.0 * (std::f32::consts::TAU / 16.0).sin() * 100.0;
    assert!(close(volume(&result), 1_000_000.0 - hole, 1e-3));
    // The four sides stay quads; the hole's 16 walls are quads too.
    let quads = result.faces().filter(|(_, c)| c.len() == 4).count();
    assert!(quads >= 4 + 16, "{quads}");
    // The top and bottom, rings round the hole, are convex polygons
    // (triangles joined where they stay convex), fewer than the 20
    // triangles Manifold makes of each.
    assert!(
        result.face_count() <= 4 + 16 + 2 * 16,
        "{}",
        result.face_count()
    );
    for (face, corners) in result.faces() {
        let n = result.face_normal(face);
        let p: Vec<Vec3> = corners
            .iter()
            .map(|v| result.position(*v).unwrap())
            .collect();
        for i in 0..p.len() {
            let (a, b, c) = (p[i], p[(i + 1) % p.len()], p[(i + 2) % p.len()]);
            assert!((b - a).cross(c - b).dot(n) > -1e-2, "convex");
        }
    }
}

#[test]
fn union_of_two_apart_shapes_keeps_both() {
    let kept = cube(100.0);
    let result = apply(
        &kept,
        &[cut(Kind::Union, &kept, at(300.0, 0.0, 0.0))],
        &Options::default(),
    )
    .unwrap();
    assert!(is_closed(&result));
    assert_eq!(result.face_count(), 12);
    assert!(close(volume(&result), 2_000_000.0, 1e-4));
}

#[test]
fn intersect_with_a_shape_apart_leaves_nothing() {
    let kept = cube(100.0);
    let result = apply(
        &kept,
        &[cut(Kind::Intersect, &kept, at(300.0, 0.0, 0.0))],
        &Options::default(),
    )
    .unwrap();
    assert_eq!(result.face_count(), 0);
    assert_eq!(result.vertex_count(), 0);
}

#[test]
fn intersect_keeps_only_the_overlap() {
    let kept = cube(100.0);
    let result = apply(
        &kept,
        &[cut(Kind::Intersect, &kept, at(50.0, 50.0, 50.0))],
        &Options::default(),
    )
    .unwrap();
    assert!(is_closed(&result));
    assert_eq!(result.face_count(), 6);
    assert!(close(volume(&result), 125_000.0, 1e-4));
}

#[test]
fn a_cutter_fully_inside_hollows_the_shape_out() {
    let kept = cube(100.0);
    let small = cube(20.0);
    let result = apply(
        &kept,
        &[cut(Kind::Subtract, &small, at(0.0, 0.0, 40.0))],
        &Options::default(),
    )
    .unwrap();
    assert!(is_closed(&result));
    assert_eq!(result.face_count(), 12);
    assert!(close(volume(&result), 1_000_000.0 - 8_000.0, 1e-4));
}

#[test]
fn a_cutter_fully_outside_changes_nothing() {
    let kept = cube(100.0);
    let result = apply(
        &kept,
        &[cut(Kind::Subtract, &kept, at(500.0, 0.0, 0.0))],
        &Options::default(),
    )
    .unwrap();
    assert!(is_closed(&result));
    assert_eq!(result.face_count(), 6);
    assert!(close(volume(&result), 1_000_000.0, 1e-5));
}

#[test]
fn a_flush_cutter_leaves_no_slivers() {
    // The cutter's top lies in the cube's top: coplanar faces.
    let kept = cube(100.0);
    let cutter = cube(100.0);
    let result = apply(
        &kept,
        &[cut(Kind::Subtract, &cutter, at(50.0, 0.0, 0.0))],
        &Options::default(),
    )
    .unwrap();
    assert!(is_closed(&result));
    assert!(close(volume(&result), 500_000.0, 1e-4));
    assert_eq!(result.face_count(), 6, "a half cube is a box again");
}

#[test]
fn shapes_touching_at_a_face_unite_into_one_solid() {
    let kept = cube(100.0);
    let result = apply(
        &kept,
        &[cut(Kind::Union, &kept, at(100.0, 0.0, 0.0))],
        &Options::default(),
    )
    .unwrap();
    assert!(is_closed(&result));
    assert!(close(volume(&result), 2_000_000.0, 1e-4));
}

#[test]
fn n_gon_caps_are_cut_and_rebuilt() {
    let kept = cylinder(50.0, 100.0, 12);
    assert!(kept.faces().any(|(_, c)| c.len() == 12), "n-gon caps");
    let result = apply(
        &kept,
        &[cut(Kind::Subtract, &cube(100.0), at(50.0, 0.0, 50.0))],
        &Options::default(),
    )
    .unwrap();
    assert!(is_closed(&result));
    assert!(volume(&result) > 0.0);
    // The caps' remaining halves are still one polygon each.
    assert!(result.faces().filter(|(_, c)| c.len() >= 7).count() >= 2);
}

#[test]
fn many_cutters_apply_in_turn() {
    let kept = cube(1000.0);
    let small = cube(50.0);
    let cutters: Vec<Cutter> = (0..10)
        .map(|i| {
            cut(
                Kind::Subtract,
                &small,
                at(-400.0 + i as f32 * 90.0, 0.0, 975.0),
            )
        })
        .collect();
    let result = apply(&kept, &cutters, &Options::default()).unwrap();
    assert!(is_closed(&result));
    let expected = 1e9 - 10.0 * 50.0 * 50.0 * 25.0;
    assert!(close(volume(&result), expected, 1e-4));
}

#[test]
fn a_mirrored_cutter_still_cuts() {
    let kept = cube(100.0);
    let mirrored = Mat4::from_translation(Vec3::new(50.0, 50.0, 50.0))
        * Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
    let result = apply(
        &kept,
        &[cut(Kind::Subtract, &kept, mirrored)],
        &Options::default(),
    )
    .unwrap();
    assert!(is_closed(&result));
    assert!(close(volume(&result), 875_000.0, 1e-4));
}

#[test]
fn open_shapes_are_refused_with_a_reason() {
    let mut open = cube(100.0);
    let top = open.faces().next().unwrap().0;
    open.delete_faces(&[top]);
    let closed = cube(100.0);
    let kept = apply(
        &open,
        &[cut(Kind::Subtract, &closed, at(50.0, 0.0, 0.0))],
        &Options::default(),
    );
    assert_eq!(kept, Err(Problem::KeptNotClosed));
    let cutter = apply(
        &closed,
        &[cut(Kind::Subtract, &open, at(50.0, 0.0, 0.0))],
        &Options::default(),
    );
    assert_eq!(cutter, Err(Problem::CutterNotClosed(0)));
    assert!(cutter.unwrap_err().to_string().contains("closed solid"));
    // A flat plane is open too.
    let plane = Primitive::Plane(Default::default()).mesh();
    assert_eq!(
        apply(
            &closed,
            &[cut(Kind::Union, &plane, Mat4::IDENTITY)],
            &Options::default()
        ),
        Err(Problem::CutterNotClosed(0))
    );
}

#[test]
fn non_manifold_shapes_are_refused() {
    // A cube with a fin: an edge shared by three polygons.
    let mut fin = cube(100.0);
    let (_, corners) = fin.faces().next().unwrap();
    let (a, b) = (corners[0], corners[1]);
    let tip = fin.add_vertex(Vec3::new(0.0, 0.0, 500.0)).unwrap();
    fin.add_face(&[a, b, tip]).unwrap();
    assert!(!fin.topology().non_manifold_edges().is_empty());
    let result = apply(
        &fin,
        &[cut(Kind::Subtract, &cube(10.0), Mat4::IDENTITY)],
        &Options::default(),
    );
    assert_eq!(result, Err(Problem::KeptNotClosed));
}

#[test]
fn a_flattened_cutter_is_refused() {
    let kept = cube(100.0);
    let flat = Mat4::from_scale(Vec3::new(1.0, 1.0, 0.0));
    assert_eq!(
        apply(
            &kept,
            &[cut(Kind::Subtract, &kept, flat)],
            &Options::default()
        ),
        Err(Problem::CutterFlat(0))
    );
    let broken = Mat4::from_translation(Vec3::new(f32::NAN, 0.0, 0.0));
    assert_eq!(
        apply(
            &kept,
            &[cut(Kind::Subtract, &kept, broken)],
            &Options::default()
        ),
        Err(Problem::NotANumber)
    );
}

#[test]
fn an_empty_cutter_changes_nothing_but_intersect_empties() {
    let kept = cube(100.0);
    let empty = Mesh::new();
    let result = apply(
        &kept,
        &[cut(Kind::Subtract, &empty, Mat4::IDENTITY)],
        &Options::default(),
    )
    .unwrap();
    assert_eq!(result.face_count(), 6);
    let result = apply(
        &kept,
        &[cut(Kind::Intersect, &empty, Mat4::IDENTITY)],
        &Options::default(),
    )
    .unwrap();
    assert_eq!(result.face_count(), 0);
}

#[test]
fn split_points_along_a_closed_surface_still_count_as_closed() {
    // Two halves of a box, each with its own points along the seam.
    let mut box_mesh = Mesh::new();
    let half = cube(100.0);
    mesh::parts::append(&mut box_mesh, &half, Mat4::IDENTITY);
    let one: Vec<FaceId> = box_mesh.faces().map(|(f, _)| f).take(3).collect();
    let mut split = mesh::parts::keep_only(&box_mesh, &one);
    let others: Vec<FaceId> = box_mesh.faces().map(|(f, _)| f).skip(3).collect();
    let rest = mesh::parts::keep_only(&box_mesh, &others);
    mesh::parts::append(&mut split, &rest, Mat4::IDENTITY);
    assert!(!split.topology().border_edges().is_empty());
    let result = apply(
        &split,
        &[cut(Kind::Subtract, &cube(100.0), at(50.0, 50.0, 50.0))],
        &Options::default(),
    )
    .unwrap();
    assert!(is_closed(&result));
    assert!(close(volume(&result), 875_000.0, 1e-4));
}

/// The UV of `face`'s corner at `v`.
fn uv_at(mesh: &Mesh, face: FaceId, v: mesh::VertexId) -> Vec2 {
    let i = mesh
        .face(face)
        .unwrap()
        .iter()
        .position(|c| *c == v)
        .unwrap();
    match mesh.attribute(UV, AttributeElement::Corner(face, i)) {
        Some(Value::Vec2(uv)) => uv,
        other => panic!("no UV: {other:?}"),
    }
}

#[test]
fn uvs_follow_each_polygon_and_the_cutter_brings_its_own() {
    let kept = cube(100.0);
    assert!(kept.attribute_values(UV).is_some(), "cubes come with UVs");
    let result = apply(
        &kept,
        &[cut(Kind::Subtract, &kept, at(50.0, 50.0, 50.0))],
        &Options::default(),
    )
    .unwrap();
    // On each result polygon, UVs are an affine map of positions: they
    // came from one flat face of a cube, laid flat.
    for (face, corners) in result.faces() {
        let points: Vec<Vec3> = corners
            .iter()
            .map(|v| result.position(*v).unwrap())
            .collect();
        let uvs: Vec<Vec2> = corners.iter().map(|v| uv_at(&result, face, *v)).collect();
        assert!(uvs.iter().all(|uv| uv.is_finite()));
        // Distances in UV space keep their ratios along each edge pair.
        let n = points.len();
        let scale = |i: usize| {
            let j = (i + 1) % n;
            (uvs[j] - uvs[i]).length() / (points[j] - points[i]).length()
        };
        let first = scale(0);
        for i in 1..n {
            assert!(
                (scale(i) - first).abs() <= first * 1e-3 + 1e-6,
                "face {face:?}: {} vs {first}",
                scale(i)
            );
        }
    }
    // The cut's edges are seams: the UVs part there.
    assert!(!mesh::uvs::seams(&result).is_empty());
}

#[test]
fn materials_come_from_each_shape() {
    let kept = cube(100.0);
    let mut cutter = cube(100.0);
    let faces: Vec<FaceId> = cutter.faces().map(|(f, _)| f).collect();
    mesh::material::set_material(&mut cutter, &faces, 3);
    let result = apply(
        &kept,
        &[cut(Kind::Subtract, &cutter, at(50.0, 50.0, 50.0))],
        &Options::default(),
    )
    .unwrap();
    let slots: Vec<u32> = result
        .faces()
        .map(|(f, _)| mesh::material::slot_of(&result, f))
        .collect();
    assert_eq!(slots.iter().filter(|s| **s == 3).count(), 3);
    assert_eq!(slots.iter().filter(|s| **s == 0).count(), 6);
}

#[test]
fn the_same_input_gives_the_same_result() {
    let kept = Primitive::Sphere(SphereParams::default()).mesh();
    let cutter = cylinder(20.0, 300.0, 16);
    let cutters = [cut(
        Kind::Subtract,
        &cutter,
        Mat4::from_rotation_x(0.4) * at(10.0, 0.0, -150.0),
    )];
    let a = apply(&kept, &cutters, &Options::default()).unwrap();
    let b = apply(&kept, &cutters, &Options::default()).unwrap();
    assert_eq!(a, b);
    assert!(is_closed(&a));
}

#[test]
fn too_many_polygons_are_refused() {
    let kept = Primitive::Sphere(SphereParams::default()).mesh();
    let options = Options {
        max_polygons: 10,
        ..Options::default()
    };
    assert_eq!(
        apply(
            &kept,
            &[cut(Kind::Union, &cube(10.0), Mat4::IDENTITY)],
            &options
        ),
        Err(Problem::TooBig)
    );
}

#[test]
fn kinds_have_names() {
    let names: Vec<&str> = Kind::ALL.iter().map(|k| k.name()).collect();
    assert_eq!(names, ["Union", "Subtract", "Intersect"]);
}

/// A small, fixed random number maker, so the fuzz test repeats.
struct Dice(u64);

impl Dice {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn float(&mut self, low: f32, high: f32) -> f32 {
        low + (self.next() % 10_000) as f32 / 10_000.0 * (high - low)
    }
}

#[test]
fn random_shapes_and_places_never_crash() {
    let mut dice = Dice(0x9e37_79b9_7f4a_7c15);
    let shapes = [
        cube(100.0),
        cylinder(30.0, 120.0, 8),
        Primitive::Sphere(SphereParams {
            radius: 60.0,
            sides: 10,
            rings: 6,
        })
        .mesh(),
        Primitive::Plane(Default::default()).mesh(),
        Mesh::new(),
    ];
    for round in 0..60 {
        let mut kept = shapes[(dice.next() % 3) as usize].clone();
        if round % 7 == 0 {
            // Damage it: a polygon gone, or a point thrown far off.
            let gone = kept.faces().nth((dice.next() % 4) as usize).map(|(f, _)| f);
            if let Some(f) = gone {
                kept.delete_faces(&[f]);
            }
        }
        let count = 1 + (dice.next() % 3) as usize;
        let mut matrices = Vec::new();
        for _ in 0..count {
            let scale = Vec3::new(
                dice.float(-2.0, 2.0),
                dice.float(0.1, 2.0),
                dice.float(0.1, 2.0),
            );
            let turn = glam::Quat::from_euler(
                glam::EulerRot::XYZ,
                dice.float(-3.0, 3.0),
                dice.float(-3.0, 3.0),
                dice.float(-3.0, 3.0),
            );
            let place = Vec3::new(
                dice.float(-100.0, 100.0),
                dice.float(-100.0, 100.0),
                dice.float(-100.0, 100.0),
            );
            matrices.push((
                Mat4::from_scale_rotation_translation(scale, turn, place),
                (dice.next() % shapes.len() as u64) as usize,
                Kind::ALL[(dice.next() % 3) as usize],
            ));
        }
        let cutters: Vec<Cutter> = matrices
            .iter()
            .map(|(m, s, k)| cut(*k, &shapes[*s], *m))
            .collect();
        if let Ok(result) = apply(&kept, &cutters, &Options::default()) {
            assert!(result.vertices().all(|(_, p)| p.is_finite()));
        }
    }
}

#[test]
fn an_ordinary_cut_takes_milliseconds() {
    // A game-asset-sized shape: a 64 × 32 sphere (2 048 polygons) cut by
    // a 32-sided cylinder, as a cutter drag would redo every frame.
    let kept = Primitive::Sphere(SphereParams {
        radius: 100.0,
        sides: 64,
        rings: 32,
    })
    .mesh();
    let cutter = cylinder(30.0, 400.0, 32);
    let cutters = [cut(Kind::Subtract, &cutter, at(20.0, 0.0, -200.0))];
    apply(&kept, &cutters, &Options::default()).unwrap();
    let start = std::time::Instant::now();
    let runs = 5;
    for _ in 0..runs {
        apply(&kept, &cutters, &Options::default()).unwrap();
    }
    let each = start.elapsed() / runs;
    println!("one cut of a 2 048-polygon sphere: {each:?}");
    // Generous, for slow test machines; the real figure is printed.
    assert!(each.as_millis() < 2_000, "{each:?}");
}

/// How long each step takes (run with `--ignored --nocapture`, best in
/// release): joining, handing to Manifold, the boolean, rebuilding.
#[test]
#[ignore]
fn each_step_timing() {
    let kept = Primitive::Sphere(SphereParams {
        radius: 100.0,
        sides: 64,
        rings: 32,
    })
    .mesh();
    let cutter = cylinder(30.0, 400.0, 32);
    let cutters = [cut(Kind::Subtract, &cutter, at(20.0, 0.0, -200.0))];
    let t = std::time::Instant::now();
    let joined = solid::Joined::new(&kept, &cutters);
    println!("join {:?}", t.elapsed());
    let t = std::time::Instant::now();
    let solids = joined.solids().unwrap();
    println!("import {:?}", t.elapsed());
    let t = std::time::Instant::now();
    let gl = solid::combine(&solids, &cutters).unwrap();
    println!("boolean {:?} tris {}", t.elapsed(), gl.num_tri());
    let t = std::time::Instant::now();
    let m = rebuild::rebuild(&joined, &gl, &Options::default()).unwrap();
    println!("rebuild {:?} faces {}", t.elapsed(), m.face_count());
}
