//! Each point's frame on the surface (decision 0304): which way the
//! surface faces there (the area-weighted facing of its polygons) and a
//! direction along it (towards its first neighbour, laid flat on the
//! surface). A sculpt's moves are measured in these frames, so when the
//! shape under a sculpt changes, the detail rides along with it.
//!
//! The same function works a frame out wherever one is needed (evaluating
//! a Sculpt layer, ending a stroke), so the same shape always gives the
//! same frames, to the last bit.

use glam::Vec3;
use mesh::{Mesh, Topology, VertexId};

/// A point's frame: its facing and a direction along the surface, both
/// of unit length and square to each other. A loose point (no polygons)
/// has the object's own axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    /// Out of the surface.
    pub normal: Vec3,
    /// Along the surface.
    pub tangent: Vec3,
}

impl Frame {
    /// The object's own axes: for points with no surface round them.
    pub const AXES: Self = Self {
        normal: Vec3::Z,
        tangent: Vec3::X,
    };

    /// The third direction, square to both.
    pub fn across(&self) -> Vec3 {
        self.normal.cross(self.tangent)
    }

    /// A move given in this frame (along, across, out), in the object's
    /// space.
    pub fn to_object(&self, m: Vec3) -> Vec3 {
        self.tangent * m.x + self.across() * m.y + self.normal * m.z
    }

    /// A move in the object's space, in this frame (along, across, out).
    pub fn from_object(&self, d: Vec3) -> Vec3 {
        Vec3::new(
            d.dot(self.tangent),
            d.dot(self.across()),
            d.dot(self.normal),
        )
    }
}

/// The area-weighted facing of polygon `corners` (twice its area long),
/// by Newell's method.
pub(crate) fn newell(mesh: &Mesh, corners: &[VertexId]) -> Vec3 {
    let mut normal = Vec3::ZERO;
    for (i, a) in corners.iter().enumerate() {
        let b = corners[(i + 1) % corners.len()];
        let (Some(p), Some(q)) = (mesh.position(*a), mesh.position(b)) else {
            continue;
        };
        normal += Vec3::new(
            (p.y - q.y) * (p.z + q.z),
            (p.z - q.z) * (p.x + q.x),
            (p.x - q.x) * (p.y + q.y),
        );
    }
    normal
}

/// Point `v`'s frame on `mesh` (whose topology is `topology`).
pub fn frame(mesh: &Mesh, topology: &Topology, v: VertexId) -> Frame {
    let Some(p) = mesh.position(v) else {
        return Frame::AXES;
    };
    let mut sum = Vec3::ZERO;
    for face in topology.vertex_faces(v) {
        if let Some(corners) = mesh.face(*face) {
            sum += newell(mesh, corners);
        }
    }
    let Some(normal) = sum.try_normalize().filter(|n| n.is_finite()) else {
        return Frame::AXES;
    };
    let along = topology
        .vertex_edges(v)
        .first()
        .and_then(|e| {
            let other = mesh::topology::other_end(*e, v);
            mesh.position(other)
        })
        .map(|q| q - p)
        .map(|d| d - normal * d.dot(normal))
        .and_then(Vec3::try_normalize)
        .filter(|t| t.is_finite())
        .unwrap_or_else(|| normal.any_orthonormal_vector());
    Frame {
        normal,
        tangent: along,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quad() -> Mesh {
        let mut m = Mesh::new();
        let a = m.add_vertex(Vec3::new(0.0, 0.0, 0.0)).unwrap();
        let b = m.add_vertex(Vec3::new(1.0, 0.0, 0.0)).unwrap();
        let c = m.add_vertex(Vec3::new(1.0, 1.0, 0.0)).unwrap();
        let d = m.add_vertex(Vec3::new(0.0, 1.0, 0.0)).unwrap();
        m.add_face(&[a, b, c, d]).unwrap();
        m
    }

    #[test]
    fn a_flat_quads_points_face_up_with_a_tangent_on_the_plane() {
        let m = quad();
        let t = m.topology();
        for (v, _) in m.vertices() {
            let f = frame(&m, t, v);
            assert!((f.normal - Vec3::Z).length() < 1e-6);
            assert!(f.tangent.z.abs() < 1e-6);
            assert!((f.tangent.length() - 1.0).abs() < 1e-6);
            assert!(f.normal.dot(f.tangent).abs() < 1e-6);
        }
    }

    #[test]
    fn a_move_goes_to_object_space_and_back() {
        let m = quad();
        let f = frame(&m, m.topology(), VertexId::from_raw(2));
        let d = Vec3::new(0.3, -0.7, 1.1);
        let back = f.to_object(f.from_object(d));
        assert!((back - d).length() < 1e-6);
    }

    #[test]
    fn a_loose_point_has_the_objects_axes() {
        let mut m = quad();
        let loose = m.add_vertex(Vec3::new(5.0, 5.0, 5.0)).unwrap();
        assert_eq!(frame(&m, m.topology(), loose), Frame::AXES);
        assert_eq!(frame(&m, m.topology(), VertexId::from_raw(99)), Frame::AXES);
    }
}
