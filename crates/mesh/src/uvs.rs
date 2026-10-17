//! UV coordinates and seams on a mesh ([decision 0095](../../../docs/decisions/0095-uvs-before-advanced-modelling.md)):
//! reading the [`UV`] corner layer, telling its islands apart, and the
//! [`SEAM`] edge layer.
//!
//! A **UV** is a point on the flat texture square (0 to 1 across and up)
//! for each polygon corner: a point shared by four polygons has four
//! corners, which usually share one UV and can each have their own where
//! the surface is cut open. An **island** is a piece of the surface laid
//! flat in one piece: polygons joined across edges whose two ends have the
//! same UVs on both sides. A **seam** is an edge marked to be cut open when
//! the surface is unwrapped; the UV tools keep seams and the splits in the
//! UVs in step.
//!
//! The tools that make UVs (unwrapping, relaxing, packing) are in the `uv`
//! crate; this module is what every part of the app needs to read them.

use crate::attributes::{AttributeValues, Domain, SEAM, UV, Value};
use crate::{AttributeElement, EdgeKey, FaceId, Mesh};
use glam::Vec2;

/// How close two UVs must be to count as the same point of the texture.
pub const SAME_UV: f32 = 1e-5;

/// A mesh's UVs, read once for many lookups: one per corner, in the
/// mesh's corner order.
#[derive(Debug, Clone)]
pub struct Uvs {
    values: Vec<Vec2>,
    default: Vec2,
}

impl Uvs {
    /// The UVs of `mesh`, or `None` if it has no UV layer.
    pub fn of(mesh: &Mesh) -> Option<Self> {
        let (domain, default, values) = mesh.attribute_values(UV)?;
        if domain != Domain::Corner {
            return None;
        }
        let (Value::Vec2(default), AttributeValues::Vec2(values)) = (default, values) else {
            return None;
        };
        Some(Self { values, default })
    }

    /// The UV at corner `index` of the mesh's corner list.
    pub fn at(&self, index: usize) -> Vec2 {
        self.values.get(index).copied().unwrap_or(self.default)
    }

    /// The UV of polygon `face`'s corner `k`.
    pub fn corner(&self, mesh: &Mesh, face: FaceId, k: usize) -> Option<Vec2> {
        let (starts, _) = mesh.face_layout();
        let start = *starts.get(face.get() as usize)? as usize;
        let end = *starts.get(face.get() as usize + 1)? as usize;
        (start + k < end).then(|| self.at(start + k))
    }
}

/// Whether `mesh` has UVs.
pub fn has_uvs(mesh: &Mesh) -> bool {
    mesh.attributes()
        .any(|(name, domain, _)| name == UV && domain == Domain::Corner)
}

/// The UV of `face`'s corner at point slot `k`, if the mesh has UVs.
pub fn corner_uv(mesh: &Mesh, face: FaceId, k: usize) -> Option<Vec2> {
    match mesh.attribute(UV, AttributeElement::Corner(face, k))? {
        Value::Vec2(uv) => Some(uv),
        _ => None,
    }
}

/// Whether edge `edge` is marked as a seam.
pub fn is_seam(mesh: &Mesh, edge: EdgeKey) -> bool {
    matches!(
        mesh.attribute(SEAM, AttributeElement::Edge(edge)),
        Some(Value::Bool(true))
    )
}

/// Every edge marked as a seam.
pub fn seams(mesh: &Mesh) -> Vec<EdgeKey> {
    match mesh.attribute_values(SEAM) {
        Some((Domain::Edge, _, AttributeValues::Edges(list))) => list
            .into_iter()
            .filter(|(_, v)| *v == Value::Bool(true))
            .map(|(e, _)| e)
            .collect(),
        _ => Vec::new(),
    }
}

/// Where, in the mesh's corner list, polygon `face`'s corner at point
/// `v` is.
fn corner_at(mesh: &Mesh, face: FaceId, v: crate::VertexId) -> Option<usize> {
    let (starts, corners) = mesh.face_layout();
    let start = *starts.get(face.get() as usize)? as usize;
    let end = *starts.get(face.get() as usize + 1)? as usize;
    (start..end).find(|i| corners[*i] == v)
}

/// Whether the UVs of `edge` are the same on the two polygons `f` and `g`
/// that share it: the surface is joined there in the texture.
fn joined(mesh: &Mesh, uvs: &Uvs, edge: EdgeKey, f: FaceId, g: FaceId) -> bool {
    let (a, b) = edge.ends();
    [a, b]
        .iter()
        .all(|v| match (corner_at(mesh, f, *v), corner_at(mesh, g, *v)) {
            (Some(i), Some(j)) => uvs.at(i).distance(uvs.at(j)) <= SAME_UV,
            _ => false,
        })
}

/// Edges where the UVs split: used by two polygons whose UVs differ at
/// either end. These are the edges cut open in the texture.
pub fn uv_splits(mesh: &Mesh) -> Vec<EdgeKey> {
    let Some(uvs) = Uvs::of(mesh) else {
        return Vec::new();
    };
    let topology = mesh.topology();
    topology
        .edges()
        .filter(|e| {
            let faces: Vec<FaceId> = topology.edge_faces(*e).collect();
            faces.len() == 2 && !joined(mesh, &uvs, *e, faces[0], faces[1])
        })
        .collect()
}

/// Marks every edge where the UVs split as a seam (and no other), adding
/// the seam layer if needed: what a shape made with its own UVs (a
/// primitive) starts with.
pub fn seams_from_splits(mesh: &mut Mesh) {
    let splits = uv_splits(mesh);
    mesh.remove_attribute(SEAM);
    if mesh
        .add_attribute(SEAM, Domain::Edge, Value::Bool(false))
        .is_err()
    {
        return;
    }
    for edge in splits {
        let _ = mesh.set_attribute(SEAM, AttributeElement::Edge(edge), Value::Bool(true));
    }
}

/// The UV islands: groups of polygons joined in the texture (sharing an
/// edge with the same UVs on both sides), each in id order, the groups in
/// order of their first polygon. Empty without UVs.
pub fn islands(mesh: &Mesh) -> Vec<Vec<FaceId>> {
    let Some(uvs) = Uvs::of(mesh) else {
        return Vec::new();
    };
    let slots = mesh.face_layout().0.len().saturating_sub(1);
    let mut parent: Vec<usize> = (0..slots).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    let topology = mesh.topology();
    for edge in topology.edges() {
        let faces: Vec<FaceId> = topology.edge_faces(edge).collect();
        if faces.len() != 2 || !joined(mesh, &uvs, edge, faces[0], faces[1]) {
            continue;
        }
        let (a, b) = (
            root(&mut parent, faces[0].get() as usize),
            root(&mut parent, faces[1].get() as usize),
        );
        if a != b {
            parent[a.max(b)] = a.min(b);
        }
    }
    let mut groups: std::collections::BTreeMap<usize, Vec<FaceId>> = Default::default();
    for (f, _) in mesh.faces() {
        let r = root(&mut parent, f.get() as usize);
        groups.entry(r).or_default().push(f);
    }
    groups.into_values().collect()
}

/// The island each polygon is in, by polygon slot (`usize::MAX` for an
/// empty slot), and how many islands there are.
pub fn island_of_each(mesh: &Mesh) -> (Vec<usize>, usize) {
    let groups = islands(mesh);
    let slots = mesh.face_layout().0.len().saturating_sub(1);
    let mut of = vec![usize::MAX; slots];
    for (i, group) in groups.iter().enumerate() {
        for f in group {
            of[f.get() as usize] = i;
        }
    }
    (of, groups.len())
}

#[cfg(test)]
mod tests;
