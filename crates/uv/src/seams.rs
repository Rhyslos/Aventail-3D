//! Where to cut a surface open so it lies flat ([decision 0098](../../../docs/decisions/0098-seams.md)).
//!
//! A piece of surface can be laid flat in one go only if it is, in shape,
//! a **disk**: one piece, with one border all the way round and no
//! handles. A cube's side is a disk; a tube's side (two borders) or a
//! whole sphere (no border) isn't, and needs cutting first. [`is_disk`]
//! tells them apart.
//!
//! [`suggest`] picks seams for a whole shape the way a hard-surface artist
//! would:
//!
//! 1. Cut along the hard edges (a normal-map bake needs a UV split
//!    wherever the shading breaks) and along sharp corners.
//! 2. Make every piece that's left a disk with as little cutting as
//!    possible, hiding the cuts: a piece with no border is first cut along
//!    a hidden path from its lowest point to its highest; then the edges
//!    to keep joined are chosen by how visible they are (a spanning tree
//!    of the polygons, most visible edges first), and the rest are cut,
//!    trimmed back to what is needed (cuts that lead nowhere are joined
//!    again).
//!
//! "Hidden" means facing down or to the back (away from the Front view),
//! in a concave corner, or on a corner rather than across a flat stretch:
//! the places a viewer least looks at.

use glam::Vec3;
use mesh::{EdgeKey, FaceId, Mesh, VertexId};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap};

/// Corners sharper than this (degrees between the two polygons' facings)
/// are cut as well as hard edges: a piece bent further than this round
/// one edge stretches badly when laid flat.
pub const CORNER_ANGLE: f32 = 66.0;

/// A simple union-find over `0..n`.
struct Sets {
    parent: Vec<usize>,
}

impl Sets {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
        }
    }

    fn find(&mut self, mut x: usize) -> usize {
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]];
            x = self.parent[x];
        }
        x
    }

    /// Joins the sets of `a` and `b`; whether they were apart.
    fn join(&mut self, a: usize, b: usize) -> bool {
        let (a, b) = (self.find(a), self.find(b));
        if a == b {
            return false;
        }
        self.parent[a] = b;
        true
    }
}

/// The edges shared by exactly two polygons: the only edges a seam can be
/// on (an open edge is cut already; an edge of three or more polygons
/// can't be laid flat joined).
fn inner_edges(mesh: &Mesh) -> Vec<EdgeKey> {
    let topology = mesh.topology();
    topology
        .edges()
        .filter(|e| topology.edge_uses(*e).len() == 2)
        .collect()
}

/// The two polygons of an inner edge.
fn sides(mesh: &Mesh, edge: EdgeKey) -> Option<(FaceId, FaceId)> {
    match mesh.topology().edge_uses(edge) {
        [a, b] if a.face != b.face => Some((a.face, b.face)),
        _ => None,
    }
}

/// The pieces left when `cuts` are cut open: polygons joined across inner
/// edges that aren't cut, each piece in id order, in order of their first
/// polygon.
pub fn pieces(mesh: &Mesh, cuts: &BTreeSet<EdgeKey>) -> Vec<Vec<FaceId>> {
    let faces: Vec<FaceId> = mesh.faces().map(|(f, _)| f).collect();
    let slot: HashMap<FaceId, usize> = faces.iter().enumerate().map(|(i, f)| (*f, i)).collect();
    let mut sets = Sets::new(faces.len());
    for edge in inner_edges(mesh) {
        if cuts.contains(&edge) {
            continue;
        }
        if let Some((a, b)) = sides(mesh, edge) {
            sets.join(slot[&a], slot[&b]);
        }
    }
    let mut groups: BTreeMap<usize, Vec<FaceId>> = BTreeMap::new();
    let mut order: Vec<usize> = Vec::new();
    for (i, f) in faces.iter().enumerate() {
        let root = sets.find(i);
        let group = groups.entry(root).or_default();
        if group.is_empty() {
            order.push(root);
        }
        group.push(*f);
    }
    order
        .into_iter()
        .filter_map(|root| groups.remove(&root))
        .collect()
}

/// Whether `piece` (polygons of `mesh`) lies flat in one go once `cuts`
/// are cut open: one piece, shaped like a disk (one border, no handles).
/// Worked out from the Euler characteristic of the cut-open piece, which
/// is 1 exactly for a disk.
pub fn is_disk(mesh: &Mesh, piece: &[FaceId], cuts: &BTreeSet<EdgeKey>) -> bool {
    if piece.is_empty() {
        return false;
    }
    let inside: BTreeSet<FaceId> = piece.iter().copied().collect();
    let topology = mesh.topology();
    // The corners of the piece: one per polygon and point. Corners of a
    // point are one point of the cut-open piece when joined across an
    // edge that isn't cut.
    let mut corner_index: HashMap<(FaceId, VertexId), usize> = HashMap::new();
    let mut edges_inside: BTreeSet<EdgeKey> = BTreeSet::new();
    let mut edge_sides = 0usize;
    for f in piece {
        let Some(points) = mesh.face(*f) else {
            return false;
        };
        for (k, v) in points.iter().enumerate() {
            let next = corner_index.len();
            corner_index.entry((*f, *v)).or_insert(next);
            let w = points[(k + 1) % points.len()];
            edges_inside.insert(EdgeKey::new(*v, w));
        }
    }
    let mut sets = Sets::new(corner_index.len());
    let mut joined_edges = 0usize;
    for edge in &edges_inside {
        let uses: Vec<FaceId> = topology
            .edge_faces(*edge)
            .filter(|f| inside.contains(f))
            .collect();
        let joined = uses.len() == 2
            && topology.edge_uses(*edge).len() == 2
            && !cuts.contains(edge)
            && uses[0] != uses[1];
        if joined {
            joined_edges += 1;
            let (a, b) = edge.ends();
            for v in [a, b] {
                if let (Some(x), Some(y)) = (
                    corner_index.get(&(uses[0], v)),
                    corner_index.get(&(uses[1], v)),
                ) {
                    sets.join(*x, *y);
                }
            }
        } else {
            edge_sides += uses.len();
        }
    }
    let mut roots = BTreeSet::new();
    for i in 0..corner_index.len() {
        roots.insert(sets.find(i));
    }
    // The piece must hang together across its joined edges.
    let mut faces = Sets::new(piece.len());
    let slot: HashMap<FaceId, usize> = piece.iter().enumerate().map(|(i, f)| (*f, i)).collect();
    for edge in &edges_inside {
        let uses: Vec<FaceId> = topology
            .edge_faces(*edge)
            .filter(|f| inside.contains(f))
            .collect();
        if uses.len() == 2 && topology.edge_uses(*edge).len() == 2 && !cuts.contains(edge) {
            faces.join(slot[&uses[0]], slot[&uses[1]]);
        }
    }
    let one_piece = (0..piece.len()).all(|i| faces.find(i) == faces.find(0));
    let points = roots.len() as i64;
    let edges = (joined_edges + edge_sides) as i64;
    let euler = points - edges + piece.len() as i64;
    one_piece && euler == 1
}

/// The middle of polygon `f`.
fn middle(mesh: &Mesh, f: FaceId) -> Vec3 {
    let points = mesh.face(f).unwrap_or(&[]);
    let sum: Vec3 = points.iter().filter_map(|v| mesh.position(*v)).sum();
    sum / points.len().max(1) as f32
}

/// How much `edge` is seen: higher for edges facing up or to the front,
/// across flat stretches and on outward corners; lower facing down or to
/// the back (+Y, away from the Front view), in concave corners and on
/// sharp ones. Cuts go where this is low.
fn visibility(mesh: &Mesh, edge: EdgeKey) -> f32 {
    let Some((f, g)) = sides(mesh, edge) else {
        return 0.0;
    };
    let (nf, ng) = (mesh.face_normal(f), mesh.face_normal(g));
    let facing = (nf + ng).normalize_or_zero();
    let flat = nf.dot(ng).clamp(-1.0, 1.0);
    let concave = nf.dot(middle(mesh, g) - middle(mesh, f)) > 1e-6;
    // The front view looks along +Y: the back faces +Y.
    2.0 + facing.z - 0.5 * facing.y + 0.5 * flat - if concave { 1.0 } else { 0.0 }
}

/// A path's cost along `edge`: its length, more where it is seen.
fn path_cost(mesh: &Mesh, edge: EdgeKey) -> f32 {
    let (a, b) = edge.ends();
    let length = match (mesh.position(a), mesh.position(b)) {
        (Some(p), Some(q)) => p.distance(q),
        _ => 1.0,
    };
    length.max(1e-6) * (visibility(mesh, edge) + 2.0).max(0.1)
}

/// An entry of the path search's queue (smallest cost first).
#[derive(PartialEq)]
struct Step(f32, VertexId);

impl Eq for Step {}

impl PartialOrd for Step {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Step {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.total_cmp(&self.0).then(other.1.cmp(&self.1))
    }
}

/// The cheapest path from `from` to `to` along `edges`, as edges.
fn hidden_path(mesh: &Mesh, edges: &[EdgeKey], from: VertexId, to: VertexId) -> Vec<EdgeKey> {
    let mut next: HashMap<VertexId, Vec<(VertexId, EdgeKey, f32)>> = HashMap::new();
    for e in edges {
        let (a, b) = e.ends();
        let cost = path_cost(mesh, *e);
        next.entry(a).or_default().push((b, *e, cost));
        next.entry(b).or_default().push((a, *e, cost));
    }
    let mut best: HashMap<VertexId, (f32, Option<(VertexId, EdgeKey)>)> = HashMap::new();
    best.insert(from, (0.0, None));
    let mut queue = BinaryHeap::new();
    queue.push(Step(0.0, from));
    while let Some(Step(cost, v)) = queue.pop() {
        if v == to {
            break;
        }
        if best.get(&v).is_some_and(|(c, _)| *c < cost) {
            continue;
        }
        for (w, e, step) in next.get(&v).map(Vec::as_slice).unwrap_or(&[]) {
            let total = cost + step;
            if best.get(w).is_none_or(|(c, _)| total < *c) {
                best.insert(*w, (total, Some((v, *e))));
                queue.push(Step(total, *w));
            }
        }
    }
    let mut path = Vec::new();
    let mut at = to;
    while let Some((_, Some((back, e)))) = best.get(&at) {
        path.push(*e);
        at = *back;
        if path.len() > edges.len() {
            return Vec::new();
        }
    }
    if at == from { path } else { Vec::new() }
}

/// The cuts that make `piece` a disk, given what is cut already: none for
/// a disk; for a piece with no border, a hidden path from its lowest
/// point to its highest first; then what a spanning tree of its polygons
/// (most visible edges kept joined) leaves, trimmed back.
fn disk_cuts(mesh: &Mesh, piece: &[FaceId], cuts: &BTreeSet<EdgeKey>) -> BTreeSet<EdgeKey> {
    let mut added = BTreeSet::new();
    if is_disk(mesh, piece, cuts) {
        return added;
    }
    let topology = mesh.topology();
    let inside: BTreeSet<FaceId> = piece.iter().copied().collect();
    let mut joined: Vec<EdgeKey> = Vec::new();
    let mut border: Vec<EdgeKey> = Vec::new();
    let mut seen = BTreeSet::new();
    for f in piece {
        let points = mesh.face(*f).unwrap_or(&[]);
        for (k, v) in points.iter().enumerate() {
            let e = EdgeKey::new(*v, points[(k + 1) % points.len()]);
            if !seen.insert(e) {
                continue;
            }
            let both_inside = topology
                .edge_faces(e)
                .filter(|g| inside.contains(g))
                .count()
                == 2;
            if both_inside && topology.edge_uses(e).len() == 2 && !cuts.contains(&e) {
                joined.push(e);
            } else {
                border.push(e);
            }
        }
    }
    // No border at all: open it along a hidden path, bottom to top.
    if border.is_empty() {
        let points: BTreeSet<VertexId> = joined
            .iter()
            .flat_map(|e| {
                let (a, b) = e.ends();
                [a, b]
            })
            .collect();
        let height = |v: &VertexId| mesh.position(*v).map_or(0.0, |p| p.z);
        let low = points
            .iter()
            .copied()
            .min_by(|a, b| height(a).total_cmp(&height(b)).then(a.cmp(b)));
        let high = points
            .iter()
            .copied()
            .max_by(|a, b| height(a).total_cmp(&height(b)).then(b.cmp(a)));
        let mut path = match (low, high) {
            (Some(l), Some(h)) if l != h => hidden_path(mesh, &joined, l, h),
            _ => Vec::new(),
        };
        if path.is_empty()
            && let Some(e) = joined
                .iter()
                .copied()
                .min_by(|a, b| visibility(mesh, *a).total_cmp(&visibility(mesh, *b)))
        {
            path.push(e);
        }
        for e in path {
            added.insert(e);
            border.push(e);
        }
        joined.retain(|e| !added.contains(e));
    }
    // Keep the most visible edges joined: a spanning tree of the polygons.
    let slot: HashMap<FaceId, usize> = piece.iter().enumerate().map(|(i, f)| (*f, i)).collect();
    let mut order: Vec<(f32, EdgeKey)> =
        joined.iter().map(|e| (visibility(mesh, *e), *e)).collect();
    order.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut sets = Sets::new(piece.len());
    let mut open: BTreeSet<EdgeKey> = BTreeSet::new();
    for (_, e) in order {
        let Some((a, b)) = sides(mesh, e) else {
            continue;
        };
        if !sets.join(slot[&a], slot[&b]) {
            open.insert(e);
        }
    }
    // Trim cuts that lead nowhere: an end only one cut (or border) meets
    // can be joined again.
    let mut degree: HashMap<VertexId, usize> = HashMap::new();
    for e in open.iter().chain(border.iter()) {
        let (a, b) = e.ends();
        *degree.entry(a).or_default() += 1;
        *degree.entry(b).or_default() += 1;
    }
    let mut changed = true;
    while changed {
        changed = false;
        let dangling: Vec<EdgeKey> = open
            .iter()
            .copied()
            .filter(|e| {
                let (a, b) = e.ends();
                degree.get(&a) == Some(&1) || degree.get(&b) == Some(&1)
            })
            .collect();
        for e in dangling {
            open.remove(&e);
            let (a, b) = e.ends();
            for v in [a, b] {
                if let Some(d) = degree.get_mut(&v) {
                    *d -= 1;
                }
            }
            changed = true;
        }
    }
    added.extend(open);
    added
}

/// Seams for the whole of `mesh`, as a hard-surface artist would place
/// them: along `hard` (the edges shading hard, as shown) and corners
/// sharper than [`CORNER_ANGLE`], then whatever each piece needs to lie
/// flat, in hidden places. Inner edges only, in order.
pub fn suggest(mesh: &Mesh, hard: &[EdgeKey]) -> Vec<EdgeKey> {
    let inner: BTreeSet<EdgeKey> = inner_edges(mesh).into_iter().collect();
    let mut cuts: BTreeSet<EdgeKey> = hard
        .iter()
        .copied()
        .chain(mesh::hardness::sharp_edges(mesh, CORNER_ANGLE))
        .filter(|e| inner.contains(e))
        .collect();
    for piece in pieces(mesh, &cuts) {
        let more = disk_cuts(mesh, &piece, &cuts);
        cuts.extend(more);
    }
    cuts.into_iter().filter(|e| inner.contains(e)).collect()
}

#[cfg(test)]
mod tests;
