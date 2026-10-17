//! Keeping UVs whole through modelling
//! ([decision 0104](../../../docs/decisions/0104-uvs-that-survive-modelling.md)).
//!
//! Modelling tools carry UVs over to what they make ([decision
//! 0043](../../../docs/decisions/0043-attribute-layers-and-the-builder.md)):
//! a loop cut's new polygons blend the UVs they cut through, and come out
//! right. But some new polygons start with no room on the texture (an
//! extrude's walls, an inset's ring, a filled hole, a bevelled corner), and
//! some come out stretched across two pieces. After an edit that changes
//! the shape's polygons, [`mend`]:
//!
//! 1. finds the polygons the edit made or changed whose UVs came out
//!    **broken**: squashed flat, mirrored against the polygons around them,
//!    stretched far more one way than the other, or far bigger or smaller
//!    than the polygons around them;
//! 2. marks a **seam** wherever two polygons the edit changed, neither
//!    broken, now meet with different UVs, so pieces stay apart (a shape
//!    without seam marks first gets them where its UVs split);
//! 3. **lays the broken polygons out again**, each joined group of them
//!    (a *cluster*) on its own. A cluster carries on from the kept
//!    polygons next to it: their UVs stay exactly as they are and the
//!    cluster is solved around them (least squares conformal maps with the
//!    kept corners held), so it continues the piece smoothly. If it
//!    touches kept polygons of several pieces that meet only through it
//!    (a bevelled corner between three pieces; walls between a cap and
//!    what's around it), it carries on from the one it shares the longest
//!    border with, and seams part it from the others;
//! 4. lays out as pieces of their own (as [`crate::unwrap()`] does: strips
//!    straight, at the shape's texel density, where they were) the
//!    clusters with nothing kept next to them (an extrude's ring of walls,
//!    cut off by seams) and any that still lie badly after solving.
//!
//! Nothing else changes, so pieces placed, turned or scaled by hand stay
//! as they were. Every step looks only near what the edit changed, so a
//! small edit on a big shape mends quickly.

use crate::chart::{Chart, Sets};
use crate::unwrap::{conformal_pinned, cuts_of, lay_out};
use glam::{Vec2, Vec3};
use mesh::uvs::Uvs;
use mesh::{EdgeKey, FaceId, Mesh, VertexId};
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

/// The most broken polygons mended after one edit; past this the UVs are
/// left as the edit made them (mending would hold the edit up), and
/// Unwrap fixes them.
pub const MEND_UP_TO: usize = 20_000;

/// From how many times more stretched one way than the other a polygon
/// the edit changed counts as broken.
const UNEVEN: f32 = 3.0;

/// From how many times bigger or smaller (in length) than the polygons
/// around it a polygon the edit changed counts as broken.
const SIZE_OFF: f32 = 3.0;

/// How many polygons around a cluster of changed ones are measured to tell
/// their usual room and facing.
const NEARBY: usize = 64;

/// How many polygons of the whole shape are measured for its usual room.
const SAMPLE: usize = 2_000;

/// How many kept polygons are searched to tell whether those next to a
/// cluster are joined to each other; past this, the ones not yet known to
/// be apart are taken as joined.
const SEARCH: usize = 50_000;

/// How one polygon lies on the texture.
#[derive(Debug, Clone, Copy)]
struct Look {
    /// Its area on the surface.
    weight: f32,
    /// How many times more stretched one way than the other.
    uneven: f32,
    /// Texture length per surface length.
    room: f32,
    /// Whether most of it lies mirrored.
    mirrored: bool,
}

/// What a mend works with: the shape, its UVs and where it is cut.
struct Shape<'a> {
    mesh: &'a Mesh,
    uvs: &'a Uvs,
    cuts: &'a BTreeSet<EdgeKey>,
}

impl Shape<'_> {
    fn look(&self, f: FaceId) -> Option<Look> {
        let list = self.mesh.face(f)?;
        let start = *self.mesh.face_layout().0.get(f.get() as usize)? as usize;
        let mut weight = 0.0;
        let mut uneven = 0.0;
        let mut room = 0.0;
        let mut mirrored = 0.0;
        let uvs = self.uvs;
        mesh::triangulate::face_triangles(self.mesh, list, |[a, b, c]| {
            let p = |i: usize| self.mesh.position(list[i]).unwrap_or(Vec3::ZERO);
            let corners = [p(a), p(b), p(c)];
            let area = (corners[1] - corners[0])
                .cross(corners[2] - corners[0])
                .length();
            if let Some((most, least, flipped)) = crate::stretch::stretches(
                corners,
                [uvs.at(start + a), uvs.at(start + b), uvs.at(start + c)],
            ) {
                weight += area;
                uneven += area * (most / least.max(most * 1e-6).max(1e-30)).log2();
                room += area * (most * least).sqrt();
                if flipped {
                    mirrored += area;
                }
            }
        });
        Some(if weight <= 0.0 {
            Look {
                weight: 0.0,
                uneven: 1.0,
                room: 0.0,
                mirrored: false,
            }
        } else {
            Look {
                weight,
                uneven: (uneven / weight).exp2(),
                room: room / weight,
                mirrored: mirrored > weight / 2.0,
            }
        })
    }

    /// The edges of polygon `f`.
    fn edges(&self, f: FaceId) -> impl Iterator<Item = EdgeKey> + '_ {
        let list = self.mesh.face(f).unwrap_or(&[]);
        (0..list.len()).map(move |k| EdgeKey::new(list[k], list[(k + 1) % list.len()]))
    }

    /// The polygons joined to `f` across edges that aren't cut, with the
    /// edge.
    fn joined(&self, f: FaceId) -> Vec<(EdgeKey, FaceId)> {
        let topology = self.mesh.topology();
        let mut out = Vec::new();
        for e in self.edges(f) {
            if self.cuts.contains(&e) {
                continue;
            }
            for u in topology.edge_uses(e) {
                if u.face != f {
                    out.push((e, u.face));
                }
            }
        }
        out
    }

    /// `faces` in parts joined across edges between two of them that
    /// aren't cut.
    fn parts(&self, faces: &BTreeSet<FaceId>) -> Vec<Vec<FaceId>> {
        let list: Vec<FaceId> = faces.iter().copied().collect();
        let slot: HashMap<FaceId, usize> = list.iter().enumerate().map(|(i, f)| (*f, i)).collect();
        let mut sets = Sets::new(list.len());
        for (i, f) in list.iter().enumerate() {
            for (_, g) in self.joined(*f) {
                if let Some(j) = slot.get(&g) {
                    sets.join(i, *j);
                }
            }
        }
        let mut parts: BTreeMap<usize, Vec<FaceId>> = BTreeMap::new();
        for (i, f) in list.iter().enumerate() {
            parts.entry(sets.find(i)).or_default().push(*f);
        }
        parts.into_values().collect()
    }

    /// `cluster` and up to [`NEARBY`] polygons around it (joined to it
    /// across edges not cut), nearest first.
    fn around(&self, cluster: &[FaceId]) -> Vec<FaceId> {
        let mut seen: BTreeSet<FaceId> = cluster.iter().copied().collect();
        let mut queue: VecDeque<FaceId> = cluster.iter().copied().collect();
        let mut out = cluster.to_vec();
        let limit = cluster.len() + NEARBY;
        while let Some(f) = queue.pop_front() {
            for (_, g) in self.joined(f) {
                if out.len() >= limit {
                    return out;
                }
                if seen.insert(g) {
                    out.push(g);
                    queue.push_back(g);
                }
            }
        }
        out
    }

    /// The usual room of the shape's polygons, from a sample of them.
    fn usual_room(&self) -> f32 {
        let every = (self.mesh.face_count() / SAMPLE).max(1);
        let mut rooms: Vec<f32> = self
            .mesh
            .faces()
            .step_by(every)
            .filter_map(|(f, _)| self.look(f))
            .filter(|l| l.weight > 0.0 && l.room > 0.0)
            .map(|l| l.room)
            .collect();
        rooms.sort_by(f32::total_cmp);
        rooms.get(rooms.len() / 2).copied().unwrap_or(0.0)
    }
}

/// The polygons of `changed` whose UVs are broken (see the module notes),
/// each judged against the polygons around it.
fn broken_among(shape: &Shape, changed: &BTreeSet<FaceId>) -> BTreeSet<FaceId> {
    // Below this a polygon has no room at all.
    let none = shape.usual_room() * 1e-3;
    let mut broken = BTreeSet::new();
    for cluster in shape.parts(changed) {
        let looks: Vec<(FaceId, Look)> = shape
            .around(&cluster)
            .into_iter()
            .filter_map(|f| Some((f, shape.look(f)?)))
            .collect();
        // The usual room around, and which way it mostly faces.
        let mut rooms: Vec<f32> = looks
            .iter()
            .filter(|(_, l)| l.weight > 0.0 && l.room > none)
            .map(|(_, l)| l.room)
            .collect();
        rooms.sort_by(f32::total_cmp);
        let usual = rooms.get(rooms.len() / 2).copied();
        let total: f32 = looks.iter().map(|(_, l)| l.weight).sum();
        let mirrored: f32 = looks
            .iter()
            .filter(|(_, l)| l.mirrored)
            .map(|(_, l)| l.weight)
            .sum();
        let mostly_mirrored = mirrored > total / 2.0;
        for (f, l) in looks.iter().take(cluster.len()) {
            if l.weight <= 0.0 {
                continue;
            }
            let empty = l.room <= none || none <= 0.0;
            let off_size = usual.is_some_and(|u| l.room > u * SIZE_OFF || l.room * SIZE_OFF < u);
            if empty || off_size || l.uneven >= UNEVEN || l.mirrored != mostly_mirrored {
                broken.insert(*f);
            }
        }
    }
    broken
}

/// Where, in the mesh's corner list, polygon `face`'s corner at `v` is.
fn corner_at(mesh: &Mesh, face: FaceId, v: VertexId) -> Option<usize> {
    let (starts, corners) = mesh.face_layout();
    let start = *starts.get(face.get() as usize)? as usize;
    let end = *starts.get(face.get() as usize + 1)? as usize;
    (start..end).find(|i| corners[*i] == v)
}

/// Edges of the changed, unbroken polygons where they meet another
/// unbroken polygon with different UVs, and which aren't seams yet.
fn new_splits(
    shape: &Shape,
    changed: &BTreeSet<FaceId>,
    broken: &BTreeSet<FaceId>,
) -> Vec<EdgeKey> {
    let mut out = BTreeSet::new();
    for f in changed.iter().filter(|f| !broken.contains(f)) {
        for (e, g) in shape.joined(*f) {
            if broken.contains(&g) {
                continue;
            }
            let (a, b) = e.ends();
            let apart = [a, b].iter().any(|v| {
                match (corner_at(shape.mesh, *f, *v), corner_at(shape.mesh, g, *v)) {
                    (Some(i), Some(j)) => {
                        shape.uvs.at(i).distance(shape.uvs.at(j)) > mesh::uvs::SAME_UV
                    }
                    _ => false,
                }
            });
            if apart {
                out.insert(e);
            }
        }
    }
    out.into_iter().collect()
}

/// Which of `seeds` (kept polygons) are joined to each other through kept
/// polygons (across edges not cut): a group number for each. The search
/// spreads from all of them at once and stops as soon as at most one
/// group can still grow, so a small part next to a big one is told apart
/// quickly.
fn kept_groups(
    shape: &Shape,
    seeds: &BTreeSet<FaceId>,
    broken: &BTreeSet<FaceId>,
) -> HashMap<FaceId, usize> {
    let seeds: Vec<FaceId> = seeds.iter().copied().collect();
    let mut sets = Sets::new(seeds.len());
    // How many faces wait to be spread from, per group (at its root).
    let mut waiting = vec![1usize; seeds.len()];
    let mut growing = seeds.len();
    let mut label: HashMap<FaceId, usize> =
        seeds.iter().enumerate().map(|(i, f)| (*f, i)).collect();
    let mut queue: VecDeque<(FaceId, usize)> =
        seeds.iter().enumerate().map(|(i, f)| (*f, i)).collect();
    let mut searched = 0;
    while growing > 1 && searched < SEARCH {
        let Some((f, l)) = queue.pop_front() else {
            break;
        };
        searched += 1;
        let root = sets.find(l);
        waiting[root] -= 1;
        if waiting[root] == 0 {
            growing -= 1;
        }
        for (_, g) in shape.joined(f) {
            if broken.contains(&g) {
                continue;
            }
            match label.get(&g) {
                Some(other) => {
                    let (a, b) = (sets.find(l), sets.find(*other));
                    if a != b {
                        let both = waiting[a] > 0 && waiting[b] > 0;
                        let sum = waiting[a] + waiting[b];
                        sets.join(a, b);
                        waiting[sets.find(a)] = sum;
                        if both {
                            growing -= 1;
                        }
                    }
                }
                None => {
                    label.insert(g, l);
                    let root = sets.find(l);
                    if waiting[root] == 0 {
                        growing += 1;
                    }
                    waiting[root] += 1;
                    queue.push_back((g, l));
                }
            }
        }
    }
    // Groups still growing when the search stopped are taken as one.
    let open: Vec<usize> = (0..seeds.len())
        .filter(|i| sets.find(*i) == *i && waiting[*i] > 0)
        .collect();
    for pair in open.windows(2) {
        sets.join(pair[0], pair[1]);
    }
    seeds
        .iter()
        .enumerate()
        .map(|(i, f)| (*f, sets.find(i)))
        .collect()
}

/// The kept polygons `cluster` carries on from (those next to it in the
/// group it shares the longest border with) and the edges parting it from
/// other groups; `None` if no kept polygon is next to it.
fn longest_border(
    shape: &Shape,
    cluster: &[FaceId],
    broken: &BTreeSet<FaceId>,
) -> Option<(BTreeSet<FaceId>, Vec<EdgeKey>)> {
    let mut borders: Vec<(EdgeKey, FaceId)> = Vec::new();
    for f in cluster {
        for (e, g) in shape.joined(*f) {
            if !broken.contains(&g) {
                borders.push((e, g));
            }
        }
    }
    let next: BTreeSet<FaceId> = borders.iter().map(|(_, g)| *g).collect();
    if next.is_empty() {
        return None;
    }
    let group = kept_groups(shape, &next, broken);
    let mut shared: BTreeMap<usize, f32> = BTreeMap::new();
    for (e, g) in &borders {
        let (a, b) = e.ends();
        let length = match (shape.mesh.position(a), shape.mesh.position(b)) {
            (Some(p), Some(q)) => p.distance(q),
            _ => 0.0,
        };
        *shared.entry(group[g]).or_default() += length;
    }
    // Ties go to the lowest group, so the result doesn't depend on chance.
    let best = shared
        .iter()
        .max_by(|a, b| a.1.total_cmp(b.1).then(b.0.cmp(a.0)))
        .map(|(g, _)| *g)?;
    let mut ring = BTreeSet::new();
    let mut seams = Vec::new();
    for (e, g) in borders {
        if group[&g] == best {
            ring.insert(g);
        } else {
            seams.push(e);
        }
    }
    Some((ring, seams))
}

/// Solves `cluster` around the kept polygons `ring`, writing its corners'
/// UVs into `values`; whether it could be solved.
fn solve_around(
    shape: &Shape,
    cluster: &[FaceId],
    ring: &BTreeSet<FaceId>,
    values: &mut [Vec2],
) -> bool {
    let mesh = shape.mesh;
    let starts = mesh.face_layout().0;
    let at_corner = |f: FaceId, k: usize| starts[f.get() as usize] as usize + k;
    let mut faces = cluster.to_vec();
    faces.extend(ring.iter().copied());
    let chart = Chart::of(mesh, &faces, shape.cuts);
    // The kept polygons' corners hold their UVs.
    let mut pins: Vec<Option<Vec2>> = vec![None; chart.len()];
    let mut start: Vec<Vec2> = vec![Vec2::ZERO; chart.len()];
    let mut mirrored = 0.0f32;
    for (f, wedges) in chart.faces.iter().zip(&chart.corners) {
        let keep = ring.contains(f);
        for (k, w) in wedges.iter().enumerate() {
            let uv = values[at_corner(*f, k)];
            start[*w] = uv;
            if keep {
                pins[*w] = Some(uv);
            }
        }
        if keep && let Some(l) = shape.look(*f) {
            mirrored += if l.mirrored { l.weight } else { -l.weight };
        }
    }
    let (lo, hi) = pins
        .iter()
        .flatten()
        .fold((Vec2::INFINITY, Vec2::NEG_INFINITY), |(lo, hi), v| {
            (lo.min(*v), hi.max(*v))
        });
    if pins.iter().flatten().count() < 2 || (hi - lo).length() < 1e-7 {
        return false;
    }
    // Solving keeps angles the right way round, so polygons lying mirrored
    // are solved mirrored back and turned round after.
    let flip = |v: Vec2| {
        if mirrored > 0.0 {
            Vec2::new(-v.x, v.y)
        } else {
            v
        }
    };
    let pins: Vec<Option<Vec2>> = pins.into_iter().map(|p| p.map(flip)).collect();
    let start: Vec<Vec2> = start.into_iter().map(flip).collect();
    let Some(laid) = conformal_pinned(mesh, &chart, &pins, &start) else {
        return false;
    };
    let mine: BTreeSet<FaceId> = cluster.iter().copied().collect();
    for (f, wedges) in chart.faces.iter().zip(&chart.corners) {
        if mine.contains(f) {
            for (k, w) in wedges.iter().enumerate() {
                values[at_corner(*f, k)] = flip(laid[*w]);
            }
        }
    }
    true
}

/// Mends the UVs an edit broke: `before` is the shape before the edit,
/// `mesh` after it. Returns how many polygons were laid out again. A shape
/// without UVs, or whose polygons the edit didn't change, is left alone.
pub fn mend(before: &Mesh, mesh: &mut Mesh) -> usize {
    let Some(uvs) = Uvs::of(mesh) else {
        return 0;
    };
    let changed: BTreeSet<FaceId> = mesh
        .faces()
        .filter(|(f, list)| before.face(*f) != Some(*list))
        .map(|(f, _)| f)
        .collect();
    if changed.is_empty() {
        return 0;
    }
    let cuts = cuts_of(mesh);
    let shape = Shape {
        mesh,
        uvs: &uvs,
        cuts: &cuts,
    };
    let broken = broken_among(&shape, &changed);
    if broken.is_empty() || broken.len() > MEND_UP_TO {
        return 0;
    }
    let splits = new_splits(&shape, &changed, &broken);
    // Pieces are told apart by seams from here on: a shape without seam
    // marks (imported, say) gets them where its UVs split, which cuts it
    // the same way.
    if mesh.attribute_values(mesh::attributes::SEAM).is_none() {
        mesh::uvs::seams_from_splits(mesh);
    }
    // Pieces stay apart where they now meet with different UVs.
    if !splits.is_empty() {
        mesh::uvs::set_seams(mesh, &splits, true);
    }
    let mut cuts = cuts_of(mesh);
    let mut values = uvs.into_values(mesh.face_layout().1.len());
    // Each cluster of broken polygons is solved around the kept ones next
    // to it, or laid out on its own.
    let mut alone: BTreeSet<FaceId> = BTreeSet::new();
    let mut apart: Vec<EdgeKey> = Vec::new();
    let Some(current) = Uvs::of(mesh) else {
        return 0;
    };
    let clusters = Shape {
        mesh,
        uvs: &current,
        cuts: &cuts,
    }
    .parts(&broken);
    for cluster in &clusters {
        let found = longest_border(
            &Shape {
                mesh,
                uvs: &current,
                cuts: &cuts,
            },
            cluster,
            &broken,
        );
        let Some((ring, seams)) = found else {
            alone.extend(cluster.iter().copied());
            continue;
        };
        cuts.extend(seams.iter().copied());
        apart.extend(seams);
        let shape = Shape {
            mesh,
            uvs: &current,
            cuts: &cuts,
        };
        if !solve_around(&shape, cluster, &ring, &mut values) {
            alone.extend(cluster.iter().copied());
        }
    }
    mesh::uvs::set_uvs(mesh, values);
    if !apart.is_empty() {
        mesh::uvs::set_seams(mesh, &apart, true);
    }
    // Polygons that still lie badly after being solved go on their own
    // too.
    let solved: BTreeSet<FaceId> = broken.difference(&alone).copied().collect();
    let Some(now) = Uvs::of(mesh) else {
        return broken.len();
    };
    let shape = Shape {
        mesh,
        uvs: &now,
        cuts: &cuts,
    };
    alone.extend(broken_among(&shape, &solved));
    if alone.is_empty() {
        return broken.len();
    }
    // Laid out at the shape's usual density (texture area per surface
    // area), with seams around them making them pieces of their own.
    let density = shape.usual_room().powi(2);
    let topology = mesh.topology();
    let mut around = BTreeSet::new();
    for f in &alone {
        for e in shape.edges(*f) {
            if topology
                .edge_uses(e)
                .iter()
                .any(|u| !alone.contains(&u.face))
            {
                around.insert(e);
            }
        }
    }
    cuts.extend(around.iter().copied());
    let pieces = Shape {
        mesh,
        uvs: &now,
        cuts: &cuts,
    }
    .parts(&alone);
    let around: Vec<EdgeKey> = around.into_iter().collect();
    mesh::uvs::set_seams(mesh, &around, true);
    lay_out(mesh, &pieces, cuts, density, true);
    keep_inside(mesh, &pieces);
    broken.len()
}

/// Slides each of `pieces` back inside the texture square where it sticks
/// out (a new piece is put where its squashed UVs were, which can be at
/// the square's edge). A piece bigger than the square lines up with its
/// lower left corner.
fn keep_inside(mesh: &mut Mesh, pieces: &[Vec<FaceId>]) {
    let Some(uvs) = Uvs::of(mesh) else {
        return;
    };
    let mut values = uvs.into_values(mesh.face_layout().1.len());
    let starts = mesh.face_layout().0;
    let mut moved = false;
    for piece in pieces {
        let corners: Vec<usize> = piece
            .iter()
            .flat_map(|f| {
                let start = starts[f.get() as usize] as usize;
                let n = mesh.face(*f).map_or(0, <[VertexId]>::len);
                start..start + n
            })
            .collect();
        let (lo, hi) = corners
            .iter()
            .fold((Vec2::INFINITY, Vec2::NEG_INFINITY), |(lo, hi), i| {
                (lo.min(values[*i]), hi.max(values[*i]))
            });
        if !lo.is_finite() || !hi.is_finite() {
            continue;
        }
        let shift = Vec2::new(inside(lo.x, hi.x), inside(lo.y, hi.y));
        if shift != Vec2::ZERO {
            moved = true;
            for i in corners {
                values[i] += shift;
            }
        }
    }
    if moved {
        mesh::uvs::set_uvs(mesh, values);
    }
}

/// How far to slide the span `lo..hi` to lie within 0 to 1.
fn inside(lo: f32, hi: f32) -> f32 {
    if hi - lo >= 1.0 || lo < 0.0 {
        -lo
    } else if hi > 1.0 {
        1.0 - hi
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests;
