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
    // `phi` is measured from the top pole: 0 at the top, pi at the bottom.
    let at = |theta: f32, phi: f32| {
        let r = radius * bump(theta, phi);
        Vec3::new(
            r * phi.sin() * theta.cos(),
            r * phi.sin() * theta.sin(),
            radius + r * phi.cos(),
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

    /// The polygon and point counts the heavy-mesh check and the
    /// benchmarks were measured with: fixing the shape must not change them.
    #[test]
    fn polygon_counts_are_unchanged() {
        for (asked, faces, points) in [
            (0, 18, 14),
            (5_000, 5_000, 4_902),
            (20_000, 20_000, 19_802),
            (3_000_000, 3_001_250, 2_998_802),
        ] {
            // rings = round(sqrt(asked / 2)), at least 3; sides = rings × 2.
            let rings = ((asked as f32 / 2.0).sqrt().round() as usize).max(3);
            let sides = rings * 2;
            assert_eq!(rings * sides, faces, "{asked}");
            assert_eq!(2 + (rings - 1) * sides, points, "{asked}");
            if asked <= 20_000 {
                let mesh = heavy_ball(asked, 100.0);
                assert_eq!(mesh.face_count(), faces, "{asked}");
                assert_eq!(mesh.vertex_count(), points, "{asked}");
            }
        }
    }

    /// The volume inside, positive when every polygon faces outward.
    fn signed_volume(mesh: &Mesh) -> f32 {
        crate::shading::shaded_triangles(mesh)
            .iter()
            .map(|t| t.positions[0].dot(t.positions[1].cross(t.positions[2])) / 6.0)
            .sum()
    }

    // Regression: the poles were joined to the rings at the far end, so the
    // ball was inside out and its pole fans cut right through it.
    #[test]
    fn every_polygon_faces_outward() {
        for (asked, radius) in [(0, 10.0), (50, 10.0), (5_000, 100.0), (40_000, 50.0)] {
            let mesh = heavy_ball(asked, radius);
            let volume = signed_volume(&mesh);
            let ball = 4.0 / 3.0 * std::f32::consts::PI * radius.powi(3);
            assert!(
                (0.5 * ball..1.2 * ball).contains(&volume),
                "{asked}: volume {volume}, a true ball {ball}"
            );
            let centre = mesh.bounds().unwrap().centre();
            for (face, corners) in mesh.faces() {
                let middle = corners
                    .iter()
                    .map(|v| mesh.position(*v).unwrap())
                    .sum::<Vec3>()
                    / corners.len() as f32;
                let away = (middle - centre).normalize();
                let facing = mesh.face_normal(face).dot(away);
                assert!(facing > 0.0, "{asked}: face {face:?} faces {facing}");
            }
        }
    }

    /// Whether the line from `a` to `b` passes through the triangle `t`
    /// or touches it (Möller–Trumbore). Polygons that share no point must
    /// not even touch.
    fn crosses(a: Vec3, b: Vec3, t: &[Vec3; 3]) -> bool {
        let along = b - a;
        let (e1, e2) = (t[1] - t[0], t[2] - t[0]);
        let p = along.cross(e2);
        let det = e1.dot(p);
        if det.abs() < 1e-9 {
            return false;
        }
        let s = a - t[0];
        let u = s.dot(p) / det;
        let q = s.cross(e1);
        let v = along.dot(q) / det;
        let k = e2.dot(q) / det;
        let inside = -1e-4;
        u > inside && v > inside && u + v < 1.0 - inside && k > inside && k < 1.0 - inside
    }

    #[test]
    fn small_balls_never_cut_through_themselves() {
        for asked in [0, 8, 50, 200, 400] {
            let mesh = heavy_ball(asked, 10.0);
            // Each triangle with the points of the polygon it is cut from;
            // polygons sharing a point meet there, so they are skipped.
            let triangles: Vec<([Vec3; 3], &[VertexId])> = crate::shading::shaded_triangles(&mesh)
                .iter()
                .map(|t| (t.positions, mesh.face(t.face).unwrap()))
                .collect();
            for (i, (a, a_points)) in triangles.iter().enumerate() {
                for (b, b_points) in &triangles[i + 1..] {
                    if a_points.iter().any(|v| b_points.contains(v)) {
                        continue;
                    }
                    for k in 0..3 {
                        assert!(
                            !crosses(a[k], a[(k + 1) % 3], b) && !crosses(b[k], b[(k + 1) % 3], a),
                            "{asked}: triangles cut through each other"
                        );
                    }
                }
            }
        }
    }
}
