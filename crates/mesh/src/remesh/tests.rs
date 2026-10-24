use super::*;
use crate::{MeshReport, Primitive, PrimitiveKind, VertexId};
use glam::Mat4;

fn cube() -> Mesh {
    Primitive::Cube(Default::default()).mesh()
}

fn options(voxel_size: f32, smooth: u32) -> RemeshOptions {
    RemeshOptions { voxel_size, smooth }
}

/// The surface is a clean closed solid: no open borders, no edge shared by
/// more than two polygons, no polygon facing the wrong way.
fn assert_clean_solid(mesh: &Mesh) {
    let report = MeshReport::new(mesh);
    assert!(report.faces > 0);
    assert_eq!(report.border_edges, 0, "{report:?}");
    assert_eq!(report.non_manifold_edges, 0, "{report:?}");
    assert_eq!(report.flipped_edges, 0, "{report:?}");
    assert_eq!(report.loose_points, 0, "{report:?}");
    assert!(is_closed(mesh));
}

/// The volume inside a closed mesh (positive when its polygons face out).
fn volume(mesh: &Mesh) -> f32 {
    let mut total = 0.0;
    for (_, corners) in mesh.faces() {
        let p: Vec<Vec3> = corners.iter().map(|v| mesh.position(*v).unwrap()).collect();
        for i in 1..p.len() - 1 {
            total += p[0].dot(p[i].cross(p[i + 1])) / 6.0;
        }
    }
    total
}

#[test]
fn a_cube_becomes_an_even_closed_surface_of_quads() {
    let out = remesh(&cube(), &options(10.0, 0)).unwrap();
    assert_clean_solid(&out);
    assert!(out.faces().all(|(_, c)| c.len() == 4), "quads only");
    // About the same size and place: within a voxel.
    let (b, was) = (out.bounds().unwrap(), cube().bounds().unwrap());
    assert!(b.min.distance(was.min) < 10.0, "{b:?}");
    assert!(b.max.distance(was.max) < 10.0, "{b:?}");
    // Facing out: a positive volume close to the cube's (corners are
    // rounded off by about a voxel).
    let v = volume(&out);
    assert!(v > 0.85e6 && v < 1.05e6, "{v}");
    // Even: a 100 cm side at 10 cm makes about ten quads across.
    assert!(
        out.face_count() > 6 * 64 && out.face_count() < 6 * 144,
        "{}",
        out.face_count()
    );
}

#[test]
fn a_sphere_stays_round() {
    let sphere = PrimitiveKind::Sphere.standard().mesh();
    let (radius, centre) = {
        let b = sphere.bounds().unwrap();
        (b.size().x / 2.0, b.centre())
    };
    let out = remesh(&sphere, &options(5.0, 2)).unwrap();
    assert_clean_solid(&out);
    for (_, p) in out.vertices() {
        let d = p.distance(centre);
        assert!((d - radius).abs() < 3.0, "{d} vs {radius}");
    }
}

#[test]
fn the_same_shape_and_settings_give_the_same_result() {
    let shape = PrimitiveKind::Torus.standard().mesh();
    let a = remesh(&shape, &options(6.0, 2)).unwrap();
    let b = remesh(&shape, &options(6.0, 2)).unwrap();
    assert_eq!(a, b);
    assert_clean_solid(&a);
}

#[test]
fn smoothing_moves_points_but_keeps_the_polygons() {
    let rough = remesh(&cube(), &options(10.0, 0)).unwrap();
    let smooth = remesh(&cube(), &options(10.0, 3)).unwrap();
    assert_eq!(rough.face_count(), smooth.face_count());
    let faces = |m: &Mesh| m.faces().map(|(f, c)| (f, c.to_vec())).collect::<Vec<_>>();
    assert_eq!(faces(&rough), faces(&smooth));
    assert_ne!(rough, smooth);
    // Without shrinking much.
    assert!(volume(&smooth) > 0.9 * volume(&rough));
}

#[test]
fn n_gons_are_remeshed() {
    // The cylinder's caps are n-gons.
    let out = remesh(&PrimitiveKind::Cylinder.standard().mesh(), &options(8.0, 1)).unwrap();
    assert_clean_solid(&out);
}

#[test]
fn shapes_touching_along_an_edge_give_a_clean_solid() {
    // Two boxes sharing one edge: the edge is used by four polygons.
    // Their touch is filled in so every new edge has two polygons.
    let two = crate::parts::combine(&[
        (&cube(), Mat4::IDENTITY),
        (
            &cube(),
            Mat4::from_translation(Vec3::new(100.0, 100.0, 0.0)),
        ),
    ]);
    assert!(
        MeshReport::new(&two).non_manifold_edges == 0,
        "separate pieces"
    );
    let out = remesh(&two, &options(10.0, 0)).unwrap();
    assert_clean_solid(&out);
    // Shapes meeting only at a point too.
    let corner = crate::parts::combine(&[
        (&cube(), Mat4::IDENTITY),
        (&cube(), Mat4::from_translation(Vec3::splat(100.0))),
    ]);
    assert_clean_solid(&remesh(&corner, &options(10.0, 0)).unwrap());
}

#[test]
fn zero_length_edges_are_fine() {
    // A box whose top has a fifth corner on top of another one.
    let mut mesh = Mesh::new();
    let p: Vec<VertexId> = [
        (0., 0., 0.),
        (50., 0., 0.),
        (50., 50., 0.),
        (0., 50., 0.),
        (0., 0., 50.),
        (50., 0., 50.),
        (50., 50., 50.),
        (0., 50., 50.),
        (50., 50., 50.),
    ]
    .iter()
    .map(|(x, y, z)| mesh.add_vertex(Vec3::new(*x, *y, *z)).unwrap())
    .collect();
    for face in [
        vec![p[0], p[3], p[2], p[1]],
        vec![p[4], p[5], p[6], p[8], p[7]],
        vec![p[0], p[1], p[5], p[4]],
        vec![p[1], p[2], p[6], p[5]],
        vec![p[2], p[3], p[7], p[8]],
        vec![p[3], p[0], p[4], p[7]],
    ] {
        mesh.add_face(&face).unwrap();
    }
    assert!(is_closed(&mesh));
    assert_clean_solid(&remesh(&mesh, &options(5.0, 0)).unwrap());
}

#[test]
fn open_shapes_are_refused() {
    let plane = PrimitiveKind::Plane.standard().mesh();
    assert!(!is_closed(&plane));
    assert_eq!(
        remesh(&plane, &options(5.0, 0)),
        Err(RemeshProblem::NotClosed)
    );
    // A cube with a side taken away.
    let mut open = cube();
    let first = open.faces().next().unwrap().0;
    open.delete_faces(&[first]);
    assert_eq!(
        remesh(&open, &options(5.0, 0)),
        Err(RemeshProblem::NotClosed)
    );
    // An edge shared by three polygons.
    let mut fin = cube();
    let (a, b) = fin.edges()[0].ends();
    let c = fin.add_vertex(Vec3::new(0.0, 0.0, 300.0)).unwrap();
    fin.add_face(&[a, b, c]).unwrap();
    assert_eq!(
        remesh(&fin, &options(5.0, 0)),
        Err(RemeshProblem::NotClosed)
    );
    let reason = RemeshProblem::NotClosed.to_string();
    assert!(reason.contains("isn't closed"), "{reason}");
}

#[test]
fn an_empty_mesh_is_refused() {
    assert_eq!(
        remesh(&Mesh::new(), &RemeshOptions::default()),
        Err(RemeshProblem::Empty)
    );
    assert_eq!(voxel_count(&Mesh::new(), 2.0), None);
}

#[test]
fn a_shape_thinner_than_a_voxel_leaves_nothing() {
    let mut thin = cube();
    crate::parts::place(&mut thin, Mat4::from_scale(Vec3::new(1.0, 1.0, 0.001)));
    assert_eq!(
        remesh(&thin, &options(10.0, 0)),
        Err(RemeshProblem::NothingLeft)
    );
}

#[test]
fn too_many_voxels_are_refused_with_a_size_that_fits() {
    let problem = remesh(&cube(), &options(0.5, 0)).unwrap_err();
    let RemeshProblem::TooFine { voxels, smallest } = problem else {
        panic!("{problem:?}");
    };
    assert!(voxels > RemeshOptions::MAX_VOXELS);
    assert!(smallest > 0.5 && smallest < 1.0, "{smallest}");
    assert!(voxel_count(&cube(), smallest).unwrap() <= RemeshOptions::MAX_VOXELS);
    assert!(voxel_count(&cube(), smallest - 0.01).unwrap() > RemeshOptions::MAX_VOXELS);
    let reason = problem.to_string();
    assert!(reason.contains("2 million"), "{reason}");
    assert!(reason.contains(&format!("{smallest:.2} cm")), "{reason}");
}

#[test]
fn odd_settings_are_brought_within_limits_never_a_panic() {
    for size in [0.0, -5.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1e-30] {
        let clamped = options(size, 0).clamped();
        assert!(clamped.voxel_size.is_finite() && clamped.voxel_size >= RemeshOptions::MIN_VOXEL);
        // Too fine for a 1 m cube (or the standard 2 cm, which fits).
        match remesh(&cube(), &options(size, u32::MAX)) {
            Ok(out) => assert_clean_solid(&out),
            Err(RemeshProblem::TooFine { .. }) => {}
            Err(other) => panic!("{size}: {other:?}"),
        }
    }
    // Huge voxels: the cube is thinner than one voxel... or a single
    // blob; never a panic.
    let big = remesh(&cube(), &options(1e9, 0));
    assert!(
        matches!(big, Ok(_) | Err(RemeshProblem::NothingLeft)),
        "{big:?}"
    );
    assert_eq!(
        options(1.0, 1_000).clamped().smooth,
        RemeshOptions::MAX_SMOOTH
    );
}

#[test]
fn points_not_at_real_positions_are_refused() {
    let mut mesh = cube();
    let v = mesh.vertices().next().unwrap().0;
    mesh.set_position(v, Vec3::NAN);
    assert_eq!(
        remesh(&mesh, &options(10.0, 0)),
        Err(RemeshProblem::NotANumber)
    );
}

#[test]
fn materials_come_from_the_nearest_original_polygon() {
    let mut box_ = cube();
    // The top (highest middle) gets material 1.
    let top = box_
        .faces()
        .map(|(f, _)| f)
        .max_by(|a, b| {
            let z = |f| box_.face_normal(f).z;
            z(*a).total_cmp(&z(*b))
        })
        .unwrap();
    crate::material::set_material(&mut box_, &[top], 1);
    let out = remesh(&box_, &options(10.0, 0)).unwrap();
    let mut on_top = 0;
    for (face, corners) in out.faces() {
        let middle = corners
            .iter()
            .map(|v| out.position(*v).unwrap())
            .sum::<Vec3>()
            / corners.len() as f32;
        let slot = crate::material::slot_of(&out, face);
        if middle.z > 45.0 && middle.x.abs() < 35.0 && middle.y.abs() < 35.0 {
            assert_eq!(slot, 1, "top at {middle}");
            on_top += 1;
        }
        if middle.z < 0.0 {
            assert_eq!(slot, 0, "below the middle at {middle}");
        }
    }
    assert!(on_top > 10);
    // Without materials, no material layer is added.
    let plain = remesh(&cube(), &options(10.0, 0)).unwrap();
    assert!(
        !plain
            .attributes()
            .any(|(name, _, _)| name == crate::material::MATERIAL)
    );
}

#[test]
fn it_gets_box_projected_uvs_with_seams() {
    let out = remesh(&cube(), &options(10.0, 0)).unwrap();
    assert!(crate::uvs::has_uvs(&out));
    let seams = crate::uvs::seams(&out);
    assert!(!seams.is_empty());
    // A cube is cut into its six sides (and a few bits round the
    // corners, where the rounded corner faces another way).
    let pieces = crate::uvs::islands(&out).len();
    assert!((6..=14).contains(&pieces), "{pieces}");
    let uvs = crate::uvs::Uvs::of(&out)
        .unwrap()
        .into_values(out.face_count() * 4);
    assert!(
        uvs.iter()
            .all(|uv| (-0.01..=1.01).contains(&uv.x) && (-0.01..=1.01).contains(&uv.y))
    );
}

#[test]
#[ignore = "timing: run with --release -- --ignored --nocapture"]
fn timings() {
    let sphere = PrimitiveKind::Sphere.standard().mesh();
    for voxel in [4.0, 2.0, 1.0] {
        let start = std::time::Instant::now();
        let out = remesh(&sphere, &options(voxel, 2)).unwrap();
        println!(
            "sphere at {voxel} cm: {} voxels, {} quads, {:?}",
            voxel_count(&sphere, voxel).unwrap(),
            out.face_count(),
            start.elapsed()
        );
    }
}

#[test]
fn overlapping_shapes_become_one_solid() {
    // Two boxes overlapping by half: their overlap is inside too (not
    // hollowed out, as counting crossings alone would make it).
    let two = crate::parts::combine(&[
        (&cube(), Mat4::IDENTITY),
        (&cube(), Mat4::from_translation(Vec3::new(50.0, 0.0, 0.0))),
    ]);
    let out = remesh(&two, &options(10.0, 0)).unwrap();
    assert_clean_solid(&out);
    let v = volume(&out);
    // One block 150 × 100 × 100 cm, corners rounded a little.
    assert!(v > 1.3e6 && v < 1.55e6, "{v}");
    // One piece, with no surface left inside.
    assert_eq!(MeshReport::new(&out).pieces, 1);
}
