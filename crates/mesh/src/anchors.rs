//! Finding an edit's points again after its shape's points were renumbered
//! ([decision 0032](../../../docs/decisions/0032-edits-find-their-points-again.md)).
//!
//! Recorded edits name points by ID. That is exact and fast, but a
//! primitive's points are numbered in the order it builds them: give a
//! cube more segments and the IDs land on different points. So an edit
//! also keeps [`Anchors`]: where its points, edges or polygons were, in
//! the shape as it was when the edit was made. When the IDs no longer sit
//! where the anchors say, the edit finds its elements again by position,
//! measured against the shape's box so a resized shape still matches:
//!
//! - **Points:** the point at each anchor's place.
//! - **Edges:** every edge lying along an anchored edge, so an edge split
//!   in two by more segments moves as a whole.
//! - **Polygons:** every polygon lying inside an anchored polygon, so a
//!   face split into four moves as a whole.

use crate::topology::Topology;
use crate::{Bounds, ElementKind, Mesh, VertexId};
use glam::{Vec2, Vec3};
use std::collections::BTreeSet;

/// Where an edit's elements were, in the shape it was made on.
#[derive(Debug, Clone, PartialEq)]
pub struct Anchors {
    /// The box around that shape.
    pub bounds: Bounds,
    /// Each point the edit names, and where it was.
    pub points: Vec<(VertexId, Vec3)>,
    /// The elements the edit was made on, by position.
    pub shape: AnchorShape,
}

/// The elements an edit was made on, by position.
#[derive(Debug, Clone, PartialEq)]
pub enum AnchorShape {
    /// Points.
    Points(Vec<Vec3>),
    /// Edges, by their two ends.
    Edges(Vec<[Vec3; 2]>),
    /// Polygons, by their corners in order.
    Polygons(Vec<Vec<Vec3>>),
}

/// How far (as a part of the shape's box diagonal) a point may be from
/// its anchored place and still be the same.
const TOLERANCE: f32 = 1e-3;

impl Anchors {
    /// The anchors of `elements` of `kind` (point IDs, edge ends or
    /// polygon corners, as listed) in `mesh`. `None` for an empty mesh.
    pub fn record(mesh: &Mesh, kind: ElementKind, elements: &[Vec<VertexId>]) -> Option<Self> {
        let bounds = mesh.bounds()?;
        let at = |v: &VertexId| mesh.position(*v);
        let mut points = BTreeSet::new();
        for element in elements {
            points.extend(element.iter().copied());
        }
        let points: Vec<(VertexId, Vec3)> = points
            .into_iter()
            .filter_map(|v| Some((v, at(&v)?)))
            .collect();
        let shape = match kind {
            ElementKind::Points => {
                AnchorShape::Points(elements.iter().flatten().filter_map(at).collect())
            }
            ElementKind::Edges => AnchorShape::Edges(
                elements
                    .iter()
                    .filter_map(|e| Some([at(e.first()?)?, at(e.get(1)?)?]))
                    .collect(),
            ),
            ElementKind::Polygons => AnchorShape::Polygons(
                elements
                    .iter()
                    .map(|corners| corners.iter().filter_map(at).collect::<Vec<_>>())
                    .filter(|c| c.len() >= 3)
                    .collect(),
            ),
        };
        Some(Self {
            bounds,
            points,
            shape,
        })
    }

    /// Where a recorded place is in `mesh`, whose box is `now`: the same
    /// spot relative to the box. A flat side of the box (a plane's height)
    /// keeps its distance instead.
    fn moved(&self, p: Vec3, now: &Bounds) -> Vec3 {
        let then = &self.bounds;
        let mut out = Vec3::ZERO;
        for i in 0..3 {
            let size = then.size()[i];
            out[i] = if size > 1e-6 {
                now.min[i] + (p[i] - then.min[i]) / size * now.size()[i]
            } else {
                now.min[i] + (p[i] - then.min[i])
            };
        }
        out
    }

    /// How close counts as "the same place" in `mesh`.
    fn tolerance(now: &Bounds) -> f32 {
        TOLERANCE * now.size().length().max(1.0) + 1e-4
    }

    /// Whether the IDs still sit where the anchors say (allowing for the
    /// shape being resized), so they can be used as they are.
    pub fn still_fit(&self, mesh: &Mesh) -> bool {
        let Some(now) = mesh.bounds() else {
            return self.points.is_empty();
        };
        let tolerance = Self::tolerance(&now);
        self.points.iter().all(|(v, then)| {
            mesh.position(*v)
                .is_some_and(|p| p.distance(self.moved(*then, &now)) <= tolerance)
        })
    }

    /// The points of `mesh` at the anchored elements, and how many of the
    /// anchored elements weren't found.
    pub fn find(&self, mesh: &Mesh, topology: &Topology) -> (Vec<VertexId>, usize) {
        let Some(now) = mesh.bounds() else {
            return (Vec::new(), self.element_count());
        };
        let tolerance = Self::tolerance(&now);
        let mut found = BTreeSet::new();
        let mut missing = 0;
        match &self.shape {
            AnchorShape::Points(places) => {
                for place in places {
                    let place = self.moved(*place, &now);
                    let nearest = mesh
                        .vertices()
                        .map(|(v, p)| (v, p.distance(place)))
                        .filter(|(_, d)| *d <= tolerance)
                        .min_by(|a, b| a.1.total_cmp(&b.1));
                    match nearest {
                        Some((v, _)) => {
                            found.insert(v);
                        }
                        None => missing += 1,
                    }
                }
            }
            AnchorShape::Edges(edges) => {
                for [a, b] in edges {
                    let (a, b) = (self.moved(*a, &now), self.moved(*b, &now));
                    let on = |v: VertexId| {
                        mesh.position(v)
                            .is_some_and(|p| distance_to_segment(p, a, b) <= tolerance)
                    };
                    let before = found.len();
                    let mut any = false;
                    for edge in topology.edges() {
                        let (x, y) = edge.ends();
                        if on(x) && on(y) {
                            found.insert(x);
                            found.insert(y);
                            any = true;
                        }
                    }
                    if !any && found.len() == before {
                        missing += 1;
                    }
                }
            }
            AnchorShape::Polygons(polygons) => {
                for corners in polygons {
                    let corners: Vec<Vec3> = corners.iter().map(|c| self.moved(*c, &now)).collect();
                    let Some(flat) = Flat::new(&corners) else {
                        missing += 1;
                        continue;
                    };
                    let mut any = false;
                    for (_, face) in mesh.faces() {
                        let inside = face.iter().all(|v| {
                            mesh.position(*v)
                                .is_some_and(|p| flat.contains(p, tolerance))
                        });
                        if inside {
                            found.extend(face.iter().copied());
                            any = true;
                        }
                    }
                    if !any {
                        missing += 1;
                    }
                }
            }
        }
        (found.into_iter().collect(), missing)
    }

    /// How many elements are anchored.
    pub fn element_count(&self) -> usize {
        match &self.shape {
            AnchorShape::Points(p) => p.len(),
            AnchorShape::Edges(e) => e.len(),
            AnchorShape::Polygons(p) => p.len(),
        }
    }

    /// Roughly how many bytes the anchors take, for the undo budget.
    pub fn memory_size(&self) -> usize {
        let shape = match &self.shape {
            AnchorShape::Points(p) => std::mem::size_of_val(p.as_slice()),
            AnchorShape::Edges(e) => std::mem::size_of_val(e.as_slice()),
            AnchorShape::Polygons(p) => p
                .iter()
                .map(|c| std::mem::size_of_val(c.as_slice()) + size_of::<Vec<Vec3>>())
                .sum(),
        };
        std::mem::size_of_val(self.points.as_slice()) + shape
    }
}

/// A polygon laid flat: its plane, and its outline in that plane.
struct Flat {
    origin: Vec3,
    normal: Vec3,
    u: Vec3,
    v: Vec3,
    outline: Vec<Vec2>,
}

impl Flat {
    fn new(corners: &[Vec3]) -> Option<Self> {
        let origin = *corners.first()?;
        // Newell's method: a normal that works for any simple polygon.
        let mut normal = Vec3::ZERO;
        for (i, a) in corners.iter().enumerate() {
            let b = corners[(i + 1) % corners.len()];
            normal += Vec3::new(
                (a.y - b.y) * (a.z + b.z),
                (a.z - b.z) * (a.x + b.x),
                (a.x - b.x) * (a.y + b.y),
            );
        }
        let normal = normal.try_normalize()?;
        let u = normal.any_orthonormal_vector();
        let v = normal.cross(u);
        let outline = corners
            .iter()
            .map(|c| Vec2::new((*c - origin).dot(u), (*c - origin).dot(v)))
            .collect();
        Some(Self {
            origin,
            normal,
            u,
            v,
            outline,
        })
    }

    /// Whether `p` lies in the polygon (on its plane, inside or on its
    /// outline), within `tolerance`.
    fn contains(&self, p: Vec3, tolerance: f32) -> bool {
        let d = p - self.origin;
        if d.dot(self.normal).abs() > tolerance {
            return false;
        }
        let q = Vec2::new(d.dot(self.u), d.dot(self.v));
        let n = self.outline.len();
        let on_outline = (0..n).any(|i| {
            distance_to_segment_2d(q, self.outline[i], self.outline[(i + 1) % n]) <= tolerance
        });
        on_outline || inside_outline(&self.outline, q)
    }
}

fn distance_to_segment(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let along = b - a;
    let length_squared = along.length_squared();
    let t = if length_squared > 1e-12 {
        ((p - a).dot(along) / length_squared).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (a + along * t).distance(p)
}

fn distance_to_segment_2d(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let along = b - a;
    let length_squared = along.length_squared();
    let t = if length_squared > 1e-12 {
        ((p - a).dot(along) / length_squared).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (a + along * t).distance(p)
}

fn inside_outline(outline: &[Vec2], p: Vec2) -> bool {
    let n = outline.len();
    let mut inside = false;
    for i in 0..n {
        let (a, b) = (outline[i], outline[(i + n - 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
    }
    inside
}

#[cfg(test)]
mod tests;
