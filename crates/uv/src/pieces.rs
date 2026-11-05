//! Editing pieces by hand ([decision 0236](../../../docs/decisions/0236-editing-pieces.md)):
//! stitching edges together, cutting them apart, lining pieces up,
//! spreading them evenly, turning a piece square to one of its edges, and
//! warping pieces with a lattice.
//!
//! Every function works out new UVs and returns them as polygon corners
//! (`(polygon, point, UV)`), for the caller to record as one edit; none
//! changes the mesh. Locked pieces (decision 0112) are never moved.

use glam::{Affine2, Vec2};
use mesh::uvs::Uvs;
use mesh::{EdgeKey, FaceId, Mesh, VertexId};
use std::collections::{BTreeMap, BTreeSet};

/// A polygon corner and its UV, as `Operation::SetUvs` takes them.
pub type Corner = (FaceId, VertexId, Vec2);

/// Where two UVs count as the same spot.
const SAME: f32 = 1e-5;

/// Every corner's UV, with each polygon's first corner's place in the
/// list.
struct Layout {
    values: Vec<Vec2>,
    starts: Vec<u32>,
}

impl Layout {
    fn of(mesh: &Mesh) -> Option<Self> {
        let uvs = Uvs::of(mesh)?;
        let (starts, corners) = mesh.face_layout();
        Some(Self {
            values: uvs.into_values(corners.len()),
            starts: starts.to_vec(),
        })
    }

    /// The UV of polygon `f`'s corner at point `v`.
    fn at(&self, mesh: &Mesh, f: FaceId, v: VertexId) -> Option<Vec2> {
        Some(self.values[self.index(mesh, f, v)?])
    }

    fn index(&self, mesh: &Mesh, f: FaceId, v: VertexId) -> Option<usize> {
        let k = mesh.face(f)?.iter().position(|w| *w == v)?;
        Some(*self.starts.get(f.get() as usize)? as usize + k)
    }
}

/// Whether polygon `f` is locked (decision 0112).
fn is_locked(lock: &Option<Vec<bool>>, f: FaceId) -> bool {
    lock.as_ref()
        .is_some_and(|l| l.get(f.get() as usize).copied().unwrap_or(false))
}

/// Every corner of `faces` with its UV in `layout`.
fn corners_of(mesh: &Mesh, layout: &Layout, faces: &[FaceId]) -> Vec<Corner> {
    let mut out = Vec::new();
    for f in faces {
        for v in mesh.face(*f).unwrap_or(&[]) {
            if let Some(uv) = layout.at(mesh, *f, *v) {
                out.push((*f, *v, uv));
            }
        }
    }
    out
}

/// The UV islands as a polygon → island number list (`usize::MAX` for a
/// polygon not in the mesh), and the islands.
fn island_map(mesh: &Mesh) -> (BTreeMap<FaceId, usize>, Vec<Vec<FaceId>>) {
    let islands = mesh::uvs::islands(mesh);
    let mut map = BTreeMap::new();
    for (i, island) in islands.iter().enumerate() {
        for f in island {
            map.insert(*f, i);
        }
    }
    (map, islands)
}

/// The similarity (move, turn and size, no mirror) taking `from` to
/// `to`, if the two points of `from` are apart.
fn similarity(from: [Vec2; 2], to: [Vec2; 2]) -> Option<Affine2> {
    let a = from[1] - from[0];
    let b = to[1] - to[0];
    if a.length_squared() < 1e-14 {
        return None;
    }
    // b = s·R·a: as complex numbers, b / a.
    let scale_rot = Vec2::new(
        (b.x * a.x + b.y * a.y) / a.length_squared(),
        (b.y * a.x - b.x * a.y) / a.length_squared(),
    );
    let m = glam::Mat2::from_cols(
        Vec2::new(scale_rot.x, scale_rot.y),
        Vec2::new(-scale_rot.y, scale_rot.x),
    );
    let t = to[0] - m * from[0];
    Some(Affine2::from_mat2_translation(m, t))
}

/// What stitching gives: the corners moved, and the edges joined (no
/// longer cut in the texture, so no longer seams).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stitched {
    /// The moved corners' new UVs.
    pub corners: Vec<Corner>,
    /// The edges now joined.
    pub joined: Vec<EdgeKey>,
}

/// Stitches `edges` of `mesh`: for each edge cut in the texture (its two
/// polygons' UVs differ there), the smaller of the two pieces (fewer
/// polygons; a locked piece never moves) is moved, turned and sized so
/// its side of the edge lies on the other's, and the two corners at each
/// end are made one. Edges that aren't cut, open edges and edges inside
/// one piece are skipped (an edge inside one piece is only joined, not
/// moved). Each piece is moved once; later edges between the same two
/// pieces are joined where they now meet.
pub fn stitch(mesh: &Mesh, edges: &[EdgeKey]) -> Stitched {
    let Some(mut layout) = Layout::of(mesh) else {
        return Stitched::default();
    };
    let lock = mesh::uvs::locked(mesh);
    let topology = mesh.topology();
    let (mut map, islands) = island_map(mesh);
    let mut members: Vec<Vec<FaceId>> = islands;
    let mut moved: BTreeSet<usize> = BTreeSet::new();
    let mut joined = Vec::new();
    for edge in edges {
        let faces: Vec<FaceId> = topology.edge_faces(*edge).collect();
        if faces.len() != 2 {
            continue;
        }
        let (a, b) = edge.ends();
        let (f, g) = (faces[0], faces[1]);
        let (Some(fa), Some(fb), Some(ga), Some(gb)) = (
            layout.at(mesh, f, a),
            layout.at(mesh, f, b),
            layout.at(mesh, g, a),
            layout.at(mesh, g, b),
        ) else {
            continue;
        };
        if fa.distance(ga) <= SAME && fb.distance(gb) <= SAME {
            continue;
        }
        let (Some(&i), Some(&j)) = (map.get(&f), map.get(&g)) else {
            continue;
        };
        if i != j {
            // Which piece moves: the smaller, never a locked one.
            let locked = |n: usize| members[n].iter().any(|h| is_locked(&lock, *h));
            let (stay, go, stay_ends, go_ends) = match (locked(i), locked(j)) {
                (true, true) => continue,
                (true, false) => (i, j, [fa, fb], [ga, gb]),
                (false, true) => (j, i, [ga, gb], [fa, fb]),
                _ if members[j].len() <= members[i].len() => (i, j, [fa, fb], [ga, gb]),
                _ => (j, i, [ga, gb], [fa, fb]),
            };
            if !moved.contains(&go) {
                let Some(fit) = similarity(go_ends, stay_ends) else {
                    continue;
                };
                for h in members[go].clone() {
                    for v in mesh.face(h).unwrap_or(&[]) {
                        if let Some(k) = layout.index(mesh, h, *v) {
                            layout.values[k] = fit.transform_point2(layout.values[k]);
                        }
                    }
                }
                moved.insert(go);
            }
            // The two pieces are one from now on.
            let taken = std::mem::take(&mut members[go]);
            for h in &taken {
                map.insert(*h, stay);
            }
            members[stay].extend(taken);
        } else if members[i].iter().any(|h| is_locked(&lock, *h)) {
            continue;
        }
        // Make the corners at each end one: the other side takes f's.
        for v in [a, b] {
            let (Some(from), Some(to)) = (layout.index(mesh, f, v), layout.index(mesh, g, v)) else {
                continue;
            };
            let spot = layout.values[from];
            // Every corner of g's piece that sat with g's corner moves to
            // the spot too, so the piece stays joined round the point.
            let was = layout.values[to];
            for h in topology.vertex_faces(v).iter().copied() {
                if let Some(k) = layout.index(mesh, h, v)
                    && layout.values[k].distance(was) <= 1e-4
                {
                    layout.values[k] = spot;
                }
            }
        }
        joined.push(*edge);
    }
    if joined.is_empty() {
        return Stitched::default();
    }
    let before = Layout::of(mesh).map(|l| l.values).unwrap_or_default();
    Stitched {
        corners: changed_corners(mesh, &layout, &before),
        joined,
    }
}

/// The corners whose UV in `layout` differs from `before`.
fn changed_corners(mesh: &Mesh, layout: &Layout, before: &[Vec2]) -> Vec<Corner> {
    let mut out = Vec::new();
    for (f, list) in mesh.faces() {
        let start = layout.starts[f.get() as usize] as usize;
        for (k, v) in list.iter().enumerate() {
            let now = layout.values[start + k];
            if before.get(start + k).is_none_or(|b| b.distance(now) > 0.0) {
                out.push((f, *v, now));
            }
        }
    }
    out
}

/// Polygons joined to `start` across edges not in `cuts` and not split
/// in the texture.
fn reach(mesh: &Mesh, layout: &Layout, start: FaceId, cuts: &BTreeSet<EdgeKey>) -> BTreeSet<FaceId> {
    let topology = mesh.topology();
    let mut seen = BTreeSet::from([start]);
    let mut stack = vec![start];
    while let Some(f) = stack.pop() {
        for e in topology.face_edges(f) {
            if cuts.contains(e) {
                continue;
            }
            let (a, b) = e.ends();
            for g in topology.edge_faces(*e) {
                if seen.contains(&g) {
                    continue;
                }
                let same = [a, b].iter().all(|v| match (layout.at(mesh, f, *v), layout.at(mesh, g, *v)) {
                    (Some(p), Some(q)) => p.distance(q) <= SAME,
                    _ => false,
                });
                if same {
                    seen.insert(g);
                    stack.push(g);
                }
            }
        }
    }
    seen
}

/// Cutting `edges` (marked as seams by the caller): where the cut parts a
/// piece in two, the smaller part is nudged `gap` (texture widths) away
/// from the cut, so the two can be told apart and moved. A cut that only
/// slits a piece (it stays one) moves nothing until it is unwrapped.
/// Locked pieces aren't moved. Returns the moved corners.
pub fn cut(mesh: &Mesh, edges: &[EdgeKey], gap: f32) -> Vec<Corner> {
    let Some(mut layout) = Layout::of(mesh) else {
        return Vec::new();
    };
    if !(gap.is_finite() && gap > 0.0) {
        return Vec::new();
    }
    let lock = mesh::uvs::locked(mesh);
    let topology = mesh.topology();
    let mut cuts: BTreeSet<EdgeKey> = mesh::uvs::seams(mesh).into_iter().collect();
    cuts.extend(edges.iter().copied());
    let before = layout.values.clone();
    let mut done: BTreeSet<FaceId> = BTreeSet::new();
    for edge in edges {
        let faces: Vec<FaceId> = topology.edge_faces(*edge).collect();
        if faces.len() != 2 || done.contains(&faces[0]) || done.contains(&faces[1]) {
            continue;
        }
        let side_f = reach(mesh, &layout, faces[0], &cuts);
        if side_f.contains(&faces[1]) {
            continue;
        }
        let side_g = reach(mesh, &layout, faces[1], &cuts);
        let (part, other) = if side_g.len() <= side_f.len() {
            (side_g, faces[0])
        } else {
            (side_f, faces[1])
        };
        if part.iter().any(|f| is_locked(&lock, *f)) {
            continue;
        }
        // Away from the cut: from the other side's middle to this part's.
        let middle = |faces: &mut dyn Iterator<Item = FaceId>| {
            let mut sum = Vec2::ZERO;
            let mut n = 0.0;
            for f in faces {
                for v in mesh.face(f).unwrap_or(&[]) {
                    if let Some(uv) = layout.at(mesh, f, *v) {
                        sum += uv;
                        n += 1.0;
                    }
                }
            }
            (n > 0.0).then(|| sum / n)
        };
        let (a, b) = edge.ends();
        let edge_mid = match (layout.at(mesh, other, a), layout.at(mesh, other, b)) {
            (Some(p), Some(q)) => (p + q) / 2.0,
            _ => continue,
        };
        let Some(part_mid) = middle(&mut part.iter().copied()) else {
            continue;
        };
        let away = (part_mid - edge_mid).normalize_or(Vec2::X);
        for f in &part {
            for v in mesh.face(*f).unwrap_or(&[]) {
                if let Some(k) = layout.index(mesh, *f, *v) {
                    layout.values[k] += away * gap;
                }
            }
            done.insert(*f);
        }
    }
    changed_corners(mesh, &layout, &before)
}

/// How pieces are lined up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    /// Their left sides on the leftmost.
    Left,
    /// Their middles across on the middle of them all.
    Centre,
    /// Their right sides on the rightmost.
    Right,
    /// Their tops on the highest.
    Top,
    /// Their middles up and down on the middle of them all.
    Middle,
    /// Their bottoms on the lowest.
    Bottom,
    /// Spread evenly across, the first and last staying.
    SpreadAcross,
    /// Spread evenly up and down, the first and last staying.
    SpreadUp,
}

impl Align {
    /// Every way, in menu order.
    pub const ALL: [Self; 8] = [
        Self::Left,
        Self::Centre,
        Self::Right,
        Self::Top,
        Self::Middle,
        Self::Bottom,
        Self::SpreadAcross,
        Self::SpreadUp,
    ];
}

/// The box round `faces`' UVs.
fn bounds(mesh: &Mesh, layout: &Layout, faces: &[FaceId]) -> Option<(Vec2, Vec2)> {
    let mut lo = Vec2::splat(f32::INFINITY);
    let mut hi = Vec2::splat(f32::NEG_INFINITY);
    for (_, _, uv) in corners_of(mesh, layout, faces) {
        lo = lo.min(uv);
        hi = hi.max(uv);
    }
    lo.is_finite().then_some((lo, hi))
}

/// Lines up (or spreads out) `pieces` of `mesh` (each a list of its
/// polygons), moving each whole. Up is +V. Locked pieces stay, but still
/// count for where the others go. Fewer than two pieces (three, to
/// spread) move nothing.
pub fn align(mesh: &Mesh, pieces: &[Vec<FaceId>], how: Align) -> Vec<Corner> {
    let Some(layout) = Layout::of(mesh) else {
        return Vec::new();
    };
    let lock = mesh::uvs::locked(mesh);
    let boxes: Vec<(usize, (Vec2, Vec2))> = pieces
        .iter()
        .enumerate()
        .filter_map(|(i, p)| Some((i, bounds(mesh, &layout, p)?)))
        .collect();
    let spreading = matches!(how, Align::SpreadAcross | Align::SpreadUp);
    if boxes.len() < if spreading { 3 } else { 2 } {
        return Vec::new();
    }
    let lo = boxes.iter().fold(Vec2::splat(f32::INFINITY), |m, (_, b)| m.min(b.0));
    let hi = boxes.iter().fold(Vec2::splat(f32::NEG_INFINITY), |m, (_, b)| m.max(b.1));
    let mid = (lo + hi) / 2.0;
    let mut shifts: Vec<(usize, Vec2)> = Vec::new();
    match how {
        Align::SpreadAcross | Align::SpreadUp => {
            let axis = usize::from(how == Align::SpreadUp);
            let mut order: Vec<(usize, f32)> = boxes
                .iter()
                .map(|(i, b)| (*i, ((b.0 + b.1) / 2.0)[axis]))
                .collect();
            order.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
            let first = order[0].1;
            let last = order[order.len() - 1].1;
            let step = (last - first) / (order.len() - 1) as f32;
            for (n, (i, now)) in order.iter().enumerate() {
                let want = first + step * n as f32;
                let mut d = Vec2::ZERO;
                d[axis] = want - now;
                shifts.push((*i, d));
            }
        }
        _ => {
            for (i, (l, h)) in &boxes {
                let c = (*l + *h) / 2.0;
                let d = match how {
                    Align::Left => Vec2::new(lo.x - l.x, 0.0),
                    Align::Right => Vec2::new(hi.x - h.x, 0.0),
                    Align::Centre => Vec2::new(mid.x - c.x, 0.0),
                    Align::Bottom => Vec2::new(0.0, lo.y - l.y),
                    Align::Top => Vec2::new(0.0, hi.y - h.y),
                    _ => Vec2::new(0.0, mid.y - c.y),
                };
                shifts.push((*i, d));
            }
        }
    }
    let mut out = Vec::new();
    for (i, d) in shifts {
        let piece = &pieces[i];
        if d.length_squared() <= 0.0 || piece.iter().any(|f| is_locked(&lock, *f)) {
            continue;
        }
        for (f, v, uv) in corners_of(mesh, &layout, piece) {
            out.push((f, v, uv + d));
        }
    }
    out
}

/// Turns `piece` about the middle of its box so `edge` (an edge of one of
/// its polygons) lies straight across or straight up in the texture,
/// whichever is nearer. A locked piece, an edge not in it or one with no
/// length in the texture gives nothing.
pub fn orient(mesh: &Mesh, piece: &[FaceId], edge: EdgeKey) -> Vec<Corner> {
    let Some(layout) = Layout::of(mesh) else {
        return Vec::new();
    };
    let lock = mesh::uvs::locked(mesh);
    if piece.iter().any(|f| is_locked(&lock, *f)) {
        return Vec::new();
    }
    let (a, b) = edge.ends();
    let Some(f) = piece
        .iter()
        .copied()
        .find(|f| mesh.topology().face_edges(*f).contains(&edge))
    else {
        return Vec::new();
    };
    let (Some(pa), Some(pb)) = (layout.at(mesh, f, a), layout.at(mesh, f, b)) else {
        return Vec::new();
    };
    let d = pb - pa;
    if d.length_squared() < 1e-14 {
        return Vec::new();
    }
    let angle = d.y.atan2(d.x);
    let quarter = std::f32::consts::FRAC_PI_2;
    let target = (angle / quarter).round() * quarter;
    let turn = target - angle;
    if turn.abs() < 1e-7 {
        return Vec::new();
    }
    let Some((lo, hi)) = bounds(mesh, &layout, piece) else {
        return Vec::new();
    };
    let c = (lo + hi) / 2.0;
    let rot = Affine2::from_translation(c)
        * Affine2::from_angle(turn)
        * Affine2::from_translation(-c);
    corners_of(mesh, &layout, piece)
        .into_iter()
        .map(|(f, v, uv)| (f, v, rot.transform_point2(uv)))
        .collect()
}

/// A lattice over the box `lo` to `hi`: `n` by `n` control points (row by
/// row from the bottom), starting evenly spaced.
pub fn lattice_grid(lo: Vec2, hi: Vec2, n: usize) -> Vec<Vec2> {
    let n = n.clamp(2, 8);
    let mut out = Vec::with_capacity(n * n);
    for j in 0..n {
        for i in 0..n {
            let s = i as f32 / (n - 1) as f32;
            let t = j as f32 / (n - 1) as f32;
            out.push(lo + (hi - lo) * Vec2::new(s, t));
        }
    }
    out
}

/// Where `uv` goes when the lattice over `lo` to `hi` (its `n` by `n`
/// points evenly spaced at first) is pulled to `points`: piecewise
/// bilinear between the four control points round it. Outside the box
/// the nearest cell carries on.
pub fn warp_point(uv: Vec2, lo: Vec2, hi: Vec2, n: usize, points: &[Vec2]) -> Vec2 {
    let n = n.clamp(2, 8);
    if points.len() != n * n {
        return uv;
    }
    let size = (hi - lo).max(Vec2::splat(1e-9));
    let st = (uv - lo) / size * (n - 1) as f32;
    let ci = (st.x.floor() as i64).clamp(0, n as i64 - 2) as usize;
    let cj = (st.y.floor() as i64).clamp(0, n as i64 - 2) as usize;
    let (s, t) = (st.x - ci as f32, st.y - cj as f32);
    let p = |i: usize, j: usize| points[j * n + i];
    let bottom = p(ci, cj).lerp(p(ci + 1, cj), s);
    let top = p(ci, cj + 1).lerp(p(ci + 1, cj + 1), s);
    bottom.lerp(top, t)
}

/// `corners` (with their UVs before the warp) moved by the lattice
/// (see [`warp_point`]).
pub fn warp(corners: &[Corner], lo: Vec2, hi: Vec2, n: usize, points: &[Vec2]) -> Vec<Corner> {
    corners
        .iter()
        .map(|(f, v, uv)| (*f, *v, warp_point(*uv, lo, hi, n, points)))
        .filter(|(_, _, uv)| uv.is_finite())
        .collect()
}

/// Every corner of `faces` with its UV now (for a lattice to warp from),
/// skipping locked polygons.
pub fn corners_now(mesh: &Mesh, faces: &[FaceId]) -> Vec<Corner> {
    let Some(layout) = Layout::of(mesh) else {
        return Vec::new();
    };
    let lock = mesh::uvs::locked(mesh);
    let free: Vec<FaceId> = faces.iter().copied().filter(|f| !is_locked(&lock, *f)).collect();
    corners_of(mesh, &layout, &free)
}

/// The pieces (UV islands) holding any of `faces`, each a list of its
/// polygons, in order.
pub fn pieces_holding(mesh: &Mesh, faces: &[FaceId]) -> Vec<Vec<FaceId>> {
    let wanted: BTreeSet<FaceId> = faces.iter().copied().collect();
    mesh::uvs::islands(mesh)
        .into_iter()
        .filter(|island| island.iter().any(|f| wanted.contains(f)))
        .collect()
}

#[cfg(test)]
mod tests;
