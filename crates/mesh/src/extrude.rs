//! Extruding and insetting: the first operations that add points and
//! polygons rather than only moving them ([decision 0038](../../../docs/decisions/0038-extrude-and-inset.md)).
//!
//! - **Extrude polygons** ([`extrude_polygons`]): the polygons move out
//!   along their facing, and a wall of new four-sided polygons joins
//!   their old border to the new one.
//! - **Extrude edges** ([`extrude_edges`]): edges on an open border grow
//!   a new strip of polygons outward, flat with the polygon they belong
//!   to, like pulling more cloth off the edge of a sheet.
//! - **Inset** ([`inset_polygons`]): the polygons shrink inward, flat
//!   where they are, leaving a ring of new polygons around them.
//!
//! [`Grouping`] says whether neighbouring polygons (or edges joined at a
//! point) work as one piece or each on its own.
//!
//! **Ids stay put.** Each extruded or inset polygon keeps its id (it gets
//! new corners), so what was selected is still selected afterwards and
//! pressing the key again carries on from there. New points and polygons
//! get the next free ids, in a fixed order, so replaying an edit gives
//! exactly the same ids.
//!
//! **Even thickness.** Where several polygons meet at a point, the point
//! moves along their average facing, a little further where they meet at
//! an angle, so every polygon ends up the full distance away (at most
//! four times as far, at very sharp corners).
//!
//! **Hard edges carry over.** The new border copies the old border's hard
//! edges, and a wall's upright edge is hard where a hard border edge meets
//! it, so an extruded cube stays crisp and an extruded sphere stays smooth.

use crate::edit::Outcome;
use crate::polygon::sides;
use crate::topology::Topology;
use crate::{EdgeKey, FaceId, Mesh, VertexId};
use glam::Vec3;
use std::collections::{BTreeMap, BTreeSet};

/// Whether neighbouring polygons (or edges) work as one piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Grouping {
    /// Polygons that share an edge (or edges that share a point) move as
    /// one piece: a block pulled out of a wall.
    #[default]
    Together,
    /// Every polygon (or edge) on its own: a separate bump for each.
    Separate,
}

/// The smallest cosine used when keeping thickness even: a point never
/// moves more than four times the distance.
const MIN_COS: f32 = 0.25;

/// The direction a point moves so that each of `directions` (unit
/// vectors: the facings of the polygons around it, say) ends up a whole
/// step away: their average, lengthened where they spread apart. Zero if
/// there is no direction to go.
fn even_direction(directions: &[Vec3]) -> Vec3 {
    let sum: Vec3 = directions.iter().copied().sum();
    let Some(direction) = sum.try_normalize() else {
        // They cancel out (a sliver folded flat): take any one of them.
        return directions
            .iter()
            .find_map(|d| d.try_normalize())
            .unwrap_or(Vec3::ZERO);
    };
    let cos = directions
        .iter()
        .filter(|d| d.length_squared() > 0.0)
        .map(|d| direction.dot(*d))
        .fold(1.0_f32, f32::min);
    direction / cos.max(MIN_COS)
}

/// The polygons of `faces` that are in the mesh, sorted and each once, and
/// how many listed weren't.
fn existing_faces(mesh: &Mesh, faces: &[FaceId]) -> (Vec<FaceId>, usize) {
    let mut found = BTreeSet::new();
    let mut missing = 0;
    for face in faces {
        if mesh.face(*face).is_some() {
            found.insert(*face);
        } else if !found.contains(face) {
            missing += 1;
        }
    }
    (found.into_iter().collect(), missing)
}

/// `faces` split into the pieces that work together: polygons joined by
/// an edge (`Together`), or each alone (`Separate`).
fn face_regions(topology: &Topology, faces: &[FaceId], grouping: Grouping) -> Vec<Vec<FaceId>> {
    if grouping == Grouping::Separate {
        return faces.iter().map(|f| vec![*f]).collect();
    }
    let chosen: BTreeSet<FaceId> = faces.iter().copied().collect();
    let mut seen = BTreeSet::new();
    let mut regions = Vec::new();
    for start in faces {
        if !seen.insert(*start) {
            continue;
        }
        let mut region = Vec::new();
        let mut stack = vec![*start];
        while let Some(face) = stack.pop() {
            region.push(face);
            for edge in topology.face_edges(face) {
                for other in topology.edge_faces(*edge) {
                    if chosen.contains(&other) && seen.insert(other) {
                        stack.push(other);
                    }
                }
            }
        }
        region.sort_unstable();
        regions.push(region);
    }
    regions
}

/// How a piece of polygons is put together.
struct Region {
    faces: Vec<FaceId>,
    /// The sides on the piece's border, each as its polygon runs along it.
    border: Vec<(FaceId, VertexId, VertexId)>,
    /// Edges with the piece on both sides.
    inner: Vec<EdgeKey>,
    /// Every corner point of the piece, and whether it lies on the border.
    points: BTreeMap<VertexId, bool>,
}

impl Region {
    fn new(mesh: &Mesh, faces: Vec<FaceId>) -> Self {
        let mut uses: BTreeMap<EdgeKey, usize> = BTreeMap::new();
        let mut points = BTreeMap::new();
        for face in &faces {
            let corners = mesh.face(*face).unwrap_or_default();
            for (a, b) in sides(corners) {
                *uses.entry(EdgeKey::new(a, b)).or_default() += 1;
            }
            for v in corners {
                points.insert(*v, false);
            }
        }
        let mut border = Vec::new();
        for face in &faces {
            for (a, b) in sides(mesh.face(*face).unwrap_or_default()) {
                if uses.get(&EdgeKey::new(a, b)) == Some(&1) {
                    border.push((*face, a, b));
                    points.insert(a, true);
                    points.insert(b, true);
                }
            }
        }
        let inner = uses
            .into_iter()
            .filter(|(_, n)| *n >= 2)
            .map(|(e, _)| e)
            .collect();
        Self {
            faces,
            border,
            inner,
            points,
        }
    }

    /// Gives the piece's polygons their new corners (`map`), and carries
    /// hard edges inside the piece over to the new corners.
    fn rewire(&self, original: &Mesh, mesh: &mut Mesh, map: &BTreeMap<VertexId, VertexId>) {
        let to = |v: VertexId| map.get(&v).copied().unwrap_or(v);
        for face in &self.faces {
            let corners: Vec<VertexId> = original
                .face(*face)
                .unwrap_or_default()
                .iter()
                .map(|v| to(*v))
                .collect();
            // Can't fail: the same number of distinct, existing corners.
            let _ = mesh.replace_corners(*face, &corners);
        }
        for edge in &self.inner {
            let (a, b) = edge.ends();
            let (x, y) = (to(a), to(b));
            if (x, y) != (a, b) && original.is_hard(a, b) {
                mesh.set_hard(a, b, false);
                mesh.set_hard(x, y, true);
            }
        }
    }

    /// Adds a four-sided polygon along each border side, from the old
    /// border to the new one, and copies the border's hard edges to the
    /// new border. With `hard_uprights`, a wall's upright edge is hard
    /// where a hard border edge meets it.
    fn walls(
        &self,
        original: &Mesh,
        mesh: &mut Mesh,
        map: &BTreeMap<VertexId, VertexId>,
        hard_uprights: bool,
    ) {
        let mut hard_points = BTreeSet::new();
        for (_, a, b) in &self.border {
            let (Some(x), Some(y)) = (map.get(a), map.get(b)) else {
                continue;
            };
            mesh.push_face(&[*a, *b, *y, *x]);
            if original.is_hard(*a, *b) {
                mesh.set_hard(*x, *y, true);
                hard_points.insert(*a);
                hard_points.insert(*b);
            }
        }
        if hard_uprights {
            for v in hard_points {
                if let Some(copy) = map.get(&v) {
                    mesh.set_hard(v, *copy, true);
                }
            }
        }
    }
}

/// Extrudes `faces` by `distance` (in the mesh's own units; negative
/// pushes them in), as one piece where they touch or each on its own
/// (`grouping`). The polygons keep their ids; the walls are new.
///
/// With [`Grouping::Together`], a point inside the piece (every polygon
/// around it chosen, and not on its border) simply moves; points on the
/// border get a copy that moves, and the old point stays for the walls.
/// A piece covering a whole closed shape has no border, so it grows (or
/// shrinks) instead.
///
/// Polygons not in the mesh are counted as missing. A distance that isn't
/// a number changes nothing.
pub fn extrude_polygons(
    mesh: &mut Mesh,
    faces: &[FaceId],
    distance: f32,
    grouping: Grouping,
) -> Outcome {
    let (faces, missing) = existing_faces(mesh, faces);
    let outcome = Outcome {
        matched: faces.len(),
        missing,
        skipped: 0,
    };
    if faces.is_empty() || !distance.is_finite() {
        return outcome;
    }
    let original = mesh.clone();
    let topology = original.topology();
    for faces in face_regions(topology, &faces, grouping) {
        let region = Region::new(&original, faces);
        let chosen: BTreeSet<FaceId> = region.faces.iter().copied().collect();
        // Each point's facings, from the piece's polygons around it.
        let mut facings: BTreeMap<VertexId, Vec<Vec3>> = BTreeMap::new();
        for face in &region.faces {
            let normal = original.face_normal(*face);
            for v in original.face(*face).unwrap_or_default() {
                facings.entry(*v).or_default().push(normal);
            }
        }
        let mut map = BTreeMap::new();
        for (v, on_border) in &region.points {
            let Some(at) = original.position(*v) else {
                continue;
            };
            let offset = even_direction(facings.get(v).map_or(&[][..], Vec::as_slice)) * distance;
            let to = if (at + offset).is_finite() {
                at + offset
            } else {
                at
            };
            // A point shared with a polygon outside the piece must stay
            // for that polygon.
            let shared = topology
                .vertex_faces(*v)
                .iter()
                .any(|f| !chosen.contains(f));
            if *on_border || shared {
                map.insert(*v, mesh.push_vertex(to));
            } else {
                mesh.set_position(*v, to);
            }
        }
        region.rewire(&original, mesh, &map);
        region.walls(&original, mesh, &map, true);
    }
    outcome
}

/// Insets `faces` by `distance` (in the mesh's own units, inward; negative
/// grows them outward), as one piece where they touch or each on its own
/// (`grouping`). The polygons keep their ids and their plane; the ring
/// around them is new.
///
/// Only the piece's border moves in: points inside a piece stay where they
/// are. Insetting further than a polygon is wide makes it cross over
/// itself, as in other modelling programs; the tool keeps the distance
/// sensible.
///
/// Polygons not in the mesh are counted as missing. A distance that isn't
/// a number changes nothing.
pub fn inset_polygons(
    mesh: &mut Mesh,
    faces: &[FaceId],
    distance: f32,
    grouping: Grouping,
) -> Outcome {
    let (faces, missing) = existing_faces(mesh, faces);
    let outcome = Outcome {
        matched: faces.len(),
        missing,
        skipped: 0,
    };
    if faces.is_empty() || !distance.is_finite() {
        return outcome;
    }
    let original = mesh.clone();
    let topology = original.topology();
    for faces in face_regions(topology, &faces, grouping) {
        let region = Region::new(&original, faces);
        // Each border point's inward directions, from the border sides
        // meeting there.
        let mut inward: BTreeMap<VertexId, Vec<Vec3>> = BTreeMap::new();
        for (face, a, b) in &region.border {
            let (Some(pa), Some(pb)) = (original.position(*a), original.position(*b)) else {
                continue;
            };
            let direction = original
                .face_normal(*face)
                .cross(pb - pa)
                .normalize_or_zero();
            inward.entry(*a).or_default().push(direction);
            inward.entry(*b).or_default().push(direction);
        }
        let mut map = BTreeMap::new();
        for (v, directions) in &inward {
            let Some(at) = original.position(*v) else {
                continue;
            };
            let offset = even_direction(directions) * distance;
            let to = if (at + offset).is_finite() {
                at + offset
            } else {
                at
            };
            map.insert(*v, mesh.push_vertex(to));
        }
        region.rewire(&original, mesh, &map);
        // The ring is flat with the polygons: no hard uprights.
        region.walls(&original, mesh, &map, false);
    }
    outcome
}

/// Extrudes `edges` by `distance` (in the mesh's own units): each edge on
/// an open border grows a new four-sided polygon outward, flat with the
/// polygon it belongs to. Edges joined at a point grow together
/// ([`Grouping::Together`]) or each on its own.
///
/// Edges not in the mesh are counted as missing; edges with polygons on
/// both sides (or none) are skipped, as there is no open side to grow
/// from. A distance that isn't a number changes nothing.
pub fn extrude_edges(
    mesh: &mut Mesh,
    edges: &[EdgeKey],
    distance: f32,
    grouping: Grouping,
) -> Outcome {
    let original = mesh.clone();
    let topology = original.topology();
    let mut outcome = Outcome::default();
    // Each border edge as its polygon runs along it, with the outward
    // direction.
    let mut border: BTreeMap<EdgeKey, (VertexId, VertexId, Vec3)> = BTreeMap::new();
    for edge in edges.iter().copied().collect::<BTreeSet<_>>() {
        if !topology.has_edge(edge) {
            outcome.missing += 1;
            continue;
        }
        let [only] = topology.edge_uses(edge) else {
            outcome.skipped += 1;
            continue;
        };
        let (low, high) = edge.ends();
        let (a, b) = if only.forward {
            (low, high)
        } else {
            (high, low)
        };
        let (Some(pa), Some(pb)) = (original.position(a), original.position(b)) else {
            outcome.missing += 1;
            continue;
        };
        let outward = (pb - pa)
            .cross(original.face_normal(only.face))
            .normalize_or_zero();
        border.insert(edge, (a, b, outward));
        outcome.matched += 1;
    }
    if border.is_empty() || !distance.is_finite() {
        return outcome;
    }
    for group in edge_regions(&border, grouping) {
        let mut outward: BTreeMap<VertexId, Vec<Vec3>> = BTreeMap::new();
        for edge in &group {
            let (a, b, direction) = border[edge];
            outward.entry(a).or_default().push(direction);
            outward.entry(b).or_default().push(direction);
        }
        let mut map = BTreeMap::new();
        for (v, directions) in &outward {
            let Some(at) = original.position(*v) else {
                continue;
            };
            let offset = even_direction(directions) * distance;
            let to = if (at + offset).is_finite() {
                at + offset
            } else {
                at
            };
            map.insert(*v, mesh.push_vertex(to));
        }
        for edge in &group {
            let (a, b, _) = border[edge];
            // The polygon runs a → b, so the new one runs b → a.
            mesh.push_face(&[b, a, map[&a], map[&b]]);
        }
    }
    outcome
}

/// Border edges split into the groups that grow together: joined at a
/// point (`Together`), or each alone (`Separate`).
fn edge_regions(
    border: &BTreeMap<EdgeKey, (VertexId, VertexId, Vec3)>,
    grouping: Grouping,
) -> Vec<Vec<EdgeKey>> {
    if grouping == Grouping::Separate {
        return border.keys().map(|e| vec![*e]).collect();
    }
    let mut at_point: BTreeMap<VertexId, Vec<EdgeKey>> = BTreeMap::new();
    for edge in border.keys() {
        let (a, b) = edge.ends();
        at_point.entry(a).or_default().push(*edge);
        at_point.entry(b).or_default().push(*edge);
    }
    let mut seen = BTreeSet::new();
    let mut groups = Vec::new();
    for start in border.keys() {
        if !seen.insert(*start) {
            continue;
        }
        let mut group = Vec::new();
        let mut stack = vec![*start];
        while let Some(edge) = stack.pop() {
            group.push(edge);
            let (a, b) = edge.ends();
            for point in [a, b] {
                for other in &at_point[&point] {
                    if seen.insert(*other) {
                        stack.push(*other);
                    }
                }
            }
        }
        group.sort_unstable();
        groups.push(group);
    }
    groups
}

#[cfg(test)]
mod tests;
