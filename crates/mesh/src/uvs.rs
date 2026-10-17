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
use crate::{AttributeElement, EdgeKey, FaceId, Mesh, VertexId};
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

    /// Every corner's UV, in the mesh's corner order (as many as the mesh
    /// has corners).
    pub fn into_values(mut self, corners: usize) -> Vec<Vec2> {
        self.values.resize(corners, self.default);
        self.values
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
fn corner_at(mesh: &Mesh, face: FaceId, v: VertexId) -> Option<usize> {
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

/// The texel density of the whole mesh, as a length ratio: texture widths
/// per unit of length on the shape (the square root of its UV area over
/// its real area), if it has UVs and some area.
pub fn average_density(mesh: &Mesh) -> Option<f32> {
    let uvs = Uvs::of(mesh)?;
    let (starts, _) = mesh.face_layout();
    let mut flat = 0.0f64;
    let mut real = 0.0f64;
    for (f, list) in mesh.faces() {
        let start = starts[f.get() as usize] as usize;
        crate::triangulate::face_triangles(mesh, list, |[a, b, c]| {
            let (ua, ub, uc) = (uvs.at(start + a), uvs.at(start + b), uvs.at(start + c));
            flat += f64::from(((ub - ua).perp_dot(uc - ua) / 2.0).abs());
            let p = |i: usize| mesh.position(list[i]).unwrap_or_default();
            real += f64::from((p(b) - p(a)).cross(p(c) - p(a)).length() / 2.0);
        });
    }
    (flat > 1e-20 && real > 1e-20).then(|| (flat / real).sqrt() as f32)
}

/// Sets every corner's UV at once (in the mesh's corner order), adding
/// the UV layer if needed. Returns whether it was set: the list must have
/// one real UV per corner.
pub fn set_uvs(mesh: &mut Mesh, values: Vec<Vec2>) -> bool {
    if values.len() != mesh.face_layout().1.len() || !values.iter().all(|v| v.is_finite()) {
        return false;
    }
    mesh.set_attribute_values(
        UV,
        Domain::Corner,
        Value::Vec2(Vec2::ZERO),
        AttributeValues::Vec2(values),
    )
    .is_ok()
}

/// Marks `edges` as seams (or clears them, with `seam` false), adding
/// the seam layer if needed (decision 0098). Edges not in the mesh are
/// missing; edges already so, and open edges (an edge of a hole is cut
/// already), are skipped. Only the marks change: the UVs follow when
/// the shape is unwrapped.
pub fn set_seams(mesh: &mut Mesh, edges: &[EdgeKey], seam: bool) -> crate::edit::Outcome {
    let mut outcome = crate::edit::Outcome::default();
    if seam
        && mesh.attribute_values(SEAM).is_none()
        && mesh
            .add_attribute(SEAM, Domain::Edge, Value::Bool(false))
            .is_err()
    {
        outcome.skipped = edges.len();
        return outcome;
    }
    let topology = mesh.topology().clone();
    for edge in edges {
        if !topology.has_edge(*edge) {
            outcome.missing += 1;
        } else if topology.edge_uses(*edge).len() < 2 || is_seam(mesh, *edge) == seam {
            outcome.skipped += 1;
        } else if mesh
            .set_attribute(SEAM, AttributeElement::Edge(*edge), Value::Bool(seam))
            .is_ok()
        {
            outcome.matched += 1;
        } else {
            outcome.skipped += 1;
        }
    }
    outcome
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

/// The corners (places in the mesh's corner list) a change to the UVs of
/// `faces` and `points` moves: every corner of those polygons and points,
/// and every other corner of the same points sitting at the same spot in
/// the texture, so an island never tears where its polygons meet ("shared
/// location", decision 0097). Corners of a point in another island, at
/// another spot, are left alone.
pub fn corners_moved(mesh: &Mesh, uvs: &Uvs, faces: &[FaceId], points: &[VertexId]) -> Vec<usize> {
    let (starts, corners) = mesh.face_layout();
    let topology = mesh.topology();
    let mut chosen: std::collections::BTreeSet<usize> = Default::default();
    for face in faces {
        let i = face.get() as usize;
        if mesh.face(*face).is_none() {
            continue;
        }
        chosen.extend(starts[i] as usize..starts[i + 1] as usize);
    }
    for point in points {
        if mesh.position(*point).is_none() {
            continue;
        }
        for face in topology.vertex_faces(*point) {
            if let Some(k) = corner_at(mesh, *face, *point) {
                chosen.insert(k);
            }
        }
    }
    // The other corners at the same spot of the same points.
    let mut spots: std::collections::BTreeMap<VertexId, Vec<Vec2>> = Default::default();
    for k in &chosen {
        spots.entry(corners[*k]).or_default().push(uvs.at(*k));
    }
    let mut out = chosen.clone();
    for (point, at) in &spots {
        for face in topology.vertex_faces(*point) {
            if let Some(k) = corner_at(mesh, *face, *point)
                && at.iter().any(|uv| uv.distance(uvs.at(k)) <= SAME_UV)
            {
                out.insert(k);
            }
        }
    }
    out.into_iter().collect()
}

/// Moves, turns or scales the UVs of `faces` and `points` (and the
/// corners sharing their spots, see [`corners_moved`]) by `transform`, in
/// the texture's own units (the square is 0 to 1). Missing elements are
/// counted; a mesh without UVs, or a transform that isn't made of real
/// numbers, changes nothing.
pub fn transform_uvs(
    mesh: &mut Mesh,
    faces: &[FaceId],
    points: &[VertexId],
    transform: glam::Affine2,
) -> crate::edit::Outcome {
    let missing = faces.iter().filter(|f| mesh.face(**f).is_none()).count()
        + points
            .iter()
            .filter(|v| mesh.position(**v).is_none())
            .count();
    let asked = faces.len() + points.len();
    let unchanged = crate::edit::Outcome {
        matched: 0,
        missing,
        skipped: asked - missing,
    };
    let Some(uvs) = Uvs::of(mesh) else {
        return unchanged;
    };
    if !transform.is_finite() {
        return unchanged;
    }
    let moved = corners_moved(mesh, &uvs, faces, points);
    let count = mesh.face_layout().1.len();
    let mut values = uvs.into_values(count);
    for k in moved {
        values[k] = transform.transform_point2(values[k]);
    }
    let changed = mesh
        .set_attribute_values(
            UV,
            Domain::Corner,
            Value::Vec2(Vec2::ZERO),
            AttributeValues::Vec2(values),
        )
        .is_ok();
    crate::edit::Outcome {
        matched: if changed { asked - missing } else { 0 },
        missing,
        skipped: if changed { 0 } else { asked - missing },
    }
}

/// The box around the UVs of `faces` and `points` (and the corners
/// sharing their spots), as its lowest and highest corner, if they have
/// any.
pub fn uv_bounds(mesh: &Mesh, faces: &[FaceId], points: &[VertexId]) -> Option<(Vec2, Vec2)> {
    let uvs = Uvs::of(mesh)?;
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);
    for k in corners_moved(mesh, &uvs, faces, points) {
        min = min.min(uvs.at(k));
        max = max.max(uvs.at(k));
    }
    min.is_finite().then_some((min, max))
}

/// The middle of the UVs of `faces` and `points` (the middle of the box
/// around them), if they have any.
pub fn uv_centre(mesh: &Mesh, faces: &[FaceId], points: &[VertexId]) -> Option<Vec2> {
    uv_bounds(mesh, faces, points).map(|(min, max)| (min + max) / 2.0)
}

#[cfg(test)]
mod tests;
