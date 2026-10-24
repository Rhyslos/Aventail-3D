//! Joining rows of points into faces, laid flat as one UV piece: what
//! sweeps and lathes are made of.

use crate::polygon::VertexId;
use crate::primitives::uvs::{Chart, corners_of};
use crate::{FaceId, Mesh};
use glam::{Vec2, Vec3};

/// Rows of points to join into faces.
pub(super) struct Grid<'a> {
    /// The rows, in order. Each has `columns` points when the columns
    /// wrap round, `columns + 1` when they don't, or one (a pole, where a
    /// row shrinks to a point).
    pub(super) rows: &'a [Vec<VertexId>],
    /// How many faces across each row.
    pub(super) columns: usize,
    /// Whether the last row joins back to the first.
    pub(super) wrap_rows: bool,
    /// Where each column's edge lies across the UV piece (`columns + 1`
    /// values).
    pub(super) u: &'a [f32],
    /// Where each row lies along the UV piece (one per row, and one more
    /// for the first row again when the rows wrap).
    pub(super) v: &'a [f32],
    /// Whether the faces look the other way.
    pub(super) flip: bool,
}

/// The point `i` of a row (a pole answers every `i`; a ring wraps).
fn at(row: &[VertexId], i: usize) -> VertexId {
    row[i % row.len()]
}

/// Joins the grid's rows with faces (quads, or triangles at a pole) and
/// returns them as one UV piece, with the faces made.
pub(super) fn join(mesh: &mut Mesh, grid: &Grid) -> (Chart, Vec<FaceId>) {
    let mut chart = Chart::default();
    let mut faces = Vec::new();
    let count = grid.rows.len();
    let bands = if grid.wrap_rows {
        count
    } else {
        count.saturating_sub(1)
    };
    for j in 0..bands {
        let (lower, upper) = (&grid.rows[j], &grid.rows[(j + 1) % count]);
        if lower.is_empty() || upper.is_empty() {
            continue;
        }
        let (v0, v1) = (grid.v[j], grid.v[j + 1]);
        for i in 0..grid.columns {
            let corners = [
                (at(lower, i), Vec2::new(grid.u[i], v0)),
                (at(lower, i + 1), Vec2::new(grid.u[i + 1], v0)),
                (at(upper, i + 1), Vec2::new(grid.u[i + 1], v1)),
                (at(upper, i), Vec2::new(grid.u[i], v1)),
            ];
            // A pole appears twice in a row: keep it once, halfway.
            let mut face: Vec<(VertexId, Vec2)> = Vec::with_capacity(4);
            for corner in corners {
                match face.last_mut() {
                    Some(last) if last.0 == corner.0 => last.1 = (last.1 + corner.1) / 2.0,
                    _ => face.push(corner),
                }
            }
            if face.len() > 1 && face[0].0 == face[face.len() - 1].0 {
                let last = face.pop().map_or(Vec2::ZERO, |l| l.1);
                face[0].1 = (face[0].1 + last) / 2.0;
            }
            if face.len() < 3 {
                continue;
            }
            if grid.flip {
                face.reverse();
            }
            let ids: Vec<VertexId> = face.iter().map(|c| c.0).collect();
            let Ok(id) = mesh.add_face(&ids) else {
                continue;
            };
            for ((corner, _), (_, uv)) in corners_of(mesh, id).into_iter().zip(face) {
                chart.corners.push((corner, uv));
            }
            faces.push(id);
        }
    }
    (chart, faces)
}

/// The facing of a loop of points by Newell's method (not of unit
/// length; nothing for a loop with no area).
pub(super) fn facing(mesh: &Mesh, ring: &[VertexId]) -> Vec3 {
    let points: Vec<Vec3> = ring.iter().filter_map(|v| mesh.position(*v)).collect();
    let mut n = Vec3::ZERO;
    for i in 0..points.len() {
        let (a, b) = (points[i], points[(i + 1) % points.len()]);
        n += Vec3::new(
            (a.y - b.y) * (a.z + b.z),
            (a.z - b.z) * (a.x + b.x),
            (a.x - b.x) * (a.y + b.y),
        );
    }
    n
}

/// Closes `ring` with one flat face (an n-gon) looking towards `outward`,
/// its edges hard, laid flat on a UV piece of its own seen from the side
/// it faces. `None` if the ring can't make a face.
pub(super) fn cap(mesh: &mut Mesh, ring: &[VertexId], outward: Vec3) -> Option<(Chart, FaceId)> {
    let mut ring: Vec<VertexId> = ring.to_vec();
    ring.dedup();
    if ring.len() > 1 && ring.first() == ring.last() {
        ring.pop();
    }
    if ring.len() < 3 {
        return None;
    }
    let n = facing(mesh, &ring);
    if n.length_squared() < 1e-12 {
        return None;
    }
    if n.dot(outward) < 0.0 {
        ring.reverse();
    }
    let id = mesh.add_face(&ring).ok()?;
    for i in 0..ring.len() {
        mesh.set_hard(ring[i], ring[(i + 1) % ring.len()], true);
    }
    let normal = mesh.face_normal(id);
    let right = normal.any_orthonormal_vector();
    let up = normal.cross(right);
    let mut chart = Chart::default();
    for (corner, p) in corners_of(mesh, id) {
        chart
            .corners
            .push((corner, Vec2::new(p.dot(right), p.dot(up))));
    }
    Some((chart, id))
}
