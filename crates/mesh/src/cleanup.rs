//! Clean-up: deleting, dissolving, filling holes and putting polygons the
//! right way round ([decision 0052](../../../docs/decisions/0052-clean-up.md)).
//!
//! - **Delete** ([`delete_elements`]): points, edges or polygons go, and the
//!   polygons using them with them, leaving a hole.
//! - **Dissolve** ([`dissolve_points`], [`dissolve_edges`],
//!   [`dissolve_faces`]): edges or points go but the surface stays closed:
//!   the polygons around them merge into one.
//! - **Fill hole** ([`fill_holes`]): a polygon closes each open border loop.
//! - **Flip** ([`flip_faces`]): polygons turned inside out.
//! - **Fix normals** ([`fix_facing`], [`faces_facing_wrong`]): every polygon of
//!   a piece turned to agree with its neighbours, and a closed piece turned
//!   to face outward.
//!
//! Polygons keep their ids where they survive (a merged polygon keeps the
//! lowest id of those merged) and values come along through the
//! [`Builder`].

use crate::builder::{Builder, CornerFrom};
use crate::edit::Outcome;
use crate::{EdgeKey, FaceId, Mesh, VertexId};
use glam::Vec3;
use std::collections::{BTreeMap, BTreeSet};

/// Deletes `points`, `edges` and `faces` with every polygon using them,
/// and then any point no polygon uses. Elements not in the mesh are
/// missing; nothing to delete is skipped.
pub fn delete_elements(
    mesh: &mut Mesh,
    points: &[VertexId],
    edges: &[EdgeKey],
    faces: &[FaceId],
) -> Outcome {
    let topology = mesh.topology().clone();
    let mut gone: BTreeSet<FaceId> = BTreeSet::new();
    let mut missing = 0;
    for v in points {
        if mesh.position(*v).is_none() {
            missing += 1;
        }
        gone.extend(topology.vertex_faces(*v));
    }
    for e in edges {
        if !topology.has_edge(*e) {
            missing += 1;
        }
        gone.extend(topology.edge_faces(*e));
    }
    for f in faces {
        if mesh.face(*f).is_some() {
            gone.insert(*f);
        } else {
            missing += 1;
        }
    }
    if gone.is_empty() && points.is_empty() {
        return Outcome {
            matched: 0,
            missing,
            skipped: 1,
        };
    }
    let mut corners: Vec<VertexId> = points.to_vec();
    for f in &gone {
        corners.extend(mesh.face(*f).unwrap_or_default());
    }
    let gone: Vec<FaceId> = gone.into_iter().collect();
    let deleted = mesh.delete_faces(&gone);
    mesh.remove_unused_vertices(&corners);
    Outcome {
        matched: deleted,
        missing,
        skipped: 0,
    }
}

/// The border of a group of polygons, as one loop of points running their
/// way round, or `None` when it isn't a single loop (a ring with a hole,
/// two pieces touching at a point).
fn outline(mesh: &Mesh, faces: &BTreeSet<FaceId>) -> Option<Vec<VertexId>> {
    // Directed edges used once (the inside ones are used both ways).
    let mut directed: BTreeSet<(VertexId, VertexId)> = BTreeSet::new();
    for f in faces {
        let c = mesh.face(*f)?;
        for i in 0..c.len() {
            directed.insert((c[i], c[(i + 1) % c.len()]));
        }
    }
    let border: Vec<(VertexId, VertexId)> = directed
        .iter()
        .copied()
        .filter(|(a, b)| !directed.contains(&(*b, *a)))
        .collect();
    let mut next: BTreeMap<VertexId, VertexId> = BTreeMap::new();
    for (a, b) in &border {
        if next.insert(*a, *b).is_some() {
            return None;
        }
    }
    let start = *next.keys().next()?;
    let mut out = vec![start];
    let mut at = next[&start];
    while at != start {
        out.push(at);
        at = *next.get(&at)?;
        if out.len() > border.len() {
            return None;
        }
    }
    (out.len() == border.len() && out.len() >= 3).then_some(out)
}

/// Groups of polygons joined through the edges in `joins`.
fn groups(
    mesh: &Mesh,
    faces: &BTreeSet<FaceId>,
    joins: &BTreeSet<EdgeKey>,
) -> Vec<BTreeSet<FaceId>> {
    let topology = mesh.topology();
    let mut seen: BTreeSet<FaceId> = BTreeSet::new();
    let mut out = Vec::new();
    for f in faces {
        if !seen.insert(*f) {
            continue;
        }
        let mut group = BTreeSet::from([*f]);
        let mut stack = vec![*f];
        while let Some(at) = stack.pop() {
            for e in topology.face_edges(at) {
                if !joins.contains(e) {
                    continue;
                }
                for g in topology.edge_faces(*e) {
                    if faces.contains(&g) && seen.insert(g) {
                        group.insert(g);
                        stack.push(g);
                    }
                }
            }
        }
        out.push(group);
    }
    out
}

/// Merges each group of polygons into one with the group's outline, kept
/// as the lowest id; groups whose outline isn't a single loop stay as they
/// are. Returns how many groups merged. Quadify joins its pairs of triangles
/// with it too ([`crate::quadify`]).
pub(crate) fn merge(mesh: &mut Mesh, groups: &[BTreeSet<FaceId>]) -> usize {
    let original = mesh.clone();
    let mut rewrites: BTreeMap<FaceId, Vec<(VertexId, CornerFrom)>> = BTreeMap::new();
    let mut gone: Vec<FaceId> = Vec::new();
    for group in groups {
        if group.len() < 2 {
            continue;
        }
        let Some(loop_) = outline(&original, group) else {
            continue;
        };
        let keep = *group.iter().next().unwrap_or(&FaceId::from_raw(0));
        // Each corner's values from a merged polygon that has it.
        let corners = loop_
            .iter()
            .map(|v| {
                let from = group
                    .iter()
                    .find(|f| original.face(**f).is_some_and(|c| c.contains(v)))
                    .copied()
                    .unwrap_or(keep);
                (*v, CornerFrom::Copy(from, *v))
            })
            .collect();
        rewrites.insert(keep, corners);
        gone.extend(group.iter().skip(1));
    }
    if rewrites.is_empty() {
        return 0;
    }
    mesh.delete_faces(&gone);
    let mut build = Builder::new(&original, mesh);
    if build.rewrite_faces(&rewrites).is_err() {
        *mesh = original;
        return 0;
    }
    let inside: Vec<VertexId> = gone
        .iter()
        .flat_map(|f| original.face(*f).unwrap_or_default().iter().copied())
        .collect();
    mesh.remove_unused_vertices(&inside);
    // The edges inside the merged polygons are gone, and so are their
    // values (a hard edge there would otherwise still be drawn).
    mesh.forget_unused_edge_values();
    rewrites.len()
}

/// Points between just two edges (with `straight_only`, only those on a
/// straight run, left over where a dissolved edge met the border) are
/// taken out of their polygons, as long as every polygon keeps three
/// corners.
fn tidy(mesh: &mut Mesh, candidates: &BTreeSet<VertexId>, straight_only: bool) {
    let mut lone: Vec<VertexId> = Vec::new();
    // The two edges each lone point joins, which become one.
    let mut joined_edges: Vec<(EdgeKey, EdgeKey)> = Vec::new();
    {
        let topology = mesh.topology();
        for v in candidates {
            let edges = topology.vertex_edges(*v);
            let straight = || {
                let at = mesh.position(*v).unwrap_or(Vec3::ZERO);
                let way = |e: &EdgeKey| {
                    let (a, b) = e.ends();
                    let other = if a == *v { b } else { a };
                    (mesh.position(other).unwrap_or(at) - at).normalize_or_zero()
                };
                edges.len() == 2 && way(&edges[0]).dot(way(&edges[1])) < -0.999
            };
            if edges.len() == 2
                && (!straight_only || straight())
                && topology
                    .vertex_faces(*v)
                    .iter()
                    .all(|f| mesh.face(*f).is_some_and(|c| c.len() > 3))
            {
                lone.push(*v);
                joined_edges.push((edges[0], edges[1]));
            }
        }
    }
    if lone.is_empty() {
        return;
    }
    let original = mesh.clone();
    let lone_set: BTreeSet<VertexId> = lone.iter().copied().collect();
    let mut rewrites = BTreeMap::new();
    for (face, corners) in original.faces() {
        if !corners.iter().any(|v| lone_set.contains(v)) {
            continue;
        }
        let kept: Vec<(VertexId, CornerFrom)> = corners
            .iter()
            .filter(|v| !lone_set.contains(v))
            .map(|v| (*v, CornerFrom::Copy(face, *v)))
            .collect();
        if kept.len() >= 3 {
            rewrites.insert(face, kept);
        }
    }
    let mut build = Builder::new(&original, mesh);
    if build.rewrite_faces(&rewrites).is_err() {
        *mesh = original;
        return;
    }
    // Two hard edges meeting at a point that goes make one hard edge.
    for (first, second) in &joined_edges {
        let ends = |e: &EdgeKey| {
            let (a, b) = e.ends();
            [a, b]
        };
        let (Some(a), Some(b)) = (
            ends(first).into_iter().find(|p| !lone_set.contains(p)),
            ends(second).into_iter().find(|p| !lone_set.contains(p)),
        ) else {
            continue;
        };
        let hard = |e: &EdgeKey| {
            let (x, y) = e.ends();
            original.is_hard(x, y)
        };
        if hard(first) && hard(second) {
            build.copy_edge(*first, a, b);
        }
    }
    mesh.remove_unused_vertices(&lone);
    mesh.forget_unused_edge_values();
}

/// Dissolves `edges`: the polygons on both sides of each merge into one
/// (keeping the lowest id), and points left between just two edges by it
/// go too. Edges with a polygon on one side only, or whose polygons
/// wouldn't make one simple polygon, stay.
pub fn dissolve_edges(mesh: &mut Mesh, edges: &[EdgeKey]) -> Outcome {
    let topology = mesh.topology().clone();
    let joins: BTreeSet<EdgeKey> = edges
        .iter()
        .copied()
        .filter(|e| topology.edge_uses(*e).len() == 2)
        .collect();
    let missing = edges.iter().filter(|e| !topology.has_edge(**e)).count();
    let faces: BTreeSet<FaceId> = joins.iter().flat_map(|e| topology.edge_faces(*e)).collect();
    let found = groups(mesh, &faces, &joins);
    let merged = merge(mesh, &found);
    let ends: BTreeSet<VertexId> = joins
        .iter()
        .flat_map(|e| {
            let (a, b) = e.ends();
            [a, b]
        })
        .collect();
    tidy(mesh, &ends, true);
    Outcome {
        matched: merged,
        missing,
        skipped: usize::from(merged == 0),
    }
}

/// Dissolves polygons: each group of neighbouring ones merges into one
/// (keeping the lowest id). Groups that wouldn't make one simple polygon
/// stay.
pub fn dissolve_faces(mesh: &mut Mesh, faces: &[FaceId]) -> Outcome {
    let chosen: BTreeSet<FaceId> = faces
        .iter()
        .copied()
        .filter(|f| mesh.face(*f).is_some())
        .collect();
    let missing = faces.len().saturating_sub(chosen.len());
    let topology = mesh.topology().clone();
    // Inside edges: both polygons chosen.
    let joins: BTreeSet<EdgeKey> = chosen
        .iter()
        .flat_map(|f| topology.face_edges(*f).iter().copied())
        .filter(|e| {
            let uses = topology.edge_uses(*e);
            uses.len() == 2 && uses.iter().all(|u| chosen.contains(&u.face))
        })
        .collect();
    let found = groups(mesh, &chosen, &joins);
    let inside: BTreeSet<VertexId> = joins
        .iter()
        .flat_map(|e| {
            let (a, b) = e.ends();
            [a, b]
        })
        .collect();
    let merged = merge(mesh, &found);
    tidy(mesh, &inside, true);
    Outcome {
        matched: merged,
        missing,
        skipped: usize::from(merged == 0),
    }
}

/// Dissolves `points`: each point inside the surface goes and the
/// polygons around it merge into one; a point between just two edges goes
/// from its polygons. Points on an open border, or whose polygons wouldn't
/// make one simple polygon, stay.
pub fn dissolve_points(mesh: &mut Mesh, points: &[VertexId]) -> Outcome {
    let missing = points
        .iter()
        .filter(|v| mesh.position(**v).is_none())
        .count();
    let chosen: BTreeSet<VertexId> = points
        .iter()
        .copied()
        .filter(|v| mesh.position(*v).is_some())
        .collect();
    let topology = mesh.topology().clone();
    let mut fans = Vec::new();
    let mut lone = BTreeSet::new();
    for v in &chosen {
        let edges = topology.vertex_edges(*v);
        if edges.len() == 2 {
            lone.insert(*v);
            continue;
        }
        let on_border = edges.iter().any(|e| topology.is_border(*e));
        if on_border {
            continue;
        }
        fans.push(
            topology
                .vertex_faces(*v)
                .iter()
                .copied()
                .collect::<BTreeSet<_>>(),
        );
    }
    // Fans sharing polygons merge into one group.
    let mut joined: Vec<BTreeSet<FaceId>> = Vec::new();
    for fan in fans {
        let mut fan = fan;
        joined.retain(|g| {
            if g.is_disjoint(&fan) {
                true
            } else {
                fan.extend(g.iter().copied());
                false
            }
        });
        joined.push(fan);
    }
    let before = mesh.vertex_count();
    let merged = merge(mesh, &joined);
    tidy(mesh, &lone, false);
    let removed = before - mesh.vertex_count();
    Outcome {
        matched: merged.max(removed),
        missing,
        skipped: usize::from(merged == 0 && removed == 0),
    }
}

/// The loops of open border edges, each running the way a polygon closing
/// it must go (against the polygons beside it).
fn border_loops(mesh: &Mesh) -> Vec<Vec<VertexId>> {
    let topology = mesh.topology();
    let mut next: BTreeMap<VertexId, Vec<VertexId>> = BTreeMap::new();
    for e in topology.border_edges() {
        let [only] = topology.edge_uses(e) else {
            continue;
        };
        let (low, high) = e.ends();
        // The polygon runs low → high: the hole's polygon runs high → low.
        let (a, b) = if only.forward {
            (high, low)
        } else {
            (low, high)
        };
        next.entry(a).or_default().push(b);
    }
    let mut used: BTreeSet<(VertexId, VertexId)> = BTreeSet::new();
    let mut out = Vec::new();
    for start in next.keys().copied().collect::<Vec<_>>() {
        for first in next[&start].clone() {
            if used.contains(&(start, first)) {
                continue;
            }
            let mut loop_ = vec![start];
            used.insert((start, first));
            let mut at = first;
            let mut ok = true;
            while at != start {
                loop_.push(at);
                let Some(step) = next
                    .get(&at)
                    .and_then(|n| n.iter().copied().find(|b| !used.contains(&(at, *b))))
                else {
                    ok = false;
                    break;
                };
                used.insert((at, step));
                at = step;
            }
            if ok && loop_.len() >= 3 {
                out.push(loop_);
            }
        }
    }
    out
}

/// Fills holes: a polygon closes each loop of open border edges that has
/// one of `points` on it (every hole when `points` is empty). Returns the
/// polygons added as matched; no hole to fill is skipped.
pub fn fill_holes(mesh: &mut Mesh, points: &[VertexId]) -> Outcome {
    let wanted: BTreeSet<VertexId> = points.iter().copied().collect();
    let loops: Vec<Vec<VertexId>> = border_loops(mesh)
        .into_iter()
        .filter(|l| wanted.is_empty() || l.iter().any(|v| wanted.contains(v)))
        .collect();
    let original = mesh.clone();
    let topology = original.topology();
    let first_new = mesh.face_slots().len();
    let mut build = Builder::new(&original, mesh);
    let mut made = 0;
    for l in &loops {
        // Values like the polygon beside the first edge.
        let like = topology.edge_faces(EdgeKey::new(l[0], l[1])).next();
        let corners: Vec<(VertexId, CornerFrom)> = l
            .iter()
            .map(|v| {
                let from = topology
                    .vertex_faces(*v)
                    .first()
                    .map_or(CornerFrom::Nothing, |f| CornerFrom::Copy(*f, *v));
                (*v, from)
            })
            .collect();
        made += usize::from(build.face(&corners, like).is_some());
    }
    // A cap meeting the sides at a corner gets a crisp rim.
    crate::hardness::harden_new_edges(mesh, first_new, crate::hardness::DEFAULT_ANGLE);
    Outcome {
        matched: made,
        missing: 0,
        skipped: usize::from(made == 0),
    }
}

/// Turns `faces` inside out (their corners in the other order), keeping
/// each corner's values with its point.
pub fn flip_faces(mesh: &mut Mesh, faces: &[FaceId]) -> Outcome {
    let original = mesh.clone();
    let mut rewrites = BTreeMap::new();
    let mut missing = 0;
    for f in faces.iter().collect::<BTreeSet<_>>() {
        let Some(corners) = original.face(*f) else {
            missing += 1;
            continue;
        };
        let flipped: Vec<(VertexId, CornerFrom)> = corners
            .iter()
            .rev()
            .map(|v| (*v, CornerFrom::Copy(*f, *v)))
            .collect();
        rewrites.insert(*f, flipped);
    }
    let matched = rewrites.len();
    let mut build = Builder::new(&original, mesh);
    if build.rewrite_faces(&rewrites).is_err() {
        *mesh = original;
        return Outcome {
            matched: 0,
            missing,
            skipped: 1,
        };
    }
    Outcome {
        matched,
        missing,
        skipped: usize::from(matched == 0),
    }
}

/// The polygons facing the wrong way: in each piece, those disagreeing
/// with the way most of their piece faces (worked out by walking across
/// shared edges from the piece's first polygon), and in a closed piece
/// facing inward overall, all the others instead, so the piece faces out.
pub fn faces_facing_wrong(mesh: &Mesh) -> Vec<FaceId> {
    let topology = mesh.topology();
    let mut out = Vec::new();
    for piece in topology.pieces() {
        // Each polygon's way relative to the first: true = same.
        let mut way: BTreeMap<FaceId, bool> = BTreeMap::new();
        let Some(first) = piece.first().copied() else {
            continue;
        };
        way.insert(first, true);
        let mut stack = vec![first];
        while let Some(at) = stack.pop() {
            let here = way[&at];
            for e in topology.face_edges(at) {
                let uses = topology.edge_uses(*e);
                if uses.len() != 2 {
                    continue;
                }
                let Some(mine) = uses.iter().find(|u| u.face == at) else {
                    continue;
                };
                let Some(other) = uses.iter().find(|u| u.face != at) else {
                    continue;
                };
                if way.contains_key(&other.face) {
                    continue;
                }
                // Agreeing neighbours run along a shared edge opposite ways.
                let agrees = mine.forward != other.forward;
                way.insert(other.face, if agrees { here } else { !here });
                stack.push(other.face);
            }
        }
        // Pieces only touching at points: the rest keep their way.
        let same: Vec<FaceId> = way.iter().filter(|(_, w)| **w).map(|(f, _)| *f).collect();
        let other: Vec<FaceId> = way.iter().filter(|(_, w)| !**w).map(|(f, _)| *f).collect();
        let closed = piece
            .iter()
            .flat_map(|f| topology.face_edges(*f).iter().copied())
            .all(|e| topology.edge_uses(e).len() == 2);
        let wrong = if closed {
            // The signed volume as `same` faces: outward when positive.
            let volume: f32 = way
                .iter()
                .map(|(f, w)| {
                    let v = signed_volume(mesh, *f);
                    if *w { v } else { -v }
                })
                .sum();
            if volume >= 0.0 { other } else { same }
        } else if other.len() > same.len() {
            same
        } else {
            other
        };
        out.extend(wrong);
    }
    out.sort();
    out
}

/// A polygon's part of the volume its shape encloses (a fan of tetrahedra
/// to the origin).
fn signed_volume(mesh: &Mesh, face: FaceId) -> f32 {
    let Some(c) = mesh.face(face) else {
        return 0.0;
    };
    let p: Vec<Vec3> = c.iter().filter_map(|v| mesh.position(*v)).collect();
    if p.len() < 3 {
        return 0.0;
    }
    p[1..]
        .windows(2)
        .map(|w| p[0].dot(w[0].cross(w[1])) / 6.0)
        .sum()
}

/// Turns every polygon facing the wrong way (see [`faces_facing_wrong`])
/// the right way. Nothing to turn is skipped.
pub fn fix_facing(mesh: &mut Mesh) -> Outcome {
    let wrong = faces_facing_wrong(mesh);
    if wrong.is_empty() {
        return Outcome {
            matched: 0,
            missing: 0,
            skipped: 1,
        };
    }
    flip_faces(mesh, &wrong)
}

/// The polygons facing within `angle` degrees of `way` (any length).
pub fn faces_facing(mesh: &Mesh, way: Vec3, angle: f32) -> Vec<FaceId> {
    let Some(way) = way.try_normalize() else {
        return Vec::new();
    };
    let cos = angle.clamp(0.0, 180.0).to_radians().cos();
    mesh.faces()
        .map(|(f, _)| f)
        .filter(|f| mesh.face_normal(*f).dot(way) >= cos - 1e-6)
        .collect()
}

#[cfg(test)]
mod tests;
