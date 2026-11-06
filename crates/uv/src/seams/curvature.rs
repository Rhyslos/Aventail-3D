//! Cuts for points too curved to lie flat.
//!
//! Being a disk (one border, no handles) is not enough for a piece to lie
//! flat: the surface round each point inside it must also add up to a full
//! turn. Round a cone's tip, or a cube's corner, it adds up to much less
//! (the shortfall is the point's **angle defect**, how curved the surface
//! is there). Laid flat in one go, the polygons round such a point have to
//! fold over each other to close up. A cut from the point to the piece's
//! border lets them open out instead, like slitting a paper cone to
//! flatten it.
//!
//! [`curvature_cuts`] finds the points inside a piece that fall short of a
//! full turn by more than [`CURVED_POINT_ANGLE`] and cuts each to the
//! border along the most hidden way, most curved first. Points on the
//! border are fine as they are (the border opens out), edges marked
//! "never" are never cut for this (a point only reachable through them
//! stays uncut), and a point where the surface adds up to more than a
//! full turn (a saddle) isn't helped by a cut, so it's left alone.

use super::{HideSeams, Step, path_cost};
use crate::chart::Chart;
use mesh::uvs::SeamPreference;
use mesh::{EdgeKey, FaceId, Mesh, VertexId};
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

/// How far short of a full turn (degrees) the surface round a point inside
/// a piece may fall before the point is cut to the border. A cube's corner
/// falls 90° short and a sharp cone's tip far more; the poles of a sphere
/// with the standard settings fall only a few degrees short, and smooth
/// curves spread over many points lie flat with a little stretch instead.
pub const CURVED_POINT_ANGLE: f32 = 60.0;

/// The cuts that let every point inside `piece` (cut along `cuts`) lie
/// flat: from each point that falls more than [`CURVED_POINT_ANGLE`] short
/// of a full turn, the most hidden way to the border, most curved first.
/// None for a piece with no border (it must be opened first, see
/// `disk_cuts`) or with nothing too curved.
pub(crate) fn curvature_cuts(
    mesh: &Mesh,
    piece: &[FaceId],
    cuts: &BTreeSet<EdgeKey>,
    hide: HideSeams,
) -> BTreeSet<EdgeKey> {
    let mut added = BTreeSet::new();
    if piece.is_empty() {
        return added;
    }
    let inside: BTreeSet<FaceId> = piece.iter().copied().collect();
    let joined = |e: &EdgeKey| {
        if cuts.contains(e) {
            return false;
        }
        match mesh.topology().edge_uses(*e) {
            [a, b] => a.face != b.face && inside.contains(&a.face) && inside.contains(&b.face),
            _ => false,
        }
    };
    let chart = Chart::of(mesh, piece, cuts);
    let (sums, on_border, joined_edges) = turns(mesh, &chart, &joined);
    let mut border: BTreeSet<VertexId> = (0..chart.len())
        .filter(|w| on_border[*w])
        .map(|w| chart.points[w])
        .collect();
    if border.is_empty() {
        return added;
    }
    // The points too curved to lie flat, most curved first (lowest id on
    // a tie, so the same shape always gets the same cuts).
    let limit = (360.0 - CURVED_POINT_ANGLE).to_radians();
    let mut curved: Vec<(f32, VertexId)> = (0..chart.len())
        .filter(|w| !on_border[*w] && sums[*w].is_finite() && sums[*w] < limit)
        .map(|w| (sums[w], chart.points[w]))
        .collect();
    curved.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut next: BTreeMap<VertexId, Vec<(VertexId, EdgeKey, f32)>> = BTreeMap::new();
    // Edges marked "never" are left out: a point that can only be reached
    // through them stays uncut, squeezed rather than slit where the user
    // said no.
    for e in joined_edges
        .iter()
        .filter(|e| mesh::uvs::seam_preference(mesh, **e) != SeamPreference::Never)
    {
        let (a, b) = e.ends();
        let cost = path_cost(mesh, *e, hide);
        next.entry(a).or_default().push((b, *e, cost));
        next.entry(b).or_default().push((a, *e, cost));
    }
    for (_, point) in curved {
        // An earlier cut may have reached it already.
        if border.contains(&point) {
            continue;
        }
        let path = to_border(&next, &border, &added, point);
        for e in path {
            let (a, b) = e.ends();
            border.insert(a);
            border.insert(b);
            added.insert(e);
        }
    }
    added
}

/// For each wedge of `chart`, how much the surface round it adds up to
/// (radians, NaN where a polygon there has no size), and whether it is on
/// the border (a side of one of its polygons isn't `joined`); and the
/// edges joined inside the chart.
fn turns(
    mesh: &Mesh,
    chart: &Chart,
    joined: &dyn Fn(&EdgeKey) -> bool,
) -> (Vec<f32>, Vec<bool>, BTreeSet<EdgeKey>) {
    let mut sums = vec![0.0f32; chart.len()];
    let position = |w: usize| mesh.position(chart.points[w]);
    for [a, b, c] in chart.triangles(mesh) {
        for (at, p, q) in [(a, b, c), (b, c, a), (c, a, b)] {
            let angle = match (position(at), position(p), position(q)) {
                (Some(o), Some(p), Some(q)) => {
                    let (u, v) = (p - o, q - o);
                    if u.length_squared() > 0.0 && v.length_squared() > 0.0 {
                        u.angle_between(v)
                    } else {
                        f32::NAN
                    }
                }
                _ => f32::NAN,
            };
            sums[at] += angle;
        }
    }
    let mut on_border = vec![false; chart.len()];
    let mut joined_edges = BTreeSet::new();
    for (f, wedges) in chart.faces.iter().zip(&chart.corners) {
        let list = mesh.face(*f).unwrap_or(&[]);
        for k in 0..list.len() {
            let e = EdgeKey::new(list[k], list[(k + 1) % list.len()]);
            if joined(&e) {
                joined_edges.insert(e);
            } else {
                on_border[wedges[k]] = true;
                on_border[wedges[(k + 1) % list.len()]] = true;
            }
        }
    }
    (sums, on_border, joined_edges)
}

/// The most hidden way from `from` to the nearest point of `border`, along
/// joined edges not already cut (`taken`), as edges; empty when there's
/// none.
fn to_border(
    next: &BTreeMap<VertexId, Vec<(VertexId, EdgeKey, f32)>>,
    border: &BTreeSet<VertexId>,
    taken: &BTreeSet<EdgeKey>,
    from: VertexId,
) -> Vec<EdgeKey> {
    let mut best: BTreeMap<VertexId, (f32, Option<(VertexId, EdgeKey)>)> = BTreeMap::new();
    best.insert(from, (0.0, None));
    let mut queue = BinaryHeap::new();
    queue.push(Step(0.0, from));
    let mut reached = None;
    while let Some(Step(cost, v)) = queue.pop() {
        if best.get(&v).is_some_and(|(c, _)| *c < cost) {
            continue;
        }
        if v != from && border.contains(&v) {
            reached = Some(v);
            break;
        }
        for (w, e, step) in next.get(&v).map(Vec::as_slice).unwrap_or(&[]) {
            if taken.contains(e) {
                continue;
            }
            let total = cost + step;
            if best.get(w).is_none_or(|(c, _)| total < *c) {
                best.insert(*w, (total, Some((v, *e))));
                queue.push(Step(total, *w));
            }
        }
    }
    let mut path = Vec::new();
    let Some(mut at) = reached else {
        return path;
    };
    while let Some((_, Some((back, e)))) = best.get(&at) {
        path.push(*e);
        at = *back;
        if path.len() > best.len() {
            return Vec::new();
        }
    }
    path
}

#[cfg(test)]
mod tests;
