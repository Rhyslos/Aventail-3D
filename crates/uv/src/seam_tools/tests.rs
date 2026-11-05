//! Tests for the seam slider's settings, loops and rings, and mirrored
//! seams (decision 0234).

use super::*;
use crate::seams::CORNER_ANGLE;
use mesh::{CubeParams, CylinderParams, Primitive, SphereParams, VertexId};

#[test]
fn the_slider_runs_from_fewest_seams_through_the_standard_to_least_stretch() {
    assert_eq!(balanced(0.0), (180.0, None), "no corners, no budget");
    let (corner, budget) = balanced(STANDARD_BALANCE);
    assert!((corner - CORNER_ANGLE).abs() < 1e-4);
    assert_eq!(budget, None, "the standard has no budget");
    let (corner, budget) = balanced(1.0);
    assert!((corner - TIGHTEST_CORNER).abs() < 1e-4);
    assert!((budget.unwrap() - TIGHTEST_BUDGET).abs() < 1e-4);
    // Just past the middle: the loosest budget.
    let (_, budget) = balanced(0.5001);
    assert!((budget.unwrap() - LOOSEST_BUDGET).abs() < 0.01);
}

#[test]
fn more_slider_means_sharper_corners_and_a_tighter_budget() {
    let mut last = balanced(0.0);
    for i in 1..=20 {
        let now = balanced(i as f32 / 20.0);
        assert!(now.0 <= last.0, "the corner angle never rises");
        if let (Some(a), Some(b)) = (last.1, now.1) {
            assert!(b <= a, "the budget never loosens");
        }
        last = now;
    }
}

#[test]
fn a_broken_or_far_slider_value_is_kept_in_range() {
    assert_eq!(balanced(f32::NAN), balanced(STANDARD_BALANCE));
    assert_eq!(balanced(-3.0), balanced(0.0));
    assert_eq!(balanced(9.0), balanced(1.0));
    let (corner, budget) = balanced(f32::INFINITY);
    assert!(corner.is_finite() && budget.is_none_or(|b| b.is_finite()));
}

#[test]
fn every_budget_the_slider_sets_is_one_a_piece_may_have() {
    for i in 0..=100 {
        if let (_, Some(b)) = balanced(i as f32 / 100.0) {
            assert!(mesh::uvs::BUDGET_RANGE.contains(&b), "{b}");
        }
    }
}

fn side_edge(mesh: &Mesh) -> EdgeKey {
    // A cylinder's upright edge: its loop runs up, its ring runs round.
    let topology = mesh.topology();
    topology
        .edges()
        .find(|e| {
            let (a, b) = e.ends();
            let (pa, pb) = (mesh.position(a).unwrap(), mesh.position(b).unwrap());
            (pa.x - pb.x).abs() < 1e-4 && (pa.z - pb.z).abs() < 1e-4 && (pa.y - pb.y).abs() > 1e-3
        })
        .unwrap()
}

#[test]
fn a_loop_runs_along_and_a_ring_runs_across() {
    let tube = Primitive::Cylinder(CylinderParams {
        height_segments: 4,
        ..Default::default()
    })
    .mesh();
    let edge = side_edge(&tube);
    let along = loops(&tube, &[edge]);
    assert!(along.contains(&edge));
    assert!(along.len() >= 4, "up the whole side: {}", along.len());
    let across = rings(&tube, &[edge]);
    assert!(across.contains(&edge));
    assert!(across.len() > along.len(), "round the cylinder");
    // Nothing given, nothing found; an unknown edge is left out.
    assert!(loops(&tube, &[]).is_empty());
    let unknown = EdgeKey::new(VertexId::from_raw(9000), VertexId::from_raw(9001));
    assert!(loops(&tube, &[unknown]).is_empty());
    assert!(rings(&tube, &[unknown]).is_empty());
}

#[test]
fn seams_on_one_side_are_mirrored_to_the_other() {
    let cube = Primitive::Cube(CubeParams {
        segments: [2, 2, 2],
        ..Default::default()
    })
    .mesh();
    // The edges on the right (+X) side only.
    let right: Vec<EdgeKey> = cube
        .topology()
        .edges()
        .filter(|e| {
            let (a, b) = e.ends();
            cube.position(a).unwrap().x > 1e-3 && cube.position(b).unwrap().x > 1e-3
        })
        .collect();
    assert!(!right.is_empty());
    let added = mirrored(&cube, &right, 0);
    assert_eq!(added.len(), right.len(), "each has a mirror on the left");
    for e in &added {
        let (a, b) = e.ends();
        assert!(cube.position(a).unwrap().x < -1e-3);
        assert!(cube.position(b).unwrap().x < -1e-3);
    }
    // Mirroring both sides adds nothing more.
    let both: Vec<EdgeKey> = right.iter().chain(&added).copied().collect();
    assert!(mirrored(&cube, &both, 0).is_empty());
    // An edge on the middle plane is its own mirror.
    let middle: Vec<EdgeKey> = cube
        .topology()
        .edges()
        .filter(|e| {
            let (a, b) = e.ends();
            cube.position(a).unwrap().x.abs() < 1e-4 && cube.position(b).unwrap().x.abs() < 1e-4
        })
        .collect();
    assert!(mirrored(&cube, &middle, 0).is_empty());
}

#[test]
fn a_lopsided_shape_mirrors_what_it_can() {
    let mut sphere = Primitive::Sphere(SphereParams::default()).mesh();
    // Move one point on the right so its edges have no mirror.
    let (v, p) = sphere
        .vertices()
        .find(|(_, p)| p.x > 0.3)
        .unwrap();
    sphere.set_position(v, p + glam::Vec3::new(0.2, 0.1, 0.0));
    let edges: Vec<EdgeKey> = sphere.topology().edges().collect();
    let added = mirrored(&sphere, &edges, 0);
    // Every edge is a seam already where a mirror exists: nothing to add.
    assert!(added.is_empty());
    let around: Vec<EdgeKey> = edges
        .iter()
        .copied()
        .filter(|e| e.ends().0 == v || e.ends().1 == v)
        .collect();
    assert!(mirrored(&sphere, &around, 0).is_empty(), "no mirror: skipped");
    // Other axes work too, and a far axis number counts as Z.
    assert_eq!(mirrored(&sphere, &[], 1), Vec::new());
    assert_eq!(mirror_across(7).axes, [false, false, true]);
}
