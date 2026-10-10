//! The flat-sided primitives: the cube and the plane.

use super::{CubeParams, PlaneParams};
use crate::Mesh;
use crate::polygon::VertexId;
use glam::Vec3;
use std::collections::HashMap;

/// A box from `-size/2` to `size/2` across, `0` to `size.z` up, each side
/// cut into a grid. Points on the edges between sides are shared, and
/// only the box's twelve edges are hard, so cutting a side into more
/// pieces never changes how it shades.
pub(super) fn cube(p: &CubeParams) -> Mesh {
    let mut mesh = Mesh::new();
    let min = Vec3::new(-p.size.x / 2.0, -p.size.y / 2.0, 0.0);
    let segs = p.segments;
    let at = |cell: [u32; 3]| {
        Vec3::new(
            min.x + p.size.x * cell[0] as f32 / segs[0] as f32,
            min.y + p.size.y * cell[1] as f32 / segs[1] as f32,
            min.z + p.size.z * cell[2] as f32 / segs[2] as f32,
        )
    };
    let mut points: HashMap<[u32; 3], VertexId> = HashMap::new();
    let mut point = |mesh: &mut Mesh, cell: [u32; 3]| {
        *points
            .entry(cell)
            .or_insert_with(|| mesh.push_vertex(at(cell)))
    };
    // Each side: the axis it faces along, whether it is the far side, and
    // the two axes across it, ordered so that across × up points outward
    // on the far side.
    let sides = [
        (2, false),
        (2, true),
        (1, false),
        (1, true),
        (0, false),
        (0, true),
    ];
    let mut edges = Vec::new();
    for (axis, far) in sides {
        let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
        let level = if far { segs[axis] } else { 0 };
        for i in 0..segs[u] {
            for j in 0..segs[v] {
                let cell = |di: u32, dj: u32| {
                    let mut c = [0; 3];
                    c[axis] = level;
                    c[u] = i + di;
                    c[v] = j + dj;
                    c
                };
                let mut quad = [cell(0, 0), cell(1, 0), cell(1, 1), cell(0, 1)];
                if !far {
                    quad.reverse();
                }
                let ids = quad.map(|c| point(&mut mesh, c));
                mesh.push_face(&ids);
                for k in 0..4 {
                    edges.push((quad[k], quad[(k + 1) % 4], ids[k], ids[(k + 1) % 4]));
                }
            }
        }
    }
    // An edge lies along one of the box's twelve edges when, on at least
    // two axes, both its ends sit at the same end of the box.
    for (a, b, ia, ib) in edges {
        let shared = (0..3)
            .filter(|&k| {
                let end = |c: [u32; 3]| (c[k] == 0, c[k] == segs[k]);
                let (a_min, a_max) = end(a);
                let (b_min, b_max) = end(b);
                (a_min && b_min) || (a_max && b_max)
            })
            .count();
        if shared >= 2 {
            mesh.set_hard(ia, ib, true);
        }
    }
    mesh
}

/// A flat grid on the ground facing up, centred on the origin.
pub(super) fn plane(p: &PlaneParams) -> Mesh {
    let mut mesh = Mesh::new();
    let [sx, sy] = p.segments;
    let mut rows = Vec::with_capacity(sy as usize + 1);
    for j in 0..=sy {
        let y = -p.depth / 2.0 + p.depth * j as f32 / sy as f32;
        let row: Vec<VertexId> = (0..=sx)
            .map(|i| {
                let x = -p.width / 2.0 + p.width * i as f32 / sx as f32;
                mesh.push_vertex(Vec3::new(x, y, 0.0))
            })
            .collect();
        rows.push(row);
    }
    for j in 0..sy as usize {
        for i in 0..sx as usize {
            mesh.push_face(&[
                rows[j][i],
                rows[j][i + 1],
                rows[j + 1][i + 1],
                rows[j + 1][i],
            ]);
        }
    }
    mesh
}
