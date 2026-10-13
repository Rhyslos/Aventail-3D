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
//!
//! **New rows stretch along.** A move keeps its *reach*: the edges from
//! the points it moved to their neighbours. When more segments put new
//! points along such an edge, [`Anchors::stretch`] says how much of the
//! move each one gets: all of it next to a moved point, none next to a
//! point that stayed, in between along the way.

use crate::topology::Topology;
use crate::{Bounds, ElementKind, FaceId, Mesh, VertexId};
use glam::{Vec2, Vec3};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// Where an edit's elements were, in the shape it was made on.
#[derive(Debug, Clone, PartialEq)]
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
    /// For edits that move points: the edges from those points to their
    /// neighbours, so new points along them can stretch (empty otherwise,
    /// and in older files).
    pub reach: Vec<Reach>,
}

/// One edge from a moved point (by its place in [`Anchors::points`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reach {
    /// The moved point it starts at.
    pub from: u32,
    /// Where it ends.
    pub to: ReachEnd,
}

/// The far end of a [`Reach`] edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReachEnd {
    /// Another moved point, by its place in [`Anchors::points`].
    Moved(u32),
    /// A point that stayed, where it was.
    Still(Vec3),
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
            structure: Some(mesh.structure_fingerprint()),
            reach: Vec::new(),
        })
    }

    /// The anchors with their reach: every edge of `mesh` from one of the
    /// anchored points to a neighbour. For edits that move the anchored
    /// points, so new points along those edges can stretch.
    #[must_use]
    pub fn with_reach(mut self, mesh: &Mesh) -> Self {
        let index: HashMap<VertexId, u32> = self
            .points
            .iter()
            .enumerate()
            .map(|(i, (v, _))| (*v, i as u32))
            .collect();
        let topology = mesh.topology();
        let mut reach = Vec::new();
        for (i, (v, _)) in self.points.iter().enumerate() {
            for edge in topology.vertex_edges(*v) {
                let other = crate::topology::other_end(*edge, *v);
                match index.get(&other) {
                    // Each edge between two moved points once.
                    Some(j) if *j > i as u32 => reach.push(Reach {
                        from: i as u32,
                        to: ReachEnd::Moved(*j),
                    }),
                    Some(_) => {}
                    None => {
                        if let Some(p) = mesh.position(other) {
                            reach.push(Reach {
                                from: i as u32,
                                to: ReachEnd::Still(p),
                            });
                        }
                    }
                }
            }
        }
        self.reach = reach;
        self
    }

    /// Whether `mesh` has the structure the edit was made on, so its ids
    /// can be trusted as they are, wherever the points now are.
    pub fn same_structure(&self, mesh: &Mesh) -> bool {
        self.structure == Some(mesh.structure_fingerprint())
    }

    /// How much of a move each point of `mesh` that isn't one of `found`
    /// gets, for the points lying along the anchors' reach: 1 next to a
    /// moved point, 0 next to one that stayed, in between along the edge
    /// (and all of it between two moved points). Points that get nothing
    /// aren't listed. For use after [`Self::find`], when more segments put
    /// new points along the edges the edit moved.
    pub fn stretch(&self, mesh: &Mesh, found: &[VertexId]) -> Vec<(VertexId, f32)> {
        if self.reach.is_empty() {
            return Vec::new();
        }
        let Some(now) = mesh.bounds() else {
            return Vec::new();
        };
        let found: HashSet<VertexId> = found.iter().copied().collect();
        let tolerance = Self::tolerance(&now);
        let grid = PointGrid::new(mesh, &now, tolerance);
        let place = |i: u32| {
            self.points
                .get(i as usize)
                .map(|(_, p)| self.moved(*p, &now))
        };
        let mut share: BTreeMap<VertexId, f32> = BTreeMap::new();
        for reach in &self.reach {
            let Some(a) = place(reach.from) else {
                continue;
            };
            let (b, far_share) = match reach.to {
                ReachEnd::Moved(j) => match place(j) {
                    Some(b) => (b, 1.0),
                    None => continue,
                },
                ReachEnd::Still(p) => (self.moved(p, &now), 0.0),
            };
            let length = a.distance(b);
            if length <= tolerance * 2.0 {
                continue;
            }
            let tolerance = tighter(tolerance, length);
            for (v, p) in grid.near(mesh, a.min(b), a.max(b), tolerance) {
                if found.contains(&v) || distance_to_segment(p, a, b) > tolerance {
                    continue;
                }
                let along = ((p - a).dot(b - a) / (length * length)).clamp(0.0, 1.0);
                // The two ends themselves aren't new points.
                if along * length <= tolerance || (1.0 - along) * length <= tolerance {
                    continue;
                }
                let amount = 1.0 - along + along * far_share;
                if amount > 0.0 {
                    let entry = share.entry(v).or_insert(0.0);
                    *entry = entry.max(amount);
                }
            }
        }
        share.into_iter().collect()
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
                for place in places {
                    let place = self.moved(*place, &now);
                    let nearest = grid
                        .near(mesh, place, place, tolerance)
                        .map(|(v, p)| (v, p.distance(place)))
                        .filter(|(_, d)| *d <= tolerance)
                        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
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
        std::mem::size_of_val(self.points.as_slice())
            + shape
            + std::mem::size_of_val(self.reach.as_slice())
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
