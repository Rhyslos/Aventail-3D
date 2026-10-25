//! Surface nets: the new surface between a remesh grid's inside and
//! outside points, then evened out and made into a mesh with materials
//! and box-projected UVs.

use super::field::{Field, NO_TRIANGLE};
use super::{Grid, RemeshProblem, Soup};
use crate::attributes::{AttributeValues, Domain, UV, Value};
use crate::material::MATERIAL;
use crate::{Mesh, VertexId};
use glam::{Vec2, Vec3};

/// No point in this cell.
const NONE: u32 = u32::MAX;

/// A cell's twelve edges, as pairs of its corners (bit 0 along X, bit 1
/// along Y, bit 2 along Z).
const EDGES: [(usize, usize); 12] = [
    (0, 1),
    (2, 3),
    (4, 5),
    (6, 7),
    (0, 2),
    (1, 3),
    (4, 6),
    (5, 7),
    (0, 4),
    (1, 5),
    (2, 6),
    (3, 7),
];

/// How much each relax pass moves a point towards its neighbours' middle,
/// and then back out (Taubin's λ and μ: together they even the surface
/// without shrinking it).
const RELAX_IN: f32 = 0.5;
const RELAX_OUT: f32 = -0.53;

/// The new surface, before it is a mesh.
pub(super) struct Surface {
    pub(super) points: Vec<Vec3>,
    /// The original triangle nearest each point ([`NO_TRIANGLE`] if none).
    nearest: Vec<u32>,
    /// Quads of points, facing out of the shape.
    pub(super) quads: Vec<[u32; 4]>,
}

impl Surface {
    /// The surface through `grid`'s cells where `field` changes side.
    pub(super) fn extract(grid: &Grid, field: &Field) -> Self {
        let [nx, ny, nz] = grid.size;
        let mut surface = Self {
            points: Vec::new(),
            nearest: Vec::new(),
            quads: Vec::new(),
        };
        if nx < 2 || ny < 2 || nz < 2 {
            return surface;
        }
        let (cx, cy) = (nx - 1, ny - 1);
        let cell_of = |i: usize, j: usize, k: usize| i + cx * (j + cy * k);
        let mut cell_point = vec![NONE; cx * cy * (nz - 1)];
        for k in 0..nz - 1 {
            for j in 0..ny - 1 {
                for i in 0..nx - 1 {
                    if let Some((p, near)) = cell_point_at(grid, field, i, j, k) {
                        cell_point[cell_of(i, j, k)] = surface.points.len() as u32;
                        surface.points.push(p);
                        surface.nearest.push(near);
                    }
                }
            }
        }
        // One quad across each grid edge the surface crosses, made of the
        // points of the four cells round it.
        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx {
                    let here = field.inside[grid.index(i, j, k)];
                    for axis in 0..3 {
                        let step = [
                            usize::from(axis == 0),
                            usize::from(axis == 1),
                            usize::from(axis == 2),
                        ];
                        let (ni, nj, nk) = (i + step[0], j + step[1], k + step[2]);
                        if ni >= nx || nj >= ny || nk >= nz {
                            continue;
                        }
                        if field.inside[grid.index(ni, nj, nk)] == here {
                            continue;
                        }
                        // The other two axes, in the order whose cross
                        // product is this axis (y × z = x, z × x = y,
                        // x × y = z).
                        let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
                        let at = [i, j, k];
                        if at[u] == 0
                            || at[v] == 0
                            || at[u] >= grid.size[u] - 1
                            || at[v] >= grid.size[v] - 1
                        {
                            continue;
                        }
                        let corner = |du: usize, dv: usize| {
                            let mut c = at;
                            c[u] = c[u] + du - 1;
                            c[v] = c[v] + dv - 1;
                            cell_point[cell_of(c[0], c[1], c[2])]
                        };
                        // Round the edge anticlockwise seen from its +axis
                        // end: facing +axis.
                        let mut quad = [corner(0, 0), corner(1, 0), corner(1, 1), corner(0, 1)];
                        if quad.contains(&NONE) {
                            continue;
                        }
                        // Inside here, outside past it: the surface faces
                        // along +axis; otherwise the other way.
                        if !here {
                            quad.reverse();
                        }
                        surface.quads.push(quad);
                    }
                }
            }
        }
        surface
    }

    /// Evens the surface out with `passes` relax passes, each moving
    /// every point towards its neighbours' middle and back out a little,
    /// so it doesn't shrink.
    pub(super) fn relax(&mut self, passes: u32) {
        if passes == 0 {
            return;
        }
        let mut neighbours: Vec<Vec<u32>> = vec![Vec::new(); self.points.len()];
        for quad in &self.quads {
            for n in 0..4 {
                let (a, b) = (quad[n], quad[(n + 1) % 4]);
                for (from, to) in [(a, b), (b, a)] {
                    let list = &mut neighbours[from as usize];
                    if !list.contains(&to) {
                        list.push(to);
                    }
                }
            }
        }
        for _ in 0..passes {
            for factor in [RELAX_IN, RELAX_OUT] {
                let moved: Vec<Vec3> = self
                    .points
                    .iter()
                    .zip(&neighbours)
                    .map(|(p, list)| {
                        if list.is_empty() {
                            return *p;
                        }
                        let middle = list.iter().map(|n| self.points[*n as usize]).sum::<Vec3>()
                            / list.len() as f32;
                        *p + (middle - *p) * factor
                    })
                    .collect();
                self.points = moved;
            }
        }
    }

    /// The surface as a mesh: each polygon with the material of the
    /// original polygons nearest its corners (the most common), and UVs
    /// by box projection with seams where they part.
    pub(super) fn into_mesh(self, soup: &Soup) -> Result<Mesh, RemeshProblem> {
        let mut mesh = Mesh::new();
        let mut ids: Vec<VertexId> = Vec::with_capacity(self.points.len());
        for p in &self.points {
            ids.push(mesh.add_vertex(*p).map_err(|_| RemeshProblem::TooBig)?);
        }
        for quad in &self.quads {
            let corners = quad.map(|q| ids[q as usize]);
            mesh.add_face(&corners).map_err(|_| RemeshProblem::TooBig)?;
        }
        if soup.has_materials {
            let slots: Vec<i32> = self
                .quads
                .iter()
                .map(|quad| self.material_of(quad, soup) as i32)
                .collect();
            mesh.set_attribute_values(
                MATERIAL,
                Domain::Face,
                Value::Int(0),
                AttributeValues::Int(slots),
            )
            .map_err(|_| RemeshProblem::TooBig)?;
        }
        let uvs = self.box_uvs();
        if mesh
            .set_attribute_values(
                UV,
                Domain::Corner,
                Value::Vec2(Vec2::ZERO),
                AttributeValues::Vec2(uvs),
            )
            .is_ok()
        {
            crate::uvs::seams_from_splits(&mut mesh);
        }
        Ok(mesh)
    }

    /// The material slot most of `quad`'s corners are nearest (the
    /// lowest of equals).
    fn material_of(&self, quad: &[u32; 4], soup: &Soup) -> u32 {
        let slots: Vec<u32> = quad
            .iter()
            .filter_map(|p| {
                let t = *self.nearest.get(*p as usize)?;
                (t != NO_TRIANGLE)
                    .then(|| soup.slots.get(t as usize).copied())
                    .flatten()
            })
            .collect();
        let mut best = (0, 0);
        for slot in &slots {
            let count = slots.iter().filter(|s| *s == slot).count();
            if count > best.1 || (count == best.1 && *slot < best.0) {
                best = (*slot, count);
            }
        }
        best.0
    }

    /// Box-projected UVs, corner by corner in the mesh's order: each quad
    /// seen from the side its normal points to most, the whole shape
    /// fitted into the texture square (sides overlap).
    fn box_uvs(&self) -> Vec<Vec2> {
        let Some(bounds) = crate::Bounds::around(self.points.iter().copied()) else {
            return Vec::new();
        };
        let centre = bounds.centre();
        let size = bounds.size().max_element().max(1e-6);
        let mut uvs = Vec::with_capacity(self.quads.len() * 4);
        for quad in &self.quads {
            let p = quad.map(|q| self.points[q as usize]);
            let normal = (p[2] - p[0]).cross(p[3] - p[1]);
            let a = normal.abs();
            for corner in p {
                let d = (corner - centre) / size;
                let uv = if a.x >= a.y && a.x >= a.z {
                    Vec2::new(if normal.x >= 0.0 { d.y } else { -d.y }, d.z)
                } else if a.y >= a.z {
                    Vec2::new(if normal.y >= 0.0 { -d.x } else { d.x }, d.z)
                } else {
                    Vec2::new(d.x, if normal.z >= 0.0 { d.y } else { -d.y })
                };
                uvs.push(uv + Vec2::splat(0.5));
            }
        }
        uvs
    }
}

/// The surface's point in cell (i, j, k), if the surface passes through
/// it: the middle of where it crosses the cell's edges, with the original
/// triangle nearest the cell.
fn cell_point_at(grid: &Grid, field: &Field, i: usize, j: usize, k: usize) -> Option<(Vec3, u32)> {
    let at = |b: usize| grid.index(i + (b & 1), j + ((b >> 1) & 1), k + (b >> 2));
    let first = field.inside[at(0)];
    if (1..8).all(|b| field.inside[at(b)] == first) {
        return None;
    }
    let corner = |b: usize| grid.point(i + (b & 1), j + ((b >> 1) & 1), k + (b >> 2));
    let mut sum = Vec3::ZERO;
    let mut count = 0.0;
    for (a, b) in EDGES {
        if field.inside[at(a)] == field.inside[at(b)] {
            continue;
        }
        let (va, vb) = (field.value[at(a)], field.value[at(b)]);
        let gap = va - vb;
        let t = if gap.abs() > f32::MIN_POSITIVE {
            (va / gap).clamp(0.0, 1.0)
        } else {
            0.5
        };
        sum += corner(a).lerp(corner(b), t);
        count += 1.0;
    }
    let mut near = NO_TRIANGLE;
    let mut closest = f32::INFINITY;
    for b in 0..8 {
        let t = field.nearest[at(b)];
        let d = field.value[at(b)].abs();
        if t != NO_TRIANGLE && d < closest {
            closest = d;
            near = t;
        }
    }
    Some((sum / count, near))
}
