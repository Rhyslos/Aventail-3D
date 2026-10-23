//! The shapes as Manifold sees them: closed solids made of triangles,
//! each triangle tagged with the polygon it came from.

use crate::{Cutter, Kind, Problem};
use glam::Mat4;
use manifold_rust::manifold::Manifold;
use manifold_rust::types::{Error, MeshGL64};
use mesh::{FaceId, Mesh, VertexId};
use std::collections::HashMap;

/// Every shape in one mesh (the kept shape first, then each cutter, in
/// the kept shape's space), and where each shape's polygons are in it.
pub(crate) struct Joined {
    /// The shapes, one after another, with all their values.
    pub(crate) mesh: Mesh,
    /// Where each shape's polygons end: shape `i` has the polygon ids
    /// from `ends[i - 1]` (0 for the first) up to `ends[i]`.
    ends: Vec<u32>,
}

impl Joined {
    /// The kept shape and the cutters, joined.
    pub(crate) fn new(kept: &Mesh, cutters: &[Cutter]) -> Self {
        let mut mesh = Mesh::new();
        let mut ends = Vec::with_capacity(cutters.len() + 1);
        mesh::parts::append(&mut mesh, kept, Mat4::IDENTITY);
        ends.push(slot_count(&mesh));
        for cutter in cutters {
            mesh::parts::append(&mut mesh, cutter.mesh, cutter.matrix);
            ends.push(slot_count(&mesh));
        }
        Self { mesh, ends }
    }

    /// Which shape polygon `face` belongs to: 0 for the kept shape, 1 for
    /// the first cutter, and so on.
    pub(crate) fn part_of(&self, face: FaceId) -> usize {
        self.ends.partition_point(|end| *end <= face.get())
    }

    /// Each shape as a Manifold solid, or why one isn't a closed solid.
    pub(crate) fn solids(&self) -> Result<Vec<Manifold>, Problem> {
        if self.mesh.vertices().any(|(_, p)| !p.is_finite()) {
            return Err(Problem::NotANumber);
        }
        let mut parts: Vec<MeshGL64> = vec![triangles(); self.ends.len()];
        let mut points: Vec<HashMap<VertexId, u64>> = vec![HashMap::new(); self.ends.len()];
        for (face, corners) in self.mesh.faces() {
            let part = self.part_of(face);
            let (Some(gl), Some(index)) = (parts.get_mut(part), points.get_mut(part)) else {
                continue;
            };
            let mut local = Vec::with_capacity(corners.len());
            for v in corners {
                let next = index.len() as u64;
                let at = *index.entry(*v).or_insert_with(|| {
                    let p = self.mesh.position(*v).unwrap_or_default();
                    gl.vert_properties
                        .extend([f64::from(p.x), f64::from(p.y), f64::from(p.z)]);
                    next
                });
                local.push(at);
            }
            mesh::face_triangles(&self.mesh, corners, |[a, b, c]| {
                gl.tri_verts.extend([local[a], local[b], local[c]]);
                gl.face_id.push(u64::from(face.get()));
            });
        }
        let mut solids = Vec::with_capacity(parts.len());
        for (i, mut gl) in parts.into_iter().enumerate() {
            // Points at the same place along open edges are one point:
            // shapes made of separate pieces that meet still close up.
            gl.merge();
            let solid = std::panic::catch_unwind(|| Manifold::from_mesh_gl64(&gl))
                .map_err(|_| Problem::Failed("the boolean library stopped".to_string()))?;
            match solid.status() {
                Error::NoError => solids.push(solid),
                Error::NotManifold if i == 0 => return Err(Problem::KeptNotClosed),
                Error::NotManifold => return Err(Problem::CutterNotClosed(i - 1)),
                Error::NonFiniteVertex => return Err(Problem::NotANumber),
                other => return Err(Problem::Failed(format!("{other:?}"))),
            }
        }
        Ok(solids)
    }
}

/// How many polygon ids `mesh` has handed out.
fn slot_count(mesh: &Mesh) -> u32 {
    u32::try_from(mesh.face_slots().len()).unwrap_or(u32::MAX)
}

/// An empty triangle mesh with positions only.
fn triangles() -> MeshGL64 {
    MeshGL64 {
        num_prop: 3,
        ..MeshGL64::default()
    }
}

/// The kept solid (first of `solids`) with each cutter applied in turn,
/// as triangles tagged with the polygons they came from.
pub(crate) fn combine(solids: &[Manifold], cutters: &[Cutter]) -> Result<MeshGL64, Problem> {
    let Some((first, rest)) = solids.split_first() else {
        return Ok(triangles());
    };
    let mut result = first.clone();
    for (solid, cutter) in rest.iter().zip(cutters) {
        result = match cutter.kind {
            Kind::Union => result.union(solid),
            Kind::Subtract => result.difference(solid),
            Kind::Intersect => result.intersection(solid),
        };
        if result.status() != Error::NoError {
            return Err(Problem::Failed(format!("{:?}", result.status())));
        }
    }
    Ok(result.get_mesh_gl64(-1))
}
