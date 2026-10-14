//! Connect, Bridge and Weld: joining parts of a shape
//! ([decision 0051](../../../docs/decisions/0051-connect-bridge-weld.md)).
//!
//! - **Connect** ([`connect_points`], [`connect_edges`]): a new edge across a
//!   polygon between selected points, or between new points along selected
//!   edges, splitting the polygon.
//! - **Bridge** ([`bridge_edges`], [`bridge_faces`]): a band of new polygons
//!   between two open borders (chains of edges with a polygon on one side
//!   only), or between two polygons, which are removed so the band joins
//!   their borders, like a tube between two holes.
//! - **Weld** ([`weld`]): points melted into one, at their middle or onto a
//!   target point; polygons left with fewer than three corners go.
//!
//! As with every edit, polygons keep their ids where they survive, new
//! points and polygons take the next ids in a fixed order, and values come
//! along through the [`Builder`].

use crate::builder::{Builder, CornerFrom, PointFrom};
use crate::edit::Outcome;
use crate::{EdgeKey, FaceId, Mesh, VertexId};
use glam::Vec3;
use std::collections::{BTreeMap, BTreeSet};

/// The most cuts Connect makes along each edge.
pub const MAX_CONNECT_CUTS: u32 = 64;
/// The most rows a bridge can have.
pub const MAX_BRIDGE_SEGMENTS: u32 = 64;

// ------------------------------------------------------------- connect ---

/// Splits each piece in `pieces` holding both `a` and `b` (not next to each
/// other) in two along a new edge between them: the first piece keeps its
/// place, the second is added. Returns whether a split happened.
fn split_between(pieces: &mut Vec<Vec<VertexId>>, a: VertexId, b: VertexId) -> bool {
    let Some(k) = pieces.iter().position(|p| p.contains(&a) && p.contains(&b)) else {
        return false;
    };
    let piece = &pieces[k];
    let n = piece.len();
    let (Some(i), Some(j)) = (
        piece.iter().position(|v| *v == a),
        piece.iter().position(|v| *v == b),
    ) else {
        return false;
    };
    if i == j || (i + 1) % n == j || (j + 1) % n == i {
        return false;
    }
    let walk = |from: usize, to: usize| {
        let mut out = Vec::new();
        let mut at = from;
        loop {
            out.push(piece[at]);
            if at == to {
                break;
            }
            at = (at + 1) % n;
        }
        out
    };
    let (one, two) = (walk(i, j), walk(j, i));
    pieces[k] = one;
    pieces.push(two);
    true
}

/// Gives each polygon in `pieces` its new corners (the first piece keeps
/// its id, the others are new polygons like it), with values from the
/// polygon it came from: original corners copy, points along an original
/// edge blend its ends (`along`). Returns how many polygons were split.
fn rebuild(
    build: &mut Builder,
    original: &Mesh,
    pieces: &BTreeMap<FaceId, Vec<Vec<VertexId>>>,
    made: &BTreeMap<VertexId, (EdgeKey, f32)>,
) -> Result<usize, crate::MeshError> {
    let corner = |face: FaceId, v: VertexId| match made.get(&v) {
        None => CornerFrom::Copy(face, v),
        Some((e, t)) => {
            let (a, b) = e.ends();
            CornerFrom::Between(face, a, face, b, *t)
        }
    };
    let mut rewrites = BTreeMap::new();
    let mut new_faces = Vec::new();
    let mut split = 0;
    for (face, list) in pieces {
        if original.face(*face).is_none() {
            continue;
        }
        split += usize::from(list.len() > 1);
        for (k, piece) in list.iter().enumerate() {
            let corners: Vec<(VertexId, CornerFrom)> =
                piece.iter().map(|v| (*v, corner(*face, *v))).collect();
            if k == 0 {
                rewrites.insert(*face, corners);
            } else {
                new_faces.push((corners, *face));
            }
        }
    }
    build.rewrite_faces(&rewrites)?;
    for (corners, like) in &new_faces {
        build.face(corners, Some(*like));
    }
    Ok(split)
}

/// Connects `points` across each polygon holding two or more of them that
/// aren't next to each other: a new edge joins them in the polygon's
/// order (all the way round when there are three or more), splitting it.
/// Points not in the mesh are counted as missing; polygons with nothing to
/// connect are left alone, and nothing connected at all counts as skipped.
pub fn connect_points(mesh: &mut Mesh, points: &[VertexId]) -> Outcome {
    let chosen: BTreeSet<VertexId> = points.iter().copied().collect();
    let missing = chosen
        .iter()
        .filter(|v| mesh.position(**v).is_none())
        .count();
    let original = mesh.clone();
    let mut pieces: BTreeMap<FaceId, Vec<Vec<VertexId>>> = BTreeMap::new();
    for (face, corners) in original.faces() {
        let picked: Vec<VertexId> = corners
            .iter()
            .copied()
            .filter(|v| chosen.contains(v))
            .collect();
        if picked.len() < 2 {
            continue;
        }
        let mut list = vec![corners.to_vec()];
        let pairs = if picked.len() == 2 { 1 } else { picked.len() };
        let mut any = false;
        for k in 0..pairs {
            any |= split_between(&mut list, picked[k], picked[(k + 1) % picked.len()]);
        }
        if any {
            pieces.insert(face, list);
        }
    }
    finish(mesh, &original, &pieces, &BTreeMap::new(), missing)
}

/// Rebuilds `mesh` from `pieces`, or says nothing was connected.
fn finish(
    mesh: &mut Mesh,
    original: &Mesh,
    pieces: &BTreeMap<FaceId, Vec<Vec<VertexId>>>,
    made: &BTreeMap<VertexId, (EdgeKey, f32)>,
    missing: usize,
) -> Outcome {
    let nothing = Outcome {
        matched: 0,
        missing,
        skipped: 1,
    };
    if pieces.values().all(|l| l.len() < 2) {
        *mesh = original.clone();
        return nothing;
    }
    let mut build = Builder::new(original, mesh);
    match rebuild(&mut build, original, pieces, made) {
        Ok(split) => Outcome {
            matched: split,
            missing,
            skipped: 0,
        },
        Err(_) => {
            *mesh = original.clone();
            nothing
        }
    }
}

/// Connects `edges`: each is cut `cuts` times, evenly (1 to
/// [`MAX_CONNECT_CUTS`]), and in each polygon holding two or more of them,
/// the new points are joined across it, in the polygon's order (a ring of
/// selected edges gets a new loop through it). Polygons beside a cut edge
/// that aren't split take its new points as corners, so no gap opens.
pub fn connect_edges(mesh: &mut Mesh, edges: &[EdgeKey], cuts: u32) -> Outcome {
    let cuts = cuts.clamp(1, MAX_CONNECT_CUTS) as usize;
    let original = mesh.clone();
    let topology = original.topology();
    let chosen: BTreeSet<EdgeKey> = edges
        .iter()
        .copied()
        .filter(|e| topology.has_edge(*e))
        .collect();
    let missing = edges.len().saturating_sub(chosen.len());
    let mut build = Builder::new(&original, mesh);
    // The new points along each edge, from its lower point id.
    let mut along: BTreeMap<EdgeKey, Vec<VertexId>> = BTreeMap::new();
    let mut made: BTreeMap<VertexId, (EdgeKey, f32)> = BTreeMap::new();
    for e in &chosen {
        let (a, b) = e.ends();
        let (Some(pa), Some(pb)) = (original.position(a), original.position(b)) else {
            continue;
        };
        let list = (1..=cuts)
            .map(|k| {
                let t = k as f32 / (cuts + 1) as f32;
                let v = build.point(pa.lerp(pb, t), PointFrom::Between(a, b, t));
                made.insert(v, (*e, t));
                v
            })
            .collect();
        along.insert(*e, list);
    }
    let touched: BTreeSet<FaceId> = chosen
        .iter()
        .flat_map(|e| topology.edge_faces(*e))
        .collect();
    let mut pieces: BTreeMap<FaceId, Vec<Vec<VertexId>>> = BTreeMap::new();
    for face in touched {
        let Some(corners) = original.face(face) else {
            continue;
        };
        let n = corners.len();
        let mut outline = Vec::new();
        // The cut edges' new points, in the polygon's order, one list per
        // edge.
        let mut groups: Vec<Vec<VertexId>> = Vec::new();
        for i in 0..n {
            let (a, b) = (corners[i], corners[(i + 1) % n]);
            outline.push(a);
            if let Some(list) = along.get(&EdgeKey::new(a, b)) {
                let list: Vec<VertexId> = if EdgeKey::new(a, b).ends().0 == a {
                    list.clone()
                } else {
                    list.iter().rev().copied().collect()
                };
                outline.extend(&list);
                groups.push(list);
            }
        }
        let mut list = vec![outline];
        if groups.len() >= 2 {
            // Opposite points of consecutive cut edges meet: the k-th cut on
            // one edge with the k-th from the other end on the next, so
            // parallel cuts stay parallel.
            let rounds = if groups.len() == 2 { 1 } else { groups.len() };
            for g in 0..rounds {
                let (one, two) = (&groups[g], &groups[(g + 1) % groups.len()]);
                for k in 0..cuts {
                    split_between(&mut list, one[k], two[cuts - 1 - k]);
                }
            }
        }
        pieces.insert(face, list);
    }
    if pieces.values().all(|l| l.len() < 2) {
        *mesh = original.clone();
        return Outcome {
            matched: 0,
            missing,
            skipped: 1,
        };
    }
    {
        let mut build = Builder::new(&original, mesh);
        let split = match rebuild(&mut build, &original, &pieces, &made) {
            Ok(split) => split,
            Err(_) => {
                *mesh = original.clone();
                return Outcome {
                    matched: 0,
                    missing,
                    skipped: 1,
                };
            }
        };
        // A cut edge's values go to each piece.
        for (edge, list) in &along {
            let (a, b) = edge.ends();
            let mut chain = vec![a];
            chain.extend(list.iter().copied());
            chain.push(b);
            for pair in chain.windows(2) {
                build.copy_edge(*edge, pair[0], pair[1]);
            }
            build.clear_edge(*edge);
        }
        Outcome {
            matched: split,
            missing,
            skipped: 0,
        }
    }
}

// -------------------------------------------------------------- bridge ---

/// The edges in `edges` joined end to end into chains, each as its points
/// in order and whether it closes on itself. Chains are found in a fixed
/// order (from the lowest point id), so the result never changes between
/// runs; branching points end a chain.
fn chains(edges: &BTreeSet<EdgeKey>) -> Vec<(Vec<VertexId>, bool)> {
    let mut next: BTreeMap<VertexId, Vec<VertexId>> = BTreeMap::new();
    for e in edges {
        let (a, b) = e.ends();
        next.entry(a).or_default().push(b);
        next.entry(b).or_default().push(a);
    }
    let mut used: BTreeSet<EdgeKey> = BTreeSet::new();
    let mut out = Vec::new();
    // Ends first (points with one edge), then whatever is left (loops).
    let starts: Vec<VertexId> = next
        .iter()
        .filter(|(_, n)| n.len() == 1)
        .map(|(v, _)| *v)
        .chain(next.keys().copied())
        .collect();
    for start in starts {
        let fresh = next[&start]
            .iter()
            .any(|n| !used.contains(&EdgeKey::new(start, *n)));
        if !fresh {
            continue;
        }
        let mut chain = vec![start];
        let mut at = start;
        let mut closed = false;
        while let Some(step) = next[&at]
            .iter()
            .copied()
            .find(|n| !used.contains(&EdgeKey::new(at, *n)))
        {
            used.insert(EdgeKey::new(at, step));
            if step == start {
                closed = true;
                break;
            }
            chain.push(step);
            at = step;
            if next[&at].len() > 2 {
                break;
            }
        }
        out.push((chain, closed));
    }
    out
}

/// Turns `chain` (a border) to run the way a new polygon beside it must
/// go: against the polygon already there.
fn free_way(mesh: &Mesh, chain: &mut [VertexId], closed: bool) {
    let topology = mesh.topology();
    let n = chain.len();
    if n < 2 {
        return;
    }
    let (a, b) = (chain[0], chain[1]);
    let forward_used = topology.edge_uses(EdgeKey::new(a, b)).iter().any(|u| {
        let (low, _) = EdgeKey::new(a, b).ends();
        // `forward`: the polygon runs from the lower id to the higher.
        (u.forward && low == a) || (!u.forward && low == b)
    });
    if forward_used {
        chain.reverse();
    }
    let _ = closed;
}

/// Builds a band between two point chains running opposite ways (`one`
/// as a new polygon must go, `two` already turned round): quads between
/// matching points, `segments` rows deep, or, when they have different
/// numbers of points, one row of a zip of quads and triangles.
#[allow(clippy::too_many_arguments)]
fn band(
    build: &mut Builder,
    one: &[VertexId],
    two: &[VertexId],
    closed: bool,
    segments: u32,
    like: Option<FaceId>,
    corner: &dyn Fn(VertexId) -> CornerFrom,
) -> usize {
    let mut made = 0;
    let place = |build: &mut Builder, v: VertexId| build.mesh().position(v).unwrap_or(Vec3::ZERO);
    if one.len() == two.len() {
        let n = one.len();
        let segments = segments.clamp(1, MAX_BRIDGE_SEGMENTS) as usize;
        // Rows of points from `one` to `two`.
        let mut rows: Vec<Vec<VertexId>> = vec![one.to_vec()];
        for s in 1..segments {
            let t = s as f32 / segments as f32;
            let row = (0..n)
                .map(|i| {
                    let (a, b) = (place(build, one[i]), place(build, two[i]));
                    build.point(a.lerp(b, t), PointFrom::Between(one[i], two[i], t))
                })
                .collect();
            rows.push(row);
        }
        rows.push(two.to_vec());
        let spans = if closed { n } else { n - 1 };
        for pair in rows.windows(2) {
            let (r, s) = (&pair[0], &pair[1]);
            for i in 0..spans {
                let j = (i + 1) % n;
                let quad = [r[i], r[j], s[j], s[i]];
                let corners: Vec<(VertexId, CornerFrom)> =
                    quad.iter().map(|v| (*v, corner(*v))).collect();
                made += usize::from(build.face(&corners, like).is_some());
            }
        }
        return made;
    }
    // A zip: step along whichever side is behind.
    let (n, m) = (one.len(), two.len());
    let (spans_n, spans_m) = if closed { (n, m) } else { (n - 1, m - 1) };
    let (mut i, mut j) = (0, 0);
    while i < spans_n || j < spans_m {
        let share = |k: usize, of: usize| k as f32 / of.max(1) as f32;
        let take_one =
            j >= spans_m || (i < spans_n && share(i + 1, spans_n) <= share(j + 1, spans_m));
        let tri: [VertexId; 3] = if take_one {
            let t = [one[i], one[(i + 1) % n], two[j % m]];
            i += 1;
            t
        } else {
            let t = [one[i % n], two[(j + 1) % m], two[j]];
            j += 1;
            t
        };
        let corners: Vec<(VertexId, CornerFrom)> = tri.iter().map(|v| (*v, corner(*v))).collect();
        made += usize::from(build.face(&corners, like).is_some());
    }
    made
}

/// Lines `two` up with `one` (both closed, the same length): the turn of
/// `two` that brings matching points closest, then `twist` steps more.
fn line_up(mesh: &Mesh, one: &[VertexId], two: &mut [VertexId], twist: i32) {
    let n = two.len();
    if n == 0 {
        return;
    }
    let at = |v: VertexId| mesh.position(v).unwrap_or(Vec3::ZERO);
    let best = (0..n)
        .min_by(|a, b| {
            let cost = |k: usize| -> f32 {
                (0..n)
                    .map(|i| at(one[i]).distance_squared(at(two[(i + k) % n])))
                    .sum()
            };
            cost(*a).total_cmp(&cost(*b))
        })
        .unwrap_or(0);
    let turn = (best as i64 + i64::from(twist)).rem_euclid(n as i64) as usize;
    two.rotate_left(turn);
}

/// Bridges `edges`: the border edges among them are joined into chains,
/// and the first two chains get a band of new polygons between them,
/// `segments` rows deep, turned by `twist` steps when both are closed
/// loops. Chains of different lengths are joined by a zip of triangles
/// (one row). Edges not in the mesh are missing; fewer than two chains of
/// border edges is skipped.
pub fn bridge_edges(mesh: &mut Mesh, edges: &[EdgeKey], segments: u32, twist: i32) -> Outcome {
    let original = mesh.clone();
    let topology = original.topology();
    let present: Vec<EdgeKey> = edges
        .iter()
        .copied()
        .filter(|e| topology.has_edge(*e))
        .collect();
    let missing = edges.len() - present.len();
    let border: BTreeSet<EdgeKey> = present
        .into_iter()
        .filter(|e| topology.is_border(*e))
        .collect();
    let found = chains(&border);
    let skipped = Outcome {
        matched: 0,
        missing,
        skipped: 1,
    };
    let [(one, one_closed), (two, two_closed), ..] = found.as_slice() else {
        return skipped;
    };
    if one_closed != two_closed || one.len() < 2 || two.len() < 2 {
        return skipped;
    }
    let closed = *one_closed;
    let (mut one, mut two) = (one.clone(), two.clone());
    free_way(&original, &mut one, closed);
    free_way(&original, &mut two, closed);
    two.reverse();
    if closed && one.len() == two.len() {
        line_up(&original, &one, &mut two, twist);
    }
    // A polygon beside each chain, for the band's values.
    let like = topology.edge_faces(EdgeKey::new(one[0], one[1])).next();
    let corner_face = |v: VertexId| {
        topology
            .vertex_faces(v)
            .first()
            .map_or(CornerFrom::Nothing, |f| CornerFrom::Copy(*f, v))
    };
    let first_new = mesh.face_slots().len();
    let mut build = Builder::new(&original, mesh);
    let made = band(&mut build, &one, &two, closed, segments, like, &corner_face);
    crate::hardness::harden_new_edges(mesh, first_new, crate::hardness::DEFAULT_ANGLE);
    Outcome {
        matched: made,
        missing,
        skipped: usize::from(made == 0),
    }
}

/// Bridges two polygons: both are removed and a band of new polygons,
/// `segments` rows deep and turned by `twist` steps, joins their borders,
/// like a tube between two holes. They should face each other (or away
/// from each other, for a tube through the inside). Polygons not in the
/// mesh are missing; anything but two polygons is skipped.
pub fn bridge_faces(mesh: &mut Mesh, faces: &[FaceId], segments: u32, twist: i32) -> Outcome {
    let present: Vec<FaceId> = faces
        .iter()
        .copied()
        .filter(|f| mesh.face(*f).is_some())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let missing = faces.len().saturating_sub(present.len());
    let [a, b] = present.as_slice() else {
        return Outcome {
            matched: 0,
            missing,
            skipped: 1,
        };
    };
    let original = mesh.clone();
    let one: Vec<VertexId> = original.face(*a).unwrap_or_default().to_vec();
    let mut two: Vec<VertexId> = original.face(*b).unwrap_or_default().to_vec();
    if one.iter().any(|v| two.contains(v)) {
        return Outcome {
            matched: 0,
            missing,
            skipped: 1,
        };
    }
    // Once the polygons go, a new polygon beside their border runs their
    // own way round; the far side turned round to meet it.
    two.reverse();
    if one.len() == two.len() {
        line_up(&original, &one, &mut two, twist);
    }
    mesh.delete_faces(&[*a, *b]);
    let from_one: BTreeSet<VertexId> = one.iter().copied().collect();
    let corner = |v: VertexId| {
        if from_one.contains(&v) {
            CornerFrom::Copy(*a, v)
        } else {
            CornerFrom::Copy(*b, v)
        }
    };
    let first_new = mesh.face_slots().len();
    let mut build = Builder::new(&original, mesh);
    let made = band(&mut build, &one, &two, true, segments, Some(*a), &corner);
    crate::hardness::harden_new_edges(mesh, first_new, crate::hardness::DEFAULT_ANGLE);
    Outcome {
        matched: made,
        missing,
        skipped: usize::from(made == 0),
    }
}

// ---------------------------------------------------------------- weld ---

/// Welds `points` into one: at `target` (which must be in the mesh) or, if
/// none, at their middle. The point that stays is the target, or the
/// lowest id; the others go. Polygons using several of them lose the
/// repeats, and polygons left with fewer than three corners are deleted.
/// Points not in the mesh are missing; fewer than two points to weld (with
/// the target) is skipped.
pub fn weld(mesh: &mut Mesh, points: &[VertexId], target: Option<VertexId>) -> Outcome {
    let mut chosen: BTreeSet<VertexId> = points
        .iter()
        .copied()
        .filter(|v| mesh.position(*v).is_some())
        .collect();
    let missing = points
        .iter()
        .collect::<BTreeSet<_>>()
        .len()
        .saturating_sub(chosen.len());
    let target = target.filter(|t| mesh.position(*t).is_some());
    if let Some(t) = target {
        chosen.insert(t);
    }
    let skipped = Outcome {
        matched: 0,
        missing,
        skipped: 1,
    };
    if chosen.len() < 2 {
        return skipped;
    }
    let keep = target.unwrap_or(*chosen.iter().next().unwrap_or(&VertexId::from_raw(0)));
    let place = match target {
        Some(t) => mesh.position(t).unwrap_or(Vec3::ZERO),
        None => {
            let sum: Vec3 = chosen.iter().filter_map(|v| mesh.position(*v)).sum();
            sum / chosen.len() as f32
        }
    };
    let original = mesh.clone();
    let mut rewrites: BTreeMap<FaceId, Vec<(VertexId, CornerFrom)>> = BTreeMap::new();
    let mut gone: Vec<FaceId> = Vec::new();
    for (face, corners) in original.faces() {
        if !corners.iter().any(|v| chosen.contains(v) && *v != keep) {
            continue;
        }
        let mut out: Vec<(VertexId, CornerFrom)> = Vec::new();
        for v in corners {
            let to = if chosen.contains(v) { keep } else { *v };
            if out.last().is_some_and(|(last, _)| *last == to) {
                continue;
            }
            out.push((to, CornerFrom::Copy(face, *v)));
        }
        if out.len() > 1 && out[0].0 == out[out.len() - 1].0 {
            out.pop();
        }
        // A corner repeated further apart would make a broken polygon.
        let distinct: BTreeSet<VertexId> = out.iter().map(|(v, _)| *v).collect();
        if out.len() < 3 || distinct.len() != out.len() {
            gone.push(face);
        } else {
            rewrites.insert(face, out);
        }
    }
    mesh.delete_faces(&gone);
    let mut build = Builder::new(&original, mesh);
    if build.rewrite_faces(&rewrites).is_err() {
        *mesh = original;
        return skipped;
    }
    // Edges that went to the kept point take their values with them.
    let others: Vec<VertexId> = chosen.iter().copied().filter(|v| *v != keep).collect();
    let topology = original.topology();
    for v in &others {
        for e in topology.vertex_edges(*v) {
            let (a, b) = e.ends();
            let other = if a == *v { b } else { a };
            if chosen.contains(&other) {
                build.clear_edge(*e);
            } else {
                build.move_edge(*e, keep, other);
            }
        }
    }
    build.mesh().set_position(keep, place);
    // The points that went, and any left with no polygon by the polygons
    // that collapsed.
    let mut unused = others.clone();
    for face in &gone {
        unused.extend(original.face(*face).unwrap_or_default());
    }
    unused.push(keep);
    mesh.remove_unused_vertices(&unused);
    Outcome {
        matched: chosen.len(),
        missing,
        skipped: 0,
    }
}

#[cfg(test)]
mod tests;
