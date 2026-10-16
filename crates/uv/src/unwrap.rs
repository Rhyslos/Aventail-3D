//! Laying a surface flat: unwrapping, straightening and relaxing
//! ([decision 0099](../../../docs/decisions/0099-unwrap-relax-straighten.md)).
//!
//! **Unwrapping** cuts the shape along its seams into charts and lays each
//! flat with least squares conformal maps (LSCM, Lévy et al. 2002): every
//! triangle keeps its angles as well as it can, so a texture's squares
//! stay square. A chart that can't lie flat in one go (a tube, a closed
//! ball) is first cut open in a hidden place (see [`crate::seams`]), and
//! the cut is marked as a seam. Charts made only of rectangular quads
//! joined as a grid (a cylinder's side, a bevel's strip) are laid out as a
//! perfectly straight grid instead, the way hard-surface work wants them.
//!
//! Each new chart is then turned so its box is as small as can be (its
//! edges lined up with the square), turned a quarter at a time to match
//! the way it lay before, scaled to the object's texel density (so pieces
//! keep their real sizes against each other) and put where it was.
//!
//! **Straightening** lays quad charts out as a straight grid. **Relaxing**
//! evens out the UVs of the chosen polygons, pulling each side towards its
//! real length.

use crate::chart::Chart;
use crate::methods::{UnwrapMethod, conformal_shapes, lay_flat, real_shapes};
use crate::seams::HideSeams;
use glam::{Vec2, Vec3};
use mesh::edit::Outcome;
use mesh::uvs::Uvs;
use mesh::{EdgeKey, FaceId, Mesh};
use std::collections::{BTreeSet, HashMap, VecDeque};

/// Charts with more wedges than this aren't solved but projected flat on
/// their main plane: solving them would take too long (a chart of 33,000
/// wedges takes about 4 seconds; cut big shapes into more pieces).
pub const SOLVE_UP_TO: usize = 60_000;

/// How far (degrees) a quad's corners may be from square for its chart to
/// count as a strip and be straightened when unwrapping: the standard for
/// [`UnwrapOptions::strip_angle`].
pub const SQUARE_WITHIN: f32 = 25.0;

/// The most a quad's corners may be from square, in degrees, for any
/// choice of [`UnwrapOptions::strip_angle`].
pub const MAX_STRIP_ANGLE: f32 = 60.0;

/// How many steps relaxing takes: the standard.
pub const RELAX_STEPS: usize = 60;

/// The most steps relaxing may be asked to take.
pub const MAX_RELAX_STEPS: u32 = 1_000;

/// What a chart is cut along: the seams, or, on a shape with no seam marks
/// at all (imported, say), where its UVs split.
pub fn cuts_of(mesh: &Mesh) -> BTreeSet<EdgeKey> {
    if mesh.attribute_values(mesh::attributes::SEAM).is_some() {
        mesh::uvs::seams(mesh).into_iter().collect()
    } else {
        mesh::uvs::uv_splits(mesh).into_iter().collect()
    }
}

/// The 3D area of triangle `t` of `chart`.
fn area_3d(mesh: &Mesh, chart: &Chart, t: [usize; 3]) -> f32 {
    let p = |w: usize| mesh.position(chart.points[w]).unwrap_or(Vec3::ZERO);
    (p(t[1]) - p(t[0])).cross(p(t[2]) - p(t[0])).length() / 2.0
}

pub(crate) fn signed_area(at: &[Vec2], t: [usize; 3]) -> f32 {
    (at[t[1]] - at[t[0]]).perp_dot(at[t[2]] - at[t[0]]) / 2.0
}

/// Two wedges far apart, to pin while solving.
pub(crate) fn far_pair(mesh: &Mesh, chart: &Chart) -> Option<(usize, usize)> {
    let p = |w: usize| mesh.position(chart.points[w]).unwrap_or(Vec3::ZERO);
    let farthest = |from: usize| {
        (0..chart.len()).max_by(|a, b| {
            p(*a)
                .distance_squared(p(from))
                .total_cmp(&p(*b).distance_squared(p(from)))
        })
    };
    let a = farthest(0)?;
    let b = farthest(a)?;
    (a != b && p(a).distance(p(b)) > 1e-9).then_some((a, b))
}

/// The chart laid flat by least squares conformal maps, if it can be.
/// `guess` (a rough layout, such as the old UVs) only speeds it up.
pub(crate) fn conformal(mesh: &Mesh, chart: &Chart, guess: &[Vec2]) -> Option<Vec<Vec2>> {
    let none = vec![None; chart.len()];
    lay_flat(mesh, chart, UnwrapMethod::Conformal, &none, guess).map(|(at, _)| at)
}

/// The chart laid flat by least squares conformal maps with the wedges
/// that have a spot in `pins` held there (at least two, apart), if it can
/// be. `start` (one spot per wedge, or empty) is where solving starts.
pub(crate) fn conformal_pinned(
    mesh: &Mesh,
    chart: &Chart,
    pins: &[Option<Vec2>],
    start: &[Vec2],
) -> Option<Vec<Vec2>> {
    let n = chart.len();
    if !(3..=SOLVE_UP_TO).contains(&n) {
        return None;
    }
    let (triangles, shapes) = real_shapes(mesh, chart);
    conformal_shapes(n, &triangles, &shapes, pins, start)
}

/// The chart projected flat on the plane it faces most: the fallback when
/// it can't be solved.
pub(crate) fn projected(mesh: &Mesh, chart: &Chart) -> Vec<Vec2> {
    let facing: Vec3 = chart.faces.iter().map(|f| mesh.face_normal(*f)).sum();
    let normal = facing.try_normalize().unwrap_or(Vec3::Z);
    let x = normal.any_orthonormal_vector();
    let y = normal.cross(x);
    chart
        .points
        .iter()
        .map(|v| {
            let p = mesh.position(*v).unwrap_or(Vec3::ZERO);
            Vec2::new(p.dot(x), p.dot(y))
        })
        .collect()
}

/// Whether every polygon of the chart is a quad with corners within
/// `within` degrees of square.
fn all_square_quads(mesh: &Mesh, chart: &Chart, within: f32) -> bool {
    let limit = within.clamp(0.0, MAX_STRIP_ANGLE).to_radians().sin();
    chart.faces.iter().all(|f| {
        let Some(list) = mesh.face(*f) else {
            return false;
        };
        list.len() == 4
            && (0..4).all(|k| {
                let p = |i: usize| mesh.position(list[i % 4]).unwrap_or(Vec3::ZERO);
                let a = (p(k + 3) - p(k)).normalize_or_zero();
                let b = (p(k + 1) - p(k)).normalize_or_zero();
                a.dot(b).abs() <= limit
            })
    })
}

/// The chart's quads laid out as a straight grid, following each quad
/// from its neighbour (each side as long as it is on the shape), if every
/// polygon is a quad and every one is reached.
pub(crate) fn grid(mesh: &Mesh, chart: &Chart) -> Option<Vec<Vec2>> {
    if chart.corners.iter().any(|c| c.len() != 4) || chart.faces.is_empty() {
        return None;
    }
    let p = |w: usize| mesh.position(chart.points[w]).unwrap_or(Vec3::ZERO);
    let length = |a: usize, b: usize| p(a).distance(p(b));
    // Each side, by its wedges, and the quads using it.
    let mut by_side: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
    for (i, c) in chart.corners.iter().enumerate() {
        for k in 0..4 {
            let (a, b) = (c[k], c[(k + 1) % 4]);
            by_side.entry((a.min(b), a.max(b))).or_default().push(i);
        }
    }
    let area = |i: usize| {
        let c = &chart.corners[i];
        (p(c[1]) - p(c[0])).cross(p(c[3]) - p(c[0])).length()
    };
    let seed = (0..chart.faces.len()).max_by(|a, b| area(*a).total_cmp(&area(*b)))?;
    let mut at: Vec<Option<Vec2>> = vec![None; chart.len()];
    let c = &chart.corners[seed];
    let w = (length(c[0], c[1]) + length(c[2], c[3])) / 2.0;
    let h = (length(c[1], c[2]) + length(c[3], c[0])) / 2.0;
    for (k, spot) in [
        (0, Vec2::ZERO),
        (1, Vec2::new(w, 0.0)),
        (2, Vec2::new(w, h)),
        (3, Vec2::new(0.0, h)),
    ] {
        at[c[k]] = Some(spot);
    }
    let mut done = vec![false; chart.faces.len()];
    done[seed] = true;
    let mut queue = VecDeque::from([seed]);
    while let Some(i) = queue.pop_front() {
        let c = chart.corners[i].clone();
        for k in 0..4 {
            let (a, b) = (c[k], c[(k + 1) % 4]);
            for &j in by_side
                .get(&(a.min(b), a.max(b)))
                .map(Vec::as_slice)
                .unwrap_or(&[])
            {
                if done[j] {
                    continue;
                }
                let d = &chart.corners[j];
                // The neighbour runs along the side the other way: b, a, then
                // its far side.
                let Some(m) = (0..4).find(|m| d[*m] == b && d[(m + 1) % 4] == a) else {
                    continue;
                };
                let (far_a, far_b) = (d[(m + 2) % 4], d[(m + 3) % 4]);
                let (Some(pa), Some(pb)) = (at[a], at[b]) else {
                    continue;
                };
                let along = pa - pb;
                let Some(out) = along.perp().try_normalize() else {
                    continue;
                };
                let reach = (length(a, far_a) + length(far_b, b)) / 2.0;
                if at[far_a].is_none() {
                    at[far_a] = Some(pa + out * reach);
                }
                if at[far_b].is_none() {
                    at[far_b] = Some(pb + out * reach);
                }
                done[j] = true;
                queue.push_back(j);
            }
        }
    }
    let out: Option<Vec<Vec2>> = at.into_iter().collect();
    let out = out?;
    // Every quad must come out the right way round.
    chart
        .corners
        .iter()
        .all(|c| {
            signed_area(&out, [c[0], c[1], c[2]]) > 0.0
                && signed_area(&out, [c[0], c[2], c[3]]) > 0.0
        })
        .then_some(out)
}

/// Whether the chart's old UVs make a fair starting guess: none of its
/// triangles squashed flat or folded over.
pub(crate) fn usable_guess(mesh: &Mesh, chart: &Chart, uvs: &Uvs) -> bool {
    let old: Vec<Vec2> = old_spots(mesh, chart, uvs)
        .into_iter()
        .map(|v| v.unwrap_or(Vec2::ZERO))
        .collect();
    chart
        .triangles(mesh)
        .iter()
        .all(|t| signed_area(&old, *t) > 1e-12)
}

/// The convex hull of `points` (counter-clockwise).
fn hull(points: &[Vec2]) -> Vec<Vec2> {
    let mut p: Vec<Vec2> = points.to_vec();
    p.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    p.dedup_by(|a, b| a.distance_squared(*b) < 1e-14);
    if p.len() < 3 {
        return p;
    }
    let mut lower: Vec<Vec2> = Vec::new();
    for q in &p {
        while lower.len() >= 2
            && (lower[lower.len() - 1] - lower[lower.len() - 2])
                .perp_dot(*q - lower[lower.len() - 2])
                <= 0.0
        {
            lower.pop();
        }
        lower.push(*q);
    }
    let mut upper: Vec<Vec2> = Vec::new();
    for q in p.iter().rev() {
        while upper.len() >= 2
            && (upper[upper.len() - 1] - upper[upper.len() - 2])
                .perp_dot(*q - upper[upper.len() - 2])
                <= 0.0
        {
            upper.pop();
        }
        upper.push(*q);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

/// The turn that makes the box around `points` smallest (one of its
/// convex hull's sides lying flat).
pub(crate) fn smallest_box_turn(points: &[Vec2]) -> f32 {
    let h = hull(points);
    if h.len() < 3 {
        return 0.0;
    }
    let mut best = (f32::INFINITY, 0.0);
    for k in 0..h.len() {
        let side = h[(k + 1) % h.len()] - h[k];
        if side.length_squared() < 1e-14 {
            continue;
        }
        let angle = -side.y.atan2(side.x);
        let (s, c) = angle.sin_cos();
        let turned = |v: Vec2| Vec2::new(c * v.x - s * v.y, s * v.x + c * v.y);
        let mut min = Vec2::splat(f32::INFINITY);
        let mut max = Vec2::splat(f32::NEG_INFINITY);
        for q in &h {
            let t = turned(*q);
            min = min.min(t);
            max = max.max(t);
        }
        let size = max - min;
        let area = size.x * size.y;
        // Ties (a square) keep the smaller turn.
        if area < best.0 * (1.0 - 1e-4) {
            best = (area, angle);
        }
    }
    best.1
}

/// How a chart lay before it was laid out again.
struct Before {
    /// The middle of the box around its old UVs.
    centre: Option<Vec2>,
    /// The way its longest side ran in the texture, and the wedges of that
    /// side.
    side: Option<((usize, usize), Vec2)>,
}

/// The chart's old UVs: one per wedge (the first corner's), if the shape
/// had UVs.
pub(crate) fn old_spots(mesh: &Mesh, chart: &Chart, uvs: &Uvs) -> Vec<Option<Vec2>> {
    let (starts, _) = mesh.face_layout();
    let mut out = vec![None; chart.len()];
    for (f, wedges) in chart.faces.iter().zip(&chart.corners) {
        let start = starts[f.get() as usize] as usize;
        for (k, w) in wedges.iter().enumerate() {
            if out[*w].is_none() {
                out[*w] = Some(uvs.at(start + k));
            }
        }
    }
    out
}

fn before(mesh: &Mesh, chart: &Chart, uvs: Option<&Uvs>) -> Before {
    let Some(uvs) = uvs else {
        return Before {
            centre: None,
            side: None,
        };
    };
    let old = old_spots(mesh, chart, uvs);
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);
    for v in old.iter().flatten() {
        min = min.min(*v);
        max = max.max(*v);
    }
    let centre = min.is_finite().then(|| (min + max) / 2.0);
    let p = |w: usize| mesh.position(chart.points[w]).unwrap_or(Vec3::ZERO);
    let side = chart
        .sides()
        .into_iter()
        .max_by(|x, y| p(x.0).distance(p(x.1)).total_cmp(&p(y.0).distance(p(y.1))))
        .and_then(|(a, b)| {
            let way = old[b]? - old[a]?;
            (way.length() > 1e-7).then_some(((a, b), way))
        });
    Before { centre, side }
}

/// Puts a new layout of the chart in place: the right way round, its box
/// as small as can be (when `align`), a quarter turn to match how it lay,
/// at `density` (UV area per 3D area) and where it was.
fn place(
    mesh: &Mesh,
    chart: &Chart,
    mut at: Vec<Vec2>,
    was: &Before,
    density: f32,
    align: bool,
) -> Vec<Vec2> {
    let triangles = chart.triangles(mesh);
    let signed: f32 = triangles.iter().map(|t| signed_area(&at, *t)).sum();
    if signed < 0.0 {
        for v in &mut at {
            v.x = -v.x;
        }
    }
    if align {
        let angle = smallest_box_turn(&at);
        let (s, c) = angle.sin_cos();
        for v in &mut at {
            *v = Vec2::new(c * v.x - s * v.y, s * v.x + c * v.y);
        }
    }
    // The quarter turn closest to how its longest side ran before.
    if let Some(((a, b), way)) = was.side {
        let now = at[b] - at[a];
        let quarter = |v: Vec2, k: usize| (0..k).fold(v, |v, _| v.perp());
        if let Some(k) = (0..4).max_by(|i, j| {
            quarter(now, *i)
                .normalize_or_zero()
                .dot(way.normalize_or_zero())
                .total_cmp(
                    &quarter(now, *j)
                        .normalize_or_zero()
                        .dot(way.normalize_or_zero()),
                )
        }) {
            for v in &mut at {
                *v = quarter(*v, k);
            }
        }
    }
    let area_3d: f32 = triangles.iter().map(|t| area_3d(mesh, chart, *t)).sum();
    let area_uv: f32 = triangles.iter().map(|t| signed_area(&at, *t).abs()).sum();
    if area_uv > 1e-20 && area_3d > 1e-20 && density > 0.0 {
        let k = (density * area_3d / area_uv).sqrt();
        for v in &mut at {
            *v *= k;
        }
    }
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);
    for v in &at {
        min = min.min(*v);
        max = max.max(*v);
    }
    let to = was.centre.unwrap_or(Vec2::splat(0.5));
    let shift = to - (min + max) / 2.0;
    for v in &mut at {
        *v += shift;
    }
    at
}

/// The UV area per 3D area of `faces` (by their current UVs), if they
/// have any.
fn density_of(mesh: &Mesh, faces: &[FaceId], uvs: &Uvs) -> Option<f32> {
    let (flat, real) = crate::density::areas(mesh, faces, uvs);
    (flat > 1e-12 && real > 1e-12).then(|| flat / real)
}

fn total_3d(mesh: &Mesh, faces: &[FaceId]) -> f32 {
    faces
        .iter()
        .filter_map(|f| mesh.face(*f))
        .map(|list| {
            let mut sum = 0.0;
            mesh::triangulate::face_triangles(mesh, list, |[a, b, c]| {
                let p = |i: usize| mesh.position(list[i]).unwrap_or(Vec3::ZERO);
                sum += (p(b) - p(a)).cross(p(c) - p(a)).length() / 2.0;
            });
            sum
        })
        .sum()
}

/// The pieces (by the shape's cuts) holding any of `faces`, and how many
/// of `faces` the shape has.
pub(crate) fn chosen_pieces(
    mesh: &Mesh,
    faces: &[FaceId],
    cuts: &BTreeSet<EdgeKey>,
) -> (Vec<Vec<FaceId>>, usize) {
    let wanted: BTreeSet<FaceId> = faces
        .iter()
        .copied()
        .filter(|f| mesh.face(*f).is_some())
        .collect();
    let pieces = crate::seams::pieces(mesh, cuts)
        .into_iter()
        .filter(|p| p.iter().any(|f| wanted.contains(f)))
        .collect();
    (pieces, wanted.len())
}

/// The density new layouts get: the other polygons' (so new pieces match
/// what's kept), else the chosen ones' before, else enough to fill about
/// 40% of the square.
fn density_for(mesh: &Mesh, chosen: &[FaceId], uvs: Option<&Uvs>) -> f32 {
    let inside: BTreeSet<FaceId> = chosen.iter().copied().collect();
    let others: Vec<FaceId> = mesh
        .faces()
        .map(|(f, _)| f)
        .filter(|f| !inside.contains(f))
        .collect();
    if let Some(uvs) = uvs {
        if let Some(d) = density_of(mesh, &others, uvs) {
            return d;
        }
        if let Some(d) = density_of(mesh, chosen, uvs) {
            return d;
        }
    }
    let area = total_3d(mesh, chosen);
    if area > 1e-12 { 0.4 / area } else { 1.0 }
}

/// Writes each chart's layout into the shape's UVs.
pub(crate) fn write(mesh: &mut Mesh, laid: &[(Chart, Vec<Vec2>)]) -> bool {
    let count = mesh.face_layout().1.len();
    let mut values = match Uvs::of(mesh) {
        Some(uvs) => uvs.into_values(count),
        None => vec![Vec2::ZERO; count],
    };
    let (starts, _) = mesh.face_layout();
    let starts = starts.to_vec();
    for (chart, at) in laid {
        for (f, wedges) in chart.faces.iter().zip(&chart.corners) {
            let start = starts[f.get() as usize] as usize;
            for (k, w) in wedges.iter().enumerate() {
                values[start + k] = at[*w];
            }
        }
    }
    mesh::uvs::set_uvs(mesh, values)
}

pub(crate) fn outcome(asked: usize, found: usize, changed: bool) -> Outcome {
    Outcome {
        matched: if changed { found } else { 0 },
        missing: asked - found,
        skipped: if changed { 0 } else { found },
    }
}

/// Which way an unwrap may move the UVs: both ways (the usual), or only
/// one, keeping the other as it was
/// ([decision 0109](../../../docs/decisions/0109-pins-and-unwrap-methods.md)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnfoldAxis {
    /// Only across (U) changes; every point keeps its V.
    Across,
    /// Only up (V) changes; every point keeps its U.
    Up,
}

/// How [`unwrap_with`] lays pieces flat. Every choice an unwrap makes is
/// here, so the same shape and options always give the same UVs
/// ([decision 0110](../../../docs/decisions/0110-controllable-automation.md)).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnwrapOptions {
    /// Strips of square quads come out as a straight grid.
    pub strips: bool,
    /// How far (degrees) a quad's corners may be from square for its piece
    /// to count as a strip (0 to [`MAX_STRIP_ANGLE`]).
    pub strip_angle: f32,
    /// The way pieces are laid flat.
    pub method: UnwrapMethod,
    /// Unfold one way only (`None`: both ways).
    pub only: Option<UnfoldAxis>,
    /// Where a piece that can't lie flat in one go is cut open.
    pub hide: HideSeams,
    /// The stretch budget for pieces without one of their own (`None`: no
    /// budget): pieces over it are cut further, and the cuts marked as
    /// seams ([decision 0111](../../../docs/decisions/0111-seam-preferences-and-stretch-budget.md)).
    pub budget: Option<f32>,
}

impl Default for UnwrapOptions {
    fn default() -> Self {
        Self {
            strips: false,
            strip_angle: SQUARE_WITHIN,
            method: UnwrapMethod::Conformal,
            only: None,
            hide: HideSeams::BelowAndBehind,
            budget: None,
        }
    }
}

impl UnwrapOptions {
    /// The usual unwrap: shapes kept (LSCM), both ways, strips as grids
    /// when `strips`, everything else as standard.
    pub fn plain(strips: bool) -> Self {
        Self {
            strips,
            ..Self::default()
        }
    }

    /// Whether every choice but `strips` is the standard one (as unwraps
    /// were before the choices existed).
    pub fn is_plain(&self) -> bool {
        *self == Self::plain(self.strips)
    }

    /// Whether every number is real.
    pub fn is_real(&self) -> bool {
        self.strip_angle.is_finite()
            && (0.0..=MAX_STRIP_ANGLE).contains(&self.strip_angle)
            && self
                .budget
                .is_none_or(|b| mesh::uvs::BUDGET_RANGE.contains(&b))
    }
}

/// Unwraps the pieces holding any of `faces`: each laid flat along the
/// seams (cut open first in a hidden place if it must be, marking the cut
/// as a seam), straight as a grid when it's a strip of square quads and
/// `strips` is on, at the object's texel density, where it was.
pub fn unwrap(mesh: &mut Mesh, faces: &[FaceId], strips: bool) -> Outcome {
    unwrap_with(mesh, faces, UnwrapOptions::plain(strips))
}

/// [`unwrap`], laying pieces flat as `options` say. Pinned points
/// ([`mesh::uvs::pins`]) stay where they are: with two or more pinned
/// points apart, the piece is solved around them and keeps its place,
/// size and turn; with one, the piece is laid out as usual and moved so
/// that point is back where it was.
pub fn unwrap_with(mesh: &mut Mesh, faces: &[FaceId], options: UnwrapOptions) -> Outcome {
    let cuts = cuts_of(mesh);
    let (pieces, found) = chosen_pieces(mesh, faces, &cuts);
    if pieces.is_empty() {
        return outcome(faces.len(), found, false);
    }
    let uvs = Uvs::of(mesh);
    let chosen: Vec<FaceId> = pieces.iter().flatten().copied().collect();
    let density = density_for(mesh, &chosen, uvs.as_ref());
    let changed = lay_out(mesh, &pieces, cuts, density, options);
    outcome(faces.len(), found, changed)
}

/// Each wedge's pinned spot (its old UV), if any of its corners is pinned.
pub(crate) fn pinned_spots(mesh: &Mesh, chart: &Chart, uvs: Option<&Uvs>) -> Vec<Option<Vec2>> {
    let mut out = vec![None; chart.len()];
    let (Some(pins), Some(uvs)) = (mesh::uvs::pins(mesh), uvs) else {
        return out;
    };
    let (starts, _) = mesh.face_layout();
    for (f, wedges) in chart.faces.iter().zip(&chart.corners) {
        let start = starts[f.get() as usize] as usize;
        for (k, w) in wedges.iter().enumerate() {
            if pins.get(start + k).copied().unwrap_or(false) && out[*w].is_none() {
                out[*w] = Some(uvs.at(start + k));
            }
        }
    }
    out
}

/// Whether the pinned spots are at least two, apart.
pub(crate) fn held_apart(pins: &[Option<Vec2>]) -> bool {
    let mut held = pins.iter().flatten();
    let Some(first) = held.next() else {
        return false;
    };
    held.any(|v| v.distance(*first) > 1e-7)
}

/// The chart laid flat around two or more pins, in the texture's own
/// place; a piece that lay mirrored stays mirrored.
fn around_pins(
    mesh: &Mesh,
    chart: &Chart,
    method: UnwrapMethod,
    pins: &[Option<Vec2>],
    old: &[Vec2],
) -> Option<Vec<Vec2>> {
    // Laying flat never turns a piece over, so a mirrored piece is solved
    // mirrored back, then turned over again.
    let mirrored = old.len() == chart.len()
        && chart
            .triangles(mesh)
            .iter()
            .map(|t| signed_area(old, *t))
            .sum::<f32>()
            < 0.0;
    let flip = |v: Vec2| if mirrored { Vec2::new(-v.x, v.y) } else { v };
    let pins: Vec<Option<Vec2>> = pins.iter().map(|p| p.map(flip)).collect();
    let guess: Vec<Vec2> = old.iter().map(|v| flip(*v)).collect();
    let (at, _) = lay_flat(mesh, chart, method, &pins, &guess)?;
    Some(at.into_iter().map(flip).collect())
}

/// `at` kept to one way: fitted to `old` (moved, turned and scaled to lie
/// over it as well as it can), then each wedge given back its old V (for
/// [`UnfoldAxis::Across`]) or old U (for [`UnfoldAxis::Up`]).
pub(crate) fn one_way(at: Vec<Vec2>, old: &[Option<Vec2>], axis: UnfoldAxis) -> Vec<Vec2> {
    let pairs: Vec<(Vec2, Vec2)> = at
        .iter()
        .zip(old)
        .filter_map(|(a, o)| o.map(|o| (*a, o)))
        .collect();
    if pairs.len() < 2 {
        return at;
    }
    // The best similarity z -> s z + t (s complex) from at to old.
    let n = pairs.len() as f32;
    let ma = pairs.iter().fold(Vec2::ZERO, |s, p| s + p.0) / n;
    let mo = pairs.iter().fold(Vec2::ZERO, |s, p| s + p.1) / n;
    let (mut re, mut im, mut norm) = (0.0f32, 0.0f32, 0.0f32);
    for (a, o) in &pairs {
        let (a, o) = (*a - ma, *o - mo);
        re += a.x * o.x + a.y * o.y;
        im += a.x * o.y - a.y * o.x;
        norm += a.length_squared();
    }
    let s = if norm > 1e-20 {
        Vec2::new(re, im) / norm
    } else {
        Vec2::X
    };
    let fit = |v: Vec2| {
        let d = v - ma;
        mo + Vec2::new(s.x * d.x - s.y * d.y, s.x * d.y + s.y * d.x)
    };
    at.into_iter()
        .zip(old)
        .map(|(v, o)| {
            let f = fit(v);
            match (o, axis) {
                (Some(o), UnfoldAxis::Across) => Vec2::new(f.x, o.y),
                (Some(o), UnfoldAxis::Up) => Vec2::new(o.x, f.y),
                (None, _) => f,
            }
        })
        .collect()
}

/// Lays `pieces` (whole pieces by `cuts`) flat as [`unwrap_with`] does,
/// at `density` (UV area per surface area); whether the UVs changed.
pub(crate) fn lay_out(
    mesh: &mut Mesh,
    pieces: &[Vec<FaceId>],
    mut cuts: BTreeSet<EdgeKey>,
    density: f32,
    options: UnwrapOptions,
) -> bool {
    let uvs = Uvs::of(mesh);
    // Pieces that can't lie flat are cut open first; the cuts are seams.
    let mut extra = BTreeSet::new();
    for piece in pieces {
        extra.extend(crate::seams::disk_cuts(mesh, piece, &cuts, options.hide));
    }
    cuts.extend(extra.iter().copied());
    // Pieces over their stretch budget are cut further.
    let mut over = BTreeSet::new();
    for piece in pieces {
        if let Some(budget) = mesh::uvs::budget_of(mesh, piece).or(options.budget) {
            over.extend(crate::seams::budget_cuts(
                mesh,
                piece,
                &cuts,
                budget,
                options.hide,
            ));
        }
    }
    extra.extend(over.iter().copied());
    cuts.extend(over);
    let mut laid = Vec::new();
    for piece in pieces {
        let chart = Chart::of(mesh, piece, &cuts);
        let was = before(mesh, &chart, uvs.as_ref());
        let pins = pinned_spots(mesh, &chart, uvs.as_ref());
        let old: Vec<Option<Vec2>> = match uvs.as_ref() {
            Some(u) => old_spots(mesh, &chart, u),
            None => vec![None; chart.len()],
        };
        let usable = uvs.as_ref().is_some_and(|u| usable_guess(mesh, &chart, u));
        let old_or_zero: Vec<Vec2> = old.iter().map(|v| v.unwrap_or(Vec2::ZERO)).collect();
        let around = held_apart(&pins)
            .then(|| around_pins(mesh, &chart, options.method, &pins, &old_or_zero))
            .flatten();
        let at = match around {
            Some(at) => at,
            None => {
                let as_grid = (options.strips && pins.iter().all(Option::is_none))
                    .then(|| {
                        all_square_quads(mesh, &chart, options.strip_angle)
                            .then(|| grid(mesh, &chart))
                            .flatten()
                    })
                    .flatten();
                let (at, align) = match as_grid {
                    Some(at) => (at, false),
                    None => {
                        let flat = projected(mesh, &chart);
                        let guess = if usable {
                            old_or_zero.clone()
                        } else {
                            flat.clone()
                        };
                        let none = vec![None; chart.len()];
                        match lay_flat(mesh, &chart, options.method, &none, &guess) {
                            Some((at, _)) => (at, true),
                            None => (flat, true),
                        }
                    }
                };
                let mut at = place(mesh, &chart, at, &was, density, align);
                // One pin: the piece is moved so that point is back.
                if let Some((w, spot)) =
                    pins.iter().enumerate().find_map(|(w, p)| p.map(|p| (w, p)))
                {
                    let shift = spot - at[w];
                    for v in &mut at {
                        *v += shift;
                    }
                }
                at
            }
        };
        let at = match options.only {
            Some(axis) if old.iter().any(Option::is_some) => one_way(at, &old, axis),
            _ => at,
        };
        laid.push((chart, at));
    }
    if !extra.is_empty() {
        let extra: Vec<EdgeKey> = extra.into_iter().collect();
        mesh::uvs::set_seams(mesh, &extra, true);
    }
    write(mesh, &laid)
}

/// Straightens the pieces holding any of `faces` that are made only of
/// quads: laid out as a straight grid, at their own size, where they were
/// (with a pinned point, moved so the first stays put). Pieces with other
/// polygons are skipped.
pub fn straighten(mesh: &mut Mesh, faces: &[FaceId]) -> Outcome {
    let cuts = cuts_of(mesh);
    let (pieces, found) = chosen_pieces(mesh, faces, &cuts);
    let uvs = Uvs::of(mesh);
    let mut laid = Vec::new();
    for piece in &pieces {
        let chart = Chart::of(mesh, piece, &cuts);
        let Some(at) = grid(mesh, &chart) else {
            continue;
        };
        let was = before(mesh, &chart, uvs.as_ref());
        let density = uvs
            .as_ref()
            .and_then(|u| density_of(mesh, piece, u))
            .unwrap_or_else(|| density_for(mesh, piece, uvs.as_ref()));
        let mut at = place(mesh, &chart, at, &was, density, false);
        // A pinned point keeps its spot (the first, if there are more: a
        // straight grid can't bend to meet them all).
        let pins = pinned_spots(mesh, &chart, uvs.as_ref());
        if let Some((w, spot)) = pins.iter().enumerate().find_map(|(w, p)| p.map(|p| (w, p))) {
            let shift = spot - at[w];
            for v in &mut at {
                *v += shift;
            }
        }
        laid.push((chart, at));
    }
    if laid.is_empty() {
        return outcome(faces.len(), found, false);
    }
    let changed = write(mesh, &laid);
    outcome(faces.len(), found, changed)
}

/// Relaxes the UVs of `faces`: their corners are pulled, step by step,
/// towards spots where every side of their triangles is as long (for the
/// piece's texel density) as it is on the shape. Corners of other
/// polygons, and pinned corners, stay put. A shape without UVs is left alone.
pub fn relax(mesh: &mut Mesh, faces: &[FaceId], steps: usize) -> Outcome {
    let Some(uvs) = Uvs::of(mesh) else {
        let found = faces.iter().filter(|f| mesh.face(**f).is_some()).count();
        return outcome(faces.len(), found, false);
    };
    let cuts = cuts_of(mesh);
    let (pieces, found) = chosen_pieces(mesh, faces, &cuts);
    let wanted: BTreeSet<FaceId> = faces.iter().copied().collect();
    let mut laid = Vec::new();
    for piece in &pieces {
        let chart = Chart::of(mesh, piece, &cuts);
        let old = old_spots(mesh, &chart, &uvs);
        let mut at: Vec<Vec2> = old.iter().map(|v| v.unwrap_or(Vec2::ZERO)).collect();
        let mut free = vec![false; chart.len()];
        for (f, wedges) in chart.faces.iter().zip(&chart.corners) {
            if wanted.contains(f) {
                for w in wedges {
                    free[*w] = true;
                }
            }
        }
        // Pinned points stay put.
        for (w, pin) in pinned_spots(mesh, &chart, Some(&uvs)).iter().enumerate() {
            if pin.is_some() {
                free[w] = false;
            }
        }
        let density = density_of(mesh, piece, &uvs).unwrap_or(1.0);
        let scale = density.sqrt();
        let p = |w: usize| mesh.position(chart.points[w]).unwrap_or(Vec3::ZERO);
        let mut springs: BTreeSet<(usize, usize)> = BTreeSet::new();
        for t in chart.triangles(mesh) {
            for k in 0..3 {
                let (a, b) = (t[k], t[(k + 1) % 3]);
                if a != b {
                    springs.insert((a.min(b), a.max(b)));
                }
            }
        }
        let springs: Vec<(usize, usize, f32)> = springs
            .into_iter()
            .map(|(a, b)| (a, b, p(a).distance(p(b)) * scale))
            .collect();
        for _ in 0..steps {
            let mut push = vec![Vec2::ZERO; chart.len()];
            let mut count = vec![0u32; chart.len()];
            for &(a, b, rest) in &springs {
                let d = at[b] - at[a];
                let length = d.length();
                if length < 1e-12 {
                    continue;
                }
                let fix = d * ((length - rest) / length) * 0.5;
                push[a] += fix;
                push[b] -= fix;
                count[a] += 1;
                count[b] += 1;
            }
            for w in 0..chart.len() {
                if free[w] && count[w] > 0 {
                    at[w] += push[w] / count[w] as f32;
                }
            }
        }
        if at.iter().all(|v| v.is_finite()) {
            laid.push((chart, at));
        }
    }
    if laid.is_empty() {
        return outcome(faces.len(), found, false);
    }
    let changed = write(mesh, &laid);
    outcome(faces.len(), found, changed)
}

#[cfg(test)]
mod tests;
