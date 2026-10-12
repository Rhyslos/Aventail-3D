//! Sample meshes for checking how the app copes with heavy shapes
//! ([decision 0035](../../../docs/decisions/0035-heavy-meshes.md)).
//!
//! The heavy-mesh check needs a shape like a model someone might import:
//! millions of small polygons on a bumpy surface, closed, with points
//! shared between polygons. The primitives stop well short of that on
//! purpose, so this builds one directly.

use crate::{Mesh, VertexId};
use glam::Vec3;

/// A bumpy ball of about `polygons` four-sided polygons (with a ring of
/// triangles at each pole), of about `radius` centimetres, standing on the
/// ground (about `radius` × 2 tall). Its surface waves gently, so shading and picking see a real
/// curved surface rather than flat panels. The same `polygons` always
/// gives the same mesh.
pub fn heavy_ball(polygons: usize, radius: f32) -> Mesh {
    // `sides` around, `rings` from pole to pole, about twice as many
    // sides as rings so the polygons are roughly square.
    let rings = ((polygons as f32 / 2.0).sqrt().round() as usize).max(3);
    let sides = (rings * 2).max(3);
    let mut mesh = Mesh::new();
    let bump = |theta: f32, phi: f32| {
        1.0 + 0.03 * (theta * 7.0).sin() * (phi * 5.0).sin() + 0.01 * (theta * 31.0).cos()
    };
    let at = |theta: f32, phi: f32| {
        let r = radius * bump(theta, phi);
        Vec3::new(
            r * phi.sin() * theta.cos(),
            r * phi.sin() * theta.sin(),
            radius - r * phi.cos(),
        )
    };
    let add = |mesh: &mut Mesh, p: Vec3| mesh.push_vertex(p);
    let bottom = add(&mut mesh, Vec3::new(0.0, 0.0, 0.0));
    let mut grid: Vec<VertexId> = Vec::with_capacity((rings - 1) * sides);
    for ring in 1..rings {
        // From the bottom pole (phi = pi) upwards.
        let phi = std::f32::consts::PI * (1.0 - ring as f32 / rings as f32);
        for side in 0..sides {
            let theta = std::f32::consts::TAU * side as f32 / sides as f32;
            grid.push(add(&mut mesh, at(theta, phi)));
        }
    }
    let top = add(&mut mesh, Vec3::new(0.0, 0.0, 2.0 * radius));
    let point = |ring: usize, side: usize| grid[(ring - 1) * sides + side % sides];
    let face = |mesh: &mut Mesh, corners: &[VertexId]| mesh.push_face(corners);
    for side in 0..sides {
        face(&mut mesh, &[bottom, point(1, side + 1), point(1, side)]);
        face(
            &mut mesh,
            &[top, point(rings - 1, side), point(rings - 1, side + 1)],
        );
    }
    for ring in 1..rings - 1 {
        for side in 0..sides {
            face(
                &mut mesh,
                &[
                    point(ring, side),
                    point(ring, side + 1),
                    point(ring + 1, side + 1),
                    point(ring + 1, side),
                ],
            );
        }
    }
    // The bumps reach a little below the bottom pole: stand it on the
    // ground.
    if let Some(bounds) = mesh.bounds() {
        let lift = Vec3::Z * -bounds.min.z;
        let points: Vec<(VertexId, Vec3)> = mesh.vertices().collect();
        for (v, p) in points {
            mesh.set_position(v, p + lift);
        }
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Topology;

    #[test]
    fn the_heavy_ball_is_closed_and_about_the_size_asked_for() {
        let mesh = heavy_ball(5_000, 100.0);
        let faces = mesh.face_count();
        assert!((4_500..5_500).contains(&faces), "{faces}");
        let topology = Topology::new(&mesh);
        assert!(topology.border_edges().is_empty(), "closed");
        assert!(topology.flipped_edges().is_empty(), "consistent winding");
        let bounds = mesh.bounds().unwrap();
        assert_eq!(bounds.min.z, 0.0, "on the ground");
        assert!((195.0..215.0).contains(&bounds.max.z), "{}", bounds.max.z);
        assert_eq!(heavy_ball(5_000, 100.0), mesh, "always the same");
    }

    #[test]
    fn tiny_requests_still_give_a_ball() {
        let mesh = heavy_ball(0, 10.0);
        assert!(mesh.face_count() >= 9);
    }
}
