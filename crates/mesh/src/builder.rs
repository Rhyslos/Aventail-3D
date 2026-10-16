//! The mesh builder: how tools make new points and polygons so their
//! attribute values come along ([decision 0043](../../../docs/decisions/0043-attribute-layers-and-the-builder.md)).
//!
//! A tool that adds or rewires geometry says **where each new element
//! comes from**, and the builder fills in every attribute layer the mesh
//! has, including ones the tool has never heard of (UVs, material slots,
//! vertex colours to come):
//!
//! - a new point is a copy of another ([`PointFrom::Copy`]), lies between
//!   two ([`PointFrom::Between`]) or lies inside a polygon
//!   ([`PointFrom::Inside`], blending its corners), and its point values
//!   follow;
//! - a new polygon is *like* an existing one, and its polygon values (a
//!   material slot, say) are copied;
//! - each new corner copies a corner of an original polygon
//!   ([`CornerFrom::Copy`]), blends two ([`CornerFrom::Between`]) or blends
//!   all of a polygon's at a spot inside it ([`CornerFrom::Inside`]), so
//!   UVs stretch across an extruded wall or a bevel instead of being lost;
//! - an edge that moves or is copied takes its edge values (hard, seam)
//!   with it ([`Builder::move_edge`], [`Builder::copy_edge`]).
//!
//! Points come from the mesh being built (so a new point can lie between
//! two other new ones); polygons, corners and edges come from the
//! original, as it was before the tool started.

use crate::attributes::{Domain, Value};
use crate::names::Origin;
use crate::{EdgeKey, FaceId, Mesh, MeshError, VertexId};
use glam::Vec3;
use std::collections::BTreeMap;

/// Where a new point's values come from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PointFrom {
    /// Nowhere: every layer's default.
    Nothing,
    /// A copy of this point.
    Copy(VertexId),
    /// `t` of the way from the first point to the second.
    Between(VertexId, VertexId, f32),
    /// At this spot inside the original polygon: its corners' points
    /// blended by how close the spot is to each (mean value weights).
    Inside(FaceId, Vec3),
}

/// Where a new polygon corner's values come from: corners of the original
/// polygons, each named by its polygon and point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CornerFrom {
    /// Nowhere: every layer's default.
    Nothing,
    /// A copy of polygon `.0`'s corner at point `.1`.
    Copy(FaceId, VertexId),
    /// `t` (`.4`) of the way from polygon `.0`'s corner at point `.1` to
    /// polygon `.2`'s corner at point `.3`.
    Between(FaceId, VertexId, FaceId, VertexId, f32),
    /// At this spot inside the original polygon: its corners blended by
    /// how close the spot is to each (mean value weights).
    Inside(FaceId, Vec3),
}

/// How much each corner of `face` counts at `spot`, inside or on it: mean
/// value weights, which blend smoothly across any polygon and give a
/// corner all the weight at the corner itself. Each weight goes with the
/// corner's place in the polygon and its point; they add up to 1. Empty
/// for an unknown polygon.
pub fn weights_inside(mesh: &Mesh, face: FaceId, spot: Vec3) -> Vec<(usize, VertexId, f32)> {
    let Some(corners) = mesh.face(face) else {
        return Vec::new();
    };
    let places: Vec<Vec3> = corners
        .iter()
        .map(|v| mesh.position(*v).unwrap_or(spot))
        .collect();
    let n = places.len();
    let to: Vec<Vec3> = places.iter().map(|p| *p - spot).collect();
    let lengths: Vec<f32> = to.iter().map(|d| d.length()).collect();
    // On a corner: all of it.
    if let Some(i) = lengths.iter().position(|l| *l < 1e-6) {
        return corners
            .iter()
            .enumerate()
            .map(|(k, v)| (k, *v, if k == i { 1.0 } else { 0.0 }))
            .collect();
    }
    // tan(half the angle) at the spot between corners i and i + 1.
    let half_tan = |i: usize| {
        let (a, b) = (to[i], to[(i + 1) % n]);
        let cos = (a.dot(b) / (lengths[i] * lengths[(i + 1) % n])).clamp(-1.0, 1.0);
        let angle = cos.acos();
        (angle * 0.5).tan()
    };
    let mut weights: Vec<f32> = (0..n)
        .map(|i| (half_tan((i + n - 1) % n) + half_tan(i)) / lengths[i])
        .collect();
    let mut sum: f32 = weights.iter().sum();
    if !sum.is_finite() || sum <= 1e-12 {
        // On an edge, or something odd: by closeness instead.
        weights = lengths.iter().map(|l| 1.0 / (l * l)).collect();
        sum = weights.iter().sum();
    }
    corners
        .iter()
        .enumerate()
        .map(|(k, v)| (k, *v, weights[k] / sum))
        .collect()
}

/// Blends `values` by their weights (adding up to 1), one at a time.
fn blend(values: impl IntoIterator<Item = (Value, f32)>) -> Option<Value> {
    let mut out: Option<(Value, f32)> = None;
    for (value, weight) in values {
        out = Some(match out {
            None => (value, weight),
            Some((acc, so_far)) => {
                let total = so_far + weight;
                let t = if total > 1e-12 { weight / total } else { 0.0 };
                (Value::mix(acc, value, t), total)
            }
        });
    }
    out.map(|(v, _)| v)
}

/// Makes and rewires geometry in `mesh`, carrying attribute values over
/// from `original` (the mesh as it was before the tool started).
pub struct Builder<'a> {
    original: &'a Mesh,
    mesh: &'a mut Mesh,
}

impl<'a> Builder<'a> {
    /// A builder changing `mesh`, which started as a copy of `original`.
    pub fn new(original: &'a Mesh, mesh: &'a mut Mesh) -> Self {
        Self { original, mesh }
    }

    /// The mesh being built, for changes that carry no values (moving a
    /// point, marking an edge hard on purpose).
    pub fn mesh(&mut self) -> &mut Mesh {
        self.mesh
    }

    /// The mesh as it was before the tool started.
    pub fn original(&self) -> &Mesh {
        self.original
    }

    /// Adds a point at `position`, with values from `from`.
    pub fn point(&mut self, position: Vec3, from: PointFrom) -> VertexId {
        let id = self.mesh.push_vertex(position);
        if self.mesh.names.naming() {
            let name = |v: VertexId| self.original.point_name(v).unwrap_or(0);
            let origin = match from {
                PointFrom::Nothing => Origin::Plain,
                PointFrom::Copy(v) => Origin::Copy(name(v)),
                PointFrom::Between(a, b, t) => Origin::Between(name(a), name(b), t),
                PointFrom::Inside(face, spot) => {
                    Origin::Inside(self.original.face_name(face).unwrap_or(0), spot)
                }
            };
            self.mesh.name_new_point(id, origin);
        }
        if from == PointFrom::Nothing {
            return id;
        }
        let inside = match from {
            PointFrom::Inside(face, spot) => weights_inside(self.original, face, spot),
            _ => Vec::new(),
        };
        for layer in self
            .mesh
            .attribute_layers_mut()
            .of_domain_mut(Domain::Point)
        {
            let value = match from {
                PointFrom::Nothing => layer.default(),
                PointFrom::Copy(v) => layer.at(v.get() as usize),
                PointFrom::Between(a, b, t) => {
                    Value::mix(layer.at(a.get() as usize), layer.at(b.get() as usize), t)
                }
                PointFrom::Inside(..) => blend(
                    inside
                        .iter()
                        .map(|(_, v, w)| (layer.at(v.get() as usize), *w)),
                )
                .unwrap_or_else(|| layer.default()),
            };
            layer.set_at(id.get() as usize, value);
        }
        id
    }

    /// Adds a polygon through `corners` (each with where its values come
    /// from), with polygon values copied from the original polygon `like`.
    /// `None` if the polygon is refused (as [`Mesh::add_face`] would).
    pub fn face(
        &mut self,
        corners: &[(VertexId, CornerFrom)],
        like: Option<FaceId>,
    ) -> Option<FaceId> {
        let ids: Vec<VertexId> = corners.iter().map(|c| c.0).collect();
        let face = self.mesh.add_face(&ids).ok()?;
        if let Some(like) = like {
            self.copy_face_values(like, face);
        }
        self.set_corners(face, corners);
        Some(face)
    }

    /// Gives polygons new corners (any number, three or more each), keeping
    /// their ids, with each corner's values from where it says. Refused,
    /// changing nothing, as [`Mesh::rewrite_faces`] would refuse.
    pub fn rewrite_faces(
        &mut self,
        changes: &BTreeMap<FaceId, Vec<(VertexId, CornerFrom)>>,
    ) -> Result<(), MeshError> {
        let plain: BTreeMap<FaceId, Vec<VertexId>> = changes
            .iter()
            .map(|(f, c)| (*f, c.iter().map(|(v, _)| *v).collect()))
            .collect();
        self.mesh.rewrite_faces(&plain)?;
        for (face, corners) in changes {
            self.set_corners(*face, corners);
        }
        Ok(())
    }

    /// Gives the edge `a`–`b` the values the original edge `from` has (hard,
    /// seam), as when a border is copied to the far side of an extrusion.
    /// Ignored for an edge that couldn't hold values (`a` = `b`, or a point
    /// that doesn't exist).
    pub fn copy_edge(&mut self, from: EdgeKey, a: VertexId, b: VertexId) {
        if a == b || self.mesh.position(a).is_none() || self.mesh.position(b).is_none() {
            return;
        }
        let to = EdgeKey::new(a, b);
        let values: Vec<(String, Domain, Value, Value)> = self
            .original
            .attribute_layers()
            .iter()
            .filter(|(_, l)| l.domain() == Domain::Edge)
            .map(|(n, l)| (n.to_owned(), l.domain(), l.default(), l.edge(from)))
            .filter(|(_, _, default, value)| value != default)
            .collect();
        for (name, domain, default, value) in values {
            let layers = self.mesh.attribute_layers_mut();
            if layers.add(&name, domain, default).is_ok()
                && let Some(layer) = layers.get_mut(&name)
            {
                layer.set_edge(to, value);
            }
        }
    }

    /// Moves the original edge `from`'s values onto the edge `a`–`b`: the
    /// edge itself moved (its points were swapped for copies).
    pub fn move_edge(&mut self, from: EdgeKey, a: VertexId, b: VertexId) {
        if EdgeKey::new(a, b) == from {
            return;
        }
        self.clear_edge(from);
        self.copy_edge(from, a, b);
    }

    /// Clears every value on the edge `edge` in the mesh being built (it
    /// is gone, or became something else).
    pub fn clear_edge(&mut self, edge: EdgeKey) {
        for layer in self.mesh.attribute_layers_mut().of_domain_mut(Domain::Edge) {
            let default = layer.default();
            layer.set_edge(edge, default);
        }
        self.mesh.drop_empty_hard();
    }

    /// Copies the original polygon `like`'s polygon values onto `face`.
    fn copy_face_values(&mut self, like: FaceId, face: FaceId) {
        if self.original.face(like).is_none() {
            return;
        }
        let values: Vec<(String, Value)> = self
            .original
            .attribute_layers()
            .iter()
            .filter(|(_, l)| l.domain() == Domain::Face)
            .map(|(n, l)| (n.to_owned(), l.at(like.get() as usize)))
            .collect();
        let layers = self.mesh.attribute_layers_mut();
        for (name, value) in values {
            if let Some(layer) = layers.get_mut(&name) {
                layer.set_at(face.get() as usize, value);
            }
        }
    }

    /// Sets the corner values of `face` (in the mesh being built) from
    /// where each corner says.
    fn set_corners(&mut self, face: FaceId, corners: &[(VertexId, CornerFrom)]) {
        if !self.mesh.attribute_layers().has_domain(Domain::Corner)
            || corners.iter().all(|(_, from)| *from == CornerFrom::Nothing)
        {
            return;
        }
        let Some(start) = self
            .mesh
            .face(face)
            .and_then(|c| self.mesh.corner_index(face, c[0]))
        else {
            return;
        };
        let original = self.original;
        let index = |f: FaceId, v: VertexId| original.corner_index(f, v);
        let names: Vec<String> = original
            .attribute_layers()
            .iter()
            .filter(|(_, l)| l.domain() == Domain::Corner)
            .map(|(n, _)| n.to_owned())
            .collect();
        for name in names {
            let Some(source) = original.attribute_layers().get(&name) else {
                continue;
            };
            let values: Vec<Value> = corners
                .iter()
                .map(|(_, from)| match *from {
                    CornerFrom::Nothing => source.default(),
                    CornerFrom::Copy(f, v) => {
                        index(f, v).map_or(source.default(), |i| source.at(i))
                    }
                    CornerFrom::Between(f, v, g, w, t) => {
                        let a = index(f, v).map_or(source.default(), |i| source.at(i));
                        let b = index(g, w).map_or(source.default(), |i| source.at(i));
                        Value::mix(a, b, t)
                    }
                    CornerFrom::Inside(f, spot) => blend(
                        weights_inside(original, f, spot)
                            .into_iter()
                            .map(|(_, v, w)| {
                                (index(f, v).map_or(source.default(), |i| source.at(i)), w)
                            }),
                    )
                    .unwrap_or_else(|| source.default()),
                })
                .collect();
            if let Some(layer) = self.mesh.attribute_layers_mut().get_mut(&name) {
                for (i, value) in values.into_iter().enumerate() {
                    layer.set_at(start + i, value);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
