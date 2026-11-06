//! Tests for cutting points too curved to lie flat: a cone's tip, a closed
//! cube's corners, and odd shapes (n-gons, open borders, non-manifold
//! edges, points on top of each other).

use super::*;
use crate::pack::{PackOptions, Piece};
use glam::Vec3;
use mesh::{ConeParams, CubeParams, Primitive};

fn all_faces(mesh: &Mesh) -> Vec<FaceId> {
    mesh.faces().map(|(f, _)| f).collect()
}

/// Packs every piece of `mesh` into the square; whether it could.
fn pack_all(mesh: &mut Mesh) -> bool {
    let faces = all_faces(mesh);
    let islands = crate::pack::islands_of(mesh, &faces);
    let pieces: Vec<Piece> = islands
        .iter()
        .map(|i| crate::pack::piece_of(mesh, i))
        .collect();
    let Some(moves) = crate::pack::pack(&pieces, &PackOptions::default()) else {
        return false;
    };
    let placed: Vec<_> = islands.into_iter().zip(moves).collect();
    crate::pack::place(mesh, &placed);
    true
}

fn uvs_finite(mesh: &Mesh) -> bool {
    mesh::uvs::Uvs::of(mesh)
        .unwrap()
        .into_values(mesh.face_layout().1.len())
        .iter()
        .all(|v| v.is_finite())
}

/// Suggests seams, makes them the seams, unwraps the whole shape.
fn seam_and_unwrap(mesh: &mut Mesh, method: crate::UnwrapMethod) {
    let hard: Vec<EdgeKey> = mesh.hard_edges().collect();
    let seams = super::super::suggest(mesh, &hard);
    let old = mesh::uvs::seams(mesh);
    mesh::uvs::set_seams(mesh, &old, false);
    mesh::uvs::set_seams(mesh, &seams, true);
    let faces = all_faces(mesh);
    crate::unwrap_with(
        mesh,
        &faces,
        crate::UnwrapOptions {
            method,
            ..crate::UnwrapOptions::default()
        },
    );
}

/// A cone with no base: one piece with the rim as its border and the tip
/// inside it (sides fanned round one point).
fn open_cone(sides: usize, height: f32) -> Mesh {
    let mut mesh = Mesh::new();
    let tip = mesh.add_vertex(Vec3::new(0.0, 0.0, height)).unwrap();
    let rim: Vec<VertexId> = (0..sides)
        .map(|k| {
            let a = k as f32 / sides as f32 * std::f32::consts::TAU;
            mesh.add_vertex(Vec3::new(a.cos(), a.sin(), 0.0)).unwrap()
        })
        .collect();
    for k in 0..sides {
        mesh.add_face(&[rim[k], rim[(k + 1) % sides], tip]).unwrap();
    }
    mesh
}

#[test]
fn the_cone_side_lies_flat_after_suggest_seams_and_unwrap() {
    // Found by a test pass: the side is a disk with the tip inside it, so
    // nothing was cut tip to rim and the side folded round the tip.
    for method in [
        crate::UnwrapMethod::Conformal,
        crate::UnwrapMethod::AngleBased,
        crate::UnwrapMethod::SizeKeeping,
        crate::UnwrapMethod::Automatic,
    ] {
        let mut cone = Primitive::Cone(ConeParams::default()).mesh();
        seam_and_unwrap(&mut cone, method);
        assert!(uvs_finite(&cone));
        assert!(
            crate::select::overlapping(&cone).is_empty(),
            "{method:?}: {:?} overlap",
            crate::select::overlapping(&cone)
        );
    }
}

#[test]
fn unwrapping_a_closed_cube_with_no_seams_cuts_it_open_and_lays_it_flat() {
    // Found by a test pass: only a slit was cut, and the one piece folded
    // round the six corners left inside it.
    let mut cube = Primitive::Cube(CubeParams::default()).mesh();
    let seams = mesh::uvs::seams(&cube);
    mesh::uvs::set_seams(&mut cube, &seams, false);
    let faces = all_faces(&cube);
    crate::unwrap(&mut cube, &faces, true);
    let cut = mesh::uvs::seams(&cube);
    // Every corner reached: a tree through the 8 corners has 7 edges.
    assert!(cut.len() >= 7, "{} edges cut", cut.len());
    assert!(uvs_finite(&cube));
    assert!(pack_all(&mut cube));
    assert!(crate::select::overlapping(&cube).is_empty());
    assert!(crate::select::mirrored(&cube).is_empty());
}

#[test]
fn a_cone_tip_is_cut_once_to_the_rim_along_one_edge() {
    let cone = open_cone(8, 2.0);
    let faces = all_faces(&cone);
    let cuts = curvature_cuts(&cone, &faces, &BTreeSet::new(), HideSeams::default());
    assert_eq!(cuts.len(), 1, "one side edge, tip to rim");
    let (a, b) = cuts.iter().next().unwrap().ends();
    let tip_to_rim = [a, b].iter().any(|v| cone.position(*v).unwrap().z > 1.0);
    assert!(tip_to_rim);
    // The same every time.
    assert_eq!(
        curvature_cuts(&cone, &faces, &BTreeSet::new(), HideSeams::default()),
        cuts
    );
}

#[test]
fn a_nearly_flat_point_is_left_alone() {
    // A very low cone falls short of a full turn by only a little.
    let low = open_cone(12, 0.2);
    let faces = all_faces(&low);
    assert!(curvature_cuts(&low, &faces, &BTreeSet::new(), HideSeams::default()).is_empty());
    // A flat plane, and a flat disc fanned round a middle point.
    for p in [
        Primitive::Plane(mesh::PlaneParams {
            segments: [4, 4],
            ..Default::default()
        }),
        Primitive::Disc(mesh::DiscParams {
            centre_point: true,
            ..Default::default()
        }),
    ] {
        let mesh = p.mesh();
        let faces = all_faces(&mesh);
        assert!(
            curvature_cuts(&mesh, &faces, &BTreeSet::new(), HideSeams::default()).is_empty(),
            "{:?}",
            p.kind()
        );
    }
}

#[test]
fn standard_spheres_and_rings_get_no_extra_cuts() {
    // Their curve is spread over many points, each nearly flat: Suggest
    // seams gives the same as before this check.
    for p in [
        Primitive::Sphere(Default::default()),
        Primitive::Torus(Default::default()),
        Primitive::Cylinder(Default::default()),
        Primitive::Capsule(Default::default()),
    ] {
        let mesh = p.mesh();
        let hard: Vec<EdgeKey> = mesh.hard_edges().collect();
        let cuts: BTreeSet<EdgeKey> = super::super::suggest(&mesh, &hard).into_iter().collect();
        for piece in super::super::pieces(&mesh, &cuts) {
            assert!(
                curvature_cuts(&mesh, &piece, &cuts, HideSeams::default()).is_empty(),
                "{:?}",
                p.kind()
            );
        }
    }
}

#[test]
fn after_suggesting_no_point_inside_a_piece_is_too_curved() {
    let limit = (360.0 - CURVED_POINT_ANGLE).to_radians();
    for p in Primitive::all() {
        let mesh = p.mesh();
        let hard: Vec<EdgeKey> = mesh.hard_edges().collect();
        let cuts: BTreeSet<EdgeKey> = super::super::suggest(&mesh, &hard).into_iter().collect();
        for piece in super::super::pieces(&mesh, &cuts) {
            let inside: BTreeSet<FaceId> = piece.iter().copied().collect();
            let joined = |e: &EdgeKey| {
                !cuts.contains(e)
                    && matches!(mesh.topology().edge_uses(*e),
                        [a, b] if a.face != b.face && inside.contains(&a.face) && inside.contains(&b.face))
            };
            let chart = Chart::of(&mesh, &piece, &cuts);
            let (sums, on_border, _) = turns(&mesh, &chart, &joined);
            for w in 0..chart.len() {
                assert!(
                    on_border[w] || sums[w].is_nan() || sums[w] >= limit,
                    "{:?}: a point inside a piece turns only {}°",
                    p.kind(),
                    sums[w].to_degrees()
                );
            }
        }
    }
}

#[test]
fn n_gons_open_borders_and_odd_shapes_are_handled() {
    let hide = HideSeams::default();
    // Nothing.
    let empty = Mesh::new();
    assert!(curvature_cuts(&empty, &[], &BTreeSet::new(), hide).is_empty());
    // A lone hexagon: every point on its border.
    let mut hex = Mesh::new();
    let ids: Vec<VertexId> = (0..6)
        .map(|k| {
            let a = k as f32 / 6.0 * std::f32::consts::TAU;
            hex.add_vertex(Vec3::new(a.cos(), a.sin(), 0.0)).unwrap()
        })
        .collect();
    hex.add_face(&ids).unwrap();
    assert!(curvature_cuts(&hex, &all_faces(&hex), &BTreeSet::new(), hide).is_empty());
    // A flat L-shaped n-gon beside squares round a middle point: its
    // inward corner counts as the 270° it is, so nothing is cut.
    let mut l = Mesh::new();
    let p = |m: &mut Mesh, x: f32, y: f32| m.add_vertex(Vec3::new(x, y, 0.0)).unwrap();
    let (a, b, c) = (
        p(&mut l, 0.0, 0.0),
        p(&mut l, 1.0, 0.0),
        p(&mut l, 2.0, 0.0),
    );
    let (d, e, f) = (
        p(&mut l, 0.0, 1.0),
        p(&mut l, 1.0, 1.0),
        p(&mut l, 2.0, 1.0),
    );
    let (g, h, i) = (
        p(&mut l, 0.0, 2.0),
        p(&mut l, 1.0, 2.0),
        p(&mut l, 2.0, 2.0),
    );
    l.add_face(&[a, b, e, d]).unwrap();
    l.add_face(&[b, c, f, i, h, e]).unwrap();
    l.add_face(&[d, e, h, g]).unwrap();
    assert!(curvature_cuts(&l, &all_faces(&l), &BTreeSet::new(), hide).is_empty());
    // A cone whose tip is several points on top of each other (sides of
    // no length): no size to measure there, so no cut and no crash.
    let mut squashed = Mesh::new();
    let rim: Vec<VertexId> = (0..6)
        .map(|k| {
            let a = k as f32 / 6.0 * std::f32::consts::TAU;
            squashed
                .add_vertex(Vec3::new(a.cos(), a.sin(), 0.0))
                .unwrap()
        })
        .collect();
    let tip: Vec<VertexId> = (0..6)
        .map(|_| squashed.add_vertex(Vec3::Z).unwrap())
        .collect();
    for k in 0..6 {
        let n = (k + 1) % 6;
        squashed
            .add_face(&[rim[k], rim[n], tip[n], tip[k]])
            .unwrap();
    }
    let _ = curvature_cuts(&squashed, &all_faces(&squashed), &BTreeSet::new(), hide);
    // Non-manifold: a third triangle on one of the open cone's sides. The
    // tip is then on the border (that side isn't joined), so it lies flat.
    let mut fin = open_cone(6, 2.0);
    let topology_tip = fin
        .vertices()
        .map(|(v, _)| v)
        .find(|v| fin.position(*v).unwrap().z > 1.0)
        .unwrap();
    let rim0 = fin
        .vertices()
        .map(|(v, _)| v)
        .find(|v| *v != topology_tip)
        .unwrap();
    let out = fin.add_vertex(Vec3::new(3.0, 0.0, 1.0)).unwrap();
    fin.add_face(&[rim0, topology_tip, out]).unwrap();
    let faces = all_faces(&fin);
    let cuts = curvature_cuts(&fin, &faces, &BTreeSet::new(), hide);
    assert!(cuts.is_empty(), "{cuts:?}");
    // A closed piece with no border can't be cut to one here.
    let cube = Primitive::Cube(CubeParams::default()).mesh();
    assert!(curvature_cuts(&cube, &all_faces(&cube), &BTreeSet::new(), hide).is_empty());
}
