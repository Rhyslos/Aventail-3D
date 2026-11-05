//! Selecting UVs the smart ways, and copying UVs between matching pieces
//! ([decision 0239](../../../docs/decisions/0239-uv-selection-and-copying.md)).
//!
//! Every function names polygons (or edges) of one shape for the caller
//! to select; [`copy_layout`] works out corners for `Operation::SetUvs`.
//! None changes the mesh.

use crate::pieces::Corner;
use glam::Vec2;
use mesh::uvs::Uvs;
use mesh::{EdgeKey, FaceId, Mesh, VertexId};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, VecDeque};

/// Where two UVs count as one.
const SAME: f32 = 1e-5;

/// The polygons lying over others in the texture (or folded over
/// themselves), as the UV Editor's red warning shows them.
pub fn overlapping(mesh: &Mesh) -> Vec<FaceId> {
    crate::warnings::warnings(&[mesh], |_, _| 0u8, &[])
        .pop()
        .map(|w| w.overlapping.into_iter().collect())
        .unwrap_or_default()
}

/// The polygons of mirrored pieces (laid face down), as the purple
/// warning shows them.
pub fn mirrored(mesh: &Mesh) -> Vec<FaceId> {
    crate::warnings::warnings(&[mesh], |_, _| 0u8, &[])
        .pop()
        .map(|w| w.mirrored.into_iter().collect())
        .unwrap_or_default()
}

/// The polygons stretched at least `factor` times (one way more than the
/// other, or bigger or smaller than the shape's usual), or with no area.
pub fn stretched(mesh: &Mesh, factor: f32) -> Vec<FaceId> {
    let limit = factor.max(1.0).log2();
    crate::stretch::stretch_of(mesh)
        .unwrap_or_default()
        .into_iter()
        .filter(|(_, s)| s.off() >= limit)
        .map(|(f, _)| f)
        .collect()
}

/// A piece's size and shape, to compare with others.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Signature {
    polygons: usize,
    corners: usize,
    uv_area: f32,
    real_area: f32,
}

fn signature(mesh: &Mesh, uvs: &Uvs, piece: &[FaceId]) -> Signature {
    let (flat, real) = crate::density::areas(mesh, piece, uvs);
    Signature {
        polygons: piece.len(),
        corners: piece
            .iter()
            .map(|f| mesh.face(*f).map_or(0, <[VertexId]>::len))
            .sum(),
        uv_area: flat,
        real_area: real,
    }
}

fn close(a: f32, b: f32, within: f32) -> bool {
    (a - b).abs() <= within * a.abs().max(b.abs()).max(1e-12)
}

/// The polygons of every piece like those holding `faces`: the same
/// number of polygons and corners, and areas (in the texture and on the
/// shape) within `within` (a part: 0.02 is 2%) of one of them.
pub fn similar(mesh: &Mesh, faces: &[FaceId], within: f32) -> Vec<FaceId> {
    let Some(uvs) = Uvs::of(mesh) else {
        return Vec::new();
    };
    let wanted: BTreeSet<FaceId> = faces.iter().copied().collect();
    let pieces = mesh::uvs::islands(mesh);
    let chosen: Vec<Signature> = pieces
        .iter()
        .filter(|p| p.iter().any(|f| wanted.contains(f)))
        .map(|p| signature(mesh, &uvs, p))
        .collect();
    let mut out = Vec::new();
    for piece in &pieces {
        let s = signature(mesh, &uvs, piece);
        let like = chosen.iter().any(|c| {
            c.polygons == s.polygons
                && c.corners == s.corners
                && close(c.uv_area, s.uv_area, within)
                && close(c.real_area, s.real_area, within)
        });
        if like {
            out.extend(piece.iter().copied());
        }
    }
    out.sort();
    out
}

/// Whether polygons `f` and `g` meet along `edge` with the same UVs.
fn joined(mesh: &Mesh, uvs: &Uvs, edge: EdgeKey, f: FaceId, g: FaceId) -> bool {
    let (starts, _) = mesh.face_layout();
    let at = |face: FaceId, v: VertexId| -> Option<Vec2> {
        let k = mesh.face(face)?.iter().position(|w| *w == v)?;
        Some(uvs.at(starts[face.get() as usize] as usize + k))
    };
    let (a, b) = edge.ends();
    [a, b].iter().all(|v| match (at(f, *v), at(g, *v)) {
        (Some(p), Some(q)) => p.distance(q) <= SAME,
        _ => false,
    })
}

/// `faces` and the polygons next to them in the texture (across a side
/// with the same UVs on both sides): growing never crosses a seam.
pub fn grow(mesh: &Mesh, faces: &[FaceId]) -> Vec<FaceId> {
    let Some(uvs) = Uvs::of(mesh) else {
        return faces.to_vec();
    };
    let topology = mesh.topology();
    let mut out: BTreeSet<FaceId> = faces.iter().copied().filter(|f| mesh.face(*f).is_some()).collect();
    for f in out.clone() {
        for e in topology.face_edges(f) {
            for g in topology.edge_faces(*e) {
                if g != f && joined(mesh, &uvs, *e, f, g) {
                    out.insert(g);
                }
            }
        }
    }
    out.into_iter().collect()
}

/// `faces` without those at the selection's rim in the texture: a polygon
/// stays if every polygon next to it in the texture is selected too (a
/// side on a seam or the shape's edge counts as the rim).
pub fn shrink(mesh: &Mesh, faces: &[FaceId]) -> Vec<FaceId> {
    let Some(uvs) = Uvs::of(mesh) else {
        return Vec::new();
    };
    let topology = mesh.topology();
    let chosen: BTreeSet<FaceId> = faces.iter().copied().collect();
    chosen
        .iter()
        .copied()
        .filter(|f| {
            topology.face_edges(*f).iter().all(|e| {
                let others: Vec<FaceId> = topology.edge_faces(*e).filter(|g| g != f).collect();
                !others.is_empty()
                    && others
                        .iter()
                        .all(|g| chosen.contains(g) && joined(mesh, &uvs, *e, *f, *g))
            })
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Reach(f32, VertexId);

impl Eq for Reach {}

impl PartialOrd for Reach {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Reach {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.total_cmp(&self.0).then_with(|| other.1.cmp(&self.1))
    }
}

/// The shortest way in the texture from point `from` to `to` along edges
/// joined in the texture (never across a seam), each edge as long as it
/// is in the texture. `None` when there's no such way (another piece).
pub fn shortest_path(mesh: &Mesh, from: VertexId, to: VertexId) -> Option<Vec<EdgeKey>> {
    let uvs = Uvs::of(mesh)?;
    let topology = mesh.topology();
    if mesh.position(from).is_none() || mesh.position(to).is_none() {
        return None;
    }
    if from == to {
        return Some(Vec::new());
    }
    let (starts, _) = mesh.face_layout();
    // Each edge's length in the texture, if it is joined (or open).
    let length = |e: EdgeKey| -> Option<f32> {
        let faces: Vec<FaceId> = topology.edge_faces(e).collect();
        let f = *faces.first()?;
        if faces.len() == 2 && !joined(mesh, &uvs, e, faces[0], faces[1]) {
            return None;
        }
        let (a, b) = e.ends();
        let list = mesh.face(f)?;
        let at = |v: VertexId| -> Option<Vec2> {
            let k = list.iter().position(|w| *w == v)?;
            Some(uvs.at(starts[f.get() as usize] as usize + k))
        };
        Some(at(a)?.distance(at(b)?))
    };
    let mut best: HashMap<VertexId, (f32, Option<EdgeKey>)> = HashMap::new();
    let mut heap = BinaryHeap::new();
    best.insert(from, (0.0, None));
    heap.push(Reach(0.0, from));
    while let Some(Reach(d, v)) = heap.pop() {
        if v == to {
            break;
        }
        if best.get(&v).is_some_and(|(b, _)| d > *b) {
            continue;
        }
        for &e in topology.vertex_edges(v) {
            let Some(len) = length(e) else {
                continue;
            };
            let (a, b) = e.ends();
            let w = if a == v { b } else { a };
            let nd = d + len;
            if best.get(&w).is_none_or(|(b, _)| nd < *b) {
                best.insert(w, (nd, Some(e)));
                heap.push(Reach(nd, w));
            }
        }
    }
    best.get(&to)?;
    let mut path = Vec::new();
    let mut at = to;
    while let Some((_, Some(e))) = best.get(&at) {
        path.push(*e);
        let (a, b) = e.ends();
        at = if a == at { b } else { a };
    }
    path.reverse();
    Some(path)
}

/// Matches `target`'s polygons and corners to `source`'s, walking out from
/// a first polygon with the same number of corners, each turn of it tried
/// until the whole piece matches. Returns the target corner for each
/// source corner.
fn match_pieces(
    mesh: &Mesh,
    source: &[FaceId],
    target: &[FaceId],
) -> Option<BTreeMap<(FaceId, VertexId), (FaceId, VertexId)>> {
    if source.len() != target.len() || source.is_empty() {
        return None;
    }
    let topology = mesh.topology();
    let src: BTreeSet<FaceId> = source.iter().copied().collect();
    let dst: BTreeSet<FaceId> = target.iter().copied().collect();
    let s0 = source[0];
    let n = mesh.face(s0)?.len();
    for &t0 in target {
        if mesh.face(t0)?.len() != n {
            continue;
        }
        'turn: for turn in 0..n {
            let mut point: HashMap<VertexId, VertexId> = HashMap::new();
            let mut face: HashMap<FaceId, FaceId> = HashMap::new();
            let mut taken: BTreeSet<FaceId> = BTreeSet::new();
            let sl = mesh.face(s0)?;
            let tl = mesh.face(t0)?;
            for k in 0..n {
                point.insert(sl[k], tl[(k + turn) % n]);
            }
            face.insert(s0, t0);
            taken.insert(t0);
            let mut queue = VecDeque::from([s0]);
            while let Some(sf) = queue.pop_front() {
                for e in topology.face_edges(sf) {
                    let (a, b) = e.ends();
                    let (Some(&ta), Some(&tb)) = (point.get(&a), point.get(&b)) else {
                        continue;
                    };
                    let te = EdgeKey::new(ta, tb);
                    for sg in topology.edge_faces(*e) {
                        if sg == sf || !src.contains(&sg) || face.contains_key(&sg) {
                            continue;
                        }
                        // The target's polygon across the same side.
                        let tf = face[&sf];
                        let Some(tg) = topology
                            .edge_faces(te)
                            .find(|g| *g != tf && dst.contains(g) && !taken.contains(g))
                        else {
                            continue 'turn;
                        };
                        let gl = mesh.face(sg)?;
                        let hl = mesh.face(tg)?;
                        if gl.len() != hl.len() {
                            continue 'turn;
                        }
                        // Line its corners up on the shared side.
                        let m = gl.len();
                        let ka = gl.iter().position(|v| *v == a)?;
                        let Some(ja) = hl.iter().position(|v| *v == ta) else {
                            continue 'turn;
                        };
                        let forward = gl[(ka + 1) % m] == b;
                        let target_forward = hl[(ja + 1) % m] == tb;
                        if forward != target_forward {
                            continue 'turn;
                        }
                        for k in 0..m {
                            let sv = gl[(ka + k) % m];
                            let tv = hl[(ja + k) % m];
                            match point.get(&sv) {
                                Some(have) if *have != tv => continue 'turn,
                                _ => {
                                    point.insert(sv, tv);
                                }
                            }
                        }
                        face.insert(sg, tg);
                        taken.insert(tg);
                        queue.push_back(sg);
                    }
                }
            }
            if face.len() != source.len() {
                continue;
            }
            let mut out = BTreeMap::new();
            for (sf, tf) in &face {
                for v in mesh.face(*sf)? {
                    out.insert((*sf, *v), (*tf, point[v]));
                }
            }
            return Some(out);
        }
    }
    None
}

/// Copies `source`'s UV layout onto each of `targets` (pieces with the
/// same make-up: polygons of the same corner counts joined the same way),
/// each placed with its box's middle where its own was. Pieces that don't
/// match are skipped; returns the corners, and how many pieces took it.
pub fn copy_layout(mesh: &Mesh, source: &[FaceId], targets: &[Vec<FaceId>]) -> (Vec<Corner>, usize) {
    let Some(uvs) = Uvs::of(mesh) else {
        return (Vec::new(), 0);
    };
    let (starts, _) = mesh.face_layout();
    let at = |f: FaceId, v: VertexId| -> Option<Vec2> {
        let k = mesh.face(f)?.iter().position(|w| *w == v)?;
        Some(uvs.at(starts[f.get() as usize] as usize + k))
    };
    let lock = mesh::uvs::locked(mesh);
    let Some((slo, shi)) = mesh::uvs::uv_bounds(mesh, source, &[]) else {
        return (Vec::new(), 0);
    };
    let source_mid = (slo + shi) / 2.0;
    let mut out = Vec::new();
    let mut count = 0;
    for target in targets {
        if target.iter().any(|f| {
            lock.as_ref()
                .is_some_and(|l| l.get(f.get() as usize).copied().unwrap_or(false))
        }) {
            continue;
        }
        let Some(map) = match_pieces(mesh, source, target) else {
            continue;
        };
        let Some((tlo, thi)) = mesh::uvs::uv_bounds(mesh, target, &[]) else {
            continue;
        };
        let shift = (tlo + thi) / 2.0 - source_mid;
        for ((sf, sv), (tf, tv)) in map {
            if let Some(uv) = at(sf, sv) {
                out.push((tf, tv, uv + shift));
            }
        }
        count += 1;
    }
    (out, count)
}

#[cfg(test)]
mod tests;
