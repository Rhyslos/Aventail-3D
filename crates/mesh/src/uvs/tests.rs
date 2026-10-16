//! Tests for reading UVs, islands and seams (decision 0095).

use super::*;
use crate::{CubeParams, CylinderParams, Primitive, SphereParams, TubeParams};
use std::collections::BTreeSet;

fn every_uv(mesh: &Mesh) -> Vec<Vec2> {
    let uvs = Uvs::of(mesh).unwrap();
    (0..mesh.face_layout().1.len()).map(|i| uvs.at(i)).collect()
}

fn all() -> Vec<Primitive> {
    vec![
        Primitive::Cube(CubeParams::default()),
        Primitive::Cube(CubeParams {
            segments: [3, 2, 4],
            ..Default::default()
        }),
        Primitive::Plane(Default::default()),
        Primitive::Disc(Default::default()),
        Primitive::Cylinder(CylinderParams::default()),
        Primitive::Cone(Default::default()),
        Primitive::Tube(TubeParams::default()),
        Primitive::Sphere(SphereParams::default()),
    ]
}

#[test]
fn every_primitive_comes_with_uvs_inside_the_square() {
    for p in all() {
        let mesh = p.mesh();
        assert!(has_uvs(&mesh), "{:?}", p.kind());
        for uv in every_uv(&mesh) {
            assert!(
                (0.0..=1.0).contains(&uv.x) && (0.0..=1.0).contains(&uv.y),
                "{:?}: {uv}",
                p.kind()
            );
        }
    }
}

#[test]
fn islands_match_how_each_shape_is_cut() {
    let count = |p: Primitive| islands(&p.mesh()).len();
    assert_eq!(count(Primitive::Cube(CubeParams::default())), 6);
    assert_eq!(
        count(Primitive::Cube(CubeParams {
            segments: [3, 2, 4],
            ..Default::default()
        })),
        6
    );
    assert_eq!(count(Primitive::Plane(Default::default())), 1);
    assert_eq!(count(Primitive::Disc(Default::default())), 1);
    // Side, top and bottom.
    assert_eq!(count(Primitive::Cylinder(CylinderParams::default())), 3);
    // Outside, inside, top ring and bottom ring.
    assert_eq!(count(Primitive::Tube(TubeParams::default())), 4);
    assert_eq!(count(Primitive::Sphere(SphereParams::default())), 1);
}

#[test]
fn pieces_dont_overlap() {
    for p in all() {
        let mesh = p.mesh();
        let uvs = Uvs::of(&mesh).unwrap();
        let boxes: Vec<(Vec2, Vec2)> = islands(&mesh)
            .iter()
            .map(|island| {
                let mut min = Vec2::splat(f32::INFINITY);
                let mut max = Vec2::splat(f32::NEG_INFINITY);
                for f in island {
                    for k in 0..mesh.face(*f).unwrap().len() {
                        let uv = uvs.corner(&mesh, *f, k).unwrap();
                        min = min.min(uv);
                        max = max.max(uv);
                    }
                }
                (min, max)
            })
            .collect();
        for (i, a) in boxes.iter().enumerate() {
            for b in &boxes[i + 1..] {
                let apart = a.1.x <= b.0.x + 1e-4
                    || b.1.x <= a.0.x + 1e-4
                    || a.1.y <= b.0.y + 1e-4
                    || b.1.y <= a.0.y + 1e-4;
                assert!(apart, "{:?}: {a:?} {b:?}", p.kind());
            }
        }
    }
}

#[test]
fn seams_are_where_the_uvs_split() {
    for p in all() {
        let mesh = p.mesh();
        let marked: BTreeSet<EdgeKey> = seams(&mesh).into_iter().collect();
        let splits: BTreeSet<EdgeKey> = uv_splits(&mesh).into_iter().collect();
        assert_eq!(marked, splits, "{:?}", p.kind());
    }
    // A cube is cut along all twelve of its edges.
    let cube = Primitive::Cube(CubeParams::default()).mesh();
    assert_eq!(seams(&cube).len(), 12);
    // A plane isn't cut at all.
    assert!(seams(&Primitive::Plane(Default::default()).mesh()).is_empty());
}

#[test]
fn a_cylinders_side_is_one_strip_cut_once_down_the_back() {
    let mesh = Primitive::Cylinder(CylinderParams::default()).mesh();
    let cut: Vec<EdgeKey> = seams(&mesh)
        .into_iter()
        .filter(|e| {
            let (a, b) = e.ends();
            let (pa, pb) = (mesh.position(a).unwrap(), mesh.position(b).unwrap());
            (pa.z - pb.z).abs() > 1e-3
        })
        .collect();
    // One upright edge per height segment, at the back (−X).
    assert!(!cut.is_empty());
    for e in cut {
        let p = mesh.position(e.ends().0).unwrap();
        assert!(p.x < 0.0 && p.y.abs() < 1e-3, "{p}");
    }
}

#[test]
fn pieces_keep_their_real_sizes_against_each_other() {
    // A box twice as wide as it is deep: its front is twice as wide in the
    // texture as its side.
    let mesh = Primitive::Cube(CubeParams {
        size: glam::Vec3::new(200.0, 100.0, 100.0),
        ..Default::default()
    })
    .mesh();
    let uvs = Uvs::of(&mesh).unwrap();
    let width = |normal: glam::Vec3| {
        let (f, _) = mesh
            .faces()
            .find(|(f, _)| mesh.face_normal(*f).dot(normal) > 0.99)
            .unwrap();
        let us: Vec<f32> = (0..4).map(|k| uvs.corner(&mesh, f, k).unwrap().x).collect();
        us.iter().cloned().fold(f32::MIN, f32::max) - us.iter().cloned().fold(f32::MAX, f32::min)
    };
    let front = width(glam::Vec3::NEG_Y);
    let side = width(glam::Vec3::X);
    assert!((front / side - 2.0).abs() < 1e-3, "{front} {side}");
}

#[test]
fn a_mesh_without_uvs_has_no_islands_or_splits() {
    let mut mesh = Mesh::new();
    let a = mesh.add_vertex(glam::Vec3::ZERO).unwrap();
    let b = mesh.add_vertex(glam::Vec3::X).unwrap();
    let c = mesh.add_vertex(glam::Vec3::Y).unwrap();
    mesh.add_face(&[a, b, c]).unwrap();
    assert!(!has_uvs(&mesh));
    assert!(islands(&mesh).is_empty());
    assert!(uv_splits(&mesh).is_empty());
    assert!(seams(&mesh).is_empty());
}

#[test]
fn uvs_ride_along_with_an_extrude() {
    // The builder carries corners, so an extruded cube keeps UVs on every
    // polygon, the new walls stretched from the old corners.
    let mut mesh = Primitive::Cube(CubeParams::default()).mesh();
    let top = mesh
        .faces()
        .map(|(f, _)| f)
        .find(|f| mesh.face_normal(*f).z > 0.99)
        .unwrap();
    crate::edit::extrude_polygons(&mut mesh, &[top], 30.0, crate::edit::Grouping::Together);
    assert!(has_uvs(&mesh));
    for uv in every_uv(&mesh) {
        assert!(uv.is_finite());
    }
}

// ------------------------------------------------------------ moving ---

/// A plane cut in two each way: one island of four polygons.
fn plane_2x2() -> Mesh {
    Primitive::Plane(crate::PlaneParams {
        segments: [2, 2],
        ..Default::default()
    })
    .mesh()
}

#[test]
fn moving_a_polygons_uvs_drags_its_neighbours_corners_along() {
    let mut mesh = plane_2x2();
    let before = every_uv(&mesh);
    let (face, _) = mesh.faces().next().unwrap();
    let shift = glam::Affine2::from_translation(Vec2::new(0.1, 0.0));
    let outcome = transform_uvs(&mut mesh, &[face], &[], shift);
    assert_eq!(outcome.matched, 1);
    // Still one island: the corners its neighbours share moved too.
    assert_eq!(islands(&mesh).len(), 1);
    let after = every_uv(&mesh);
    let moved = before
        .iter()
        .zip(&after)
        .filter(|(a, b)| a.distance(**b) > 1e-6)
        .count();
    // Its 4 corners, and the 5 corners of neighbours on its 3 inner points.
    assert_eq!(moved, 4 + 5);
}

#[test]
fn a_cubes_corner_moves_in_one_island_only_when_its_uv_spot_is_chosen() {
    // A point of a cube sits in three islands; moving the point moves all
    // three of its spots, each with its own neighbours.
    let mut mesh = Primitive::Cube(CubeParams::default()).mesh();
    let v = crate::VertexId::from_raw(0);
    let before = islands(&mesh).len();
    transform_uvs(
        &mut mesh,
        &[],
        &[v],
        glam::Affine2::from_translation(Vec2::new(0.01, 0.01)),
    );
    assert_eq!(islands(&mesh).len(), before);
    let uvs = Uvs::of(&mesh).unwrap();
    let moved = corners_moved(&mesh, &uvs, &[], &[v]);
    assert_eq!(moved.len(), 3);
}

#[test]
fn turning_uvs_about_their_middle_keeps_the_middle() {
    let mut mesh = Primitive::Plane(Default::default()).mesh();
    let (face, _) = mesh.faces().next().unwrap();
    let middle = uv_centre(&mesh, &[face], &[]).unwrap();
    let turn = glam::Affine2::from_translation(middle)
        * glam::Affine2::from_angle(std::f32::consts::FRAC_PI_2)
        * glam::Affine2::from_translation(-middle);
    transform_uvs(&mut mesh, &[face], &[], turn);
    let after = uv_centre(&mesh, &[face], &[]).unwrap();
    assert!(after.distance(middle) < 1e-5);
}

#[test]
fn bad_transforms_and_meshes_without_uvs_change_nothing() {
    let mut mesh = plane_2x2();
    let before = every_uv(&mesh);
    let (face, _) = mesh.faces().next().unwrap();
    let nan = glam::Affine2::from_translation(Vec2::new(f32::NAN, 0.0));
    let outcome = transform_uvs(&mut mesh, &[face], &[], nan);
    assert_eq!(outcome.matched, 0);
    assert_eq!(every_uv(&mesh), before);
    // A missing polygon is counted, the rest still moves.
    let gone = FaceId::from_raw(999);
    let outcome = transform_uvs(
        &mut mesh,
        &[face, gone],
        &[],
        glam::Affine2::from_translation(Vec2::X * 0.1),
    );
    assert_eq!((outcome.matched, outcome.missing), (1, 1));
    let mut bare = Mesh::new();
    let a = bare.add_vertex(glam::Vec3::ZERO).unwrap();
    let b = bare.add_vertex(glam::Vec3::X).unwrap();
    let c = bare.add_vertex(glam::Vec3::Y).unwrap();
    let f = bare.add_face(&[a, b, c]).unwrap();
    let outcome = transform_uvs(&mut bare, &[f], &[], glam::Affine2::IDENTITY);
    assert_eq!(outcome.matched, 0);
    assert!(!has_uvs(&bare));
}

#[test]
fn seams_are_marked_and_cleared_on_inner_edges_only() {
    let mut mesh = plane_2x2();
    mesh.remove_attribute(SEAM);
    let topology = mesh.topology().clone();
    let inner: Vec<EdgeKey> = topology
        .edges()
        .filter(|e| topology.edge_uses(*e).len() == 2)
        .collect();
    let border = topology
        .edges()
        .find(|e| topology.edge_uses(*e).len() == 1)
        .unwrap();
    let gone = EdgeKey::new(VertexId::from_raw(9000), VertexId::from_raw(9001));
    // No seam layer yet: marking adds one.
    let done = set_seams(&mut mesh, &[inner[0], inner[1], border, gone], true);
    assert_eq!((done.matched, done.skipped, done.missing), (2, 1, 1));
    assert!(is_seam(&mesh, inner[0]) && is_seam(&mesh, inner[1]));
    assert!(!is_seam(&mesh, border), "an open edge is cut already");
    // Again: nothing to do.
    let again = set_seams(&mut mesh, &[inner[0]], true);
    assert_eq!((again.matched, again.skipped), (0, 1));
    // Cleared.
    let cleared = set_seams(&mut mesh, &[inner[0]], false);
    assert_eq!(cleared.matched, 1);
    assert!(!is_seam(&mesh, inner[0]) && is_seam(&mesh, inner[1]));
    assert_eq!(seams(&mesh), vec![inner[1]]);
    // The UVs are left alone: only the marks change.
    assert_eq!(uv_splits(&mesh), uv_splits(&plane_2x2()));
}

#[test]
fn clearing_seams_on_a_mesh_without_any_changes_nothing() {
    let mut mesh = plane_2x2();
    mesh.remove_attribute(SEAM);
    let edges: Vec<EdgeKey> = mesh.topology().edges().collect();
    let done = set_seams(&mut mesh, &edges, false);
    assert_eq!(done.matched, 0);
    assert!(mesh.attribute_values(SEAM).is_none(), "no layer made");
    assert!(set_seams(&mut mesh, &[], true).matched == 0);
}

#[test]
fn every_uv_can_be_set_at_once_but_only_whole_and_real() {
    let mut mesh = plane_2x2();
    let count = mesh.face_layout().1.len();
    let values: Vec<Vec2> = (0..count).map(|i| Vec2::splat(i as f32 / 100.0)).collect();
    assert!(set_uvs(&mut mesh, values.clone()));
    assert_eq!(every_uv(&mesh), values);
    assert!(!set_uvs(&mut mesh, values[1..].to_vec()), "one short");
    let mut bad = values.clone();
    bad[3] = Vec2::NAN;
    assert!(!set_uvs(&mut mesh, bad));
    assert_eq!(every_uv(&mesh), values, "unchanged");
    // A mesh without UVs gets them.
    mesh.remove_attribute(UV);
    assert!(set_uvs(&mut mesh, values.clone()));
    assert!(has_uvs(&mesh));
}
