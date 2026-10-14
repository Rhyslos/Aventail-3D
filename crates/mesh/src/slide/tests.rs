//! Tests for sliding points.

use super::*;
use crate::{CubeParams, Primitive};

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

#[test]
fn a_middle_ring_slides_up_its_edges_and_stops_at_the_end() {
    // Two rows up the sides: the middle ring at height 50.
    let mut mesh = cube([1, 1, 2]);
    let ring: Vec<VertexId> = mesh
        .vertices()
        .filter(|(_, p)| (p.z - 50.0).abs() < 1e-3)
        .map(|(v, _)| v)
        .collect();
    assert_eq!(ring.len(), 4);
    let slides = slides_for(&mesh, &ring, Vec3::new(0.0, 0.0, 20.0));
    assert_eq!(slides.len(), 4);
    assert!(
        slides.iter().all(|s| (s.amount - 0.4).abs() < 1e-4),
        "{slides:?}"
    );
    slide_points(&mut mesh, &slides);
    for v in &ring {
        let p = mesh.position(*v).unwrap();
        assert!((p.z - 70.0).abs() < 1e-3);
        assert!((p.x.abs() - 50.0).abs() < 1e-3, "stays on the side: {p}");
    }
    // Further than the edge reaches: it stops at the top.
    let slides = slides_for(&mesh, &ring, Vec3::new(0.0, 0.0, 500.0));
    assert!(slides.iter().all(|s| s.amount == 1.0));
}

#[test]
fn a_sideways_drag_follows_the_nearest_edge() {
    let mesh = cube([2, 1, 1]);
    let middle = at(&mesh, Vec3::new(0.0, -50.0, 100.0));
    // Dragged a little up and mostly right: it slides along the top edge
    // to the right corner, keeping its height.
    let slides = slides_for(&mesh, &[middle], Vec3::new(25.0, 0.0, 5.0));
    let right = at(&mesh, Vec3::new(50.0, -50.0, 100.0));
    assert_eq!(slides[0].towards, right);
    assert!((slides[0].amount - 0.5).abs() < 1e-4);
}

#[test]
fn nothing_that_way_and_missing_points() {
    let mut mesh = cube([1, 1, 1]);
    let corner = at(&mesh, Vec3::new(50.0, 50.0, 100.0));
    // Straight out of the cube: no edge goes that way.
    let slides = slides_for(&mesh, &[corner], Vec3::new(10.0, 10.0, 10.0));
    assert_eq!(slides[0].amount, 0.0);
    let before = mesh.position(corner);
    slide_points(&mut mesh, &slides);
    assert_eq!(mesh.position(corner), before);
    // Ids not in the mesh, or a point sliding towards itself.
    let ghost = VertexId::from_raw(999);
    let outcome = slide_points(
        &mut mesh,
        &[
            Slide {
                point: ghost,
                towards: corner,
                amount: 0.5,
            },
            Slide {
                point: corner,
                towards: corner,
                amount: 0.5,
            },
        ],
    );
    assert_eq!(outcome.missing, 2);
    // A number that isn't real stays put.
    let other = at(&mesh, Vec3::new(-50.0, 50.0, 100.0));
    slide_points(
        &mut mesh,
        &[Slide {
            point: corner,
            towards: other,
            amount: f32::NAN,
        }],
    );
    assert_eq!(mesh.position(corner), before);
}
