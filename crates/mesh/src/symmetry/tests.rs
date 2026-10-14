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
    let plan = mirror_plan(&mesh, &[right], X);
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
    let plan = mirror_plan(&mesh, &[middle], X);
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
    let plan = mirror_plan(&mesh, &[left, right], X);
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
    let plan = mirror_plan(&mesh, &[corner], [true, true, false]);
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
    let plan = mirror_plan(&sphere, &[v], X);
    assert_eq!(plan.points().count(), 2, "{p}");
    // ...but a moved point has none.
    let mut lopsided = cube([1, 1, 1]);
    let right = at(&lopsided, Vec3::new(50.0, 50.0, 100.0));
    lopsided.set_position(right, Vec3::new(57.0, 50.0, 100.0));
    let plan = mirror_plan(&lopsided, &[right], X);
    assert_eq!(plan.points().collect::<Vec<_>>(), vec![right]);
    // No axis: one group, as chosen, missing points left out.
    let mesh = cube([1, 1, 1]);
    let ghost = VertexId::from_raw(999);
    let plan = mirror_plan(&mesh, &[right, ghost, right], [false; 3]);
    assert_eq!(plan.groups.len(), 1);
    assert_eq!(plan.groups[0].points, vec![right]);
    // Nothing chosen: nothing moves.
    assert_eq!(mirror_plan(&mesh, &[], X).points().count(), 0);
}
