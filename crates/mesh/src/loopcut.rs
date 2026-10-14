//! Loop cut: a new loop of edges all the way round, through a ring of
//! four-sided polygons ([decision 0047](../../../docs/decisions/0047-loop-cut.md)).
//!
//! Pick an edge, and the cut runs across it and on through every
//! four-sided polygon in its *ring* (from each polygon to the edge
//! opposite, the way [`crate::Topology::edge_ring`] walks), splitting each
//! one. Several evenly spaced cuts can be made at once. Where the ring
//! ends (a border, a triangle, a polygon with five or more sides), the new
//! points are added to that polygon's corners so no gaps open.
//!
//! **Where the cut sits.** `position` runs from 0 to 1 across the picked
//! edge, from its lower point id to its higher: 0.5 is the middle. Every
//! edge of the ring is cut at the same share of its length, measured from
//! the same side, so the loop runs straight across polygons of different
//! sizes. With several cuts, they are spread evenly (a third and two
//! thirds of the way for two), and `position` slides them together.
//!
//! As with every edit, polygons keep their ids (each cut polygon keeps its
//! id for its first piece), new points and polygons take the next ids in
//! a fixed order, and attribute values are carried over through the
//! [`Builder`]: new points and corners blend the two ends they lie
//! between, new polygons are like the one they were cut from, and a cut
//! edge's values (hard, seam) go to each of its pieces.

use crate::builder::{Builder, CornerFrom, PointFrom};
use crate::edit::Outcome;
use crate::topology::Topology;
use crate::{EdgeKey, FaceId, Mesh, VertexId};
use glam::Vec3;
use std::collections::{BTreeMap, BTreeSet};

/// The most cuts one loop cut can make.
pub const MAX_CUTS: u32 = 64;

/// How close to the ends of an edge a cut may sit, as a share of it.
const EDGE_MARGIN: f32 = 0.01;

/// An edge of the ring, from the side the cut is measured from.
type Directed = (VertexId, VertexId);

/// The ring through `edge`: its edges, each from the side cuts are
/// measured from, and each polygon it crosses with the edge it enters by
/// and the edge it leaves by. Empty when `edge` has no four-sided polygon
/// next to it.
struct Ring {
    edges: Vec<Directed>,
    crossings: Vec<(FaceId, Directed, Directed)>,
}

/// The edge of a four-sided polygon `face` opposite `(p, q)`, from the
/// same side: in a polygon a–b–c–d, opposite (a, b) is (d, c).
fn opposite(mesh: &Mesh, face: FaceId, (p, q): Directed) -> Option<Directed> {
    let corners = mesh.face(face)?;
    if corners.len() != 4 {
        return None;
    }
    let at = |v| corners.iter().position(|c| *c == v);
    let (i, j) = (at(p)?, at(q)?);
    // p and q are next to each other; each one's other neighbour is
    // across the polygon from it.
    let other = |k: usize, not: usize| {
        let (before, after) = ((k + 3) % 4, (k + 1) % 4);
        if before == not { after } else { before }
    };
    Some((corners[other(i, j)], corners[other(j, i)]))
}

impl Ring {
    fn walk(mesh: &Mesh, topology: &Topology, edge: EdgeKey) -> Self {
        let mut ring = Self {
            edges: Vec::new(),
            crossings: Vec::new(),
        };
        let uses = topology.edge_uses(edge);
        if uses.is_empty() || topology.is_non_manifold(edge) {
            return ring;
        }
        let start: Directed = edge.ends();
        let mut seen_edges = BTreeSet::from([edge]);
        let mut seen_faces = BTreeSet::new();
        // Each way from the picked edge: forwards, then backwards (in front
        // of the picked edge, so the list runs in order along the ring).
        let mut backwards: Vec<Directed> = Vec::new();
        let mut back_crossings = Vec::new();
        let faces: Vec<FaceId> = uses.iter().take(2).map(|u| u.face).collect();
        for (way, first) in faces.into_iter().enumerate() {
            let (mut from, mut through) = (start, first);
            loop {
                if !seen_faces.insert(through) {
                    break;
                }
                let Some(next) = opposite(mesh, through, from) else {
                    break;
                };
                let key = EdgeKey::new(next.0, next.1);
                if way == 0 {
                    ring.crossings.push((through, from, next));
                } else {
                    back_crossings.push((through, from, next));
                }
                // Already cut (all the way round, the ring closes on the
                // picked edge): the walk ends.
                if !seen_edges.insert(key) {
                    break;
                }
                if way == 0 {
                    ring.edges.push(next);
                } else {
                    backwards.push(next);
                }
                // An edge shared by three or more polygons is cut, and the
                // polygons beyond it only take the new points.
                if topology.is_non_manifold(key) {
                    break;
                }
                let Some(after) = topology.edge_faces(key).find(|f| *f != through) else {
                    break;
                };
                from = next;
                through = after;
            }
        }
        backwards.reverse();
        let mut edges = backwards;
        edges.push(start);
        edges.append(&mut ring.edges);
        ring.edges = edges;
        ring.crossings.extend(back_crossings);
        ring
    }
}

/// Where each of `cuts` cuts sits along every ring edge, as a share of it,
/// with the group slid to `position` (0.5: centred) and kept inside the
/// edge.
fn shares(cuts: u32, position: f32) -> Vec<f32> {
    let n = cuts.clamp(1, MAX_CUTS) as usize;
    let even: Vec<f32> = (1..=n).map(|k| k as f32 / (n + 1) as f32).collect();
    let position = if position.is_finite() {
        position.clamp(0.0, 1.0)
    } else {
        0.5
    };
    let (first, last) = (even[0], even[n - 1]);
    let shift = (position - 0.5).clamp(EDGE_MARGIN - first, 1.0 - EDGE_MARGIN - last);
    even.into_iter().map(|t| t + shift).collect()
}

/// How many polygons a loop cut across `edge` would cut: each gains one
/// more polygon per cut. 0 for an edge it would skip.
pub fn ring_polygons(mesh: &Mesh, edge: EdgeKey) -> usize {
    let topology = mesh.topology();
    if !topology.has_edge(edge) {
        return 0;
    }
    Ring::walk(mesh, topology, edge).crossings.len()
}

/// The lines a loop cut across `edge` would add, one per cut across each
/// polygon it crosses: what the tool shows before the click. Empty for an
/// edge it would skip.
pub fn preview(mesh: &Mesh, edge: EdgeKey, cuts: u32, position: f32) -> Vec<[Vec3; 2]> {
    let topology = mesh.topology();
    if !topology.has_edge(edge) {
        return Vec::new();
    }
    let ring = Ring::walk(mesh, topology, edge);
    let shares = shares(cuts, position);
    let mut lines = Vec::with_capacity(ring.crossings.len() * shares.len());
    for (_, (p, q), (s, r)) in &ring.crossings {
        let (Some(p), Some(q), Some(s), Some(r)) = (
            mesh.position(*p),
            mesh.position(*q),
            mesh.position(*s),
            mesh.position(*r),
        ) else {
            continue;
        };
        for t in &shares {
            lines.push([p.lerp(q, *t), s.lerp(r, *t)]);
        }
    }
    lines
}

/// Cuts a loop through the ring of four-sided polygons across `edge`:
/// `cuts` evenly spaced cuts (1 to [`MAX_CUTS`]), slid to `position` (0 to
/// 1 across `edge`, from its lower point id; 0.5 is the middle). An edge
/// not in the mesh is counted as missing; one with no four-sided polygon
/// next to it is skipped. A position that isn't a number cuts through the
/// middle.
pub fn loop_cut(mesh: &mut Mesh, edge: EdgeKey, cuts: u32, position: f32) -> Outcome {
    let original = mesh.clone();
    let topology = original.topology();
    if !topology.has_edge(edge) {
        return Outcome {
            missing: 1,
            ..Outcome::default()
        };
    }
    let ring = Ring::walk(&original, topology, edge);
    if ring.crossings.is_empty() {
        return Outcome {
            skipped: 1,
            ..Outcome::default()
        };
    }
    let shares = shares(cuts, position);
    let mut build = Builder::new(&original, mesh);

    // 1. The new points along every ring edge, from its measuring side.
    let mut along: BTreeMap<Directed, Vec<VertexId>> = BTreeMap::new();
    for (p, q) in &ring.edges {
        let (Some(a), Some(b)) = (original.position(*p), original.position(*q)) else {
            continue;
        };
        let points = shares
            .iter()
            .map(|t| build.point(a.lerp(b, *t), PointFrom::Between(*p, *q, *t)))
            .collect();
        along.insert((*p, *q), points);
    }
    // The points along an edge from either side.
    let points_on = |p: VertexId, q: VertexId| -> Option<Vec<VertexId>> {
        along.get(&(p, q)).cloned().or_else(|| {
            along.get(&(q, p)).map(|l| {
                let mut l = l.clone();
                l.reverse();
                l
            })
        })
    };

    // 2. Each crossed polygon becomes a stack of strips, one more than the
    // cuts; the first keeps its id.
    let crossed: BTreeSet<FaceId> = ring.crossings.iter().map(|c| c.0).collect();
    let mut rewrites: BTreeMap<FaceId, Vec<(VertexId, CornerFrom)>> = BTreeMap::new();
    let mut new_faces = Vec::new();
    for (face, (p, q), (s, r)) in &ring.crossings {
        let Some(corners) = original.face(*face) else {
            continue;
        };
        let (Some(enter), Some(leave)) = (points_on(*p, *q), points_on(*s, *r)) else {
            continue;
        };
        // The rails from the entry edge to the exit edge: the polygon's
        // own sides, and one per cut.
        let mut rails: Vec<(VertexId, VertexId, CornerFrom, CornerFrom)> = vec![(
            *p,
            *s,
            CornerFrom::Copy(*face, *p),
            CornerFrom::Copy(*face, *s),
        )];
        for (k, t) in shares.iter().enumerate() {
            rails.push((
                enter[k],
                leave[k],
                CornerFrom::Between(*face, *p, *face, *q, *t),
                CornerFrom::Between(*face, *s, *face, *r, *t),
            ));
        }
        rails.push((
            *q,
            *r,
            CornerFrom::Copy(*face, *q),
            CornerFrom::Copy(*face, *r),
        ));
        // Keep the polygon's winding: does it run p → q?
        let i = corners.iter().position(|c| c == p).unwrap_or(0);
        let forward = corners[(i + 1) % 4] == *q;
        for (k, pair) in rails.windows(2).enumerate() {
            let (a, b) = (pair[0], pair[1]);
            let mut strip = vec![(a.0, a.2), (b.0, b.2), (b.1, b.3), (a.1, a.3)];
            if !forward {
                strip.reverse();
            }
            if k == 0 {
                rewrites.insert(*face, strip);
            } else {
                new_faces.push((strip, *face));
            }
        }
    }

    // 3. Polygons at the ring's ends (and any other polygon using a cut
    // edge) take the new points into their corners, so no gap opens.
    let ends: BTreeSet<FaceId> = ring
        .edges
        .iter()
        .flat_map(|(p, q)| topology.edge_faces(EdgeKey::new(*p, *q)))
        .filter(|f| !crossed.contains(f))
        .collect();
    for face in ends {
        let Some(corners) = original.face(face) else {
            continue;
        };
        let n = corners.len();
        let mut out = Vec::new();
        for (i, c) in corners.iter().enumerate() {
            out.push((*c, CornerFrom::Copy(face, *c)));
            let next = corners[(i + 1) % n];
            let Some(points) = points_on(*c, next) else {
                continue;
            };
            // The shares measured from this corner.
            let shares_here: Vec<f32> = if along.contains_key(&(*c, next)) {
                shares.clone()
            } else {
                shares.iter().rev().map(|t| 1.0 - t).collect()
            };
            for (v, t) in points.iter().zip(shares_here) {
                out.push((*v, CornerFrom::Between(face, *c, face, next, t)));
            }
        }
        rewrites.insert(face, out);
    }
    if build.rewrite_faces(&rewrites).is_err() {
        return Outcome {
            skipped: 1,
            ..Outcome::default()
        };
    }
    for (strip, like) in &new_faces {
        build.face(strip, Some(*like));
    }

    // 4. A cut edge's values (hard, seam) go to each of its pieces.
    for (p, q) in &ring.edges {
        let Some(points) = along.get(&(*p, *q)) else {
            continue;
        };
        let whole = EdgeKey::new(*p, *q);
        let mut chain = vec![*p];
        chain.extend(points.iter().copied());
        chain.push(*q);
        for pair in chain.windows(2) {
            build.copy_edge(whole, pair[0], pair[1]);
        }
        build.clear_edge(whole);
    }
    Outcome {
        matched: 1,
        missing: 0,
        skipped: 0,
    }
}

#[cfg(test)]
mod tests;
