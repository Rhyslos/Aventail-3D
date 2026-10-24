//! A shell's rims ([decision 0154](../../../../docs/decisions/0154-shell.md)):
//! their hard edges, their UVs and their seams.
//!
//! The rims along one open border make a **strip**, followed round from
//! polygon to polygon. If the shape has UVs, each strip is laid flat in
//! a row of its own just above the texture square: along U as far as the
//! border is long, up V as far as the shell is thick, both at the shape's
//! texel density, so a rim's texture is as sharp as the surface's and
//! never overlaps the shape's own UVs. The rows stack upward.
//!
//! Each rim's edges along the two layers are marked **hard** (the rim
//! shades as a face of its own) and, with UVs, as **seams**, as is where
//! a strip's row starts and ends (a closed border's strip is cut once).

use crate::uvs::{Uvs, average_density, has_uvs, set_seams, set_uvs, uv_splits};
use crate::{EdgeKey, FaceId, Mesh, VertexId};
use glam::Vec2;
use std::collections::{BTreeSet, HashMap};

/// The space left between rows, and below the first, in texture widths.
const GAP: f32 = 0.02;

/// One rim polygon, filling one open border edge. Its corners are, in
/// order: `front.1`, `front.0`, `behind.0`, `behind.1`.
pub(super) struct Rim {
    /// The polygon.
    pub(super) face: FaceId,
    /// The border edge on the layer in front, the way its polygon runs.
    pub(super) front: (VertexId, VertexId),
    /// Its copy on the layer behind.
    pub(super) behind: (VertexId, VertexId),
}

/// Marks the rims' edges hard, and gives them UVs and seams if the shape
/// (`input`, before the shell) has UVs.
pub(super) fn finish(out: &mut Mesh, input: &Mesh, rims: &[Rim]) {
    if rims.is_empty() {
        return;
    }
    for rim in rims {
        out.set_hard(rim.front.0, rim.front.1, true);
        out.set_hard(rim.behind.0, rim.behind.1, true);
    }
    if !has_uvs(out) {
        return;
    }
    let density = average_density(input)
        .or_else(|| {
            let size = input.bounds()?.size().max_element();
            (size > 1e-6).then(|| 1.0 / size)
        })
        .filter(|d| d.is_finite() && *d > 0.0)
        .unwrap_or(1.0);
    lay_out(out, rims, density);
    let along: Vec<EdgeKey> = rims
        .iter()
        .flat_map(|r| {
            [
                EdgeKey::new(r.front.0, r.front.1),
                EdgeKey::new(r.behind.0, r.behind.1),
            ]
        })
        .collect();
    set_seams(out, &along, true);
    let across: BTreeSet<EdgeKey> = rims
        .iter()
        .flat_map(|r| {
            [
                EdgeKey::new(r.front.0, r.behind.0),
                EdgeKey::new(r.front.1, r.behind.1),
            ]
        })
        .collect();
    let cuts: Vec<EdgeKey> = uv_splits(out)
        .into_iter()
        .filter(|e| across.contains(e))
        .collect();
    set_seams(out, &cuts, true);
}

/// Gives every rim its UVs: a row per strip (see the module's notes).
fn lay_out(out: &mut Mesh, rims: &[Rim], density: f32) {
    let corner_count = out.face_layout().1.len();
    let starts: Vec<u32> = out.face_layout().0.to_vec();
    let Some(uvs) = Uvs::of(out) else {
        return;
    };
    let mut values = uvs.into_values(corner_count);
    let place = |v: VertexId| out.position(v).unwrap_or_default();
    let mut base = 1.0 + GAP;
    for strip in strips(rims) {
        let mut u = 0.0f32;
        let mut tallest = 0.0f32;
        for i in strip {
            let rim = &rims[i];
            let (a, b) = rim.front;
            let (a2, b2) = rim.behind;
            let long = place(a).distance(place(b)) * density;
            let high_a = place(a).distance(place(a2)) * density;
            let high_b = place(b).distance(place(b2)) * density;
            tallest = tallest.max(high_a).max(high_b);
            let Some(start) = starts.get(rim.face.get() as usize) else {
                continue;
            };
            let start = *start as usize;
            let corners = [
                Vec2::new(u + long, base + high_b),
                Vec2::new(u, base + high_a),
                Vec2::new(u, base),
                Vec2::new(u + long, base),
            ];
            for (k, uv) in corners.into_iter().enumerate() {
                if let Some(slot) = values.get_mut(start + k) {
                    *slot = if uv.is_finite() { uv } else { Vec2::ZERO };
                }
            }
            u += long;
        }
        base += tallest + GAP;
    }
    set_uvs(out, values);
}

/// The rims in strips, each followed round its border from polygon to
/// polygon (indices into `rims`), in a fixed order. An open strip starts
/// at its first end; a closed one where it was first met.
fn strips(rims: &[Rim]) -> Vec<Vec<usize>> {
    let mut from: HashMap<VertexId, Vec<usize>> = HashMap::new();
    let mut to: HashMap<VertexId, Vec<usize>> = HashMap::new();
    for (i, rim) in rims.iter().enumerate() {
        from.entry(rim.front.0).or_default().push(i);
        to.entry(rim.front.1).or_default().push(i);
    }
    let mut used = vec![false; rims.len()];
    let mut out = Vec::new();
    for first in 0..rims.len() {
        if used[first] {
            continue;
        }
        // Back to the strip's start.
        let mut start = first;
        for _ in 0..rims.len() {
            let before = to
                .get(&rims[start].front.0)
                .and_then(|list| list.iter().copied().find(|j| !used[*j] && *j != start));
            match before {
                Some(j) if j != first => start = j,
                Some(_) => {
                    start = first;
                    break;
                }
                None => break,
            }
        }
        // Then forward to its end.
        let mut strip = vec![start];
        used[start] = true;
        let mut at = start;
        while let Some(next) = from
            .get(&rims[at].front.1)
            .and_then(|list| list.iter().copied().find(|j| !used[*j]))
        {
            used[next] = true;
            strip.push(next);
            at = next;
        }
        out.push(strip);
    }
    out
}
