//! A saved UV layout ([decision 0114](../../../../docs/decisions/0114-layout-snapshots.md)):
//! every polygon's corner UVs and the seams, kept to put back later.
//!
//! A layout snapshot (in the `scene` crate) holds one of these per object.
//! Putting it back sets the UVs of the polygons it knows, in the order
//! their corners had, and the seams between them. Polygons made since,
//! or whose number of corners changed, are left as they are.

use super::{Uvs, is_seam, set_seams, set_uvs};
use crate::{EdgeKey, FaceId, Mesh};
use glam::Vec2;
use std::collections::BTreeSet;

/// A mesh's UVs and seams, as they were.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UvLayout {
    /// Each polygon's corner UVs, in its corners' order, polygons in id
    /// order.
    pub faces: Vec<(FaceId, Vec<Vec2>)>,
    /// The edges marked as seams, in order.
    pub seams: Vec<EdgeKey>,
}

impl UvLayout {
    /// `mesh`'s layout, or `None` if it has no UVs.
    pub fn of(mesh: &Mesh) -> Option<Self> {
        let uvs = Uvs::of(mesh)?;
        let (starts, _) = mesh.face_layout();
        let faces = mesh
            .faces()
            .map(|(f, list)| {
                let start = starts[f.get() as usize] as usize;
                (f, (0..list.len()).map(|k| uvs.at(start + k)).collect())
            })
            .collect();
        let mut seams = super::seams(mesh);
        seams.sort();
        Some(Self { faces, seams })
    }

    /// Whether every UV is a real number (a file may hold others).
    pub fn is_real(&self) -> bool {
        self.faces
            .iter()
            .all(|(_, uvs)| uvs.iter().all(|uv| uv.is_finite()))
    }

    /// How many polygons it holds.
    pub fn polygon_count(&self) -> usize {
        self.faces.len()
    }

    /// Roughly how much memory it takes, in bytes.
    pub fn memory_size(&self) -> usize {
        self.faces
            .iter()
            .map(|(_, uvs)| size_of::<(FaceId, Vec<Vec2>)>() + size_of_val(uvs.as_slice()))
            .sum::<usize>()
            + size_of_val(self.seams.as_slice())
    }
}

/// Puts `layout` back on `mesh` (decision 0114): each polygon it knows
/// gets its corners' UVs back, and every edge between such polygons is a
/// seam exactly if it was one. Polygons not in the mesh are missing;
/// polygons whose number of corners changed, and an unreal layout, are
/// skipped. Other polygons keep their UVs, and their edges their marks.
pub fn restore_layout(mesh: &mut Mesh, layout: &UvLayout) -> crate::edit::Outcome {
    let missing = layout
        .faces
        .iter()
        .filter(|(f, _)| mesh.face(*f).is_none())
        .count();
    let mut outcome = crate::edit::Outcome {
        matched: 0,
        missing,
        skipped: layout.faces.len() - missing,
    };
    if !layout.is_real() {
        return outcome;
    }
    let (starts, corners) = mesh.face_layout();
    let count = corners.len();
    let starts = starts.to_vec();
    let mut values =
        Uvs::of(mesh).map_or_else(|| vec![Vec2::ZERO; count], |u| u.into_values(count));
    let mut known = BTreeSet::new();
    for (f, uvs) in &layout.faces {
        let Some(n) = mesh.face(*f).map(<[crate::VertexId]>::len) else {
            continue;
        };
        if n != uvs.len() {
            continue;
        }
        let start = starts[f.get() as usize] as usize;
        values[start..start + n].copy_from_slice(uvs);
        known.insert(*f);
    }
    if known.is_empty() || !set_uvs(mesh, values) {
        return outcome;
    }
    // Seams between the polygons put back.
    let wanted: BTreeSet<EdgeKey> = layout.seams.iter().copied().collect();
    let topology = mesh.topology().clone();
    let (mut mark, mut clear) = (Vec::new(), Vec::new());
    for edge in topology.edges() {
        let mut faces = topology.edge_faces(edge).peekable();
        if faces.peek().is_none() || !faces.all(|f| known.contains(&f)) {
            continue;
        }
        match (wanted.contains(&edge), is_seam(mesh, edge)) {
            (true, false) => mark.push(edge),
            (false, true) => clear.push(edge),
            _ => {}
        }
    }
    set_seams(mesh, &mark, true);
    set_seams(mesh, &clear, false);
    outcome.matched = known.len();
    outcome.skipped = layout.faces.len() - missing - known.len();
    outcome
}

#[cfg(test)]
mod tests;
