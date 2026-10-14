//! Slit: cutting new edges along a line drawn over the shape
//! ([decision 0049](../../../docs/decisions/0049-slit.md)).
//!
//! The line is a chain of points, seen from an eye (the camera when it was
//! drawn). Each stretch of it between two points sweeps a flat fan out from
//! the eye, and wherever that fan crosses a polygon, the polygon is cut
//! along it. In a flat (orthographic) view the eye is a direction instead,
//! and each stretch sweeps a flat strip. A line point inside a polygon
//! becomes a new point there, so the cut can turn inside a polygon.
//!
//! **Closed lines** ([decision 0058](../../../docs/decisions/0058-closed-slit.md)).
//! A line whose last point is its first ([`SlitLine::is_closed`]) runs
//! round in a loop. Crossing polygons, it cuts like any line, the stretch
//! back to the start included. Drawn wholly inside one polygon, it cuts a
//! shape into it: the shape becomes a polygon of its own, and the ring
//! round it is cut in two from the polygon's corners, so every polygon
//! stays a simple one without a hole.
//!
//! **Visible only, or through.** Unless the slit cuts *through*, only
//! polygons that face the eye and aren't hidden behind others are cut, as
//! you'd expect from cutting what you see. Through, every polygon under the
//! line is cut, the far side too.
//!
//! The cut is worked out from the shape as it is, not from ids, so a slit
//! keeps cutting the same place when the shape's settings change.
//!
//! As with every edit, polygons keep their ids (each cut polygon keeps its
//! id for one piece), new points and polygons take the next ids in a fixed
//! order, and values come along through the [`Builder`]: points on an edge
//! blend its ends, points inside a polygon blend its corners, and a cut
//! edge's values go to each of its pieces. Polygons beside a cut edge that
//! aren't cut themselves take the new point into their corners, so no gap
//! opens.

use crate::builder::{Builder, CornerFrom, PointFrom};
use crate::edit::Outcome;
use crate::{EdgeKey, FaceId, Mesh, VertexId};
use glam::Vec3;
use std::collections::{BTreeMap, BTreeSet};

/// The most points a slit's line may have.
pub const MAX_LINE_POINTS: usize = 256;

/// Where a slit's line is seen from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Eye {
    /// From this point (a perspective view).
    At(Vec3),
    /// Looking this way from far off (a flat view); need not be unit length.
    Along(Vec3),
}

/// A slit: its line's points (in the mesh's own space), where it is seen
/// from, and whether it cuts through to the far side.
#[derive(Debug, Clone, PartialEq)]
pub struct SlitLine {
    /// The line's points, in order. Only their direction from the eye
    /// counts, not how far away they are.
    pub points: Vec<Vec3>,
    /// Where the line is seen from.
    pub eye: Eye,
    /// Cut every polygon under the line, not only the ones seen.
    pub through: bool,
}

impl SlitLine {
    /// Whether the line runs round in a loop: its last point is its first
    /// (and it has at least three others). Closed lines are saved this way,
    /// so no new field is needed in project files.
    pub fn is_closed(&self) -> bool {
        let p = &self.points;
        p.len() >= 4 && p.first() == p.last()
    }
}

/// A closed line drawn wholly inside one polygon: the spots of its
/// points there, in order.
#[derive(Debug, Clone, PartialEq)]
struct InnerLoop {
    face: FaceId,
    spots: Vec<Vec3>,
}

/// Something the line meets on a polygon's border or inside it.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Meets {
    /// One of the polygon's points.
    Point(VertexId),
    /// Its edge from the lower point id to the higher, `t` of the way, cut
    /// by stretch number `.2` of the line.
    Edge(EdgeKey, f32, usize),
    /// A spot inside it (a line point).
    Inside(Vec3),
}

/// One cut across a polygon: in at a point on its border, through any
/// spots inside, out at another point on its border.
#[derive(Debug, Clone, PartialEq)]
struct Chain {
    face: FaceId,
    start: Meets,
    inside: Vec<Vec3>,
    end: Meets,
}

/// The geometry of a line: its stretches as fans (or strips) from the eye.
struct Sweep<'a> {
    line: &'a SlitLine,
    /// A length beyond anything in the mesh, for rays from a flat view.
    far: f32,
    /// How close counts as touching.
    near: f32,
}

impl Sweep<'_> {
    /// The ray towards line point `p`: where it starts and which way.
    fn ray(&self, p: Vec3) -> Option<(Vec3, Vec3)> {
        match self.line.eye {
            Eye::At(e) => Some((e, (p - e).try_normalize()?)),
            Eye::Along(d) => {
                let d = d.try_normalize()?;
                Some((p - d * self.far, d))
            }
        }
    }

    /// The flat fan of stretch `s`: a point on it and its facing (unit).
    fn plane(&self, s: usize) -> Option<(Vec3, Vec3)> {
        let (a, b) = (self.line.points[s], self.line.points[s + 1]);
        let normal = match self.line.eye {
            Eye::At(e) => (a - e).cross(b - e),
            Eye::Along(d) => (b - a).cross(d),
        };
        Some((a, normal.try_normalize()?))
    }

    /// How far along stretch `s` the point `x` (on its fan) is seen: 0 at
    /// its first point, 1 at its second; `None` behind the eye.
    fn along(&self, s: usize, x: Vec3) -> Option<f32> {
        let (a, b) = (self.line.points[s], self.line.points[s + 1]);
        match self.line.eye {
            Eye::At(e) => {
                // x - e = α (a - e) + β (b - e), both in front of the eye.
                let (u, v, w) = (a - e, b - e, x - e);
                let (uu, uv, vv) = (u.dot(u), u.dot(v), v.dot(v));
                let (wu, wv) = (w.dot(u), w.dot(v));
                let det = uu * vv - uv * uv;
                if det.abs() < 1e-12 {
                    return None;
                }
                let alpha = (wu * vv - wv * uv) / det;
                let beta = (wv * uu - wu * uv) / det;
                let sum = alpha + beta;
                (sum > 1e-9).then(|| beta / sum)
            }
            Eye::Along(d) => {
                let d = d.try_normalize()?;
                let flat = |p: Vec3| p - d * p.dot(d);
                let (fa, fb, fx) = (flat(a), flat(b), flat(x));
                let span = fb - fa;
                let len = span.length_squared();
                (len > 1e-12).then(|| (fx - fa).dot(span) / len)
            }
        }
    }
}

/// A polygon's facing (Newell's method, unit) and middle.
fn facing(mesh: &Mesh, face: FaceId) -> (Vec3, Vec3) {
    let corners: Vec<Vec3> = mesh
        .face(face)
        .unwrap_or(&[])
        .iter()
        .filter_map(|v| mesh.position(*v))
        .collect();
    let n = corners.len().max(1);
    let middle = corners.iter().copied().sum::<Vec3>() / n as f32;
    (mesh.face_normal(face), middle)
}

/// Whether `p`, on the plane of `face`, lies inside it and further than
/// `near` from its border.
fn strictly_inside(mesh: &Mesh, face: FaceId, normal: Vec3, p: Vec3, near: f32) -> bool {
    let Some(corners) = mesh.face(face) else {
        return false;
    };
    let points: Vec<Vec3> = corners.iter().filter_map(|v| mesh.position(*v)).collect();
    let n = points.len();
    // Drop the axis the polygon faces most along, and count crossings.
    let axis = normal.abs().max_position();
    let flat = |v: Vec3| match axis {
        0 => (v.y, v.z),
        1 => (v.z, v.x),
        _ => (v.x, v.y),
    };
    let (px, py) = flat(p);
    let mut inside = false;
    for i in 0..n {
        let (a, b) = (points[i], points[(i + 1) % n]);
        // Too close to the border: not inside.
        let ab = b - a;
        let t = if ab.length_squared() > 0.0 {
            ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
        } else {
            0.0
        };
        if p.distance(a + ab * t) < near {
            return false;
        }
        let ((ax, ay), (bx, by)) = (flat(a), flat(b));
        if (ay > py) != (by > py) && px < ax + (py - ay) / (by - ay) * (bx - ax) {
            inside = !inside;
        }
    }
    inside
}

/// Whether the ray from `from` going `way` hits a polygon other than
/// `skip` closer than `limit`.
fn blocked(mesh: &Mesh, skip: FaceId, from: Vec3, way: Vec3, limit: f32) -> bool {
    for (face, corners) in mesh.faces() {
        if face == skip || corners.len() < 3 {
            continue;
        }
        let Some(first) = mesh.position(corners[0]) else {
            continue;
        };
        for pair in corners[1..].windows(2) {
            let (Some(b), Some(c)) = (mesh.position(pair[0]), mesh.position(pair[1])) else {
                continue;
            };
            if let Some(t) = hit_triangle(from, way, first, b, c)
                && t > limit * 1e-4
                && t < limit * (1.0 - 1e-4)
            {
                return true;
            }
        }
    }
    false
}

/// Where the ray hits the triangle `a b c` (Möller–Trumbore), as a
/// distance along `way`.
fn hit_triangle(from: Vec3, way: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
    let (e1, e2) = (b - a, c - a);
    let p = way.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1.0 / det;
    let s = from - a;
    let u = s.dot(p) * inv;
    if !(-1e-6..=1.0 + 1e-6).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = way.dot(q) * inv;
    if v < -1e-6 || u + v > 1.0 + 1e-6 {
        return None;
    }
    Some(e2.dot(q) * inv)
}

/// How sure "on the line" has to be, as a share of a stretch.
const ALONG_SLACK: f32 = 1e-4;

/// The chains of cuts the line makes, polygon by polygon, in id order,
/// and the closed loops drawn wholly inside one polygon.
fn chains(mesh: &Mesh, line: &SlitLine) -> (Vec<Chain>, Vec<InnerLoop>) {
    let none = (Vec::new(), Vec::new());
    let points = &line.points;
    if points.len() < 2 || points.iter().any(|p| !p.is_finite()) {
        return none;
    }
    let closed = line.is_closed();
    // Its own points: a closed line's last is its first again.
    let own = if closed {
        points.len() - 1
    } else {
        points.len()
    };
    let valid_eye = match line.eye {
        Eye::At(e) => e.is_finite(),
        Eye::Along(d) => d.is_finite() && d.length_squared() > 1e-12,
    };
    if !valid_eye {
        return none;
    }
    let Some(bounds) = mesh.bounds() else {
        return none;
    };
    let size = bounds.radius().max(1e-3);
    let reach = points
        .iter()
        .chain(match &line.eye {
            Eye::At(e) => std::slice::from_ref(e),
            Eye::Along(_) => &[],
        })
        .map(|p| p.distance(bounds.centre()))
        .fold(size, f32::max);
    let sweep = Sweep {
        line,
        far: reach * 4.0 + 1.0,
        near: size * 1e-5,
    };
    let planes: Vec<Option<(Vec3, Vec3)>> = (0..points.len() - 1).map(|s| sweep.plane(s)).collect();
    let mut out = Vec::new();
    let mut loops = Vec::new();
    for (face, corners) in mesh.faces() {
        let (normal, middle) = facing(mesh, face);
        if !line.through {
            let towards = match line.eye {
                Eye::At(e) => e - middle,
                Eye::Along(d) => -d,
            };
            if normal.dot(towards) <= 0.0 {
                continue;
            }
        }
        // Everything the line meets on this polygon, by where along the
        // line it is (stretch number plus share).
        let mut meets: Vec<(f32, Meets)> = Vec::new();
        let n = corners.len();
        for (s, plane) in planes.iter().enumerate() {
            let Some((on, across)) = *plane else {
                continue;
            };
            let side = |v: VertexId| mesh.position(v).map(|p| (p - on).dot(across));
            for i in 0..n {
                let (a, b) = (corners[i], corners[(i + 1) % n]);
                let (Some(da), Some(db)) = (side(a), side(b)) else {
                    continue;
                };
                let (Some(pa), Some(pb)) = (mesh.position(a), mesh.position(b)) else {
                    continue;
                };
                if da.abs() < sweep.near {
                    if let Some(u) = sweep.along(s, pa)
                        && (-ALONG_SLACK..=1.0 + ALONG_SLACK).contains(&u)
                    {
                        meets.push((s as f32 + u.clamp(0.0, 1.0), Meets::Point(a)));
                    }
                    continue;
                }
                if db.abs() < sweep.near || (da > 0.0) == (db > 0.0) {
                    continue;
                }
                let t = da / (da - db);
                let x = pa.lerp(pb, t);
                let Some(u) = sweep.along(s, x) else {
                    continue;
                };
                if !(-ALONG_SLACK..=1.0 + ALONG_SLACK).contains(&u) {
                    continue;
                }
                let key = EdgeKey::new(a, b);
                let t_low = if key.ends().0 == a { t } else { 1.0 - t };
                meets.push((s as f32 + u.clamp(0.0, 1.0), Meets::Edge(key, t_low, s)));
            }
        }
        // Line points inside the polygon.
        for (k, p) in points.iter().enumerate().take(own) {
            let Some((from, way)) = sweep.ray(*p) else {
                continue;
            };
            let denom = way.dot(normal);
            if denom.abs() < 1e-9 {
                continue;
            }
            let t = (middle - from).dot(normal) / denom;
            if t <= 0.0 {
                continue;
            }
            let spot = from + way * t;
            if strictly_inside(mesh, face, normal, spot, sweep.near * 10.0) {
                meets.push((k as f32, Meets::Inside(spot)));
            }
        }
        if meets.is_empty() {
            continue;
        }
        meets.sort_by(|a, b| a.0.total_cmp(&b.0));
        // The same point met twice in a row (where two stretches join, or
        // by both edges at a corner) counts once.
        meets.dedup_by(|b, a| {
            (b.0 - a.0).abs() < 1e-3
                && match (a.1, b.1) {
                    (Meets::Point(x), Meets::Point(y)) => x == y,
                    (Meets::Edge(x, tx, _), Meets::Edge(y, ty, _)) => {
                        x == y && (tx - ty).abs() < 1e-3
                    }
                    _ => false,
                }
        });
        if closed {
            let first_border = meets
                .iter()
                .position(|(_, m)| !matches!(m, Meets::Inside(_)));
            match first_border {
                // Never crossing the border: wholly inside, or not here.
                None => {
                    if meets.len() == own {
                        let spots: Vec<Vec3> = meets
                            .iter()
                            .filter_map(|(_, m)| match m {
                                Meets::Inside(p) => Some(*p),
                                _ => None,
                            })
                            .collect();
                        let inner = InnerLoop { face, spots };
                        if seen_loop(mesh, &sweep, &inner) {
                            loops.push(inner);
                        }
                    }
                    continue;
                }
                // Starting inside this polygon: the loop leaves at the first
                // border meeting and comes back in later, so the cut runs
                // from the last entry round through the start to that exit.
                Some(b) if b > 0 => meets.rotate_left(b + 1),
                Some(_) => {}
            }
        }
        // In at one border meeting, out at the next.
        let mut start: Option<Meets> = None;
        let mut inside = Vec::new();
        for (_, m) in meets {
            match (m, start) {
                (Meets::Inside(p), Some(_)) => inside.push(p),
                (Meets::Inside(_), None) => {}
                (border, None) => start = Some(border),
                (border, Some(first)) => {
                    let chain = Chain {
                        face,
                        start: first,
                        inside: std::mem::take(&mut inside),
                        end: border,
                    };
                    if worth_cutting(mesh, &chain) && seen(mesh, &sweep, &chain) {
                        out.push(chain);
                    }
                    start = None;
                }
            }
        }
    }
    (out, loops)
}

/// Whether a loop inside a polygon can be seen from the eye (always,
/// cutting through).
fn seen_loop(mesh: &Mesh, sweep: &Sweep, inner: &InnerLoop) -> bool {
    if sweep.line.through {
        return true;
    }
    let Some(spot) = inner.spots.first().copied() else {
        return false;
    };
    let (way, limit) = match sweep.line.eye {
        Eye::At(e) => {
            let to = e - spot;
            (to.normalize_or_zero(), to.length())
        }
        Eye::Along(d) => (-d.normalize_or_zero(), sweep.far),
    };
    !blocked(mesh, inner.face, spot, way, limit)
}

/// Whether a chain cuts anything new: not along one edge, not between
/// neighbouring corners without turning inside.
fn worth_cutting(mesh: &Mesh, chain: &Chain) -> bool {
    if !chain.inside.is_empty() {
        return chain.start != chain.end;
    }
    let corners = mesh.face(chain.face).unwrap_or(&[]);
    let n = corners.len();
    let neighbours = |a: VertexId, b: VertexId| {
        (0..n).any(|i| {
            let (x, y) = (corners[i], corners[(i + 1) % n]);
            (x, y) == (a, b) || (y, x) == (a, b)
        })
    };
    match (chain.start, chain.end) {
        (Meets::Point(a), Meets::Point(b)) => a != b && !neighbours(a, b),
        (Meets::Edge(e, ..), Meets::Edge(f, ..)) => e != f,
        (Meets::Point(a), Meets::Edge(e, ..)) | (Meets::Edge(e, ..), Meets::Point(a)) => {
            let (x, y) = e.ends();
            a != x && a != y
        }
        _ => false,
    }
}

/// Where a meeting is.
fn place(mesh: &Mesh, m: Meets) -> Option<Vec3> {
    match m {
        Meets::Point(v) => mesh.position(v),
        Meets::Edge(e, t, _) => {
            let (a, b) = e.ends();
            Some(mesh.position(a)?.lerp(mesh.position(b)?, t))
        }
        Meets::Inside(p) => Some(p),
    }
}

/// Whether a chain can be seen from the eye (always, cutting through).
fn seen(mesh: &Mesh, sweep: &Sweep, chain: &Chain) -> bool {
    if sweep.line.through {
        return true;
    }
    let (Some(a), Some(b)) = (
        place(mesh, chain.start),
        chain
            .inside
            .first()
            .copied()
            .or_else(|| place(mesh, chain.end)),
    ) else {
        return false;
    };
    let middle = a.lerp(b, 0.5);
    let (way, limit) = match sweep.line.eye {
        Eye::At(e) => {
            let to = e - middle;
            (to.normalize_or_zero(), to.length())
        }
        Eye::Along(d) => (-d.normalize_or_zero(), sweep.far),
    };
    !blocked(mesh, chain.face, middle, way, limit)
}

/// How many polygons a slit would add, without changing anything (for the
/// size guard).
pub fn slit_size(mesh: &Mesh, line: &SlitLine) -> usize {
    if line.points.len() > MAX_LINE_POINTS {
        return 0;
    }
    let (chains, loops) = chains(mesh, line);
    chains.len() + 3 * loops.len()
}

/// Where a new point came from.
#[derive(Debug, Clone, Copy)]
enum Made {
    /// On an original edge, `t` of the way from its lower point id.
    Edge(EdgeKey, f32),
    /// Inside an original polygon.
    Inside(FaceId, Vec3),
}

/// Cuts `mesh` along `line`. Lines of more than [`MAX_LINE_POINTS`]
/// points, or with numbers that aren't real, are skipped; so is a line
/// that cuts nothing. Polygons the cut would leave broken (repeated
/// corners) refuse the whole cut, changing nothing.
pub fn slit(mesh: &mut Mesh, line: &SlitLine) -> Outcome {
    let skipped = Outcome {
        skipped: 1,
        ..Outcome::default()
    };
    if line.points.len() > MAX_LINE_POINTS {
        return skipped;
    }
    let original = mesh.clone();
    let topology = original.topology();
    let (chains, loops) = chains(&original, line);
    if chains.is_empty() && loops.is_empty() {
        return skipped;
    }
    let mut build = Builder::new(&original, mesh);

    // 1. New points: on edges (shared by the polygons on both sides, one
    //    per edge and stretch), and inside polygons.
    let mut on_edges: BTreeMap<(EdgeKey, usize), VertexId> = BTreeMap::new();
    let mut made: BTreeMap<VertexId, Made> = BTreeMap::new();
    let mut ends: Vec<(VertexId, Vec<VertexId>, VertexId)> = Vec::new();
    let mut point_at = |build: &mut Builder, m: Meets, face: FaceId| -> Option<VertexId> {
        match m {
            Meets::Point(v) => Some(v),
            Meets::Edge(e, t, s) => {
                if let Some(v) = on_edges.get(&(e, s)) {
                    return Some(*v);
                }
                let (a, b) = e.ends();
                let p = original.position(a)?.lerp(original.position(b)?, t);
                let v = build.point(p, PointFrom::Between(a, b, t));
                on_edges.insert((e, s), v);
                made.insert(v, Made::Edge(e, t));
                Some(v)
            }
            Meets::Inside(p) => {
                let v = build.point(p, PointFrom::Inside(face, p));
                made.insert(v, Made::Inside(face, p));
                Some(v)
            }
        }
    };
    for chain in &chains {
        let start = point_at(&mut build, chain.start, chain.face);
        let inside: Option<Vec<VertexId>> = chain
            .inside
            .iter()
            .map(|p| point_at(&mut build, Meets::Inside(*p), chain.face))
            .collect();
        let end = point_at(&mut build, chain.end, chain.face);
        if let (Some(s), Some(i), Some(e)) = (start, inside, end) {
            ends.push((s, i, e));
        } else {
            ends.push((
                VertexId::from_raw(u32::MAX),
                Vec::new(),
                VertexId::from_raw(u32::MAX),
            ));
        }
    }
    // The new points along each cut edge, by how far along they are.
    let mut along: BTreeMap<EdgeKey, Vec<(f32, VertexId)>> = BTreeMap::new();
    for ((edge, _), v) in &on_edges {
        if let Some(Made::Edge(_, t)) = made.get(v) {
            along.entry(*edge).or_default().push((*t, *v));
        }
    }
    for list in along.values_mut() {
        list.sort_by(|a, b| a.0.total_cmp(&b.0));
    }

    // 2. Every polygon beside a cut edge takes its new points as corners.
    let touched: BTreeSet<FaceId> = along
        .keys()
        .flat_map(|e| topology.edge_faces(*e))
        .chain(chains.iter().map(|c| c.face))
        .collect();
    let mut outlines: BTreeMap<FaceId, Vec<VertexId>> = BTreeMap::new();
    for face in &touched {
        let Some(corners) = original.face(*face) else {
            continue;
        };
        let n = corners.len();
        let mut out = Vec::new();
        for i in 0..n {
            let (a, b) = (corners[i], corners[(i + 1) % n]);
            out.push(a);
            if let Some(list) = along.get(&EdgeKey::new(a, b)) {
                if EdgeKey::new(a, b).ends().0 == a {
                    out.extend(list.iter().map(|(_, v)| *v));
                } else {
                    out.extend(list.iter().rev().map(|(_, v)| *v));
                }
            }
        }
        outlines.insert(*face, out);
    }

    // 3. Each chain splits the piece of its polygon it runs across.
    let mut pieces: BTreeMap<FaceId, Vec<Vec<VertexId>>> = outlines
        .iter()
        .map(|(f, outline)| (*f, vec![outline.clone()]))
        .collect();
    for (chain, (start, inside, end)) in chains.iter().zip(&ends) {
        let Some(list) = pieces.get_mut(&chain.face) else {
            continue;
        };
        let Some(k) = list
            .iter()
            .position(|p| p.contains(start) && p.contains(end))
        else {
            continue;
        };
        let piece = &list[k];
        let n = piece.len();
        let (Some(i), Some(j)) = (
            piece.iter().position(|v| v == start),
            piece.iter().position(|v| v == end),
        ) else {
            continue;
        };
        if i == j || (inside.is_empty() && ((i + 1) % n == j || (j + 1) % n == i)) {
            continue;
        }
        // From the start round to the end, then back inside; and from the
        // end round to the start, then on inside.
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
        let mut one = walk(i, j);
        one.extend(inside.iter().rev());
        let mut two = walk(j, i);
        two.extend(inside.iter());
        list[k] = one;
        list.push(two);
    }

    // 3b. A loop inside a polygon: its shape becomes a polygon of its own,
    //     and the ring round it is cut in two from the polygon's corners.
    let mut looped = 0;
    for inner in &loops {
        let outline = match pieces.get(&inner.face) {
            Some(list) if list.len() == 1 => list[0].clone(),
            Some(_) => continue,
            None => match original.face(inner.face) {
                Some(c) => c.to_vec(),
                None => continue,
            },
        };
        let Some((one, two, shape)) = ring_cut(&original, inner, &outline, |p| {
            let v = build.point(p, PointFrom::Inside(inner.face, p));
            made.insert(v, Made::Inside(inner.face, p));
            v
        }) else {
            continue;
        };
        pieces.insert(inner.face, vec![one, two, shape]);
        looped += 1;
    }
    if chains.is_empty() && looped == 0 {
        *build.mesh() = original.clone();
        return skipped;
    }

    // Each corner's values: from its polygon's corners.
    let corner = |face: FaceId, v: VertexId| -> CornerFrom {
        match made.get(&v) {
            None => CornerFrom::Copy(face, v),
            Some(Made::Edge(e, t)) => {
                let (a, b) = e.ends();
                CornerFrom::Between(face, a, face, b, *t)
            }
            Some(Made::Inside(f, p)) => CornerFrom::Inside(*f, *p),
        }
    };
    let mut rewrites: BTreeMap<FaceId, Vec<(VertexId, CornerFrom)>> = BTreeMap::new();
    let mut new_faces = Vec::new();
    for (face, list) in &pieces {
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
    if build.rewrite_faces(&rewrites).is_err() {
        *build.mesh() = original.clone();
        return skipped;
    }
    for (corners, like) in &new_faces {
        build.face(corners, Some(*like));
    }

    // 4. A cut edge's values (hard, seam) go to each of its pieces.
    for (edge, list) in &along {
        let (a, b) = edge.ends();
        let mut chain = vec![a];
        chain.extend(list.iter().map(|(_, v)| *v));
        chain.push(b);
        for pair in chain.windows(2) {
            build.copy_edge(*edge, pair[0], pair[1]);
        }
        build.clear_edge(*edge);
    }
    Outcome {
        matched: chains.len() + looped,
        missing: 0,
        skipped: 0,
    }
}

/// The three polygons a loop inside a polygon makes: the two halves of the
/// ring between the polygon's `outline` and the loop, and the loop's own
/// shape, each running the polygon's way round. `point` makes the loop's
/// points. The ring is cut by two lines from outline corners to loop
/// points on opposite sides, each from a corner to the loop point facing
/// it, so neither half has a hole. `None` for a loop of fewer than three
/// points, or a polygon with no facing.
fn ring_cut(
    mesh: &Mesh,
    inner: &InnerLoop,
    outline: &[VertexId],
    mut point: impl FnMut(Vec3) -> VertexId,
) -> Option<(Vec<VertexId>, Vec<VertexId>, Vec<VertexId>)> {
    let (n, m) = (outline.len(), inner.spots.len());
    if n < 3 || m < 3 {
        return None;
    }
    let normal = mesh.face_normal(inner.face).try_normalize()?;
    let outer: Vec<Vec3> = outline
        .iter()
        .map(|v| mesh.position(*v))
        .collect::<Option<_>>()?;
    // The loop runs the polygon's way round.
    let mut spots = inner.spots.clone();
    let middle = spots.iter().copied().sum::<Vec3>() / m as f32;
    let turn: f32 = spots
        .iter()
        .zip(spots.iter().cycle().skip(1))
        .map(|(a, b)| (*a - middle).cross(*b - middle).dot(normal))
        .sum();
    if turn.abs() < 1e-9 {
        return None;
    }
    if turn < 0.0 {
        spots.reverse();
    }
    // Angles round the loop's middle, on the polygon's plane.
    let x = normal.any_orthonormal_vector();
    let y = normal.cross(x);
    let angle = |p: Vec3| {
        let d = p - middle;
        d.dot(y).atan2(d.dot(x))
    };
    let apart = |a: f32, b: f32| {
        let d = (a - b).rem_euclid(std::f32::consts::TAU);
        d.min(std::f32::consts::TAU - d)
    };
    let facing = |at: f32, skip: Option<usize>| {
        (0..m)
            .filter(|q| Some(*q) != skip)
            .min_by(|a, b| apart(angle(spots[*a]), at).total_cmp(&apart(angle(spots[*b]), at)))
    };
    // Each half as positions on the polygon's plane, for checking it.
    let flat = |p: Vec3| glam::Vec2::new(p.dot(x), p.dot(y));
    let half_at = |from_o: usize, to_o: usize, from_q: usize, to_q: usize| {
        let mut out = Vec::new();
        let mut o = from_o;
        loop {
            out.push(flat(outer[o]));
            if o == to_o {
                break;
            }
            o = (o + 1) % n;
        }
        let mut q = from_q;
        loop {
            out.push(flat(spots[q]));
            if q == to_q {
                break;
            }
            q = (q + m - 1) % m;
        }
        out
    };
    let fits = |(i_o, i_q, j_o, j_q): (usize, usize, usize, usize)| {
        i_o != j_o
            && i_q != j_q
            && simple(&half_at(i_o, j_o, j_q, i_q))
            && simple(&half_at(j_o, i_o, i_q, j_q))
    };
    // The first line from the outline corner nearest the loop, to the loop
    // point facing it; the second from the corner most nearly opposite.
    let nearest_corner = (0..n).min_by(|a, b| {
        let near = |o: usize| {
            spots
                .iter()
                .map(|s| s.distance_squared(outer[o]))
                .fold(f32::INFINITY, f32::min)
        };
        near(*a).total_cmp(&near(*b))
    })?;
    let choice = |i_o: usize| -> Option<(usize, usize, usize, usize)> {
        let i_q = facing(angle(outer[i_o]), None)?;
        let opposite = angle(outer[i_o]) + std::f32::consts::PI;
        let j_o = (0..n).filter(|o| *o != i_o).min_by(|a, b| {
            apart(angle(outer[*a]), opposite).total_cmp(&apart(angle(outer[*b]), opposite))
        })?;
        let j_q = facing(angle(outer[j_o]), Some(i_q))?;
        Some((i_o, i_q, j_o, j_q))
    };
    // If those lines would cross the loop (a star-shaped loop, say), try
    // other corners and points, nearest first, a limited number of times.
    let tried = choice(nearest_corner).filter(|c| fits(*c));
    let (i_o, i_q, j_o, j_q) = match tried {
        Some(c) => c,
        None => {
            let by_distance = |at: Vec3| {
                let mut order: Vec<usize> = (0..m).collect();
                order.sort_by(|a, b| {
                    spots[*a]
                        .distance_squared(at)
                        .total_cmp(&spots[*b].distance_squared(at))
                });
                order
            };
            let mut found = None;
            let mut budget = 400;
            'search: for a in 0..n {
                for &qa in by_distance(outer[a]).iter().take(8) {
                    for b in (0..n).filter(|b| *b != a) {
                        for &qb in by_distance(outer[b]).iter().take(8) {
                            budget -= 1;
                            if budget == 0 {
                                break 'search;
                            }
                            if fits((a, qa, b, qb)) {
                                found = Some((a, qa, b, qb));
                                break 'search;
                            }
                        }
                    }
                }
            }
            found?
        }
    };
    let ids: Vec<VertexId> = spots.iter().map(|p| point(*p)).collect();
    // Outline forward from one corner to the other, then the loop back.
    let half = |from_o: usize, to_o: usize, from_q: usize, to_q: usize| {
        let mut out = Vec::new();
        let mut o = from_o;
        loop {
            out.push(outline[o]);
            if o == to_o {
                break;
            }
            o = (o + 1) % n;
        }
        let mut q = from_q;
        loop {
            out.push(ids[q]);
            if q == to_q {
                break;
            }
            q = (q + m - 1) % m;
        }
        out
    };
    let one = half(i_o, j_o, j_q, i_q);
    let two = half(j_o, i_o, i_q, j_q);
    Some((one, two, ids))
}

/// Whether a flat polygon doesn't cross itself: no two of its sides that
/// aren't neighbours meet.
fn simple(points: &[glam::Vec2]) -> bool {
    let n = points.len();
    if n < 3 {
        return false;
    }
    let turn = |a: glam::Vec2, b: glam::Vec2, c: glam::Vec2| (b - a).perp_dot(c - a);
    let crosses = |a: glam::Vec2, b: glam::Vec2, c: glam::Vec2, d: glam::Vec2| {
        let (d1, d2, d3, d4) = (turn(a, b, c), turn(a, b, d), turn(c, d, a), turn(c, d, b));
        (d1 > 0.0) != (d2 > 0.0) && (d3 > 0.0) != (d4 > 0.0) && d1 * d2 <= 0.0 && d3 * d4 <= 0.0
    };
    for i in 0..n {
        for j in i + 1..n {
            let neighbours = j == i + 1 || (i == 0 && j == n - 1);
            if neighbours {
                continue;
            }
            let (a, b) = (points[i], points[(i + 1) % n]);
            let (c, d) = (points[j], points[(j + 1) % n]);
            if crosses(a, b, c, d) {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests;
