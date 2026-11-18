//! Surface nets: the new surface between a remesh grid's inside and
//! outside points, then evened out (and, keeping detail, laid back on
//! the shape's surface) and made into a mesh with materials and
//! box-projected UVs. Each step works block by block on every core.

use super::field::NO_TRIANGLE;
use super::sparse::{BLOCK, Cube, SIDE, Sparse, cube_at, local};
use super::{Progress, RemeshProblem, Soup};
use crate::attributes::{AttributeValues, Domain, UV, Value};
use crate::bvh::closest_on_triangle;
use crate::material::MATERIAL;
use crate::parallel::{in_parallel, in_parallel_from};
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

/// Blocks each thread takes at least.
const BLOCKS_PER_THREAD: usize = 64;

/// A keeping-detail point is laid back on the shape's surface only when
/// it is at most this many grid steps from it.
const DETAIL_REACH: f32 = 2.0;

/// The new surface, before it is a mesh.
pub(super) struct Surface {
    pub(super) points: Vec<Vec3>,
    /// The original triangle nearest each point ([`NO_TRIANGLE`] if none).
    nearest: Vec<u32>,
    /// The grid cell each point is in (by its lowest corner).
    cells: Vec<[u32; 3]>,
    /// Quads of points, facing out of the shape.
    pub(super) quads: Vec<[u32; 4]>,
}

/// A point found in a cell: the cell's place in its block, where the
/// point is, the triangle nearest it and the cell.
type Found = (u16, Vec3, u32, [u32; 3]);

impl Surface {
    /// The surface through the cells where `field` changes side.
    pub(super) fn extract(field: &Sparse, progress: &Progress) -> Result<Self, RemeshProblem> {
        let [nx, ny, nz] = field.grid.size;
        let mut surface = Self {
            points: Vec::new(),
            nearest: Vec::new(),
            cells: Vec::new(),
            quads: Vec::new(),
        };
        if nx < 2 || ny < 2 || nz < 2 {
            return Ok(surface);
        }
        let slots = field.blocks.len();
        let found: Vec<Vec<(usize, Vec<Found>)>> =
            in_parallel_from(slots, BLOCKS_PER_THREAD, |range| {
                range
                    .map(|s| {
                        let mut here = Vec::new();
                        let start = field.slot_start(s);
                        let cube = field.cube(s);
                        field.each_cell(start, |i, j, k| {
                            let at = [i - start[0], j - start[1], k - start[2]];
                            if let Some((p, near)) = cell_point_at(field, &cube, start, at) {
                                let place = local(at[0], at[1], at[2]);
                                here.push((place as u16, p, near, [i, j, k].map(|c| c as u32)));
                            }
                        });
                        (s, here)
                    })
                    .collect()
            });
        progress.check()?;
        let mut cell_point = vec![NONE; slots * BLOCK];
        for (s, here) in found.into_iter().flatten() {
            for (at, p, near, cell) in here {
                cell_point[s * BLOCK + at as usize] =
                    u32::try_from(surface.points.len()).map_err(|_| RemeshProblem::TooBig)?;
                surface.points.push(p);
                surface.nearest.push(near);
                surface.cells.push(cell);
            }
        }
        let point_of = |c: [usize; 3]| {
            field
                .slot(c[0], c[1], c[2])
                .map_or(NONE, |s| cell_point[s * BLOCK + local(c[0] % SIDE, c[1] % SIDE, c[2] % SIDE)])
        };
        let quads = in_parallel_from(slots, BLOCKS_PER_THREAD, |range| {
            let mut quads = Vec::new();
            for s in range {
                let start = field.slot_start(s);
                let cube = field.cube(s);
                for k in start[2]..(start[2] + SIDE).min(nz) {
                    for j in start[1]..(start[1] + SIDE).min(ny) {
                        for i in start[0]..(start[0] + SIDE).min(nx) {
                            let at = [i - start[0], j - start[1], k - start[2]];
                            quads_at(field, &cube, [i, j, k], at, &point_of, &mut quads);
                        }
                    }
                }
            }
            quads
        });
        surface.quads = quads.into_iter().flatten().collect();
        Ok(surface)
    }

    /// Evens the surface out with `passes` relax passes, each moving
    /// every point towards its neighbours' middle and back out a little,
    /// so it doesn't shrink.
    pub(super) fn relax(&mut self, passes: u32, progress: &Progress) -> Result<(), RemeshProblem> {
        if passes == 0 || self.points.is_empty() {
            return Ok(());
        }
        // Each point's neighbours, packed: a quad's edge from `a` to `b`
        // makes `b` a neighbour of `a`; the quad on the edge's other side
        // goes the other way round, making `a` one of `b`'s.
        let count = self.points.len();
        let mut starts = vec![0u32; count + 1];
        for quad in &self.quads {
            for &a in quad {
                starts[a as usize + 1] += 1;
            }
        }
        for i in 1..starts.len() {
            starts[i] += starts[i - 1];
        }
        let mut filled = starts.clone();
        let mut neighbours = vec![0u32; *starts.last().unwrap_or(&0) as usize];
        for quad in &self.quads {
            for n in 0..4 {
                let (a, b) = (quad[n] as usize, quad[(n + 1) % 4]);
                neighbours[filled[a] as usize] = b;
                filled[a] += 1;
            }
        }
        for _ in 0..passes {
            for factor in [RELAX_IN, RELAX_OUT] {
                progress.check()?;
                let points = &self.points;
                let moved = in_parallel(count, |range| {
                    range
                        .map(|p| {
                            let list = &neighbours[starts[p] as usize..starts[p + 1] as usize];
                            if list.is_empty() {
                                return points[p];
                            }
                            let middle = list.iter().map(|n| points[*n as usize]).sum::<Vec3>()
                                / list.len() as f32;
                            points[p] + (middle - points[p]) * factor
                        })
                        .collect::<Vec<Vec3>>()
                });
                self.points = moved.into_iter().flatten().collect();
            }
        }
        Ok(())
    }

    /// Lays each point back on the shape's surface (the nearest place on
    /// the triangles nearest its cell's corners), so detail the voxels
    /// can hold isn't smoothed away. Points farther than
    /// [`DETAIL_REACH`] steps stay.
    pub(super) fn keep_detail(&mut self, field: &Sparse, soup: &Soup) {
        let reach = DETAIL_REACH * field.grid.step;
        let (points, cells) = (&self.points, &self.cells);
        let laid = in_parallel(points.len(), |range| {
            range
                .map(|p| {
                    let [i, j, k] = cells[p].map(|c| c as usize);
                    let at = points[p];
                    let mut best = (reach * reach, at);
                    let mut seen = [NO_TRIANGLE; 8];
                    for b in 0..8 {
                        let t = field.nearest(i + (b & 1), j + ((b >> 1) & 1), k + (b >> 2));
                        if t == NO_TRIANGLE || seen.contains(&t) {
                            continue;
                        }
                        seen[b] = t;
                        let [a, bb, c] = soup.triangles[t as usize];
                        let q = closest_on_triangle(at, a, bb, c);
                        let d = q.distance_squared(at);
                        if d < best.0 {
                            best = (d, q);
                        }
                    }
                    best.1
                })
                .collect::<Vec<Vec3>>()
        });
        self.points = laid.into_iter().flatten().collect();
    }

    /// The label each quad takes from `labels` (one per triangle of the
    /// shape): the one most of its corners' nearest triangles have (the
    /// lowest of equals).
    pub(super) fn labels(&self, labels: &[u32]) -> Vec<u32> {
        self.quads
            .iter()
            .map(|quad| self.most_common(quad, labels))
            .collect()
    }

    /// The surface as a mesh: each polygon with the material of the
    /// original polygons nearest its corners (the most common), and UVs
    /// by box projection with seams where they part.
    pub(super) fn into_mesh(self, soup: &Soup, uvs: bool) -> Result<Mesh, RemeshProblem> {
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
                .map(|quad| self.most_common(quad, &soup.slots) as i32)
                .collect();
            mesh.set_attribute_values(
                MATERIAL,
                Domain::Face,
                Value::Int(0),
                AttributeValues::Int(slots),
            )
            .map_err(|_| RemeshProblem::TooBig)?;
        }
        if !uvs {
            return Ok(mesh);
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

    /// The value of `per_triangle` most of `quad`'s corners' nearest
    /// triangles have (the lowest of equals).
    fn most_common(&self, quad: &[u32; 4], per_triangle: &[u32]) -> u32 {
        let slots: Vec<u32> = quad
            .iter()
            .filter_map(|p| {
                let t = *self.nearest.get(*p as usize)?;
                (t != NO_TRIANGLE)
                    .then(|| per_triangle.get(t as usize).copied())
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

/// The surface's point in the cell whose lowest corner is point `at` of
/// `cube` (the block starting at grid point `start`), if the surface
/// passes through it: the middle of where it crosses the cell's edges,
/// with the original triangle nearest the cell.
fn cell_point_at(
    field: &Sparse,
    cube: &Cube,
    start: [usize; 3],
    at: [usize; 3],
) -> Option<(Vec3, u32)> {
    let corner = |b: usize| [at[0] + (b & 1), at[1] + ((b >> 1) & 1), at[2] + (b >> 2)];
    let place = |b: usize| {
        let [x, y, z] = corner(b);
        cube_at(x, y, z)
    };
    let values: [f32; 8] = std::array::from_fn(|b| cube.values[place(b)]);
    let first = values[0].is_sign_negative();
    if values.iter().all(|v| v.is_sign_negative() == first) {
        return None;
    }
    let grid = &field.grid;
    let point = |b: usize| {
        let [x, y, z] = corner(b);
        grid.point(start[0] + x, start[1] + y, start[2] + z)
    };
    let mut sum = Vec3::ZERO;
    let mut count = 0.0;
    for (a, b) in EDGES {
        let (va, vb) = (values[a], values[b]);
        if va.is_sign_negative() == vb.is_sign_negative() {
            continue;
        }
        let gap = va - vb;
        let t = if gap.abs() > f32::MIN_POSITIVE {
            (va / gap).clamp(0.0, 1.0)
        } else {
            0.5
        };
        sum += point(a).lerp(point(b), t);
        count += 1.0;
    }
    let mut near = NO_TRIANGLE;
    let mut closest = f32::INFINITY;
    for (b, value) in values.iter().enumerate() {
        let t = cube.nearest[place(b)];
        let d = value.abs();
        if t != NO_TRIANGLE && d < closest {
            closest = d;
            near = t;
        }
    }
    Some((sum / count, near))
}

/// Adds the quads across the grid edges from point `at` (point `local`
/// of `cube`) towards +X, +Y and +Z that the surface crosses, each made of
/// the points of the four cells round the edge (`point_of` a cell).
fn quads_at(
    field: &Sparse,
    cube: &Cube,
    at: [usize; 3],
    local: [usize; 3],
    point_of: &impl Fn([usize; 3]) -> u32,
    quads: &mut Vec<[u32; 4]>,
) {
    let grid = &field.grid;
    let here = cube.inside(local[0], local[1], local[2]);
    for axis in 0..3 {
        let mut next = local;
        next[axis] += 1;
        if at[axis] + 1 >= grid.size[axis] || cube.inside(next[0], next[1], next[2]) == here {
            continue;
        }
        // The other two axes, in the order whose cross product is this
        // axis (y × z = x, z × x = y, x × y = z).
        let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
        if at[u] == 0 || at[v] == 0 || at[u] >= grid.size[u] - 1 || at[v] >= grid.size[v] - 1 {
            continue;
        }
        let corner = |du: usize, dv: usize| {
            let mut c = at;
            c[u] = c[u] + du - 1;
            c[v] = c[v] + dv - 1;
            point_of(c)
        };
        // Round the edge anticlockwise seen from its +axis end: facing
        // +axis.
        let mut quad = [corner(0, 0), corner(1, 0), corner(1, 1), corner(0, 1)];
        if quad.contains(&NONE) {
            continue;
        }
        // Inside here, outside past it: the surface faces along +axis;
        // otherwise the other way.
        if !here {
            quad.reverse();
        }
        quads.push(quad);
    }
}
