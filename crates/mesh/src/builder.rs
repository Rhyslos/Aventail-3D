//! The mesh builder: how tools make new points and polygons so their
//! attribute values come along ([decision 0043](../../../docs/decisions/0043-attribute-layers-and-the-builder.md)).
//!
//! A tool that adds or rewires geometry says **where each new element
//! comes from**, and the builder fills in every attribute layer the mesh
//! has, including ones the tool has never heard of (UVs, material slots,
//! vertex colours to come):
//!
//! - a new point is a copy of another ([`PointFrom::Copy`]) or lies between
//!   two ([`PointFrom::Between`]), and its point values follow;
//! - a new polygon is *like* an existing one, and its polygon values (a
//!   material slot, say) are copied;
//! - each new corner copies a corner of an original polygon
//!   ([`CornerFrom::Copy`]) or blends two ([`CornerFrom::Between`]), so
//!   UVs stretch across an extruded wall or a bevel instead of being lost;
//! - an edge that moves or is copied takes its edge values (hard, seam)
//!   with it ([`Builder::move_edge`], [`Builder::copy_edge`]).
//!
//! Points come from the mesh being built (so a new point can lie between
//! two other new ones); polygons, corners and edges come from the
//! original, as it was before the tool started.

use crate::attributes::{Domain, Value};
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
        if from == PointFrom::Nothing {
            return id;
        }
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
