//! Where to cut a surface open so it lies flat ([decision 0098](../../../docs/decisions/0098-seams.md)).
//!
//! A piece of surface can be laid flat in one go only if it is, in shape,
//! a **disk**: one piece, with one border all the way round and no
//! handles. A cube's side is a disk; a tube's side (two borders) or a
//! whole sphere (no border) isn't, and needs cutting first. [`is_disk`]
//! tells them apart.
//!
//! [`suggest`] picks seams for a whole shape the way a hard-surface artist
//! would:
//!
//! 1. Cut along the hard edges (a normal-map bake needs a UV split
//!    wherever the shading breaks) and along sharp corners.
//! 2. Make every piece that's left a disk with as little cutting as
//!    possible, hiding the cuts: a piece with no border is first cut along
//!    a hidden path from its lowest point to its highest; then the edges
//!    to keep joined are chosen by how visible they are (a spanning tree
//!    of the polygons, most visible edges first), and the rest are cut,
//!    trimmed back to what is needed (cuts that lead nowhere are joined
//!    again).
//!
//! "Hidden" means facing down or to the back (away from the Front view),
//! in a concave corner, or on a corner rather than across a flat stretch:
//! the places a viewer least looks at.
//!
//! Every choice is the user's to change ([`SuggestOptions`], [decision
//! 0110](../../../docs/decisions/0110-controllable-automation.md)): the
//! corner angle, whether hard edges are cut, and where cuts hide
//! ([`HideSeams`]). The same shape and options always give the same
//! seams: every tie is settled by element ids, never by chance.
//!
//! The user can also say where seams should and shouldn't go, and how
//! stretched a piece may be ([decision
//! 0111](../../../docs/decisions/0111-seam-preferences-and-stretch-budget.md)):
//!
//! - an edge marked **prefer** is where a needed cut goes first;
//! - an edge marked **never** is never cut by a tool: not as a hard edge
//!   or corner, and not to make a piece lie flat unless it couldn't
//!   otherwise;
//! - a **stretch budget** (for the whole shape, or a piece's own) is the
//!   most nearly all of a piece (all but its worst 5% by area) may be off
//!   once laid flat: stretched one way more than the other, or bigger or
//!   smaller than the rest. A piece over it is cut, a slit at a time, from
//!   its worst place to its border along the most hidden way, until it is
//!   within or nothing more helps ([`budget_cuts`]).

use crate::chart::{Chart, Sets};
use glam::Vec3;
use mesh::uvs::SeamPreference;
use mesh::{EdgeKey, FaceId, Mesh, VertexId};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap};

/// Corners sharper than this (degrees between the two polygons' facings)
/// are cut as well as hard edges: a piece bent further than this round
/// one edge stretches badly when laid flat. The standard for
/// [`SuggestOptions::corner_angle`].
pub const CORNER_ANGLE: f32 = 66.0;

/// Where cuts that only make a piece lie flat are hidden.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HideSeams {
    /// Underneath and at the back (away from the Front view): where a
    /// prop is least seen. The standard.
    #[default]
    BelowAndBehind,
    /// Underneath only.
    Below,
    /// At the back only.
    Behind,
    /// Nowhere in particular: the shortest cuts.
    Shortest,
}

impl HideSeams {
    /// Every choice, in the order offered.
    pub const ALL: [Self; 4] = [
        Self::BelowAndBehind,
        Self::Below,
        Self::Behind,
        Self::Shortest,
    ];
}

/// How [`suggest_with`] picks seams.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SuggestOptions {
    /// Corners sharper than this (degrees) are cut; 180 cuts none.
    pub corner_angle: f32,
    /// Whether hard edges are cut.
    pub hard_edges: bool,
    /// Where the cuts that make pieces lie flat are hidden.
    pub hide: HideSeams,
    /// The stretch budget for pieces without one of their own (`None`: no
    /// budget).
    pub budget: Option<f32>,
}

impl Default for SuggestOptions {
    fn default() -> Self {
        Self {
            corner_angle: CORNER_ANGLE,
            hard_edges: true,
            hide: HideSeams::BelowAndBehind,
            budget: None,
        }
    }
}

/// The most slits [`budget_cuts`] makes in one piece.
pub const MAX_BUDGET_CUTS: usize = 24;

/// The edges shared by exactly two polygons: the only edges a seam can be
/// on (an open edge is cut already; an edge of three or more polygons
/// can't be laid flat joined).
fn inner_edges(mesh: &Mesh) -> Vec<EdgeKey> {
    let topology = mesh.topology();
    topology
        .edges()
        .filter(|e| topology.edge_uses(*e).len() == 2)
        .collect()
}

/// The two polygons of an inner edge.
fn sides(mesh: &Mesh, edge: EdgeKey) -> Option<(FaceId, FaceId)> {
    match mesh.topology().edge_uses(edge) {
        [a, b] if a.face != b.face => Some((a.face, b.face)),
        _ => None,
    }
}

/// The pieces left when `cuts` are cut open: polygons joined across inner
/// edges that aren't cut, each piece in id order, in order of their first
/// polygon.
pub fn pieces(mesh: &Mesh, cuts: &BTreeSet<EdgeKey>) -> Vec<Vec<FaceId>> {
    let faces: Vec<FaceId> = mesh.faces().map(|(f, _)| f).collect();
    let slot: HashMap<FaceId, usize> = faces.iter().enumerate().map(|(i, f)| (*f, i)).collect();
    let mut sets = Sets::new(faces.len());
    for edge in inner_edges(mesh) {
        if cuts.contains(&edge) {
            continue;
        }
        if let Some((a, b)) = sides(mesh, edge) {
            sets.join(slot[&a], slot[&b]);
        }
    }
    let mut groups: BTreeMap<usize, Vec<FaceId>> = BTreeMap::new();
    let mut order: Vec<usize> = Vec::new();
    for (i, f) in faces.iter().enumerate() {
        let root = sets.find(i);
        let group = groups.entry(root).or_default();
        if group.is_empty() {
            order.push(root);
        }
        group.push(*f);
    }
    order
        .into_iter()
        .filter_map(|root| groups.remove(&root))
        .collect()
}

/// Whether `piece` (polygons of `mesh`) lies flat in one go once `cuts`
/// are cut open: one piece, shaped like a disk (one border, no handles).
/// Worked out from the Euler characteristic of the cut-open piece, which
/// is 1 exactly for a disk.
pub fn is_disk(mesh: &Mesh, piece: &[FaceId], cuts: &BTreeSet<EdgeKey>) -> bool {
    if piece.is_empty() {
        return false;
    }
    let inside: BTreeSet<FaceId> = piece.iter().copied().collect();
    let topology = mesh.topology();
    // The corners of the piece: one per polygon and point. Corners of a
    // point are one point of the cut-open piece when joined across an
    // edge that isn't cut.
    let mut corner_index: HashMap<(FaceId, VertexId), usize> = HashMap::new();
    let mut edges_inside: BTreeSet<EdgeKey> = BTreeSet::new();
    let mut edge_sides = 0usize;
    for f in piece {
        let Some(points) = mesh.face(*f) else {
            return false;
        };
        for (k, v) in points.iter().enumerate() {
            let next = corner_index.len();
            corner_index.entry((*f, *v)).or_insert(next);
            let w = points[(k + 1) % points.len()];
            edges_inside.insert(EdgeKey::new(*v, w));
        }
    }
    let mut sets = Sets::new(corner_index.len());
    let mut joined_edges = 0usize;
    for edge in &edges_inside {
        let uses: Vec<FaceId> = topology
            .edge_faces(*edge)
            .filter(|f| inside.contains(f))
            .collect();
        let joined = uses.len() == 2
            && topology.edge_uses(*edge).len() == 2
            && !cuts.contains(edge)
            && uses[0] != uses[1];
        if joined {
            joined_edges += 1;
            let (a, b) = edge.ends();
            for v in [a, b] {
                if let (Some(x), Some(y)) = (
                    corner_index.get(&(uses[0], v)),
                    corner_index.get(&(uses[1], v)),
                ) {
                    sets.join(*x, *y);
                }
            }
        } else {
            edge_sides += uses.len();
        }
    }
    let mut roots = BTreeSet::new();
    for i in 0..corner_index.len() {
        roots.insert(sets.find(i));
    }
    // The piece must hang together across its joined edges.
    let mut faces = Sets::new(piece.len());
    let slot: HashMap<FaceId, usize> = piece.iter().enumerate().map(|(i, f)| (*f, i)).collect();
    for edge in &edges_inside {
        let uses: Vec<FaceId> = topology
            .edge_faces(*edge)
            .filter(|f| inside.contains(f))
            .collect();
        if uses.len() == 2 && topology.edge_uses(*edge).len() == 2 && !cuts.contains(edge) {
            faces.join(slot[&uses[0]], slot[&uses[1]]);
        }
    }
    let one_piece = (0..piece.len()).all(|i| faces.find(i) == faces.find(0));
    let points = roots.len() as i64;
    let edges = (joined_edges + edge_sides) as i64;
    let euler = points - edges + piece.len() as i64;
    one_piece && euler == 1
}

/// The middle of polygon `f`.
fn middle(mesh: &Mesh, f: FaceId) -> Vec3 {
    let points = mesh.face(f).unwrap_or(&[]);
    let sum: Vec3 = points.iter().filter_map(|v| mesh.position(*v)).sum();
    sum / points.len().max(1) as f32
}

/// How much `edge` is seen: higher for edges facing up or to the front,
/// across flat stretches and on outward corners; lower facing down or to
/// the back (+Y, away from the Front view), in concave corners and on
/// sharp ones, as `hide` weighs them. Cuts go where this is low.
fn visibility(mesh: &Mesh, edge: EdgeKey, hide: HideSeams) -> f32 {
    let Some((f, g)) = sides(mesh, edge) else {
        return 0.0;
    };
    match mesh::uvs::seam_preference(mesh, edge) {
        SeamPreference::Prefer => return PREFERRED,
        SeamPreference::Never => return NEVER,
        SeamPreference::Free => {}
    }
    if hide == HideSeams::Shortest {
        return 2.0;
    }
    let (nf, ng) = (mesh.face_normal(f), mesh.face_normal(g));
    let facing = (nf + ng).normalize_or_zero();
    let flat = nf.dot(ng).clamp(-1.0, 1.0);
    let concave = nf.dot(middle(mesh, g) - middle(mesh, f)) > 1e-6;
    // The front view looks along +Y: the back faces +Y.
    let (up, back) = match hide {
        HideSeams::BelowAndBehind => (1.0, 0.5),
        HideSeams::Below => (1.0, 0.0),
        HideSeams::Behind => (0.0, 1.0),
        HideSeams::Shortest => (0.0, 0.0),
    };
    2.0 + up * facing.z - back * facing.y + 0.5 * flat - if concave { 1.0 } else { 0.0 }
}

/// How visible an edge marked "prefer" counts as: less than any other, so
/// a needed cut goes there first.
const PREFERRED: f32 = -10.0;

/// How visible an edge marked "never" counts as: far more than any other,
/// so it is cut only when nothing else would do.
const NEVER: f32 = 1_000.0;

/// A path's cost along `edge`: its length, more where it is seen.
fn path_cost(mesh: &Mesh, edge: EdgeKey, hide: HideSeams) -> f32 {
    let (a, b) = edge.ends();
    let length = match (mesh.position(a), mesh.position(b)) {
        (Some(p), Some(q)) => p.distance(q),
        _ => 1.0,
    };
    length.max(1e-6) * (visibility(mesh, edge, hide) + 2.0).max(0.1)
}

/// An entry of the path search's queue (smallest cost first).
#[derive(PartialEq)]
struct Step(f32, VertexId);

impl Eq for Step {}

impl PartialOrd for Step {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Step {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.total_cmp(&self.0).then(other.1.cmp(&self.1))
    }
}

/// The cheapest path from `from` to `to` along `edges`, as edges.
fn hidden_path(
    mesh: &Mesh,
    edges: &[EdgeKey],
    from: VertexId,
    to: VertexId,
    hide: HideSeams,
) -> Vec<EdgeKey> {
    let mut next: HashMap<VertexId, Vec<(VertexId, EdgeKey, f32)>> = HashMap::new();
    for e in edges {
        let (a, b) = e.ends();
        let cost = path_cost(mesh, *e, hide);
        next.entry(a).or_default().push((b, *e, cost));
        next.entry(b).or_default().push((a, *e, cost));
    }
    let mut best: HashMap<VertexId, (f32, Option<(VertexId, EdgeKey)>)> = HashMap::new();
    best.insert(from, (0.0, None));
    let mut queue = BinaryHeap::new();
    queue.push(Step(0.0, from));
    while let Some(Step(cost, v)) = queue.pop() {
        if v == to {
            break;
        }
        if best.get(&v).is_some_and(|(c, _)| *c < cost) {
            continue;
        }
        for (w, e, step) in next.get(&v).map(Vec::as_slice).unwrap_or(&[]) {
            let total = cost + step;
            if best.get(w).is_none_or(|(c, _)| total < *c) {
                best.insert(*w, (total, Some((v, *e))));
                queue.push(Step(total, *w));
            }
        }
    }
    let mut path = Vec::new();
    let mut at = to;
    while let Some((_, Some((back, e)))) = best.get(&at) {
        path.push(*e);
        at = *back;
        if path.len() > edges.len() {
            return Vec::new();
        }
    }
    if at == from { path } else { Vec::new() }
}

/// The cuts that make `piece` a disk, given what is cut already: none for
/// a disk; for a piece with no border, a hidden path from its lowest
/// point to its highest first; then what a spanning tree of its polygons
/// (most visible edges kept joined) leaves, trimmed back. Hidden as
/// `hide` says.
pub(crate) fn disk_cuts(
    mesh: &Mesh,
    piece: &[FaceId],
    cuts: &BTreeSet<EdgeKey>,
    hide: HideSeams,
) -> BTreeSet<EdgeKey> {
    let mut added = BTreeSet::new();
    if is_disk(mesh, piece, cuts) {
        return added;
    }
    let topology = mesh.topology();
    let inside: BTreeSet<FaceId> = piece.iter().copied().collect();
    let mut joined: Vec<EdgeKey> = Vec::new();
    let mut border: Vec<EdgeKey> = Vec::new();
    let mut seen = BTreeSet::new();
    for f in piece {
        let points = mesh.face(*f).unwrap_or(&[]);
        for (k, v) in points.iter().enumerate() {
            let e = EdgeKey::new(*v, points[(k + 1) % points.len()]);
            if !seen.insert(e) {
                continue;
            }
            let both_inside = topology
                .edge_faces(e)
                .filter(|g| inside.contains(g))
                .count()
                == 2;
            if both_inside && topology.edge_uses(e).len() == 2 && !cuts.contains(&e) {
                joined.push(e);
            } else {
                border.push(e);
            }
        }
    }
    // No border at all: open it along a hidden path, bottom to top.
    if border.is_empty() {
        let points: BTreeSet<VertexId> = joined
            .iter()
            .flat_map(|e| {
                let (a, b) = e.ends();
                [a, b]
            })
            .collect();
        let height = |v: &VertexId| mesh.position(*v).map_or(0.0, |p| p.z);
        let low = points
            .iter()
            .copied()
            .min_by(|a, b| height(a).total_cmp(&height(b)).then(a.cmp(b)));
        let high = points
            .iter()
            .copied()
            .max_by(|a, b| height(a).total_cmp(&height(b)).then(b.cmp(a)));
        let mut path = match (low, high) {
            (Some(l), Some(h)) if l != h => hidden_path(mesh, &joined, l, h, hide),
            _ => Vec::new(),
        };
        if path.is_empty()
            && let Some(e) = joined.iter().copied().min_by(|a, b| {
                visibility(mesh, *a, hide)
                    .total_cmp(&visibility(mesh, *b, hide))
                    .then(a.cmp(b))
            })
        {
            path.push(e);
        }
        for e in path {
            added.insert(e);
            border.push(e);
        }
        joined.retain(|e| !added.contains(e));
    }
    // Keep the most visible edges joined: a spanning tree of the polygons.
    let slot: HashMap<FaceId, usize> = piece.iter().enumerate().map(|(i, f)| (*f, i)).collect();
    let mut order: Vec<(f32, EdgeKey)> = joined
        .iter()
        .map(|e| (visibility(mesh, *e, hide), *e))
        .collect();
    order.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut sets = Sets::new(piece.len());
    let mut open: BTreeSet<EdgeKey> = BTreeSet::new();
    for (_, e) in order {
        let Some((a, b)) = sides(mesh, e) else {
            continue;
        };
        if !sets.join(slot[&a], slot[&b]) {
            open.insert(e);
        }
    }
    // Trim cuts that lead nowhere: an end only one cut (or border) meets
    // can be joined again.
    let mut degree: HashMap<VertexId, usize> = HashMap::new();
    for e in open.iter().chain(border.iter()) {
        let (a, b) = e.ends();
        *degree.entry(a).or_default() += 1;
        *degree.entry(b).or_default() += 1;
    }
    let mut changed = true;
    while changed {
        changed = false;
        let dangling: Vec<EdgeKey> = open
            .iter()
            .copied()
            .filter(|e| {
                let (a, b) = e.ends();
                degree.get(&a) == Some(&1) || degree.get(&b) == Some(&1)
            })
            .collect();
        for e in dangling {
            open.remove(&e);
            let (a, b) = e.ends();
            for v in [a, b] {
                if let Some(d) = degree.get_mut(&v) {
                    *d -= 1;
                }
            }
            changed = true;
        }
    }
    added.extend(open);
    added
}

/// Seams for the whole of `mesh`, as a hard-surface artist would place
/// them: along `hard` (the edges shading hard, as shown) and corners
/// sharper than [`CORNER_ANGLE`], then whatever each piece needs to lie
/// flat, in hidden places. Inner edges only, in order.
pub fn suggest(mesh: &Mesh, hard: &[EdgeKey]) -> Vec<EdgeKey> {
    suggest_with(mesh, hard, SuggestOptions::default())
}

/// [`suggest`], as `options` say: hard edges cut or not, corners sharper
/// than `options.corner_angle` cut (none at 180°), the rest hidden as
/// `options.hide` says.
pub fn suggest_with(mesh: &Mesh, hard: &[EdgeKey], options: SuggestOptions) -> Vec<EdgeKey> {
    suggest_explained(mesh, hard, options)
        .into_iter()
        .map(|(e, _)| e)
        .collect()
}

/// Why Suggest seams puts a seam on an edge
/// ([decision 0116](../../../docs/decisions/0116-why-is-this-here.md)).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SeamReason {
    /// The edge shades hard: a seam there keeps a normal map clean.
    HardEdge,
    /// The polygons meet at this sharp an angle (degrees), past the
    /// corner setting.
    SharpCorner(f32),
    /// The piece couldn't lie flat in one go (a closed or looped piece),
    /// so it was cut open here, the most hidden way.
    ToLieFlat,
    /// As [`Self::ToLieFlat`], along an edge you preferred.
    Preferred,
    /// The piece stretched past this budget once laid flat, so it was
    /// slit here.
    StretchBudget(f32),
}

/// The angle, in degrees, at which the two polygons of `edge` meet (0°
/// flat), if it has two with a facing.
pub fn edge_angle(mesh: &Mesh, edge: EdgeKey) -> Option<f32> {
    let topology = mesh.topology();
    match topology.edge_uses(edge) {
        [one, two] => {
            let (n1, n2) = (mesh.face_normal(one.face), mesh.face_normal(two.face));
            (n1.length_squared() > 0.5 && n2.length_squared() > 0.5).then(|| {
                n1.normalize()
                    .dot(n2.normalize())
                    .clamp(-1.0, 1.0)
                    .acos()
                    .to_degrees()
            })
        }
        _ => None,
    }
}

/// [`suggest_with`], with the reason for each seam, in edge order.
pub fn suggest_explained(
    mesh: &Mesh,
    hard: &[EdgeKey],
    options: SuggestOptions,
) -> Vec<(EdgeKey, SeamReason)> {
    let inner: BTreeSet<EdgeKey> = inner_edges(mesh).into_iter().collect();
    let hard = if options.hard_edges { hard } else { &[] };
    let corners = if options.corner_angle < 180.0 {
        mesh::hardness::sharp_edges(mesh, options.corner_angle.max(0.0))
    } else {
        Vec::new()
    };
    let never: BTreeSet<EdgeKey> = mesh::uvs::seam_preferences(mesh)
        .into_iter()
        .filter(|(_, p)| *p == SeamPreference::Never)
        .map(|(e, _)| e)
        .collect();
    let usable = |e: &EdgeKey| inner.contains(e) && !never.contains(e);
    let mut reasons: BTreeMap<EdgeKey, SeamReason> = BTreeMap::new();
    for e in hard.iter().copied().filter(usable) {
        reasons.insert(e, SeamReason::HardEdge);
    }
    for e in corners.into_iter().filter(usable) {
        reasons
            .entry(e)
            .or_insert_with(|| SeamReason::SharpCorner(edge_angle(mesh, e).unwrap_or(180.0)));
    }
    let mut cuts: BTreeSet<EdgeKey> = reasons.keys().copied().collect();
    for piece in pieces(mesh, &cuts) {
        let more = disk_cuts(mesh, &piece, &cuts, options.hide);
        for e in &more {
            let preferred = mesh::uvs::seam_preference(mesh, *e) == SeamPreference::Prefer;
            reasons.insert(
                *e,
                if preferred {
                    SeamReason::Preferred
                } else {
                    SeamReason::ToLieFlat
                },
            );
        }
        cuts.extend(more);
        let budget = mesh::uvs::budget_of(mesh, &piece).or(options.budget);
        if let Some(budget) = budget {
            let more = budget_cuts(mesh, &piece, &cuts, budget, options.hide);
            for e in &more {
                reasons.insert(*e, SeamReason::StretchBudget(budget));
            }
            cuts.extend(more);
        }
    }
    reasons
        .into_iter()
        .filter(|(e, _)| inner.contains(e))
        .collect()
}

/// How far off nearly all of `piece` is once laid flat, cut along `cuts`
/// (see [`crate::methods::nearly_all`]), and which polygon each of its
/// triangles belongs to with how squeezed or uneven each is. `None` when
/// it can't be laid out.
fn worst_stretch(
    mesh: &Mesh,
    piece: &[FaceId],
    cuts: &BTreeSet<EdgeKey>,
) -> Option<(f64, Vec<(FaceId, f64)>)> {
    let chart = Chart::of(mesh, piece, cuts);
    let none = vec![None; chart.len()];
    let (at, _) =
        crate::methods::lay_flat(mesh, &chart, crate::UnwrapMethod::Conformal, &none, &[])?;
    let (triangles, shapes) = crate::methods::real_shapes(mesh, &chart);
    let parts = crate::methods::stretch_parts(&triangles, &shapes, &at);
    let each: Vec<f64> = parts
        .iter()
        .map(|(uneven, size)| uneven.max(*size).max(1.0 / size))
        .collect();
    // Where to slit from: where the surface is squeezed (smaller than the
    // rest) or stretched one way. A curved piece laid flat squeezes in
    // its middle and spreads at its border; a slit from the squeeze lets
    // it open out. Spreading at the border is the same strain seen from
    // the other side.
    let owners = chart.triangle_owners(mesh);
    let by_face: Vec<(FaceId, f64)> = owners
        .iter()
        .zip(&parts)
        .map(|(o, (uneven, size))| (chart.faces[*o], uneven.max(1.0 / size)))
        .collect();
    Some((crate::methods::nearly_all(&each, &shapes), by_face))
}

/// The slits that bring `piece` (cut along `cuts`, a disk) within
/// `budget`: from the point inside the piece where it is most squeezed or
/// stretched one way to the border along the most hidden way, a slit at a time, until
/// nearly all of it is within the budget, nothing more can be cut, or
/// [`MAX_BUDGET_CUTS`] slits are made. A slit that doesn't help is taken
/// back and the search stops. Never edges are not cut.
pub fn budget_cuts(
    mesh: &Mesh,
    piece: &[FaceId],
    cuts: &BTreeSet<EdgeKey>,
    budget: f32,
    hide: HideSeams,
) -> BTreeSet<EdgeKey> {
    let mut added = BTreeSet::new();
    if !budget.is_finite() || budget < 1.0 || piece.len() < 2 {
        return added;
    }
    let budget = f64::from(budget);
    let topology = mesh.topology();
    let inside: BTreeSet<FaceId> = piece.iter().copied().collect();
    let mut all = cuts.clone();
    let Some((mut worst, mut by_face)) = worst_stretch(mesh, piece, &all) else {
        return added;
    };
    for _ in 0..MAX_BUDGET_CUTS {
        if worst <= budget {
            break;
        }
        // The edges still joined inside the piece, and its border's points.
        let mut joined: Vec<EdgeKey> = Vec::new();
        let mut border: BTreeSet<VertexId> = BTreeSet::new();
        let mut seen = BTreeSet::new();
        for f in piece {
            let points = mesh.face(*f).unwrap_or(&[]);
            for (k, v) in points.iter().enumerate() {
                let e = EdgeKey::new(*v, points[(k + 1) % points.len()]);
                if !seen.insert(e) {
                    continue;
                }
                let both_inside = topology
                    .edge_faces(e)
                    .filter(|g| inside.contains(g))
                    .count()
                    == 2;
                if both_inside && topology.edge_uses(e).len() == 2 && !all.contains(&e) {
                    if mesh::uvs::seam_preference(mesh, e) != SeamPreference::Never {
                        joined.push(e);
                    }
                } else {
                    let (a, b) = e.ends();
                    border.insert(a);
                    border.insert(b);
                }
            }
        }
        // How far each point is from the border, the most hidden way.
        let mut next: BTreeMap<VertexId, Vec<(VertexId, EdgeKey, f32)>> = BTreeMap::new();
        for e in &joined {
            let (a, b) = e.ends();
            let cost = path_cost(mesh, *e, hide);
            next.entry(a).or_default().push((b, *e, cost));
            next.entry(b).or_default().push((a, *e, cost));
        }
        let mut best: BTreeMap<VertexId, (f32, Option<(VertexId, EdgeKey)>)> = BTreeMap::new();
        let mut queue = BinaryHeap::new();
        for v in &border {
            best.insert(*v, (0.0, None));
            queue.push(Step(0.0, *v));
        }
        while let Some(Step(cost, v)) = queue.pop() {
            if best.get(&v).is_some_and(|(c, _)| *c < cost) {
                continue;
            }
            for (w, e, step) in next.get(&v).map(Vec::as_slice).unwrap_or(&[]) {
                let total = cost + step;
                if best.get(w).is_none_or(|(c, _)| total < *c) {
                    best.insert(*w, (total, Some((v, *e))));
                    queue.push(Step(total, *w));
                }
            }
        }
        // Each point inside the piece is as squeezed or uneven as the
        // worst polygon round it; the slit starts from the worst such
        // point (the one furthest inside, on a tie).
        let mut off: BTreeMap<VertexId, f64> = BTreeMap::new();
        for (f, s) in &by_face {
            for v in mesh.face(*f).unwrap_or(&[]) {
                let e = off.entry(*v).or_insert(1.0);
                *e = e.max(*s);
            }
        }
        let start = off
            .iter()
            .filter_map(|(v, s)| {
                let d = best.get(v)?.0;
                (d > 0.0).then_some((*v, *s, d))
            })
            .max_by(|a, b| {
                a.1.total_cmp(&b.1)
                    .then(a.2.total_cmp(&b.2))
                    .then(b.0.cmp(&a.0))
            })
            .map(|(v, _, _)| v);
        let Some(mut at) = start else {
            break;
        };
        let mut slit = Vec::new();
        while let Some((_, Some((back, e)))) = best.get(&at) {
            slit.push(*e);
            at = *back;
            if slit.len() > joined.len() {
                break;
            }
        }
        if slit.is_empty() {
            break;
        }
        let mut trial = all.clone();
        trial.extend(slit.iter().copied());
        let Some((now, now_by_face)) = worst_stretch(mesh, piece, &trial) else {
            break;
        };
        if now >= worst - 1e-9 {
            break;
        }
        added.extend(slit);
        all = trial;
        worst = now;
        by_face = now_by_face;
    }
    added
}

#[cfg(test)]
mod tests;
