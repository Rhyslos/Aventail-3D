//! Bevelling edges and points: cutting a sharp edge or corner away and
//! putting a strip (or a corner polygon) of new polygons in its place
//! ([decision 0040](../../../docs/decisions/0040-bevel.md)).
//!
//! **Edges.** Each bevelled edge is replaced by a strip `width` wide on
//! each side, measured across each polygon next to it. With more
//! `segments` the strip is cut lengthwise into rows that follow a
//! `profile` from flat (0, a chamfer) to round (1). Where the strip ends:
//! - a polygon corner between two bevelled edges moves to where both
//!   offsets meet;
//! - a corner between a bevelled and a plain edge slides along the plain
//!   edge;
//! - the hole left where several strips meet (a cube's corner) is closed
//!   with one corner polygon; with a single bevelled edge, the strip's end
//!   joins the polygon at its end instead.
//!
//! **Points.** Each bevelled point is cut away: a point is put `width`
//! along each of its edges, and one polygon closes the corner.
//!
//! As with every edit, polygons keep their ids (they get new corners), new
//! points and polygons take the next ids in a fixed order, and points no
//! polygon uses any more are deleted. Edges need a polygon on each side
//! and points a single fan of polygons around them; others are skipped
//! and counted. Bevels wider than the polygons around them cross over
//! (clamping comes with the improved bevel in Phase 7), but never make
//! points that aren't numbers.

use crate::edit::Outcome;
use crate::topology::Topology;
use crate::{EdgeKey, FaceId, Mesh, VertexId};
use glam::Vec3;
use std::collections::{BTreeMap, BTreeSet};

/// The most rows a bevel can have.
pub const MAX_SEGMENTS: u32 = 64;

/// Below this sine of the angle between two edges, offsets are worked out
/// as if the angle were this wide, so nearly straight corners stay sane.
const MIN_SIN: f32 = 0.05;

/// How far along an edge a sliding corner may go, as a part of the edge.
const MAX_SLIDE: f32 = 0.95;

/// The shape of a bevel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BevelShape {
    /// How far the bevel reaches into each polygon, in the mesh's units.
    pub width: f32,
    /// How many rows the strip has (1: a flat chamfer).
    pub segments: u32,
    /// From flat (0) to round (1).
    pub profile: f32,
}

impl Default for BevelShape {
    fn default() -> Self {
        Self {
            width: 10.0,
            segments: 1,
            profile: 1.0,
        }
    }
}

impl BevelShape {
    /// The shape brought within limits: width not below 0, 1 to
    /// [`MAX_SEGMENTS`] rows, profile 0 to 1. `None` if a value isn't a
    /// number.
    pub fn clamped(self) -> Option<Self> {
        (self.width.is_finite() && self.profile.is_finite()).then(|| Self {
            width: self.width.max(0.0),
            segments: self.segments.clamp(1, MAX_SEGMENTS),
            profile: self.profile.clamp(0.0, 1.0),
        })
    }
}

/// One polygon around a point: the polygon, and the corners before and
/// after the point in it.
#[derive(Debug, Clone)]
struct Wedge {
    face: FaceId,
    before: VertexId,
    after: VertexId,
}

/// The polygons around `v`, in order (each one's `after` is the next
/// one's `before`), and whether they close all the way round. `None` when
/// they don't form a single fan (several pieces touching at the point, or
/// an edge with more than two polygons).
fn fan(mesh: &Mesh, topology: &Topology, v: VertexId) -> Option<(Vec<Wedge>, bool)> {
    let mut wedges = Vec::new();
    for face in topology.vertex_faces(v) {
        let corners = mesh.face(*face)?;
        let i = corners.iter().position(|c| *c == v)?;
        let n = corners.len();
        wedges.push(Wedge {
            face: *face,
            before: corners[(i + n - 1) % n],
            after: corners[(i + 1) % n],
        });
    }
    if wedges.is_empty() {
        return None;
    }
    let mut by_before: BTreeMap<VertexId, usize> = BTreeMap::new();
    for (i, w) in wedges.iter().enumerate() {
        if by_before.insert(w.before, i).is_some() {
            return None;
        }
    }
    let afters: BTreeSet<VertexId> = wedges.iter().map(|w| w.after).collect();
    let starts: Vec<usize> = (0..wedges.len())
        .filter(|i| !afters.contains(&wedges[*i].before))
        .collect();
    let (start, closed) = match starts.as_slice() {
        [] => ((0..wedges.len()).min_by_key(|i| wedges[*i].face)?, true),
        [one] => (*one, false),
        _ => return None,
    };
    let mut order = vec![start];
    let mut at = start;
    while let Some(&next) = by_before.get(&wedges[at].after) {
        if next == start {
            break;
        }
        if order.contains(&next) {
            return None;
        }
        order.push(next);
        at = next;
    }
    if order.len() != wedges.len() {
        return None;
    }
    Some((
        order.into_iter().map(|i| wedges[i].clone()).collect(),
        closed,
    ))
}

/// Where one polygon's corner at a bevelled point goes, before new points
/// are made.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Spot {
    /// A new point here.
    At(Vec3),
    /// The point sliding along the edge towards this neighbour.
    Slide(VertexId),
    /// The original point, kept.
    Stay,
}

/// Points sliding along plain edges from one bevelled point: how far each
/// polygon asks for, to average.
#[derive(Default)]
struct Slides(BTreeMap<VertexId, (f32, u32)>);

impl Slides {
    fn ask(&mut self, towards: VertexId, distance: f32) {
        let entry = self.0.entry(towards).or_default();
        entry.0 += distance;
        entry.1 += 1;
    }

    fn has(&self, towards: VertexId) -> bool {
        self.0.contains_key(&towards)
    }
}

/// A unit vector from `from` to `to`, or zero.
fn towards(mesh: &Mesh, from: VertexId, to: VertexId) -> Vec3 {
    match (mesh.position(from), mesh.position(to)) {
        (Some(a), Some(b)) => (b - a).normalize_or_zero(),
        _ => Vec3::ZERO,
    }
}

/// `width / sin`, with the sine kept away from zero (keeping its sign).
fn over_sin(width: f32, sin: f32) -> f32 {
    if sin < 0.0 {
        width / sin.min(-MIN_SIN)
    } else {
        width / sin.max(MIN_SIN)
    }
}

/// How far to slide along an edge: `width / |sin|`, never backwards.
fn slide_by(width: f32, sin: f32) -> f32 {
    width / sin.abs().max(MIN_SIN)
}

/// A point on the quadratic curve from `a` to `b`, `t` of the way, bent
/// towards `corner` by `profile` (0: a straight line, 1: through the
/// corner's pull).
fn profile_point(a: Vec3, b: Vec3, corner: Vec3, profile: f32, t: f32) -> Vec3 {
    let control = (a + b) * 0.5 + (corner - (a + b) * 0.5) * profile;
    let s = 1.0 - t;
    a * (s * s) + control * (2.0 * s * t) + b * (t * t)
}

/// Removes repeats next to each other in a closed list of corners.
fn without_repeats(mut list: Vec<VertexId>, closed: bool) -> Vec<VertexId> {
    list.dedup();
    while closed && list.len() > 1 && list.first() == list.last() {
        list.pop();
    }
    list
}

/// Everything worked out about one bevelled point.
struct Corner {
    wedges: Vec<Wedge>,
    closed: bool,
    /// Each wedge's new corners, in the polygon's order.
    lists: Vec<Vec<VertexId>>,
}

/// Bevels `edges` with `shape`. Edges not in the mesh are counted as
/// missing; edges without a polygon on each side, or with an end where
/// polygons don't form a single fan, are skipped. A shape with a value
/// that isn't a number changes nothing.
pub fn bevel_edges(mesh: &mut Mesh, edges: &[EdgeKey], shape: BevelShape) -> Outcome {
    let original = mesh.clone();
    let topology = original.topology();
    let mut outcome = Outcome::default();
    let mut chosen = BTreeSet::new();
    for edge in edges.iter().copied().collect::<BTreeSet<_>>() {
        if !topology.has_edge(edge) {
            outcome.missing += 1;
        } else if topology.edge_uses(edge).len() != 2 {
            outcome.skipped += 1;
        } else {
            chosen.insert(edge);
        }
    }
    let mut fans = BTreeMap::new();
    for edge in &chosen {
        let (a, b) = edge.ends();
        for v in [a, b] {
            fans.entry(v).or_insert_with(|| fan(&original, topology, v));
        }
    }
    let before = chosen.len();
    chosen.retain(|e| {
        let (a, b) = e.ends();
        fans[&a].is_some() && fans[&b].is_some()
    });
    outcome.skipped += before - chosen.len();
    outcome.matched = chosen.len();
    let Some(shape) = shape.clamped() else {
        return outcome;
    };
    if chosen.is_empty() {
        return outcome;
    }
    let fans: BTreeMap<VertexId, (Vec<Wedge>, bool)> = fans
        .into_iter()
        .filter_map(|(v, f)| {
            let f = f?;
            let touched = chosen.iter().any(|e| e.ends().0 == v || e.ends().1 == v);
            touched.then_some((v, f))
        })
        .collect();
    let mut out = original.clone();
    let bevelled = |a: VertexId, b: VertexId| chosen.contains(&EdgeKey::new(a, b));

    // 1. Where each polygon corner at a bevelled point goes.
    let mut spots: BTreeMap<VertexId, Vec<Vec<Spot>>> = BTreeMap::new();
    let mut slides: BTreeMap<VertexId, Slides> = BTreeMap::new();
    for (v, (wedges, _)) in &fans {
        let pv = original.position(*v).unwrap_or_default();
        let mut asks = Slides::default();
        let mut per_wedge = Vec::new();
        for w in wedges {
            let normal = original.face_normal(w.face);
            let d_in = towards(&original, *v, w.before);
            let d_out = towards(&original, *v, w.after);
            let perp_in = d_in.cross(normal).normalize_or_zero();
            let perp_out = normal.cross(d_out).normalize_or_zero();
            let width = shape.width;
            let spot = match (bevelled(w.before, *v), bevelled(*v, w.after)) {
                (true, true) => {
                    let sin = d_in.dot(perp_out);
                    let offset = if sin.abs() < MIN_SIN {
                        (perp_in + perp_out).normalize_or_zero() * width
                    } else {
                        d_in * over_sin(width, sin) + d_out * over_sin(width, d_out.dot(perp_in))
                    };
                    vec![Spot::At(pv + offset)]
                }
                (true, false) => {
                    asks.ask(w.after, slide_by(width, d_out.dot(perp_in)));
                    vec![Spot::Slide(w.after)]
                }
                (false, true) => {
                    asks.ask(w.before, slide_by(width, d_in.dot(perp_out)));
                    vec![Spot::Slide(w.before)]
                }
                (false, false) => vec![Spot::Slide(w.before), Spot::Stay, Spot::Slide(w.after)],
            };
            per_wedge.push(spot);
        }
        // A plain wedge keeps only the slides that happen, and the point
        // itself unless slides on both sides replace it.
        for spot in &mut per_wedge {
            if spot.len() == 3 {
                let (first, last) = match (spot[0], spot[2]) {
                    (Spot::Slide(a), Spot::Slide(b)) => (asks.has(a), asks.has(b)),
                    _ => (false, false),
                };
                let mut kept = Vec::new();
                if first {
                    kept.push(spot[0]);
                }
                if !(first && last) {
                    kept.push(Spot::Stay);
                }
                if last {
                    kept.push(spot[2]);
                }
                *spot = kept;
            }
        }
        spots.insert(*v, per_wedge);
        slides.insert(*v, asks);
    }

    // 2. New points: meeting corners, then slides, point by point.
    let mut corners: BTreeMap<VertexId, Corner> = BTreeMap::new();
    let mut slid: BTreeMap<(VertexId, VertexId), VertexId> = BTreeMap::new();
    for (v, (wedges, closed)) in &fans {
        let pv = original.position(*v).unwrap_or_default();
        let mut made: Vec<Option<VertexId>> = Vec::new();
        for spot in spots[v].iter().flatten() {
            made.push(match spot {
                Spot::At(p) => Some(out.push_vertex(if p.is_finite() { *p } else { pv })),
                _ => None,
            });
        }
        for (far, (sum, count)) in &slides[v].0 {
            let pf = original.position(*far).unwrap_or(pv);
            let length = pf.distance(pv);
            let along = (sum / *count as f32).min(length * MAX_SLIDE);
            let p = pv + (pf - pv).normalize_or_zero() * along;
            slid.insert(
                (*v, *far),
                out.push_vertex(if p.is_finite() { p } else { pv }),
            );
        }
        let mut made = made.into_iter();
        let lists = spots[v]
            .iter()
            .map(|spot| {
                spot.iter()
                    .map(|s| {
                        let new = made.next().flatten();
                        match s {
                            Spot::At(_) => new.unwrap_or(*v),
                            Spot::Slide(far) => slid.get(&(*v, *far)).copied().unwrap_or(*v),
                            Spot::Stay => *v,
                        }
                    })
                    .collect()
            })
            .collect();
        corners.insert(
            *v,
            Corner {
                wedges: wedges.clone(),
                closed: *closed,
                lists,
            },
        );
    }

    // 3. Each strip's rows at each end, from its forward polygon's side to
    // its backward polygon's side.
    let side = |corner: &Corner, face: FaceId, other: VertexId| -> Option<VertexId> {
        let i = corner.wedges.iter().position(|w| w.face == face)?;
        let w = &corner.wedges[i];
        let list = &corner.lists[i];
        if w.before == other {
            list.first().copied()
        } else {
            list.last().copied()
        }
    };
    let mut rows: BTreeMap<(EdgeKey, VertexId), Vec<VertexId>> = BTreeMap::new();
    for edge in &chosen {
        let uses = topology.edge_uses(*edge);
        let forward = uses.iter().find(|u| u.forward).map(|u| u.face);
        let backward = uses.iter().find(|u| !u.forward).map(|u| u.face);
        let (Some(forward), Some(backward)) = (forward, backward) else {
            continue;
        };
        let (lo, hi) = edge.ends();
        for (x, other) in [(lo, hi), (hi, lo)] {
            let corner = &corners[&x];
            let (Some(a), Some(b)) = (side(corner, forward, other), side(corner, backward, other))
            else {
                continue;
            };
            let (pa, pb) = (
                out.position(a).unwrap_or_default(),
                out.position(b).unwrap_or_default(),
            );
            let px = original.position(x).unwrap_or_default();
            // Two strips ending between the same two points (round a
            // cube's corner, say) share their end rows.
            let shared = rows.iter().find_map(|((_, at), row)| {
                if *at != x {
                    None
                } else if row.first() == Some(&b) && row.last() == Some(&a) {
                    Some(row.iter().rev().copied().collect::<Vec<_>>())
                } else if row.first() == Some(&a) && row.last() == Some(&b) {
                    Some(row.clone())
                } else {
                    None
                }
            });
            let row = shared.unwrap_or_else(|| {
                let mut row = vec![a];
                for k in 1..shape.segments {
                    let t = k as f32 / shape.segments as f32;
                    let p = profile_point(pa, pb, px, shape.profile, t);
                    row.push(out.push_vertex(if p.is_finite() { p } else { px }));
                }
                row.push(b);
                row
            });
            rows.insert((*edge, x), row);
        }
    }

    // 4. Holes where strips meet: one corner polygon, or, when only one
    // polygon borders the hole, its corner takes the strip's end.
    let mut patches = Vec::new();
    for (v, corner) in corners.iter_mut() {
        if !corner.closed {
            continue;
        }
        let k = corner.wedges.len();
        let mut cycle: Vec<(VertexId, Option<usize>)> = Vec::new();
        for i in 0..k {
            for p in &corner.lists[i] {
                cycle.push((*p, Some(i)));
            }
            let w = &corner.wedges[i];
            if bevelled(*v, w.after) {
                let edge = EdgeKey::new(*v, w.after);
                if let Some(row) = rows.get(&(edge, *v)) {
                    let inner = &row[1..row.len() - 1];
                    let from_here = corner.lists[i].last() == row.first();
                    let inner: Vec<VertexId> = if from_here {
                        inner.to_vec()
                    } else {
                        inner.iter().rev().copied().collect()
                    };
                    cycle.extend(inner.into_iter().map(|p| (p, None)));
                }
            }
        }
        let points = without_repeats(cycle.iter().map(|(p, _)| *p).collect(), true);
        let distinct: BTreeSet<VertexId> = points.iter().copied().collect();
        // Fewer than three points, or a cycle that doubles back on itself
        // (strips sharing their ends): no hole to fill.
        if points.len() < 3 || distinct.len() != points.len() {
            continue;
        }
        // Which wedges' corners border the hole?
        let bordering: BTreeSet<usize> = (0..k)
            .filter(|i| {
                corner.lists[*i]
                    .iter()
                    .any(|p| points.contains(p) && corner.lists[*i].len() > 1)
            })
            .collect();
        let all_in_one = bordering.len() == 1 && {
            let only = *bordering.iter().next().unwrap_or(&0);
            cycle
                .iter()
                .filter(|(_, from)| from.is_some())
                .all(|(p, _)| corner.lists[only].contains(p))
        };
        if all_in_one {
            let only = *bordering.iter().next().unwrap_or(&0);
            let list = &corner.lists[only];
            let (first, last) = (list[0], list[list.len() - 1]);
            // The strip's end, from this wedge's first corner to its last.
            let end: Vec<VertexId> = cycle
                .iter()
                .filter(|(_, from)| from.is_none())
                .map(|(p, _)| *p)
                .collect();
            let starts_at_last = cycle
                .iter()
                .position(|(p, from)| from.is_none() && *p == end[0])
                .and_then(|i| i.checked_sub(1))
                .map(|i| cycle[i].0 == last)
                .unwrap_or(true);
            let mut middle = end;
            if starts_at_last {
                middle.reverse();
            }
            let mut merged = vec![first];
            merged.extend(middle);
            merged.push(last);
            corner.lists[only] = without_repeats(merged, false);
        } else {
            let mut patch = points;
            patch.reverse();
            patches.push((*v, patch));
        }
    }

    // 5. The polygons around bevelled points get their new corners.
    let mut changes: BTreeMap<FaceId, Vec<VertexId>> = BTreeMap::new();
    let touched: BTreeSet<FaceId> = corners
        .values()
        .flat_map(|c| c.wedges.iter().map(|w| w.face))
        .collect();
    for face in touched {
        let Some(old) = original.face(face) else {
            continue;
        };
        let mut new = Vec::new();
        for c in old {
            match corners.get(c) {
                Some(corner) => {
                    let i = corner.wedges.iter().position(|w| w.face == face);
                    match i {
                        Some(i) => new.extend(corner.lists[i].iter().copied()),
                        None => new.push(*c),
                    }
                }
                None => new.push(*c),
            }
        }
        changes.insert(face, without_repeats(new, true));
    }
    if out.rewrite_faces(&changes).is_err() {
        return outcome;
    }

    // 6. The strips, then the corner polygons.
    for edge in &chosen {
        let (lo, hi) = edge.ends();
        let (Some(at_lo), Some(at_hi)) = (rows.get(&(*edge, lo)), rows.get(&(*edge, hi))) else {
            continue;
        };
        for k in 0..shape.segments as usize {
            let quad = [at_hi[k], at_lo[k], at_lo[k + 1], at_hi[k + 1]];
            out.push_face(&without_repeats(quad.to_vec(), true));
        }
    }
    for (_, patch) in &patches {
        out.push_face(patch);
    }

    // 7. Hard edges: a hard bevelled edge's strip keeps hard borders (and
    // with one row, hard ends and corner polygons); plain edges that slid
    // keep their hardness.
    for edge in &chosen {
        let (lo, hi) = edge.ends();
        let hard = original.is_hard(lo, hi);
        out.set_hard(lo, hi, false);
        if !hard {
            continue;
        }
        if let (Some(a), Some(b)) = (rows.get(&(*edge, lo)), rows.get(&(*edge, hi))) {
            let n = a.len() - 1;
            out.set_hard(a[0], b[0], true);
            out.set_hard(a[n], b[n], true);
            if shape.segments == 1 {
                out.set_hard(a[0], a[1], true);
                out.set_hard(b[0], b[1], true);
            }
        }
    }
    if shape.segments == 1 {
        for (v, patch) in &patches {
            let hard = corners[v]
                .wedges
                .iter()
                .any(|w| bevelled(*v, w.after) && original.is_hard(*v, w.after));
            if hard {
                for i in 0..patch.len() {
                    out.set_hard(patch[i], patch[(i + 1) % patch.len()], true);
                }
            }
        }
    }
    for ((v, far), s) in &slid {
        let other = slid.get(&(*far, *v)).copied().unwrap_or(*far);
        if original.is_hard(*v, *far) {
            out.set_hard(*v, *far, false);
            out.set_hard(*s, other, true);
        }
    }

    // 8. Points no polygon uses any more go.
    for v in corners.keys() {
        out.remove_unused_vertex(*v);
    }
    *mesh = out;
    outcome
}

/// Bevels `points` by `width`: each is cut away, a point put `width`
/// along each of its edges (at most most of the way), and one polygon
/// closes the corner. Points not in the mesh are counted as missing;
/// points whose polygons don't form a single fan are skipped. A width that
/// isn't a number changes nothing.
pub fn bevel_points(mesh: &mut Mesh, points: &[VertexId], width: f32) -> Outcome {
    let original = mesh.clone();
    let topology = original.topology();
    let mut outcome = Outcome::default();
    let mut fans = BTreeMap::new();
    for v in points.iter().copied().collect::<BTreeSet<_>>() {
        if original.position(v).is_none() {
            outcome.missing += 1;
            continue;
        }
        match fan(&original, topology, v) {
            Some(f) => {
                fans.insert(v, f);
            }
            None => outcome.skipped += 1,
        }
    }
    outcome.matched = fans.len();
    if !width.is_finite() || fans.is_empty() {
        return outcome;
    }
    let width = width.max(0.0);
    let mut out = original.clone();
    let mut cut: BTreeMap<(VertexId, VertexId), VertexId> = BTreeMap::new();
    for (v, (wedges, _)) in &fans {
        let pv = original.position(*v).unwrap_or_default();
        let neighbours: BTreeSet<VertexId> =
            wedges.iter().flat_map(|w| [w.before, w.after]).collect();
        for far in neighbours {
            let pf = original.position(far).unwrap_or(pv);
            let along = width.min(pf.distance(pv) * MAX_SLIDE);
            let p = pv + (pf - pv).normalize_or_zero() * along;
            cut.insert(
                (*v, far),
                out.push_vertex(if p.is_finite() { p } else { pv }),
            );
        }
    }
    let mut changes: BTreeMap<FaceId, Vec<VertexId>> = BTreeMap::new();
    let touched: BTreeSet<FaceId> = fans
        .values()
        .flat_map(|(w, _)| w.iter().map(|w| w.face))
        .collect();
    for face in touched {
        let Some(old) = original.face(face) else {
            continue;
        };
        let n = old.len();
        let mut new = Vec::new();
        for (i, c) in old.iter().enumerate() {
            if fans.contains_key(c) {
                let before = old[(i + n - 1) % n];
                let after = old[(i + 1) % n];
                new.push(cut[&(*c, before)]);
                new.push(cut[&(*c, after)]);
            } else {
                new.push(*c);
            }
        }
        changes.insert(face, without_repeats(new, true));
    }
    if out.rewrite_faces(&changes).is_err() {
        return outcome;
    }
    for (v, (wedges, closed)) in &fans {
        let mut ring: Vec<VertexId> = Vec::new();
        for w in wedges {
            ring.push(cut[&(*v, w.before)]);
            ring.push(cut[&(*v, w.after)]);
        }
        let ring = without_repeats(ring, *closed);
        if ring.len() >= 3 {
            let mut patch = ring;
            patch.reverse();
            out.push_face(&patch);
        }
        // Edges that were hard stay hard up to the cut.
        for ((from, far), p) in &cut {
            if from == v && original.is_hard(*v, *far) {
                let other = cut.get(&(*far, *v)).copied().unwrap_or(*far);
                out.set_hard(*p, other, true);
            }
        }
        out.remove_unused_vertex(*v);
    }
    *mesh = out;
    outcome
}

#[cfg(test)]
mod tests;
