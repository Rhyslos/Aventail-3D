//! Sliding points along their edges, so they stay on the surface
//! ([decision 0055](../../../docs/decisions/0055-basics.md)).
//!
//! Each point slides towards one of its neighbours, by a share of the
//! edge between them: 0 stays put, 1 reaches the neighbour. [`slides_for`]
//! works out, for a move the user drags, which neighbour each point
//! follows and how far; [`slide_points`] does it.

use crate::edit::Outcome;
use crate::{Mesh, VertexId};
use glam::Vec3;
use std::collections::BTreeSet;

/// One point's slide: from where, towards which neighbour, and how far
/// along the edge (0 to 1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slide {
    /// The point that moves.
    pub point: VertexId,
    /// The neighbour it moves towards.
    pub towards: VertexId,
    /// How far along the edge, 0 (where it is) to 1 (at the neighbour).
    pub amount: f32,
}

/// The slides that follow a move by `offset` (in the shape's own space)
/// as closely as the edges allow: each point slides along the edge to a
/// neighbour that points most nearly the way of the move (preferring
/// neighbours that aren't moving themselves), as far as the move reaches
/// along it. A point with no edge going that way stays (amount 0).
pub fn slides_for(mesh: &Mesh, points: &[VertexId], offset: Vec3) -> Vec<Slide> {
    let moving: BTreeSet<VertexId> = points.iter().copied().collect();
    let topology = mesh.topology();
    let way = offset.normalize_or_zero();
    let mut out = Vec::with_capacity(moving.len());
    for &v in &moving {
        let Some(p) = mesh.position(v) else {
            continue;
        };
        let neighbours: Vec<(VertexId, Vec3)> = topology
            .vertex_edges(v)
            .iter()
            .filter_map(|e| {
                let (a, b) = e.ends();
                let other = if a == v { b } else { a };
                Some((other, mesh.position(other)? - p))
            })
            .filter(|(_, along)| along.length_squared() > 1e-12)
            .collect();
        // Neighbours staying put first: an edge between two moving points
        // would carry the point along with the other.
        let still: Vec<&(VertexId, Vec3)> = neighbours
            .iter()
            .filter(|(n, _)| !moving.contains(n))
            .collect();
        let pool: Vec<&(VertexId, Vec3)> = if still.is_empty() {
            neighbours.iter().collect()
        } else {
            still
        };
        let best = pool
            .into_iter()
            .map(|(n, along)| (*n, *along, along.normalize_or_zero().dot(way)))
            .max_by(|a, b| a.2.total_cmp(&b.2).then(b.0.cmp(&a.0)));
        let Some((towards, along, facing)) = best else {
            continue;
        };
        let amount = if facing > 0.0 {
            (offset.dot(along) / along.length_squared()).clamp(0.0, 1.0)
        } else {
            0.0
        };
        out.push(Slide {
            point: v,
            towards,
            amount,
        });
    }
    out
}

/// Slides each point towards its neighbour by its amount (brought within
/// 0 to 1). Points not in the mesh are missing. All points move from where
/// they were, so points sliding towards each other meet in step.
pub fn slide_points(mesh: &mut Mesh, slides: &[Slide]) -> Outcome {
    let mut outcome = Outcome::default();
    let mut moves = Vec::with_capacity(slides.len());
    for slide in slides {
        match (mesh.position(slide.point), mesh.position(slide.towards)) {
            (Some(p), Some(q)) if slide.point != slide.towards => {
                let t = if slide.amount.is_finite() {
                    slide.amount.clamp(0.0, 1.0)
                } else {
                    0.0
                };
                moves.push((slide.point, p + (q - p) * t));
            }
            _ => outcome.missing += 1,
        }
    }
    for (v, at) in moves {
        mesh.set_position(v, at);
        outcome.matched += 1;
    }
    outcome
}

#[cfg(test)]
mod tests;
