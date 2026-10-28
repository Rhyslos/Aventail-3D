//! Outlines for cartoon looks ([decision 0201](../../../docs/decisions/0201-outlines.md)):
//! each outlined object's shape as an *inverted hull*, a copy whose
//! corners are pushed out along their smoothed direction (the average of
//! the polygons round each point, so the shell has no gaps at hard
//! edges) and whose front is left out, so only the rim shows round the
//! object.

use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use mesh::Mesh;
use std::collections::HashMap;

/// One corner of an outline shell, in the mesh's own space.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct OutlineVertex {
    /// Where the point is, in centimetres.
    pub position: [f32; 3],
    /// Which way it is pushed out: the polygons' directions round it,
    /// averaged; unit length (or zero for a point with no polygons).
    pub out: [f32; 3],
}

impl OutlineVertex {
    pub(crate) const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<OutlineVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
    };
}

/// An object's outline shell, ready to upload: a corner per point, and
/// its polygons as triangles.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OutlineGeometry {
    /// One per point of the mesh.
    pub vertices: Vec<OutlineVertex>,
    /// Corners of triangles, three a triangle.
    pub triangles: Vec<u32>,
}

impl OutlineGeometry {
    /// The shell of `mesh`: each point once, pushed along the area-weighted
    /// average of its polygons' directions; polygons cut into fans.
    pub fn of(mesh: &Mesh) -> Self {
        let mut index: HashMap<mesh::VertexId, u32> = HashMap::new();
        let mut vertices = Vec::with_capacity(mesh.vertex_count());
        for (id, p) in mesh.vertices() {
            index.insert(id, vertices.len() as u32);
            vertices.push(OutlineVertex {
                position: p.to_array(),
                out: [0.0; 3],
            });
        }
        let mut sums = vec![Vec3::ZERO; vertices.len()];
        let mut triangles = Vec::new();
        for (_, corners) in mesh.faces() {
            let ids: Vec<u32> = corners
                .iter()
                .filter_map(|c| index.get(c).copied())
                .collect();
            if ids.len() < 3 {
                continue;
            }
            let at = |i: u32| Vec3::from(vertices[i as usize].position);
            // Twice the area, in the polygon's direction (Newell's way).
            let mut area = Vec3::ZERO;
            for k in 0..ids.len() {
                let (a, b) = (at(ids[k]), at(ids[(k + 1) % ids.len()]));
                area += a.cross(b);
            }
            if !area.is_finite() {
                continue;
            }
            for &i in &ids {
                sums[i as usize] += area;
            }
            for k in 1..ids.len() - 1 {
                triangles.extend([ids[0], ids[k], ids[k + 1]]);
            }
        }
        for (v, sum) in vertices.iter_mut().zip(&sums) {
            v.out = sum.normalize_or_zero().to_array();
        }
        Self {
            vertices,
            triangles,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cube_shell_pushes_each_corner_out_diagonally() {
        let cube = mesh::PrimitiveKind::Cube.standard().mesh();
        let shell = OutlineGeometry::of(&cube);
        assert_eq!(shell.vertices.len(), 8);
        assert_eq!(shell.triangles.len(), 6 * 2 * 3);
        for v in &shell.vertices {
            let p = Vec3::from(v.position) - Vec3::new(0.0, 0.0, 50.0);
            let out = Vec3::from(v.out);
            // Away from the middle, along the corner's diagonal.
            assert!((out.length() - 1.0).abs() < 1e-4);
            assert!(out.dot(p.normalize()) > 0.99, "{out} at {p}");
        }
    }

    #[test]
    fn an_empty_mesh_makes_an_empty_shell() {
        let shell = OutlineGeometry::of(&Mesh::new());
        assert!(shell.vertices.is_empty() && shell.triangles.is_empty());
    }
}
