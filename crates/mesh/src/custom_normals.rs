//! Custom normals: a shading direction set by hand (or by the Weighted
//! normals modifier) for each polygon corner, instead of the one worked
//! out from the hard edges and the object's Smoothing
//! ([decision 0148](../../../docs/decisions/0148-weighted-normals.md)).
//!
//! They are a corner layer, [`CUSTOM_NORMAL`]: three numbers per polygon
//! corner. A corner holding a direction shades with it; a corner holding
//! nothing (the layer's standard, no length) shades as usual. So
//! [`crate::corner_shading_with`] (and through it the viewport, exports,
//! MikkTSpace tangents and the hard edges the bake check sees) uses them
//! wherever they are set.
//!
//! A custom normal belongs to the shape it was worked out for. Once points
//! move, polygons change or hard edges change, it no longer fits, so it is
//! dropped and the shading goes back to being worked out: by the recorded
//! edits ([`shading_unchanged`] tells them), the live mirror and booleans,
//! and every modifier after the one that made them.

use crate::attributes::{Domain, Value};
use crate::{EdgeKey, Mesh};
use glam::Vec3;

/// The name of the corner layer holding custom normals.
pub const CUSTOM_NORMAL: &str = "custom_normal";

/// Whether `mesh` has a custom normal layer (of the right domain and
/// kind).
pub fn has_custom_normals(mesh: &Mesh) -> bool {
    mesh.attributes().any(|(name, domain, kind)| {
        name == CUSTOM_NORMAL
            && domain == Domain::Corner
            && kind == crate::attributes::AttributeKind::Vec3
    })
}

/// Each corner's custom normal, in the order the mesh lists its corners
/// (face by face), unit length, or `None` where none is set (no length,
/// or a value that isn't a real number). `None` for a mesh without
/// custom normals.
pub fn custom_normal_list(mesh: &Mesh) -> Option<Vec<Option<Vec3>>> {
    if !has_custom_normals(mesh) {
        return None;
    }
    let (_, values) = mesh
        .attribute_values(CUSTOM_NORMAL)
        .map(|(_, d, v)| (d, v))?;
    let stored = match values {
        crate::AttributeValues::Vec3(list) => list,
        _ => return None,
    };
    let (_, corners) = mesh.face_layout();
    Some(
        (0..corners.len())
            .map(|k| stored.get(k).copied().and_then(usable))
            .collect(),
    )
}

/// A stored value as a direction, if it is one.
fn usable(v: Vec3) -> Option<Vec3> {
    if v.is_finite() && v.length_squared() > 1e-12 {
        v.try_normalize()
    } else {
        None
    }
}

/// Sets every corner's custom normal from `normals` (in the mesh's corner
/// order; extra values are ignored, missing ones left unset). Values that
/// aren't real directions are left unset.
pub fn set_custom_normals(mesh: &mut Mesh, normals: &[Vec3]) {
    let (_, corners) = mesh.face_layout();
    let list: Vec<Vec3> = (0..corners.len())
        .map(|k| {
            normals
                .get(k)
                .copied()
                .and_then(usable)
                .unwrap_or(Vec3::ZERO)
        })
        .collect();
    let _ = mesh.set_attribute_values(
        CUSTOM_NORMAL,
        Domain::Corner,
        Value::Vec3(Vec3::ZERO),
        crate::AttributeValues::Vec3(list),
    );
}

/// Removes the custom normals, if any: the shading is worked out again.
/// Returns whether there were any.
pub fn forget_custom_normals(mesh: &mut Mesh) -> bool {
    has_custom_normals(mesh) && mesh.remove_attribute(CUSTOM_NORMAL)
}

/// Whether `after` shades the same as `before` would: the same points in
/// the same places, the same polygons and the same hard edges. Other
/// values (UVs, seams, materials) may differ. Edits that keep this keep a
/// shape's custom normals; any other edit drops them.
pub fn shading_unchanged(before: &Mesh, after: &Mesh) -> bool {
    before.face_layout() == after.face_layout()
        && before.point_slots().eq(after.point_slots())
        && before.hard_edges().eq(after.hard_edges())
}

/// The edges where the custom normals break the shading: an edge between
/// two polygons whose corners at either end point different ways, and an
/// edge shared by three or more polygons. `None` for a mesh without
/// custom normals. Open borders are never listed.
pub fn split_edges(mesh: &Mesh) -> Option<Vec<EdgeKey>> {
    if !has_custom_normals(mesh) {
        return None;
    }
    let (_, normals) = crate::corner_shading(mesh);
    let topology = mesh.topology();
    let (starts, _) = mesh.face_layout();
    let corner_of = |face: crate::FaceId, v: crate::VertexId| -> Option<Vec3> {
        let k = mesh.face(face)?.iter().position(|c| *c == v)?;
        normals
            .get(*starts.get(face.get() as usize)? as usize + k)
            .copied()
    };
    let mut out: Vec<EdgeKey> = topology
        .edges()
        .filter(|e| match topology.edge_uses(*e) {
            [] | [_] => false,
            [one, two] => {
                let (a, b) = e.ends();
                [a, b].iter().any(
                    |v| match (corner_of(one.face, *v), corner_of(two.face, *v)) {
                        (Some(n), Some(m)) => n.dot(m) < 1.0 - 1e-5,
                        _ => false,
                    },
                )
            }
            _ => true,
        })
        .collect();
    out.sort();
    Some(out)
}

/// A custom normal turned by `matrix`, as [`crate::parts::append`] places
/// a part: directions turn with the inverse transpose, so they stay
/// square to a stretched surface. No direction stays none.
pub(crate) fn placed(normal: Vec3, matrix: glam::Mat4) -> Vec3 {
    if normal.length_squared() <= 1e-12 {
        return Vec3::ZERO;
    }
    let turn = matrix.inverse().transpose();
    turn.transform_vector3(normal)
        .try_normalize()
        .unwrap_or(Vec3::ZERO)
}

#[cfg(test)]
mod tests;
