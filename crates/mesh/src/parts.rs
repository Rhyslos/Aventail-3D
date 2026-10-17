//! Splitting polygons off into a shape of their own, and joining shapes
//! into one ([decision 0055](../../../docs/decisions/0055-basics.md)).
//!
//! - [`keep_only`] is a copy of a shape with only some of its polygons
//!   (and the points they use).
//! - [`combine`] puts several shapes into one, each placed by a matrix,
//!   with its hard edges and other values.

use crate::attributes::{AttributeElement, Domain, HARD};
use crate::{FaceId, Mesh, VertexId};
use glam::Mat4;
use std::collections::{BTreeSet, HashMap};

/// A copy of `mesh` with only `faces` (those in it), and the points they
/// use. Polygons and points keep their ids.
pub fn keep_only(mesh: &Mesh, faces: &[FaceId]) -> Mesh {
    let keep: BTreeSet<FaceId> = faces.iter().copied().collect();
    let gone: Vec<FaceId> = mesh
        .faces()
        .map(|(f, _)| f)
        .filter(|f| !keep.contains(f))
        .collect();
    let mut copy = mesh.clone();
    crate::cleanup::delete_elements(&mut copy, &[], &[], &gone);
    // Points no polygon uses (a shape's loose points) go too.
    let used: BTreeSet<VertexId> = copy
        .faces()
        .flat_map(|(_, corners)| corners.to_vec())
        .collect();
    let loose: Vec<VertexId> = copy
        .vertices()
        .map(|(v, _)| v)
        .filter(|v| !used.contains(v))
        .collect();
    copy.remove_unused_vertices(&loose);
    copy
}

/// One shape made of `parts`, each placed by its matrix (into the
/// result's space). Hard edges and other values come along; a value
/// layer one part has and another hasn't takes its standard value there.
/// A part placed by a mirroring matrix is turned the right way round, so
/// it still faces out. Parts whose layers clash (the same name, another
/// kind) keep the first part's layer.
pub fn combine(parts: &[(&Mesh, Mat4)]) -> Mesh {
    let mut out = Mesh::new();
    for (part, matrix) in parts {
        append(&mut out, part, *matrix);
    }
    out
}

/// Adds a copy of `part`, placed by `matrix`, to `out`, which keeps its
/// own points and polygons (and their ids) as they are; the copy's get
/// new ids. Hard edges and other values come along, as in [`combine`],
/// and a mirroring matrix turns the copy the right way round.
pub fn append(out: &mut Mesh, part: &Mesh, matrix: Mat4) {
    {
        let flip = matrix.determinant() < 0.0;
        let mut points: HashMap<VertexId, VertexId> = HashMap::new();
        for (v, p) in part.vertices() {
            if let Ok(new) = out.add_vertex(matrix.transform_point3(p)) {
                points.insert(v, new);
            }
        }
        // Each polygon, and which old corner each new corner is.
        let mut faces: Vec<(FaceId, FaceId, Vec<usize>)> = Vec::new();
        for (f, corners) in part.faces() {
            let mut order: Vec<usize> = (0..corners.len()).collect();
            if flip {
                order.reverse();
            }
            let mapped: Option<Vec<VertexId>> = order
                .iter()
                .map(|i| points.get(&corners[*i]).copied())
                .collect();
            if let Some(mapped) = mapped
                && let Ok(new) = out.add_face(&mapped)
            {
                faces.push((f, new, order));
            }
        }
        for edge in part.hard_edges() {
            let (a, b) = edge.ends();
            if let (Some(a), Some(b)) = (points.get(&a), points.get(&b)) {
                out.set_hard(*a, *b, true);
            }
        }
        for (name, domain, _) in part.attributes() {
            if name == HARD {
                continue;
            }
            let Some((_, default, _)) = part.attribute_values(name) else {
                continue;
            };
            if out.attribute_values(name).is_none()
                && out.add_attribute(name, domain, default).is_err()
            {
                continue;
            }
            let pairs: Vec<(AttributeElement, AttributeElement)> = match domain {
                Domain::Point => points
                    .iter()
                    .map(|(old, new)| {
                        (AttributeElement::Point(*old), AttributeElement::Point(*new))
                    })
                    .collect(),
                Domain::Edge => part
                    .edges()
                    .iter()
                    .filter_map(|e| {
                        let (a, b) = e.ends();
                        let edge = crate::EdgeKey::new(*points.get(&a)?, *points.get(&b)?);
                        Some((AttributeElement::Edge(*e), AttributeElement::Edge(edge)))
                    })
                    .collect(),
                Domain::Face => faces
                    .iter()
                    .map(|(old, new, _)| {
                        (AttributeElement::Face(*old), AttributeElement::Face(*new))
                    })
                    .collect(),
                Domain::Corner => faces
                    .iter()
                    .flat_map(|(old, new, order)| {
                        order.iter().enumerate().map(move |(i, from)| {
                            (
                                AttributeElement::Corner(*old, *from),
                                AttributeElement::Corner(*new, i),
                            )
                        })
                    })
                    .collect(),
            };
            for (from, to) in pairs {
                if let Some(value) = part.attribute(name, from) {
                    // A clashing layer refuses the value; the first stays.
                    let _ = out.set_attribute(name, to, value);
                }
            }
        }
    }
}

/// Places the whole shape by `matrix`: every point moved, turned and
/// sized by it, keeping its id. A mirroring matrix (one that turns the
/// shape inside out) turns every polygon the right way round too, so the
/// shape still faces out. A matrix that isn't usable (flattening the shape,
/// or not numbers) changes nothing and counts the shape as skipped.
pub fn place(mesh: &mut Mesh, matrix: Mat4) -> crate::edit::Outcome {
    let det = matrix.determinant();
    if !matrix.is_finite() || det.abs() < 1e-12 {
        return crate::edit::Outcome {
            matched: 0,
            missing: 0,
            skipped: 1,
        };
    }
    let points: Vec<(VertexId, glam::Vec3)> = mesh.vertices().collect();
    for (v, p) in &points {
        mesh.set_position(*v, matrix.transform_point3(*p));
    }
    if det < 0.0 {
        let faces: Vec<FaceId> = mesh.faces().map(|(f, _)| f).collect();
        crate::cleanup::flip_faces(mesh, &faces);
    }
    crate::edit::Outcome {
        matched: points.len(),
        missing: 0,
        skipped: 0,
    }
}

#[cfg(test)]
mod tests;
