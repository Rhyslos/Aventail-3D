//! Turning a mesh into triangles to draw, with smooth or sharp shading
//! decided by its hard edges.
//!
//! Each corner of each face gets its own normal (the direction light
//! bounces from): the average of the faces around that point that are
//! joined to this face across soft edges, each counted by the angle it
//! fills at that point. Across a hard edge the shading
//! breaks, giving a crisp edge; across soft edges it flows, giving a
//! smooth curve. Normals are always worked out like this, never stored
//! (DESIGN.md §4.1).

use crate::polygon::{EdgeKey, FaceId, Mesh, VertexId, sides};
use glam::Vec3;
use std::collections::BTreeMap;

/// One triangle ready to draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadedTriangle {
    /// The face it is part of.
    pub face: FaceId,
    /// Its corners, counter-clockwise seen from outside.
    pub positions: [Vec3; 3],
    /// The shading direction at each corner; unit length.
    pub normals: [Vec3; 3],
}

/// Every face cut into triangles, with shading normals. Faces are cut
/// as a fan from their first corner, which is right for the convex faces
/// primitives make; faces with dents get a proper cut with the modelling
/// tools (Phase 4).
pub fn shaded_triangles(mesh: &Mesh) -> Vec<ShadedTriangle> {
    let normals = corner_normals(mesh);
    let mut out = Vec::new();
    for (face, corners) in mesh.faces() {
        let corner = |i: usize| {
            let v = corners[i];
            let position = mesh.position(v).unwrap_or(Vec3::ZERO);
            let normal = normals
                .get(&(face, v))
                .copied()
                .unwrap_or_else(|| mesh.face_normal(face));
            (position, normal)
        };
        for i in 1..corners.len().saturating_sub(1) {
            let (a, b, c) = (corner(0), corner(i), corner(i + 1));
            out.push(ShadedTriangle {
                face,
                positions: [a.0, b.0, c.0],
                normals: [a.1, b.1, c.1],
            });
        }
    }
    out
}

/// The shading normal of every face corner, keyed by face and point.
pub fn corner_normals(mesh: &Mesh) -> BTreeMap<(FaceId, VertexId), Vec3> {
    // How much each face counts at each of its corners: its normal scaled
    // by the corner's angle, so a face counts by how much of the view
    // around the point it fills, however it is cut up.
    let mut weighted: BTreeMap<(FaceId, VertexId), Vec3> = BTreeMap::new();
    for (face, corners) in mesh.faces() {
        let normal = mesh.face_normal(face);
        let n = corners.len();
        for i in 0..n {
            let at = |k: usize| mesh.position(corners[k]).unwrap_or(Vec3::ZERO);
            let here = at(i);
            let to_prev = at((i + n - 1) % n) - here;
            let to_next = at((i + 1) % n) - here;
            let angle = to_prev.angle_between(to_next);
            let angle = if angle.is_finite() { angle } else { 0.0 };
            weighted.insert((face, corners[i]), normal * angle.max(1e-6));
        }
    }
    let mut faces_at: BTreeMap<VertexId, Vec<FaceId>> = BTreeMap::new();
    let mut faces_on: BTreeMap<EdgeKey, Vec<FaceId>> = BTreeMap::new();
    let mut edges_at: BTreeMap<VertexId, Vec<EdgeKey>> = BTreeMap::new();
    for (face, corners) in mesh.faces() {
        for (a, b) in sides(corners) {
            faces_at.entry(a).or_default().push(face);
            let key = EdgeKey::new(a, b);
            let on = faces_on.entry(key).or_default();
            if on.is_empty() {
                edges_at.entry(a).or_default().push(key);
                edges_at.entry(b).or_default().push(key);
            }
            on.push(face);
        }
    }
    let mut out = BTreeMap::new();
    for (vertex, faces) in &faces_at {
        // Join faces around this point that share a soft edge through it;
        // each joined group shades as one smooth surface.
        let mut group: Vec<usize> = (0..faces.len()).collect();
        for edge in edges_at.get(vertex).into_iter().flatten() {
            let (a, b) = edge.ends();
            if mesh.is_hard(a, b) {
                continue;
            }
            let Some(sharing) = faces_on.get(edge) else {
                continue;
            };
            let members: Vec<usize> = sharing
                .iter()
                .filter_map(|f| faces.iter().position(|g| g == f))
                .collect();
            for pair in members.windows(2) {
                join(&mut group, pair[0], pair[1]);
            }
        }
        for (i, face) in faces.iter().enumerate() {
            let root = find(&mut group, i);
            let sum: Vec3 = (0..faces.len())
                .filter(|j| find(&mut group, *j) == root)
                .filter_map(|j| weighted.get(&(faces[j], *vertex)))
                .sum();
            let normal = sum
                .try_normalize()
                .unwrap_or_else(|| mesh.face_normal(*face));
            out.insert((*face, *vertex), normal);
        }
    }
    out
}

/// The group's representative, shortening the path on the way.
fn find(group: &mut [usize], mut i: usize) -> usize {
    while group[i] != i {
        group[i] = group[group[i]];
        i = group[i];
    }
    i
}

fn join(group: &mut [usize], a: usize, b: usize) {
    let (ra, rb) = (find(group, a), find(group, b));
    if ra != rb {
        group[ra.max(rb)] = ra.min(rb);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two squares meeting at a right angle along the X axis: one on the
    /// ground facing up, one standing up facing -Y.
    fn folded(hard: bool) -> (Mesh, [VertexId; 6]) {
        let mut mesh = Mesh::new();
        let v = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 0.0, -1.0),
            Vec3::new(0.0, 0.0, -1.0),
        ]
        .map(|p| mesh.add_vertex(p).unwrap());
        mesh.add_face(&[v[0], v[1], v[2], v[3]]).unwrap();
        mesh.add_face(&[v[0], v[5], v[4], v[1]]).unwrap();
        mesh.set_hard(v[0], v[1], hard);
        (mesh, v)
    }

    #[test]
    fn hard_edges_keep_each_face_its_own_normal() {
        let (mesh, v) = folded(true);
        let normals = corner_normals(&mesh);
        let top = mesh.faces().next().unwrap().0;
        assert!((normals[&(top, v[0])] - Vec3::Z).length() < 1e-5);
    }

    #[test]
    fn soft_edges_blend_the_normals_of_the_faces_they_join() {
        let (mesh, v) = folded(false);
        let normals = corner_normals(&mesh);
        let top = mesh.faces().next().unwrap().0;
        let blended = normals[&(top, v[0])];
        let expected = (Vec3::Z + Vec3::NEG_Y).normalize();
        assert!((blended - expected).length() < 1e-5, "{blended}");
        // A corner touching only one face keeps that face's normal.
        assert!((normals[&(top, v[2])] - Vec3::Z).length() < 1e-5);
    }

    #[test]
    fn faces_are_cut_into_fans_of_triangles() {
        let (mesh, _) = folded(true);
        let triangles = shaded_triangles(&mesh);
        assert_eq!(triangles.len(), 4, "two quads, two triangles each");
        for t in &triangles {
            // Counter-clockwise: the winding agrees with the normal.
            let winding = (t.positions[1] - t.positions[0]).cross(t.positions[2] - t.positions[0]);
            assert!(winding.dot(t.normals[0]) > 0.0);
        }
    }

    #[test]
    fn an_empty_mesh_draws_nothing() {
        assert!(shaded_triangles(&Mesh::new()).is_empty());
    }
}
