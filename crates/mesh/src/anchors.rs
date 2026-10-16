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
//!
//! **Ids are trusted when the structure is the same**
//! ([decision 0044](../../../docs/decisions/0044-edits-that-hold.md)). Anchors keep a fingerprint of the
//! shape's structure ([`Mesh::structure_fingerprint`]). When an earlier
//! edit changes only where points are (a longer extrude, say), the
//! structure is unchanged, so the ids still name the right elements even
//! though they moved: they are used as they are, not searched for.

use crate::topology::Topology;
use crate::{Bounds, ElementKind, FaceId, Mesh, VertexId};
use glam::{Vec2, Vec3};
use std::collections::{BTreeSet, HashMap, HashSet};

/// Where an edit's elements were, in the shape it was made on. Two
/// anchors are equal when they hold the same places, whatever names they
/// carry: names are worked out again from the edits, like a mesh's.
#[derive(Debug, Clone)]
pub struct Anchors {
    /// The box around that shape.
    pub bounds: Bounds,
    /// Each point the edit names, and where it was.
    pub points: Vec<(VertexId, Vec3)>,
    /// The elements the edit was made on, by position.
    pub shape: AnchorShape,
    /// The structure of the shape the edit was made on
    /// ([`Mesh::structure_fingerprint`]); `None` in older files.
    pub structure: Option<u64>,
    /// The stable names of `points`, in the same order
    /// ([decision 0090](../../../docs/decisions/0090-stable-element-names.md)),
    /// when every one had a trusted name; `None` otherwise, and in older
    /// files.
    pub names: Option<Vec<u64>>,
}

impl PartialEq for Anchors {
    fn eq(&self, other: &Self) -> bool {
        self.bounds == other.bounds
            && self.points == other.points
            && self.shape == other.shape
            && self.structure == other.structure
    }
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
        let names = points
            .iter()
            .map(|(v, _)| mesh.trusted_point_name(*v))
            .collect();
        Some(Self {
            bounds,
            points,
            shape,
            structure: Some(mesh.structure_fingerprint()),
            names,
        })
    }

    /// The stable name of the anchored point that was at `place`, if
    /// known.
    fn name_at(&self, place: Vec3) -> Option<u64> {
        let names = self.names.as_ref()?;
        let i = self.points.iter().position(|(_, p)| *p == place)?;
        names.get(i).copied()
    }

    /// The anchors of the same elements' mirrors across `reflect` of
    /// `mirror`, in the same shape: every place mirrored, and each point
    /// id given as `partner` maps it (its mirror's id, where known).
    pub fn mirrored(
        &self,
        mirror: &crate::symmetry::Mirror,
        reflect: [bool; 3],
        partner: impl Fn(VertexId) -> VertexId,
    ) -> Self {
        let at = |p: Vec3| mirror.point(reflect, p);
        Self {
            bounds: self.bounds,
            points: self
                .points
                .iter()
                .map(|(v, p)| (partner(*v), at(*p)))
                .collect(),
            shape: match &self.shape {
                AnchorShape::Points(p) => AnchorShape::Points(p.iter().map(|p| at(*p)).collect()),
                AnchorShape::Edges(e) => {
                    AnchorShape::Edges(e.iter().map(|[a, b]| [at(*a), at(*b)]).collect())
                }
                AnchorShape::Polygons(f) => AnchorShape::Polygons(
                    f.iter()
                        .map(|c| c.iter().map(|p| at(*p)).collect())
                        .collect(),
                ),
            },
            structure: self.structure,
            // Mirrored points have names of their own, not known here.
            names: None,
        }
    }

    /// These anchors and `other`'s together (both recorded on the same
    /// shape, for the same kind of element); elements of another kind are
    /// left out.
    #[must_use]
    pub fn joined(&self, other: &Self) -> Self {
        let mut out = self.clone();
        let other_names = other.names.as_deref();
        for (i, (v, p)) in other.points.iter().enumerate() {
            if !out.points.iter().any(|(w, _)| w == v) {
                out.points.push((*v, *p));
                out.names = match (out.names.take(), other_names.and_then(|n| n.get(i))) {
                    (Some(mut names), Some(name)) => {
                        names.push(*name);
                        Some(names)
                    }
                    _ => None,
                };
            }
        }
        match (&mut out.shape, &other.shape) {
            (AnchorShape::Points(a), AnchorShape::Points(b)) => a.extend(b.iter().copied()),
            (AnchorShape::Edges(a), AnchorShape::Edges(b)) => a.extend(b.iter().copied()),
            (AnchorShape::Polygons(a), AnchorShape::Polygons(b)) => a.extend(b.iter().cloned()),
            _ => {}
        }
        out
    }

    /// Whether `mesh` has the structure the edit was made on, so its ids
    /// can be trusted as they are, wherever the points now are.
    pub fn same_structure(&self, mesh: &Mesh) -> bool {
        self.structure == Some(mesh.structure_fingerprint())
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
    ///
    /// The points are sorted into a grid first, so each element only looks
    /// at the points near it rather than the whole mesh. Edges and polygons
    /// also tighten "the same place" to a quarter of their own shortest
    /// side, so on a dense mesh a polygon doesn't pick up its neighbours.
    pub fn find(&self, mesh: &Mesh, topology: &Topology) -> (Vec<VertexId>, usize) {
        let Some(now) = mesh.bounds() else {
            return (Vec::new(), self.element_count());
        };
        let tolerance = Self::tolerance(&now);
        let grid = PointGrid::new(mesh, &now, tolerance);
        let mut found = BTreeSet::new();
        let mut missing = 0;
        match &self.shape {
            AnchorShape::Points(places) => {
                for anchored in places {
                    // Of points on the same spot, the one with the
                    // anchored point's stable name (decision 0090).
                    let name = self.name_at(*anchored);
                    let place = self.moved(*anchored, &now);
                    let nearest = grid
                        .near(mesh, place, place, tolerance)
                        .map(|(v, p)| (v, p.distance(place)))
                        .filter(|(_, d)| *d <= tolerance)
                        .min_by(|a, b| {
                            let other =
                                |v: VertexId| name.is_some() && mesh.trusted_point_name(v) != name;
                            other(a.0)
                                .cmp(&other(b.0))
                                .then(a.1.total_cmp(&b.1))
                                .then(a.0.cmp(&b.0))
                        });
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
                    let tolerance = tighter(tolerance, a.distance(b));
                    let on: HashSet<VertexId> = grid
                        .near(mesh, a.min(b), a.max(b), tolerance)
                        .filter(|(_, p)| distance_to_segment(*p, a, b) <= tolerance)
                        .map(|(v, _)| v)
                        .collect();
                    let mut any = false;
                    for x in &on {
                        for edge in topology.vertex_edges(*x) {
                            let (lo, hi) = edge.ends();
                            if on.contains(&lo) && on.contains(&hi) {
                                found.insert(lo);
                                found.insert(hi);
                                any = true;
                            }
                        }
                    }
                    if !any {
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
                    let shortest = (0..corners.len())
                        .map(|i| corners[i].distance(corners[(i + 1) % corners.len()]))
                        .fold(f32::INFINITY, f32::min);
                    let tolerance = tighter(tolerance, shortest);
                    let (lo, hi) = corners
                        .iter()
                        .fold((corners[0], corners[0]), |(lo, hi), c| {
                            (lo.min(*c), hi.max(*c))
                        });
                    let inside: HashSet<VertexId> = grid
                        .near(mesh, lo, hi, tolerance)
                        .filter(|(_, p)| flat.contains(*p, tolerance))
                        .map(|(v, _)| v)
                        .collect();
                    let faces: BTreeSet<FaceId> = inside
                        .iter()
                        .flat_map(|v| topology.vertex_faces(*v).iter().copied())
                        .collect();
                    let mut any = false;
                    for face in faces {
                        let Some(face) = mesh.face(face) else {
                            continue;
                        };
                        if face.iter().all(|v| inside.contains(v)) {
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

/// "The same place" for an element whose shortest side is `shortest`: at
/// most a quarter of that side, so neighbouring elements on a dense mesh
/// don't count (a zero-length side keeps the shape-wide tolerance).
fn tighter(tolerance: f32, shortest: f32) -> f32 {
    if shortest.is_finite() && shortest > 1e-9 {
        tolerance.min(shortest * 0.25)
    } else {
        tolerance
    }
}

/// A mesh's points sorted into equal boxes, to find the points near a
/// place without looking at all of them.
struct PointGrid {
    min: Vec3,
    cell: f32,
    cells: HashMap<[i32; 3], Vec<VertexId>>,
    count: usize,
}

impl PointGrid {
    fn new(mesh: &Mesh, bounds: &Bounds, tolerance: f32) -> Self {
        let count = mesh.vertex_count();
        // About one box per point along the diagonal's cube root, never
        // smaller than the tolerance.
        let diagonal = bounds.size().length();
        let per_axis = (count.max(1) as f32).cbrt().max(1.0);
        let cell = (diagonal / per_axis).max(tolerance * 2.0).max(1e-6);
        let mut grid = Self {
            min: bounds.min,
            cell,
            cells: HashMap::new(),
            count,
        };
        for (v, p) in mesh.vertices() {
            grid.cells.entry(grid.key(p)).or_default().push(v);
        }
        grid
    }

    fn key(&self, p: Vec3) -> [i32; 3] {
        let k = ((p - self.min) / self.cell).floor();
        // Points that aren't numbers, or far outside, land in edge boxes.
        let clamp = |x: f32| {
            if x.is_finite() {
                x.clamp(-1e6, 1e6) as i32
            } else {
                0
            }
        };
        [clamp(k.x), clamp(k.y), clamp(k.z)]
    }

    /// The points (with where they are) in the box from `lo` to `hi`
    /// widened by `margin`, and perhaps a few more.
    fn near<'a>(
        &'a self,
        mesh: &'a Mesh,
        lo: Vec3,
        hi: Vec3,
        margin: f32,
    ) -> Box<dyn Iterator<Item = (VertexId, Vec3)> + 'a> {
        let (a, b) = (self.key(lo - margin), self.key(hi + margin));
        let boxes: i64 = (0..3)
            .map(|i| i64::from(b[i]) - i64::from(a[i]) + 1)
            .product();
        if boxes > self.count as i64 {
            // A big search: every point is quicker than every box.
            return Box::new(mesh.vertices());
        }
        Box::new(
            (a[0]..=b[0])
                .flat_map(move |x| {
                    (a[1]..=b[1]).flat_map(move |y| (a[2]..=b[2]).map(move |z| [x, y, z]))
                })
                .filter_map(|k| self.cells.get(&k))
                .flatten()
                .filter_map(|v| Some((*v, mesh.position(*v)?))),
        )
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
