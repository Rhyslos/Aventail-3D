//! Bevelling edges and points: cutting a sharp edge or corner away and
//! putting a strip (or a corner polygon) of new polygons in its place
//! ([decision 0040](../../../docs/decisions/0040-bevel.md)).
//!
//! **Edges.** Each bevelled edge is replaced by a strip `width` wide on
//! each side, measured across each polygon next to it. With more
//! `segments` the strip is cut lengthwise into rows that follow a
//! `profile` from round (1) through flat (0, a chamfer) to hollow (-1),
//! curving inward like a cove. Where the strip ends:
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
//! Attribute values come along through the [`Builder`]: new points copy
//! or blend the points they lie between, a strip's corners blend the two
//! polygons' corners it runs between (so UVs stretch across it), corner
//! polygons copy the polygons around them, and edges that slide take their
//! values (hard, seam) with them. A bevelled edge's own values go with it.
//!
//! As with every edit, polygons keep their ids (they get new corners), new
//! points and polygons take the next ids in a fixed order, and points no
//! polygon uses any more are deleted. Edges need a polygon on each side
//! and points a single fan of polygons around them; others are skipped
//! and counted. Bevels never make points that aren't numbers.
//!
//! **More control** ([`BevelOptions`], [decision 0085](../../../docs/decisions/0085-bevel-control.md)):
//! - **Even width** sizes each edge's bevel by the angle its polygons
//!   meet at, so every bevelled edge's new polygon is as wide as on a
//!   right-angled edge, however flat or sharp the edge is.
//! - **Skew** puts more of the bevel on one side of the edges than the
//!   other; the side follows each chain of edges round, and
//!   [`BevelOptions::flip_side`] swaps it.
//! - **Stop at overlaps** makes the whole bevel only as wide as fits:
//!   no corner goes more than halfway along an edge towards another
//!   bevelled point, or most of the way along any other edge.
//! - **Weld overlapping points** welds the new points that end up within
//!   [`BevelOptions::weld_distance`] of each other (where bevels meet).
//!
//! **The improved bevel** ([`BevelOptions::improved`], [decision 0141](../../../docs/decisions/0141-improved-bevel.md)):
//! - where three or more bevelled edges meet, with 2 or more segments, the
//!   corner is filled with a rounded patch of quads (`corner`) instead of
//!   one flat polygon;
//! - stopping at overlaps ("Stop at neighbours") narrows the bevel only
//!   at the points where it must, each point as much as it needs, instead
//!   of narrowing the whole bevel by the tightest point.

use crate::builder::{Builder, CornerFrom, PointFrom};
use crate::edit::Outcome;
use crate::topology::Topology;
use crate::{EdgeKey, FaceId, Mesh, VertexId};
use glam::Vec3;
use std::collections::{BTreeMap, BTreeSet, HashMap};

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
    /// How far the bevel reaches into each polygon, in the mesh's units
    /// (with [`BevelOptions::even`], on a right-angled edge).
    pub width: f32,
    /// How many rows the strip has (1: a flat chamfer).
    pub segments: u32,
    /// Round (1), flat (0) or hollow (-1, curving inward), or anything
    /// between.
    pub profile: f32,
    /// Even width, skew, stopping at overlaps and welding.
    pub options: BevelOptions,
}

impl Default for BevelShape {
    fn default() -> Self {
        Self {
            width: 10.0,
            segments: 1,
            profile: 1.0,
            options: BevelOptions::default(),
        }
    }
}

impl BevelShape {
    /// The shape brought within limits: width not below 0, 1 to
    /// [`MAX_SEGMENTS`] rows, profile -1 to 1, the options in range.
    /// `None` if a value isn't a number.
    pub fn clamped(self) -> Option<Self> {
        (self.width.is_finite() && self.profile.is_finite()).then(|| Self {
            width: self.width.max(0.0),
            segments: self.segments.clamp(1, MAX_SEGMENTS),
            profile: self.profile.clamp(-1.0, 1.0),
            options: self.options.clamped(),
        })
    }
}

/// The most skew does: at full skew one side of an edge gets this much
/// more of the width, the other this much less, so neither side ever
/// shrinks to nothing.
pub const SKEW_REACH: f32 = 0.9;

/// The standard weld distance after a bevel, in the mesh's units (1 mm).
pub const WELD_DISTANCE: f32 = 0.1;

/// The most a bevel's weld distance can be, in the mesh's units (10 cm).
pub const MAX_WELD_DISTANCE: f32 = 10.0;

/// The narrowest an edge's bevel may be made by [`BevelOptions::even`],
/// as the cosine of half the angle between its polygons' facings: very
/// sharp edges (over about 150°) are treated as that sharp.
const MIN_EVEN_COS: f32 = 0.25;

/// More control over a bevel ([decision 0085](../../../docs/decisions/0085-bevel-control.md)).
/// [`Default`] is what new bevels get; [`BevelOptions::EARLIER`] is how
/// every bevel worked before, which bevels saved before file format 23
/// keep.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BevelOptions {
    /// More of the bevel on one side of the edges: -1 to 1, 0 even.
    pub skew: f32,
    /// Swaps which side of the edges the skew favours.
    pub flip_side: bool,
    /// Every bevelled edge's new polygon as wide as on a right-angled
    /// edge, whatever angle its polygons meet at.
    pub even: bool,
    /// The bevel only as wide as fits, so it never crosses over itself.
    pub clamp: bool,
    /// New points within [`Self::weld_distance`] of each other welded.
    pub weld: bool,
    /// How close points must be to weld, in the mesh's units.
    pub weld_distance: f32,
    /// The improved bevel (decision 0141): rounded corner patches, and
    /// [`Self::clamp`] narrowing each point only as much as it needs.
    /// Off, as for bevels saved before file format 42, the corners are one
    /// polygon and clamping narrows the whole bevel.
    pub improved: bool,
}

impl Default for BevelOptions {
    /// Even, stopping at overlaps and welding (the project lead's choice
    /// for welding), with no skew, and improved (decision 0141).
    fn default() -> Self {
        Self {
            skew: 0.0,
            flip_side: false,
            even: true,
            clamp: true,
            weld: true,
            weld_distance: WELD_DISTANCE,
            improved: true,
        }
    }
}

impl BevelOptions {
    /// How every bevel worked before decision 0085: none of the options.
    pub const EARLIER: Self = Self {
        skew: 0.0,
        flip_side: false,
        even: false,
        clamp: false,
        weld: false,
        weld_distance: WELD_DISTANCE,
        improved: false,
    };

    /// The options in range: skew -1 to 1, weld distance 0 to
    /// [`MAX_WELD_DISTANCE`]; values that aren't numbers as standard.
    #[must_use]
    pub fn clamped(self) -> Self {
        let real = |x: f32, or: f32| if x.is_finite() { x } else { or };
        Self {
            skew: real(self.skew, 0.0).clamp(-1.0, 1.0),
            weld_distance: real(self.weld_distance, WELD_DISTANCE).clamp(0.0, MAX_WELD_DISTANCE),
            ..self
        }
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
/// by `profile`: 1 bulges out towards `corner` (round), 0 is a straight
/// line (flat), -1 hollows in the same amount the other way (a cove).
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

/// [`without_repeats`] for corners that carry where their values come
/// from (the first of each run of repeats is kept).
fn without_repeated_corners(
    mut list: Vec<(VertexId, CornerFrom)>,
    closed: bool,
) -> Vec<(VertexId, CornerFrom)> {
    list.dedup_by_key(|c| c.0);
    while closed && list.len() > 1 && list.first().map(|c| c.0) == list.last().map(|c| c.0) {
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
    bevel_with_weights(mesh, edges, &BTreeMap::new(), shape, false)
}

/// Bevels each edge of `edges` with `shape`, its width times the edge's
/// weight (0 to 1; brought within it): the Live bevel modifier's marked
/// edges (decision 0149). An edge whose weight is 0 or not a number is
/// skipped. Otherwise as [`bevel_edges`], and plain edges fanned between
/// bevelled ones at a corner slide too (decision 0156; the Bevel tool's
/// recorded bevels keep how they were made, so saved shapes don't change).
pub fn bevel_edges_weighted(
    mesh: &mut Mesh,
    edges: &[(EdgeKey, f32)],
    shape: BevelShape,
) -> Outcome {
    let mut weights = BTreeMap::new();
    let mut skipped = 0;
    for (edge, weight) in edges {
        let weight = crate::edge_weights::usable(*weight);
        if weight > 0.0 {
            weights.insert(*edge, weight);
        } else {
            skipped += 1;
        }
    }
    let kept: Vec<EdgeKey> = weights.keys().copied().collect();
    let mut outcome = bevel_with_weights(mesh, &kept, &weights, shape, true);
    outcome.skipped += skipped;
    outcome
}

/// [`bevel_edges`], each edge in `weights` that share of the width; with
/// `slide_fans`, plain fans at corners slide too (`fans::slide_plain_fans`).
fn bevel_with_weights(
    mesh: &mut Mesh,
    edges: &[EdgeKey],
    weights: &BTreeMap<EdgeKey, f32>,
    shape: BevelShape,
    slide_fans: bool,
) -> Outcome {
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
    // The points at the ends of the edges left to bevel (looked up once,
    // not by scanning every edge for every point).
    let ends: BTreeSet<VertexId> = chosen
        .iter()
        .flat_map(|e| {
            let (a, b) = e.ends();
            [a, b]
        })
        .collect();
    let fans: BTreeMap<VertexId, (Vec<Wedge>, bool)> = fans
        .into_iter()
        .filter_map(|(v, f)| ends.contains(&v).then_some((v, f?)))
        .collect();
    let mut out = original.clone();
    let mut build = Builder::new(&original, &mut out);
    let bevelled = |a: VertexId, b: VertexId| chosen.contains(&EdgeKey::new(a, b));

    // 1. Where each polygon corner at a bevelled point goes, each edge's
    // bevel as wide as its options make it on each side; stopping at
    // overlaps, all of it narrowed until it fits.
    let options = shape.options;
    let sides = chain_sides(topology, &chosen, options.flip_side);
    let edge_width = |edge: EdgeKey, face: FaceId| -> f32 {
        let mut width = shape.width * weights.get(&edge).copied().unwrap_or(1.0);
        if options.even {
            width *= even_factor(&original, topology, edge);
        }
        if options.skew != 0.0 {
            let lean = options.skew * SKEW_REACH;
            width *= if sides.get(&edge) == Some(&face) {
                1.0 + lean
            } else {
                1.0 - lean
            };
        }
        width
    };
    // `scales` narrows the bevel at each point (none: full width).
    let plan = |scales: &BTreeMap<VertexId, f32>| {
        let mut spots: BTreeMap<VertexId, Vec<Vec<Spot>>> = BTreeMap::new();
        let mut slides: BTreeMap<VertexId, Slides> = BTreeMap::new();
        // How much narrower the bevel must be at each point to fit (1, or
        // not listed: it fits).
        let mut fits: BTreeMap<VertexId, f32> = BTreeMap::new();
        let mut must_fit = |along: f32, from: VertexId, far: VertexId| {
            let length = original
                .position(from)
                .zip(original.position(far))
                .map_or(0.0, |(a, b)| a.distance(b));
            let share = if fans.contains_key(&far) {
                0.5
            } else {
                MAX_SLIDE
            };
            if along.abs() > 1e-9 && along.is_finite() {
                let fit = fits.entry(from).or_insert(1.0);
                *fit = fit.min(length * share / along.abs());
            }
        };
        for (v, (wedges, closed)) in &fans {
            let pv = original.position(*v).unwrap_or_default();
            let scale = scales.get(v).copied().unwrap_or(1.0);
            let mut asks = Slides::default();
            let mut per_wedge = Vec::new();
            for w in wedges {
                let normal = original.face_normal(w.face);
                let d_in = towards(&original, *v, w.before);
                let d_out = towards(&original, *v, w.after);
                let perp_in = d_in.cross(normal).normalize_or_zero();
                let perp_out = normal.cross(d_out).normalize_or_zero();
                let w_in = edge_width(EdgeKey::new(w.before, *v), w.face) * scale;
                let w_out = edge_width(EdgeKey::new(*v, w.after), w.face) * scale;
                let spot = match (bevelled(w.before, *v), bevelled(*v, w.after)) {
                    (true, true) => {
                        let sin = d_in.dot(perp_out);
                        let offset = if sin.abs() < MIN_SIN {
                            (perp_in + perp_out).normalize_or_zero() * (w_in + w_out) * 0.5
                        } else {
                            let along_in = over_sin(w_out, sin);
                            let along_out = over_sin(w_in, d_out.dot(perp_in));
                            must_fit(along_in, *v, w.before);
                            must_fit(along_out, *v, w.after);
                            d_in * along_in + d_out * along_out
                        };
                        vec![Spot::At(pv + offset)]
                    }
                    (true, false) => {
                        let along = slide_by(w_in, d_out.dot(perp_in));
                        must_fit(along, *v, w.after);
                        asks.ask(w.after, along);
                        vec![Spot::Slide(w.after)]
                    }
                    (false, true) => {
                        let along = slide_by(w_out, d_in.dot(perp_out));
                        must_fit(along, *v, w.before);
                        asks.ask(w.before, along);
                        vec![Spot::Slide(w.before)]
                    }
                    (false, false) => {
                        vec![Spot::Slide(w.before), Spot::Stay, Spot::Slide(w.after)]
                    }
                };
                per_wedge.push(spot);
            }
            // A point on an open border keeps its place (the border runs
            // on through it).
            if slide_fans && *closed {
                fans::slide_plain_fans(
                    &original,
                    *v,
                    wedges,
                    &bevelled,
                    |edge, face| edge_width(edge, face) * scale,
                    &mut asks,
                    &mut must_fit,
                );
            }
            // A plain wedge keeps only the slides that happen, and the
            // point itself unless slides on both sides replace it.
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
        (spots, slides, fits)
    };
    let (mut spots, mut slides, fits) = plan(&BTreeMap::new());
    if options.clamp && options.improved {
        // Each point narrowed as much as it needs (decision 0141).
        let scales = fits
            .into_iter()
            .filter(|(_, fit)| *fit < 1.0)
            .map(|(v, fit)| (v, fit.max(0.0)))
            .collect::<BTreeMap<_, _>>();
        if !scales.is_empty() {
            (spots, slides, _) = plan(&scales);
        }
    } else if options.clamp {
        // The whole bevel narrowed by the tightest point (decision 0085).
        let fits = fits.values().fold(1.0_f32, |a, b| a.min(*b));
        if fits < 1.0 {
            let all = fans.keys().map(|v| (*v, fits.max(0.0))).collect();
            (spots, slides, _) = plan(&all);
        }
    }

    // 2. New points: meeting corners, then slides, point by point.
    let mut corners: BTreeMap<VertexId, Corner> = BTreeMap::new();
    let mut slid: BTreeMap<(VertexId, VertexId), VertexId> = BTreeMap::new();
    for (v, (wedges, closed)) in &fans {
        let pv = original.position(*v).unwrap_or_default();
        let mut made: Vec<Option<VertexId>> = Vec::new();
        for spot in spots[v].iter().flatten() {
            made.push(match spot {
                Spot::At(p) => {
                    Some(build.point(if p.is_finite() { *p } else { pv }, PointFrom::Copy(*v)))
                }
                _ => None,
            });
        }
        for (far, (sum, count)) in &slides[v].0 {
            let pf = original.position(*far).unwrap_or(pv);
            let length = pf.distance(pv);
            let along = (sum / *count as f32).min(length * MAX_SLIDE);
            let p = pv + (pf - pv).normalize_or_zero() * along;
            let t = if length > 0.0 { along / length } else { 0.0 };
            slid.insert(
                (*v, *far),
                build.point(
                    if p.is_finite() { p } else { pv },
                    PointFrom::Between(*v, *far, t),
                ),
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
    // Each strip's forward and backward polygon: its rows run from the
    // first's side to the second's.
    let mut between: BTreeMap<EdgeKey, (FaceId, FaceId)> = BTreeMap::new();
    // Which row runs between which two points round a bevelled point:
    // (point, first, last) -> the edge whose row it is (the lowest, when
    // several do). Finds shared end rows without searching every row.
    let mut row_ends: HashMap<(VertexId, VertexId, VertexId), EdgeKey> = HashMap::new();
    for edge in &chosen {
        let uses = topology.edge_uses(*edge);
        let forward = uses.iter().find(|u| u.forward).map(|u| u.face);
        let backward = uses.iter().find(|u| !u.forward).map(|u| u.face);
        let (Some(forward), Some(backward)) = (forward, backward) else {
            continue;
        };
        between.insert(*edge, (forward, backward));
        let (lo, hi) = edge.ends();
        for (x, other) in [(lo, hi), (hi, lo)] {
            let corner = &corners[&x];
            let (Some(a), Some(b)) = (side(corner, forward, other), side(corner, backward, other))
            else {
                continue;
            };
            let (pa, pb) = (
                build.mesh().position(a).unwrap_or_default(),
                build.mesh().position(b).unwrap_or_default(),
            );
            let px = original.position(x).unwrap_or_default();
            // Two strips ending between the same two points (round a
            // cube's corner, say) share their end rows.
            let reversed = row_ends.get(&(x, b, a)).copied();
            let same = row_ends.get(&(x, a, b)).copied();
            let shared = match (reversed, same) {
                (Some(r), Some(s)) if s < r => rows.get(&(s, x)).cloned(),
                (Some(r), _) => rows
                    .get(&(r, x))
                    .map(|row| row.iter().rev().copied().collect()),
                (None, Some(s)) => rows.get(&(s, x)).cloned(),
                (None, None) => None,
            };
            let row = shared.unwrap_or_else(|| {
                let mut row = vec![a];
                for k in 1..shape.segments {
                    let t = k as f32 / shape.segments as f32;
                    let p = profile_point(pa, pb, px, shape.profile, t);
                    row.push(build.point(
                        if p.is_finite() { p } else { px },
                        PointFrom::Between(a, b, t),
                    ));
                }
                row.push(b);
                row
            });
            if let (Some(first), Some(last)) = (row.first(), row.last()) {
                let key = (x, *first, *last);
                let lowest = row_ends.get(&key).map_or(*edge, |e| (*e).min(*edge));
                row_ends.insert(key, lowest);
            }
            rows.insert((*edge, x), row);
        }
    }

    // 4. Holes where strips meet: one corner polygon, or, when only one
    // polygon borders the hole, its corner takes the strip's end. The
    // improved bevel rounds a hole whose every side is a strip's end.
    let mut patches = Vec::new();
    let mut rounded = Vec::new();
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
        } else if let Some(sides) = (options.improved && shape.segments > 1)
            .then(|| corner::round_sides(*v, &corner.wedges, &corner.lists, &rows, bevelled))
            .flatten()
        {
            rounded.push(corner::round_patch(
                &mut build,
                &original,
                *v,
                &corner.wedges,
                &corner.lists,
                &sides,
                shape.segments,
                shape.profile,
            ));
        } else {
            let mut patch = points;
            patch.reverse();
            patches.push((*v, patch));
        }
    }

    // 5. The polygons around bevelled points get their new corners.
    // Every new corner copies the polygon's corner it replaces.
    let mut changes: BTreeMap<FaceId, Vec<(VertexId, CornerFrom)>> = BTreeMap::new();
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
            let from = CornerFrom::Copy(face, *c);
            match corners.get(c) {
                Some(corner) => {
                    let i = corner.wedges.iter().position(|w| w.face == face);
                    match i {
                        Some(i) => new.extend(corner.lists[i].iter().map(|v| (*v, from))),
                        None => new.push((*c, from)),
                    }
                }
                None => new.push((*c, from)),
            }
        }
        changes.insert(face, without_repeated_corners(new, true));
    }
    if build.rewrite_faces(&changes).is_err() {
        return outcome;
    }

    // 6. The strips, then the corner polygons.
    for edge in &chosen {
        let (lo, hi) = edge.ends();
        let (Some(at_lo), Some(at_hi)) = (rows.get(&(*edge, lo)), rows.get(&(*edge, hi))) else {
            continue;
        };
        let Some(&(forward, backward)) = between.get(edge) else {
            continue;
        };
        // Row k's corners blend the two polygons' corners, k / segments
        // of the way from the forward polygon to the backward one. Across
        // a seam the two polygons lie in different pieces of the texture:
        // blending them stretched the strip across the whole texture (a
        // review found it, under Game bevel). There the strip, which joins
        // the forward polygon's piece, copies that polygon's corners, as
        // the forward polygon's own cut corners do: in the texture it is
        // as thin as a strip between two polygons of one piece.
        let seam = crate::uvs::is_seam(&original, *edge);
        let blend = |x: VertexId, k: usize| {
            if seam {
                CornerFrom::Copy(forward, x)
            } else {
                let t = k as f32 / shape.segments as f32;
                CornerFrom::Between(forward, x, backward, x, t)
            }
        };
        for k in 0..shape.segments as usize {
            let quad = vec![
                (at_hi[k], blend(hi, k)),
                (at_lo[k], blend(lo, k)),
                (at_lo[k + 1], blend(lo, k + 1)),
                (at_hi[k + 1], blend(hi, k + 1)),
            ];
            build.face(&without_repeated_corners(quad, true), Some(forward));
        }
    }
    for (v, patch) in &patches {
        // A corner polygon is like the first polygon round the point, and
        // its corners copy that polygon's corner. Without seams round the
        // point every polygon's corner there is the same in the texture;
        // with them, taking each corner from its own polygon mixed pieces
        // and stretched the corner across the texture (second review).
        let corner = &corners[v];
        let Some(first) = corner.wedges.first().map(|w| w.face) else {
            continue;
        };
        let with: Vec<(VertexId, CornerFrom)> = patch
            .iter()
            .map(|p| (*p, CornerFrom::Copy(first, *v)))
            .collect();
        build.face(&with, Some(first));
    }
    for (corners, like) in rounded.iter().flatten() {
        build.face(corners, Some(*like));
    }

    // 7. Hard edges: a hard bevelled edge's strip keeps hard borders (and
    // with one row, hard ends and corner polygons); plain edges that slid
    // keep their hardness.
    let mut seams = Vec::new();
    for edge in &chosen {
        let (lo, hi) = edge.ends();
        let hard = original.is_hard(lo, hi);
        let seam = crate::uvs::is_seam(&original, *edge);
        // The bevelled edge is gone, and its values with it.
        build.clear_edge(*edge);
        // A seam moves to the strip's far border, so the pieces on either
        // side stay apart and the strip joins the first one's
        // (decision 0104).
        if seam && let (Some(a), Some(b)) = (rows.get(&(*edge, lo)), rows.get(&(*edge, hi))) {
            let n = a.len() - 1;
            seams.push(EdgeKey::new(a[n], b[n]));
        }
        if !hard {
            continue;
        }
        if let (Some(a), Some(b)) = (rows.get(&(*edge, lo)), rows.get(&(*edge, hi))) {
            let n = a.len() - 1;
            let out = build.mesh();
            out.set_hard(a[0], b[0], true);
            out.set_hard(a[n], b[n], true);
            if shape.segments == 1 {
                out.set_hard(a[0], a[1], true);
                out.set_hard(b[0], b[1], true);
            }
        }
    }
    // All at once: marking seams works out the shape's connections.
    if !seams.is_empty() {
        crate::uvs::set_seams(build.mesh(), &seams, true);
    }
    if shape.segments == 1 {
        for (v, patch) in &patches {
            let hard = corners[v]
                .wedges
                .iter()
                .any(|w| bevelled(*v, w.after) && original.is_hard(*v, w.after));
            if hard {
                for i in 0..patch.len() {
                    build
                        .mesh()
                        .set_hard(patch[i], patch[(i + 1) % patch.len()], true);
                }
            }
        }
    }
    // Plain edges that slid take their values (hard, seam) with them.
    for ((v, far), s) in &slid {
        let other = slid.get(&(*far, *v)).copied().unwrap_or(*far);
        build.move_edge(EdgeKey::new(*v, *far), *s, other);
    }

    // 8. Points no polygon uses any more go.
    build
        .mesh()
        .remove_unused_vertices(&corners.keys().copied().collect::<Vec<_>>());
    // 9. New points on top of each other (where bevels meet) become one.
    if options.weld {
        let new: Vec<VertexId> = out
            .vertices()
            .map(|(v, _)| v)
            .filter(|v| original.position(*v).is_none())
            .collect();
        crate::join::weld_close(&mut out, &new, options.weld_distance);
    }
    *mesh = out;
    outcome
}

/// How much wider (or narrower) [`BevelOptions::even`] makes `edge`'s
/// bevel: its polygons' facings at an angle θ make its new polygon
/// `2 cos(θ / 2)` times as wide as the bevel reaches, so the reach is set
/// to give the new polygon a right-angled edge's width.
fn even_factor(mesh: &Mesh, topology: &Topology, edge: EdgeKey) -> f32 {
    let uses = topology.edge_uses(edge);
    let [a, b] = [0, 1].map(|i| uses.get(i).map(|u| mesh.face_normal(u.face)));
    let (Some(a), Some(b)) = (a, b) else {
        return 1.0;
    };
    let cos = a.dot(b).clamp(-1.0, 1.0);
    let half = ((1.0 + cos) * 0.5).sqrt();
    if !half.is_finite() {
        return 1.0;
    }
    std::f32::consts::FRAC_1_SQRT_2 / half.max(MIN_EVEN_COS)
}

/// The polygon on each bevelled edge's skewed side: along each chain of
/// edges (joined at their points) the edges are followed one way round,
/// and the side is the polygon that runs along each edge that way, so a
/// loop's skew stays on one side all the way round. `flip` takes the
/// other polygon.
fn chain_sides(
    topology: &Topology,
    chosen: &BTreeSet<EdgeKey>,
    flip: bool,
) -> BTreeMap<EdgeKey, FaceId> {
    let mut at: BTreeMap<VertexId, Vec<EdgeKey>> = BTreeMap::new();
    for edge in chosen {
        let (a, b) = edge.ends();
        at.entry(a).or_default().push(*edge);
        at.entry(b).or_default().push(*edge);
    }
    let mut way: BTreeMap<EdgeKey, (VertexId, VertexId)> = BTreeMap::new();
    for start in chosen {
        if way.contains_key(start) {
            continue;
        }
        let mut queue = std::collections::VecDeque::from([(*start, start.ends())]);
        while let Some((edge, (from, to))) = queue.pop_front() {
            if way.contains_key(&edge) {
                continue;
            }
            way.insert(edge, (from, to));
            // Carry on from where this edge ends, and back from where it
            // starts.
            for (point, onward) in [(to, true), (from, false)] {
                for next in at.get(&point).into_iter().flatten() {
                    if way.contains_key(next) {
                        continue;
                    }
                    let (a, b) = next.ends();
                    let other = if a == point { b } else { a };
                    queue.push_back((
                        *next,
                        if onward {
                            (point, other)
                        } else {
                            (other, point)
                        },
                    ));
                }
            }
        }
    }
    way.into_iter()
        .filter_map(|(edge, (from, _))| {
            let forward = from == edge.ends().0;
            let uses = topology.edge_uses(edge);
            let face = uses.iter().find(|u| u.forward == (forward != flip))?.face;
            Some((edge, face))
        })
        .collect()
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
    let mut build = Builder::new(&original, &mut out);
    let mut cut: BTreeMap<(VertexId, VertexId), VertexId> = BTreeMap::new();
    // How far along each edge its cut is, for the corners' values.
    let mut share: BTreeMap<(VertexId, VertexId), f32> = BTreeMap::new();
    for (v, (wedges, _)) in &fans {
        let pv = original.position(*v).unwrap_or_default();
        let neighbours: BTreeSet<VertexId> =
            wedges.iter().flat_map(|w| [w.before, w.after]).collect();
        for far in neighbours {
            let pf = original.position(far).unwrap_or(pv);
            let length = pf.distance(pv);
            let along = width.min(length * MAX_SLIDE);
            let p = pv + (pf - pv).normalize_or_zero() * along;
            let t = if length > 0.0 { along / length } else { 0.0 };
            share.insert((*v, far), if t.is_finite() { t } else { 0.0 });
            cut.insert(
                (*v, far),
                build.point(
                    if p.is_finite() { p } else { pv },
                    PointFrom::Between(*v, far, t),
                ),
            );
        }
    }
    let mut changes: BTreeMap<FaceId, Vec<(VertexId, CornerFrom)>> = BTreeMap::new();
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
            let from = CornerFrom::Copy(face, *c);
            if fans.contains_key(c) {
                let before = old[(i + n - 1) % n];
                let after = old[(i + 1) % n];
                // The corners blend along their edges (UVs slide with the
                // cut, so the polygon's texture isn't squashed).
                let along = |far: VertexId| {
                    CornerFrom::Between(
                        face,
                        *c,
                        face,
                        far,
                        share.get(&(*c, far)).copied().unwrap_or(0.0),
                    )
                };
                new.push((cut[&(*c, before)], along(before)));
                new.push((cut[&(*c, after)], along(after)));
            } else {
                new.push((*c, from));
            }
        }
        changes.insert(face, without_repeated_corners(new, true));
    }
    if build.rewrite_faces(&changes).is_err() {
        return outcome;
    }
    for (v, (wedges, closed)) in &fans {
        let mut ring: Vec<(VertexId, CornerFrom)> = Vec::new();
        for w in wedges {
            let from = CornerFrom::Copy(w.face, *v);
            ring.push((cut[&(*v, w.before)], from));
            ring.push((cut[&(*v, w.after)], from));
        }
        let ring = without_repeated_corners(ring, *closed);
        if ring.len() >= 3 {
            let mut patch = ring;
            patch.reverse();
            build.face(&patch, wedges.first().map(|w| w.face));
        }
        // Edges keep their values (hard, seam) up to the cut.
        let mine = (*v, VertexId::from_raw(0))..=(*v, VertexId::from_raw(u32::MAX));
        for ((_, far), p) in cut.range(mine) {
            let other = cut.get(&(*far, *v)).copied().unwrap_or(*far);
            build.copy_edge(EdgeKey::new(*v, *far), *p, other);
        }
    }
    build
        .mesh()
        .remove_unused_vertices(&fans.keys().copied().collect::<Vec<_>>());
    *mesh = out;
    outcome
}

mod corner;
mod fans;

#[cfg(test)]
mod corner_tests;
#[cfg(test)]
mod tests;
