//! Tests for symmetry.

use super::*;
use crate::{CubeParams, Primitive, SphereParams};

fn cube(segments: [u32; 3]) -> Mesh {
    Primitive::Cube(CubeParams {
        segments,
        ..Default::default()
    })
    .mesh()
}

fn at(mesh: &Mesh, p: Vec3) -> VertexId {
    mesh.vertices()
        .find(|(_, q)| q.distance(p) < 1e-3)
        .map(|(v, _)| v)
        .unwrap()
}

fn group(plan: &MirrorPlan, reflect: [bool; 3]) -> Vec<&MirrorGroup> {
    plan.groups
        .iter()
        .filter(|g| g.reflect == reflect)
        .collect()
}

const X: [bool; 3] = [true, false, false];
const NONE: [bool; 3] = [false; 3];

#[test]
fn a_point_drives_its_mirror_across_x() {
    let mesh = cube([1, 1, 1]);
    let right = at(&mesh, Vec3::new(50.0, 50.0, 100.0));
    let left = at(&mesh, Vec3::new(-50.0, 50.0, 100.0));
    let plan = mirror_plan(&mesh, &[right], Mirror::through_origin(X));
    assert_eq!(group(&plan, NONE)[0].points, vec![right]);
    let mirrored = group(&plan, X);
    assert_eq!(mirrored[0].points, vec![left]);
    // A move to the right moves the mirror to the left; up stays up.
    let d = mirrored[0].offset(Vec3::new(10.0, 0.0, 5.0));
    assert_eq!(d, Vec3::new(-10.0, 0.0, 5.0));
    let m = mirrored[0].matrix(Mat4::from_translation(Vec3::new(10.0, 0.0, 5.0)));
    assert!(
        m.transform_point3(Vec3::new(-50.0, 50.0, 100.0))
            .distance(Vec3::new(-60.0, 50.0, 105.0))
            < 1e-4
    );
}

#[test]
fn points_on_the_plane_stay_on_it() {
    // Two segments across X: the middle points sit on the X plane.
    let mesh = cube([2, 1, 1]);
    let middle = at(&mesh, Vec3::new(0.0, 50.0, 100.0));
    let plan = mirror_plan(&mesh, &[middle], Mirror::through_origin(X));
    assert_eq!(plan.groups.len(), 1, "its mirror is itself");
    let g = &plan.groups[0];
    assert_eq!(g.keep_on, X);
    assert_eq!(g.offset(Vec3::new(7.0, 3.0, 0.0)), Vec3::new(0.0, 3.0, 0.0));
    let turned = g.matrix(Mat4::from_rotation_z(0.5));
    assert!(turned.transform_point3(Vec3::new(0.0, 50.0, 100.0)).x.abs() < 1e-4);
}

#[test]
fn choosing_both_sides_keeps_the_shape_symmetrical() {
    let mesh = cube([1, 1, 1]);
    let right = at(&mesh, Vec3::new(50.0, 50.0, 100.0));
    let left = at(&mesh, Vec3::new(-50.0, 50.0, 100.0));
    let plan = mirror_plan(&mesh, &[left, right], Mirror::through_origin(X));
    let points: Vec<VertexId> = plan.points().collect();
    assert_eq!(points.len(), 2, "each point once");
    // The lower id drives, the other follows mirrored.
    let (first, second) = (left.min(right), left.max(right));
    assert_eq!(group(&plan, NONE)[0].points, vec![first]);
    assert_eq!(group(&plan, X)[0].points, vec![second]);
}

#[test]
fn two_axes_give_three_mirrors() {
    let mesh = cube([1, 1, 1]);
    let corner = at(&mesh, Vec3::new(50.0, 50.0, 100.0));
    let plan = mirror_plan(
        &mesh,
        &[corner],
        Mirror::through_origin([true, true, false]),
    );
    assert_eq!(plan.points().count(), 4, "the top's four corners");
    assert_eq!(plan.groups.len(), 4);
    let xy = group(&plan, [true, true, false]);
    assert_eq!(
        xy[0].points,
        vec![at(&mesh, Vec3::new(-50.0, -50.0, 100.0))]
    );
}

#[test]
fn no_mirror_no_follower_and_no_axes_no_mirroring() {
    // A sphere's points are mirrored across X...
    let sphere = Primitive::Sphere(SphereParams::default()).mesh();
    let (v, p) = sphere.vertices().find(|(_, p)| p.x > 10.0).unwrap();
    let plan = mirror_plan(&sphere, &[v], Mirror::through_origin(X));
    assert_eq!(plan.points().count(), 2, "{p}");
    // ...but a moved point has none.
    let mut lopsided = cube([1, 1, 1]);
    let right = at(&lopsided, Vec3::new(50.0, 50.0, 100.0));
    lopsided.set_position(right, Vec3::new(57.0, 50.0, 100.0));
    let plan = mirror_plan(&lopsided, &[right], Mirror::through_origin(X));
    assert_eq!(plan.points().collect::<Vec<_>>(), vec![right]);
    // No axis: one group, as chosen, missing points left out.
    let mesh = cube([1, 1, 1]);
    let ghost = VertexId::from_raw(999);
    let plan = mirror_plan(
        &mesh,
        &[right, ghost, right],
        Mirror::through_origin([false; 3]),
    );
    assert_eq!(plan.groups.len(), 1);
    assert_eq!(plan.groups[0].points, vec![right]);
    // Nothing chosen: nothing moves.
    assert_eq!(
        mirror_plan(&mesh, &[], Mirror::through_origin(X))
            .points()
            .count(),
        0
    );
}

#[test]
fn partners_find_mirrored_points_edges_and_polygons() {
    let mesh = crate::Primitive::Cube(crate::CubeParams {
        segments: [2, 2, 2],
        ..Default::default()
    })
    .mesh();
    let across = Partners::new(
        &mesh,
        Mirror::through_origin([true, false, false]),
        [true, false, false],
    );
    for (v, p) in mesh.vertices() {
        let m = across.point(v).expect("a symmetrical cube");
        let q = mesh.position(m).unwrap();
        assert!(q.distance(Vec3::new(-p.x, p.y, p.z)) < 1e-3);
        if p.x.abs() < 1e-3 {
            assert_eq!(m, v, "on the plane: itself");
        }
    }
    for e in mesh.edges() {
        let m = across.edge(*e).unwrap();
        assert_eq!(across.edge(m), Some(*e), "mirroring twice gives it back");
    }
    for (f, _) in mesh.faces() {
        let m = across.face(f).unwrap();
        let n = mesh.face_normal(m);
        let o = mesh.face_normal(f);
        assert!(n.distance(Vec3::new(-o.x, o.y, o.z)) < 1e-3);
    }
    // Through another middle: nothing there.
    let off = Partners::new(
        &mesh,
        Mirror {
            axes: [true, false, false],
            centre: Vec3::new(7.0, 0.0, 0.0),
            weld: 0.0,
            clip: false,
        },
        [true, false, false],
    );
    assert!(mesh.vertices().any(|(v, _)| off.point(v).is_none()));
}

#[test]
fn a_mirror_reflects_points_through_its_middle() {
    let m = Mirror {
        axes: [true, true, false],
        centre: Vec3::new(10.0, 0.0, 0.0),
        weld: 0.0,
        clip: false,
    };
    assert_eq!(m.reflections().len(), 3);
    let p = m.point([true, false, false], Vec3::new(15.0, 2.0, 3.0));
    assert!(p.distance(Vec3::new(5.0, 2.0, 3.0)) < 1e-5);
    assert_eq!(
        Mirror::vector([false, true, false], Vec3::new(1.0, 2.0, 3.0)),
        Vec3::new(1.0, -2.0, 3.0)
    );
}

#[test]
fn the_mirror_can_go_through_another_middle() {
    // A cube moved 30 to the right: its middle is at x = 30.
    let mut mesh = cube([2, 1, 1]);
    let all: Vec<VertexId> = mesh.vertices().map(|(v, _)| v).collect();
    crate::edit::move_points(&mut mesh, &all, Vec3::new(30.0, 0.0, 0.0));
    let right = at(&mesh, Vec3::new(80.0, -50.0, 100.0));
    let mirror = Mirror {
        axes: X,
        centre: Vec3::new(30.0, 0.0, 0.0),
        weld: 0.0,
        clip: false,
    };
    let plan = mirror_plan(&mesh, &[right], mirror);
    assert_eq!(plan.points().count(), 2);
    let left = at(&mesh, Vec3::new(-20.0, -50.0, 100.0));
    assert!(plan.points().any(|v| v == left));
    // A turn about the middle seen through the mirror keeps the shape
    // symmetrical about it.
    let turn = Mat4::from_rotation_z(0.3);
    let group = plan.groups.iter().find(|g| g.reflect == X).unwrap();
    let p = mesh.position(right).unwrap();
    let moved = turn.transform_point3(p);
    let q = group
        .matrix(turn)
        .transform_point3(mesh.position(left).unwrap());
    assert!(q.distance(mirror.point(X, moved)) < 1e-3, "{q} {moved}");
    // Points on the plane through the middle stay on it.
    let middle = at(&mesh, Vec3::new(30.0, -50.0, 100.0));
    let plan = mirror_plan(&mesh, &[middle], mirror);
    assert_eq!(plan.groups[0].keep_on, X);
}

/// Two flat squares side by side, a gap of `gap` between them across x = 0.
fn halves(gap: f32) -> Mesh {
    let mut mesh = Mesh::new();
    let h = gap / 2.0;
    let mut quad = |x0: f32, x1: f32| {
        let c = [
            mesh.add_vertex(Vec3::new(x0, 0.0, 0.0)).unwrap(),
            mesh.add_vertex(Vec3::new(x1, 0.0, 0.0)).unwrap(),
            mesh.add_vertex(Vec3::new(x1, 10.0, 0.0)).unwrap(),
            mesh.add_vertex(Vec3::new(x0, 10.0, 0.0)).unwrap(),
        ];
        mesh.add_face(&c).unwrap();
    };
    quad(-10.0, -h);
    quad(h, 10.0);
    mesh
}

fn welding(distance: f32) -> Mirror {
    Mirror {
        weld: distance,
        ..Mirror::through_origin(X)
    }
}

#[test]
fn welding_the_middle_closes_the_seam() {
    let mut mesh = halves(0.08);
    assert_eq!(crate::MeshReport::new(&mesh).pieces, 2);
    assert!(weld_middle(&mut mesh, &welding(0.1)) > 0);
    assert_eq!(mesh.vertex_count(), 6, "the seam's points became one");
    assert_eq!(mesh.face_count(), 2);
    assert_eq!(crate::MeshReport::new(&mesh).pieces, 1);
    assert!(mesh.vertices().filter(|(_, p)| p.x.abs() < 1e-6).count() == 2);
    // Too far apart for the distance, or welding off: nothing.
    let mut far = halves(0.5);
    assert_eq!(weld_middle(&mut far, &welding(0.1)), 0);
    assert_eq!(far, halves(0.5));
    let mut off = halves(0.08);
    assert_eq!(weld_middle(&mut off, &welding(0.0)), 0);
    // Already joined: nothing more to do.
    let mut again = mesh.clone();
    assert_eq!(weld_middle(&mut again, &welding(0.1)), 0);
    assert_eq!(again, mesh);
}

#[test]
fn clipping_keeps_points_on_their_side() {
    let before = cube([2, 1, 1]);
    let mut mesh = before.clone();
    // A right-hand point pushed past the middle comes back onto it.
    let right = at(&mesh, Vec3::new(50.0, -50.0, 100.0));
    mesh.set_position(right, Vec3::new(-20.0, -50.0, 100.0));
    let clip = Mirror {
        clip: true,
        ..Mirror::through_origin(X)
    };
    assert_eq!(keep_sides(&before, &mut mesh, &clip), 1);
    assert_eq!(mesh.position(right).unwrap().x, 0.0);
    // Off: left where it is.
    let mut free = before.clone();
    free.set_position(right, Vec3::new(-20.0, -50.0, 100.0));
    assert_eq!(
        keep_sides(&before, &mut free, &Mirror::through_origin(X)),
        0
    );
    // How far points may move: right-hand ones no further left than the
    // middle; a point on the middle doesn't limit.
    let middle = at(&before, Vec3::new(0.0, -50.0, 100.0));
    let limits = move_limits(&before, &[right, middle], &clip);
    assert_eq!(limits[0], (-50.0, f32::INFINITY));
    assert_eq!(limits[1], (f32::NEG_INFINITY, f32::INFINITY));
    let left = at(&before, Vec3::new(-50.0, -50.0, 100.0));
    assert_eq!(
        move_limits(&before, &[left], &clip)[0],
        (f32::NEG_INFINITY, 50.0)
    );
}

#[test]
fn with_welding_points_near_the_middle_stay_on_it() {
    let mut mesh = cube([2, 1, 1]);
    let middle = at(&mesh, Vec3::new(0.0, -50.0, 100.0));
    mesh.set_position(middle, Vec3::new(0.05, -50.0, 100.0));
    // Not on the plane within the tolerance; within the weld distance.
    assert_eq!(
        mirror_plan(&mesh, &[middle], Mirror::through_origin(X)).groups[0].keep_on,
        NONE
    );
    let plan = mirror_plan(&mesh, &[middle], welding(0.1));
    assert_eq!(plan.groups[0].keep_on, X);
    // Its move puts it on the plane.
    let moved = plan.groups[0]
        .matrix(Mat4::from_translation(Vec3::new(5.0, 3.0, 0.0)))
        .transform_point3(Vec3::new(0.05, -50.0, 100.0));
    assert!(moved.x.abs() < 1e-6 && (moved.y + 47.0).abs() < 1e-4);
}
