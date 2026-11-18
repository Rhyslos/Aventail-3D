//! Telling quickly whether a shape is closed, and closing its holes
//! first when remeshing for sculpting (decision 0312).
//!
//! A shape of millions of polygons is checked by its points' ids: each
//! edge, sorted, should be used an even number of times. The few edges
//! that aren't are checked again by position, so pieces whose points
//! sit at the same place (a seam split for UVs, say) still close up.
//! Each hole left is closed with a fan of triangles round its middle.

use super::Soup;
use crate::{FaceId, Mesh};
use glam::Vec3;
use std::collections::HashMap;

/// Points closer than this (in centimetres) are one point.
const SAME_SPOT: f32 = 1e-3;

/// An edge on a hole's rim: from `a` to `b` the way its polygon goes
/// round, and the polygon.
#[derive(Debug, Clone, Copy)]
pub(super) struct OpenEdge {
    a: Vec3,
    b: Vec3,
    face: FaceId,
}

type Key = [i64; 3];

fn key(p: Vec3) -> Key {
    let q = (p / SAME_SPOT).round();
    [q.x as i64, q.y as i64, q.z as i64]
}

/// The edges of `mesh` on the rims of holes (used an odd number of times,
/// counting points at the same place as one), each the way the polygon
/// beside it goes round.
pub(super) fn open_edges(mesh: &Mesh) -> Vec<OpenEdge> {
    // By id first: each edge as its two ids, the lower first, and a last
    // bit for which way the polygon goes.
    // Gathered and sorted on several threads for a heavy shape (decision
    // 0313).
    let count = mesh.face_slots().len();
    let parts = crate::parallel::in_parallel(count, |range| {
        let mut edges: Vec<u64> = Vec::with_capacity(range.len() * 4);
        for f in range {
            let Some(corners) = mesh.face(crate::FaceId::from_raw(f as u32)) else {
                continue;
            };
            let n = corners.len();
            for i in 0..n {
                let (a, b) = (
                    u64::from(corners[i].get()),
                    u64::from(corners[(i + 1) % n].get()),
                );
                if a == b {
                    continue;
                }
                let (low, high, way) = if a < b { (a, b, 0) } else { (b, a, 1) };
                edges.push((low << 33) | (high << 1) | way);
            }
        }
        edges
    });
    let mut edges = parts.concat();
    crate::parallel::sort(&mut edges);
    let mut odd: Vec<(u32, u32, bool)> = Vec::new();
    let mut at = 0;
    while at < edges.len() {
        let pair = edges[at] >> 1;
        let mut end = at;
        let mut net = 0i64;
        while end < edges.len() && edges[end] >> 1 == pair {
            net += if edges[end] & 1 == 0 { 1 } else { -1 };
            end += 1;
        }
        if (end - at) % 2 == 1 {
            let (low, high) = ((pair >> 32) as u32, (pair & 0xFFFF_FFFF) as u32);
            odd.push((low, high, net > 0));
        }
        at = end;
    }
    if odd.is_empty() {
        return Vec::new();
    }
    // Then by position, for the few odd ones: each with its polygon.
    let wanted: HashMap<(u32, u32), bool> = odd.iter().map(|(a, b, w)| ((*a, *b), *w)).collect();
    let mut found = Vec::new();
    for (face, corners) in mesh.faces() {
        let n = corners.len();
        for i in 0..n {
            let (a, b) = (corners[i], corners[(i + 1) % n]);
            let (low, high) = if a.get() < b.get() {
                (a.get(), b.get())
            } else {
                (b.get(), a.get())
            };
            let Some(forward) = wanted.get(&(low, high)) else {
                continue;
            };
            // The edge's net way round, as one of its polygons goes.
            if (a.get() < b.get()) != *forward {
                continue;
            }
            if let (Some(pa), Some(pb)) = (mesh.position(a), mesh.position(b)) {
                found.push(OpenEdge { a: pa, b: pb, face });
            }
        }
    }
    found.sort_by_key(|e| (key(e.a), key(e.b), e.face));
    found.dedup_by_key(|e| (key(e.a), key(e.b)));
    let mut count: HashMap<(Key, Key), i32> = HashMap::new();
    for e in &found {
        let (ka, kb) = (key(e.a), key(e.b));
        if ka == kb {
            continue;
        }
        if ka < kb {
            *count.entry((ka, kb)).or_insert(0) += 1;
        } else {
            *count.entry((kb, ka)).or_insert(0) += 1;
        }
    }
    found.retain(|e| {
        let (ka, kb) = (key(e.a), key(e.b));
        ka != kb
            && count
                .get(&(ka.min(kb), ka.max(kb)))
                .is_some_and(|n| n % 2 == 1)
    });
    found
}

/// Closes the holes `rims` of the shape `soup` was made from: each loop
/// of rim edges gets a fan of triangles round its middle, with the
/// polygon beside its first edge's material (`face` of `mesh` for
/// labels). How many rim edges were left open (on no loop).
pub(super) fn close(soup: &mut Soup, rims: &[OpenEdge], mesh: &Mesh) -> usize {
    let mut starting: HashMap<Key, Vec<usize>> = HashMap::new();
    for (n, e) in rims.iter().enumerate() {
        starting.entry(key(e.a)).or_default().push(n);
    }
    let mut used = vec![false; rims.len()];
    let mut left = 0;
    for first in 0..rims.len() {
        if used[first] {
            continue;
        }
        used[first] = true;
        let mut chain = vec![first];
        let end = key(rims[first].a);
        let mut at = key(rims[first].b);
        let mut closed = at == end;
        while !closed {
            let Some(next) = starting
                .get(&at)
                .and_then(|list| list.iter().copied().find(|n| !used[*n]))
            else {
                break;
            };
            used[next] = true;
            chain.push(next);
            at = key(rims[next].b);
            closed = at == end;
        }
        if !closed || chain.len() < 3 {
            left += chain.len();
            continue;
        }
        let middle = chain.iter().map(|n| rims[*n].a).sum::<Vec3>() / chain.len() as f32;
        let face = rims[first].face;
        let slot = crate::material::slot_of(mesh, face);
        for n in chain {
            let e = rims[n];
            soup.triangles.push([e.b, e.a, middle]);
            soup.slots.push(slot);
            soup.faces.push(face.get());
        }
    }
    left
}
