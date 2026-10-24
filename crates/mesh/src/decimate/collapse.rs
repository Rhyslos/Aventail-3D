//! Collapsing edges, cheapest first: each point's cheapest allowed
//! collapse is kept in a heap, and worked out again when its
//! neighbourhood changes.

use super::setup::TANGLED;
use super::{Best, Class, Decimator, Entry, Marks, key};
use glam::DVec3;

/// How far (as the cosine of the angle) a triangle's facing may turn in
/// one collapse: about 75°. More, and the surface is folding over.
const MOST_TURN: f64 = 0.25;

/// How sharply (as the cosine of the angle) a kept line may turn at a
/// point for the point to slide along it: 30°. Where it turns more (a
/// square border's corner) the point stays, so the line keeps its shape.
const LINE_CORNER: f64 = 0.866_025_4;

/// How much a UV straying from the texture's slope counts against a
/// point straying from the surface (both in a shape scaled to size 1).
const UV_WEIGHT: f64 = 1.0;

/// A point's neighbours, each with how many live triangles hold both.
type Ring = Vec<(u32, u32)>;

impl Decimator {
    /// Collapses edges, cheapest first, until `target` triangles or fewer
    /// are left, or nothing more can go (every point left is kept, or
    /// would fold the surface). Gives how many triangles are left. A
    /// target above that changes nothing.
    pub fn reduce_to(&mut self, target: usize) -> usize {
        while self.live > target {
            let Some(entry) = self.heap.pop() else {
                break;
            };
            let u = entry.point;
            if !self.point_alive[u as usize] || self.version[u as usize] != entry.version {
                continue;
            }
            let Some(best) = self.best[u as usize] else {
                continue;
            };
            // Candidates are chosen by cost alone (checking every one
            // would be slow), and something two steps away may have
            // changed since: check now, and if it can't be done, look again
            // checking each.
            let ring = self.ring(u);
            if self.allowed(u, best.target, &ring) {
                self.collapse(u, best.target, &ring);
            } else {
                self.consider(u, true);
            }
        }
        self.live
    }

    /// The live triangles of point `p`.
    fn live_tris(&self, p: u32) -> impl Iterator<Item = u32> + '_ {
        self.tris_of[p as usize]
            .iter()
            .copied()
            .filter(|t| self.tri_alive[*t as usize])
    }

    /// The points of triangle `t`.
    fn points_of(&self, t: u32) -> [u32; 3] {
        self.tris[t as usize].map(|w| self.wedge_point[w as usize])
    }

    /// `p`'s neighbours, in order, with their shared triangle counts.
    fn ring(&self, p: u32) -> Ring {
        let mut others: Vec<u32> = Vec::with_capacity(16);
        for t in self.live_tris(p) {
            others.extend(self.points_of(t).into_iter().filter(|x| *x != p));
        }
        others.sort_unstable();
        let mut ring: Ring = Vec::with_capacity(others.len() / 2 + 1);
        for x in others {
            match ring.last_mut() {
                Some((last, count)) if *last == x => *count += 1,
                _ => ring.push((x, 1)),
            }
        }
        ring
    }

    /// How point `p` may move, from the kept lines and borders round it.
    pub(super) fn classify(&self, p: u32) -> Class {
        let ring = self.ring(p);
        let mut lines = 0;
        let mut first_kind = None;
        let mut mixed = false;
        let mut borders = 0;
        let mut ends: Vec<u32> = Vec::with_capacity(2);
        for (x, count) in &ring {
            if *count == 1 {
                borders += 1;
            }
            if *count > 2 {
                return Class::Locked;
            }
            if let Some(kind) = self.features.get(&key(p, *x)) {
                if kind & TANGLED != 0 {
                    return Class::Locked;
                }
                lines += 1;
                ends.push(*x);
                mixed |= first_kind.is_some_and(|k| k != *kind);
                first_kind.get_or_insert(*kind);
            }
        }
        // A border point not on exactly one border (two border edges) is
        // where surfaces touch at a point.
        if borders != 0 && borders != 2 {
            return Class::Locked;
        }
        match (lines, ends.as_slice()) {
            (0, _) => Class::Free,
            (2, [a, b]) if !mixed => {
                let here = self.pos[p as usize];
                let into = (here - self.pos[*a as usize]).normalize_or_zero();
                let out = (self.pos[*b as usize] - here).normalize_or_zero();
                if into.dot(out) >= LINE_CORNER {
                    Class::Line
                } else {
                    Class::Locked
                }
            }
            _ => Class::Locked,
        }
    }

    /// Whether collapsing `u` onto its neighbour `v` keeps the surface
    /// whole and unfolded (`ring` is `u`'s).
    fn allowed(&self, u: u32, v: u32, ring: &Ring) -> bool {
        if !self.point_alive[u as usize] || !self.point_alive[v as usize] {
            return false;
        }
        let Some(shared) = ring.iter().find(|(x, _)| *x == v).map(|(_, c)| *c) else {
            return false;
        };
        match self.class[u as usize] {
            Class::Locked => return false,
            Class::Line if !self.features.contains_key(&key(u, v)) => return false,
            _ => {}
        }
        if shared > 2 {
            return false;
        }
        let v_ring = self.ring(v);
        // Two border points joined across the surface would pinch it.
        let border = |r: &Ring| r.iter().any(|(_, c)| *c == 1);
        if shared != 1 && border(ring) && border(&v_ring) {
            return false;
        }
        // The link condition: the only neighbours they share are the far
        // corners of the triangles on the edge.
        let common = ring
            .iter()
            .filter(|(x, _)| *x != v && v_ring.binary_search_by_key(x, |(y, _)| *y).is_ok())
            .count();
        common == shared as usize && self.unfolded(u, v)
    }

    /// Whether moving `u` onto `v` keeps every triangle of `u` facing the
    /// way it did, with some area, and makes no triangle twice.
    fn unfolded(&self, u: u32, v: u32) -> bool {
        let to = self.pos[v as usize];
        for t in self.live_tris(u) {
            let p = self.points_of(t);
            if p.contains(&v) {
                continue;
            }
            let at = |x: u32| if x == u { to } else { self.pos[x as usize] };
            let before = self.normal(p.map(|x| self.pos[x as usize]));
            let after = self.normal(p.map(at));
            let (lb, la) = (before.length(), after.length());
            if la <= 1e-14 {
                return false;
            }
            if lb > 1e-14 && before.dot(after) < MOST_TURN * lb * la {
                return false;
            }
            // The triangle it would become mustn't be one `v` has.
            let k = p.iter().position(|x| *x == u).unwrap_or(0);
            let others = [p[(k + 1) % 3], p[(k + 2) % 3]];
            let twice = self.live_tris(v).any(|s| {
                let q = self.points_of(s);
                others.iter().all(|x| q.contains(x))
            });
            if twice {
                return false;
            }
        }
        true
    }

    fn normal(&self, p: [DVec3; 3]) -> DVec3 {
        (p[1] - p[0]).cross(p[2] - p[0])
    }

    /// The corners (wedges) of `p`'s live triangles.
    fn corners_of(&self, p: u32) -> Vec<[u32; 3]> {
        self.live_tris(p).map(|t| self.tris[t as usize]).collect()
    }

    /// Which of `v`'s wedges each of `u`'s wedges becomes when `u` moves
    /// onto `v` (`corners` are `u`'s triangles): the one beside it across
    /// the edge, or the nearest in UV. Into `map`, using `loose`.
    fn wedge_map(
        &self,
        corners: &[[u32; 3]],
        (u, v): (u32, u32),
        map: &mut Vec<(u32, u32)>,
        loose: &mut Vec<u32>,
    ) {
        map.clear();
        loose.clear();
        for corners in corners {
            let find = |p: u32| {
                corners
                    .iter()
                    .copied()
                    .find(|w| self.wedge_point[*w as usize] == p)
            };
            let (Some(wu), wv) = (find(u), find(v)) else {
                continue;
            };
            match wv {
                Some(wv) if !map.iter().any(|(a, _)| *a == wu) => map.push((wu, wv)),
                None if !loose.contains(&wu) => loose.push(wu),
                _ => {}
            }
        }
        let across = map.len();
        for wu in loose.iter().copied() {
            if map.iter().any(|(a, _)| *a == wu) {
                continue;
            }
            let uv = self.wedge_uv[wu as usize];
            let nearest = map[..across].iter().map(|(_, wv)| *wv).min_by(|a, b| {
                let da = self.wedge_uv[*a as usize].distance_squared(uv);
                let db = self.wedge_uv[*b as usize].distance_squared(uv);
                da.total_cmp(&db).then(a.cmp(b))
            });
            if let Some(wv) = nearest {
                map.push((wu, wv));
            }
        }
    }

    /// What collapsing `u` onto `v` costs (`corners` are `u`'s triangles,
    /// `map` and `loose` room to work in).
    fn cost(
        &self,
        (u, v): (u32, u32),
        corners: &[[u32; 3]],
        map: &mut Vec<(u32, u32)>,
        loose: &mut Vec<u32>,
    ) -> f64 {
        let to = self.pos[v as usize];
        let mut cost = self.quadrics[u as usize].error(to);
        if self.has_uvs {
            self.wedge_map(corners, (u, v), map, loose);
            for (wu, wv) in map.iter() {
                let uv = self.wedge_uv[*wv as usize].as_dvec2();
                cost += UV_WEIGHT * self.uv_quadrics[*wu as usize].error(to, [uv.x, uv.y]);
            }
        }
        cost + self.tie * self.pos[u as usize].distance_squared(to)
    }

    /// Works out `u`'s cheapest collapse again and puts it on the heap (or
    /// nothing, when it can't move). With `checked`, only collapses that
    /// keep the surface whole count; without, those the point's kind
    /// allows (quicker: most are fine, and each is checked before it is
    /// done).
    pub(super) fn consider(&mut self, u: u32, checked: bool) {
        let i = u as usize;
        self.version[i] = self.version[i].wrapping_add(1);
        self.best[i] = None;
        if !self.point_alive[i] || self.class[i] == Class::Locked {
            return;
        }
        let ring = self.ring(u);
        let corners = if self.has_uvs {
            self.corners_of(u)
        } else {
            Vec::new()
        };
        let (mut map, mut loose) = (Vec::new(), Vec::new());
        let mut best: Option<Best> = None;
        for (v, shared) in &ring {
            let possible = if checked {
                self.allowed(u, *v, &ring)
            } else {
                *shared <= 2
                    && (self.class[i] == Class::Free || self.features.contains_key(&key(u, *v)))
            };
            if !possible {
                continue;
            }
            let cost = self.cost((u, *v), &corners, &mut map, &mut loose);
            if !cost.is_finite() {
                continue;
            }
            if best.is_none_or(|b| cost < b.cost) {
                best = Some(Best { target: *v, cost });
            }
        }
        self.best[i] = best;
        if let Some(b) = best {
            self.heap.push(Entry {
                cost: b.cost,
                point: u,
                version: self.version[i],
            });
        }
    }

    /// Moves `u` onto `v` (`ring` is `u`'s): the triangles on the edge go,
    /// `u`'s others take `v` (and its wedges) instead, `v` takes on `u`'s
    /// quadrics, and lines and marks along the edge carry on.
    fn collapse(&mut self, u: u32, v: u32, ring: &Ring) {
        let (mut map, mut loose) = (Vec::new(), Vec::new());
        self.wedge_map(&self.corners_of(u), (u, v), &mut map, &mut loose);
        let lines_change = self.class[u as usize] != Class::Free
            || ring.iter().any(|(_, c)| *c == 1)
            || self.class[v as usize] != Class::Free;
        let tris: Vec<u32> = self.live_tris(u).collect();
        for t in tris {
            let ti = t as usize;
            if self.points_of(t).contains(&v) {
                self.tri_alive[ti] = false;
                self.live -= 1;
                continue;
            }
            for w in &mut self.tris[ti] {
                if self.wedge_point[*w as usize] == u
                    && let Some((_, wv)) = map.iter().find(|(wu, _)| *wu == *w)
                {
                    *w = *wv;
                }
            }
            self.tris_of[v as usize].push(t);
        }
        let q = self.quadrics[u as usize];
        self.quadrics[v as usize].add(&q);
        for (wu, wv) in &map {
            let q = self.uv_quadrics[*wu as usize];
            self.uv_quadrics[*wv as usize].add(&q);
        }
        // A kept line shortens: its next edge from `u` now runs from `v`.
        let along = self.features.remove(&key(u, v)).unwrap_or(0);
        let along_marks = self.marks.remove(&key(u, v)).unwrap_or_default();
        for (x, _) in ring {
            if *x == v {
                continue;
            }
            if let Some(kind) = self.features.remove(&key(u, *x))
                && kind & along != 0
            {
                *self.features.entry(key(v, *x)).or_default() |= kind & along;
            }
            if let Some(marks) = self.marks.remove(&key(u, *x)) {
                let carried: Marks = marks.carried_by(along_marks);
                if !carried.is_empty() {
                    self.marks.entry(key(v, *x)).or_default().merge(carried);
                }
            }
        }
        self.point_alive[u as usize] = false;
        self.tris_of[u as usize] = Vec::new();
        let alive = &self.tri_alive;
        self.tris_of[v as usize].retain(|t| alive[*t as usize]);
        self.version[u as usize] = self.version[u as usize].wrapping_add(1);
        let around = self.ring(v);
        // Where a line or border was involved, the points round it may
        // have become corners (or a border may have closed).
        if lines_change {
            self.class[v as usize] = self.classify(v);
            for (x, _) in &around {
                self.class[*x as usize] = self.classify(*x);
            }
        }
        self.consider(v, false);
        for (x, _) in around {
            self.consider(x, false);
        }
    }
}
