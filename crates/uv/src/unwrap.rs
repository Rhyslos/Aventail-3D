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
use crate::solve::Normal;
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
/// count as a strip and be straightened when unwrapping.
pub const SQUARE_WITHIN: f32 = 25.0;

/// How many steps relaxing takes.
pub const RELAX_STEPS: usize = 60;

/// What a chart is cut along: the seams, or, on a shape with no seam marks
/// at all (imported, say), where its UVs split.
fn cuts_of(mesh: &Mesh) -> BTreeSet<EdgeKey> {
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

fn signed_area(at: &[Vec2], t: [usize; 3]) -> f32 {
    (at[t[1]] - at[t[0]]).perp_dot(at[t[2]] - at[t[0]]) / 2.0
}

/// Two wedges far apart, to pin while solving.
fn far_pair(mesh: &Mesh, chart: &Chart) -> Option<(usize, usize)> {
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
fn conformal(mesh: &Mesh, chart: &Chart, guess: &[Vec2]) -> Option<Vec<Vec2>> {
    let n = chart.len();
    if !(3..=SOLVE_UP_TO).contains(&n) {
        return None;
    }
    let (pa, pb) = far_pair(mesh, chart)?;
    let p = |w: usize| mesh.position(chart.points[w]).unwrap_or(Vec3::ZERO);
    let pinned = |w: usize| -> Option<[f64; 2]> {
        if w == pa {
            Some([0.0, 0.0])
        } else if w == pb {
            Some([f64::from(p(pa).distance(p(pb))), 0.0])
        } else {
            None
        }
    };
    // Unknowns: u then v of each wedge that isn't pinned.
    let mut slot = vec![usize::MAX; n];
    let mut count = 0;
    for (w, s) in slot.iter_mut().enumerate() {
        if pinned(w).is_none() {
            *s = count;
            count += 1;
        }
    }
    let mut normal = Normal::new(2 * count);
    let mut rhs = vec![0.0f64; 2 * count];
    let mut any = false;
    for t in chart.triangles(mesh) {
        let (p0, p1, p2) = (p(t[0]), p(t[1]), p(t[2]));
        let e1 = p1 - p0;
        let e2 = p2 - p0;
        let twice = e1.cross(e2).length();
        if twice < 1e-12 || e1.length() < 1e-9 {
            continue;
        }
        any = true;
        let x = e1 / e1.length();
        let y = e1.cross(e2).normalize().cross(x);
        let q = [
            [0.0f64, 0.0],
            [f64::from(e1.length()), 0.0],
            [f64::from(e2.dot(x)), f64::from(e2.dot(y))],
        ];
        let s = 1.0 / f64::from(twice).sqrt();
        // W_j = q[j+2] - q[j+1], as complex numbers a + ib.
        let w: Vec<(f64, f64)> = (0..3)
            .map(|j| {
                let (k, l) = ((j + 2) % 3, (j + 1) % 3);
                ((q[k][0] - q[l][0]) * s, (q[k][1] - q[l][1]) * s)
            })
            .collect();
        // Real and imaginary rows of sum W_j (u_j + i v_j).
        for real in [true, false] {
            let mut entries = Vec::with_capacity(6);
            let mut constant = 0.0;
            for j in 0..3 {
                let (a, b) = w[j];
                let (cu, cv) = if real { (a, -b) } else { (b, a) };
                match pinned(t[j]) {
                    Some([u, v]) => constant += cu * u + cv * v,
                    None => {
                        let k = slot[t[j]];
                        entries.push((2 * k, cu));
                        entries.push((2 * k + 1, cv));
                    }
                }
            }
            normal.add_row(&entries);
            for (i, c) in &entries {
                rhs[*i] -= c * constant;
            }
        }
    }
    if !any {
        return None;
    }
    // Start from the guess, moved so its pinned wedges sit on the pins.
    let mut x = vec![0.0f64; 2 * count];
    if guess.len() == n {
        let (ga, gb) = (guess[pa], guess[pb]);
        let to_b = Vec2::new(p(pa).distance(p(pb)), 0.0);
        let from = gb - ga;
        if from.length() > 1e-9 {
            // The similarity taking ga to 0 and gb to to_b.
            let k = to_b.length() / from.length();
            let turn = -from.y.atan2(from.x);
            let (sin, cos) = turn.sin_cos();
            for w in 0..n {
                if slot[w] == usize::MAX {
                    continue;
                }
                let d = (guess[w] - ga) * k;
                let v = Vec2::new(cos * d.x - sin * d.y, sin * d.x + cos * d.y);
                x[2 * slot[w]] = f64::from(v.x);
                x[2 * slot[w] + 1] = f64::from(v.y);
            }
        }
    }
    let steps = (8 * count).clamp(400, 6_000);
    normal.solve(&rhs, &mut x, steps);
    let out: Vec<Vec2> = (0..n)
        .map(|w| match pinned(w) {
            Some([u, v]) => Vec2::new(u as f32, v as f32),
            None => {
                let k = slot[w];
                Vec2::new(x[2 * k] as f32, x[2 * k + 1] as f32)
            }
        })
        .collect();
    out.iter().all(|v| v.is_finite()).then_some(out)
}

/// The chart projected flat on the plane it faces most: the fallback when
/// it can't be solved.
fn projected(mesh: &Mesh, chart: &Chart) -> Vec<Vec2> {
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
/// [`SQUARE_WITHIN`] of square.
fn all_square_quads(mesh: &Mesh, chart: &Chart) -> bool {
    let limit = SQUARE_WITHIN.to_radians().sin();
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
fn grid(mesh: &Mesh, chart: &Chart) -> Option<Vec<Vec2>> {
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
fn usable_guess(mesh: &Mesh, chart: &Chart, uvs: &Uvs) -> bool {
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
fn old_spots(mesh: &Mesh, chart: &Chart, uvs: &Uvs) -> Vec<Option<Vec2>> {
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
fn chosen_pieces(
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
fn write(mesh: &mut Mesh, laid: &[(Chart, Vec<Vec2>)]) -> bool {
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

fn outcome(asked: usize, found: usize, changed: bool) -> Outcome {
    Outcome {
        matched: if changed { found } else { 0 },
        missing: asked - found,
        skipped: if changed { 0 } else { found },
    }
}

/// Unwraps the pieces holding any of `faces`: each laid flat along the
/// seams (cut open first in a hidden place if it must be, marking the cut
/// as a seam), straight as a grid when it's a strip of square quads and
/// `strips` is on, at the object's texel density, where it was.
pub fn unwrap(mesh: &mut Mesh, faces: &[FaceId], strips: bool) -> Outcome {
    let mut cuts = cuts_of(mesh);
    let (pieces, found) = chosen_pieces(mesh, faces, &cuts);
    if pieces.is_empty() {
        return outcome(faces.len(), found, false);
    }
    let uvs = Uvs::of(mesh);
    let chosen: Vec<FaceId> = pieces.iter().flatten().copied().collect();
    let density = density_for(mesh, &chosen, uvs.as_ref());
    // Pieces that can't lie flat are cut open first; the cuts are seams.
    let mut extra = BTreeSet::new();
    for piece in &pieces {
        extra.extend(crate::seams::disk_cuts(mesh, piece, &cuts));
    }
    cuts.extend(extra.iter().copied());
    let mut laid = Vec::new();
    for piece in &pieces {
        let chart = Chart::of(mesh, piece, &cuts);
        let was = before(mesh, &chart, uvs.as_ref());
        let (at, align) = match strips
            .then(|| {
                all_square_quads(mesh, &chart)
                    .then(|| grid(mesh, &chart))
                    .flatten()
            })
            .flatten()
        {
            Some(at) => (at, false),
            None => {
                let flat = projected(mesh, &chart);
                let guess: Vec<Vec2> = match uvs.as_ref() {
                    Some(u) if usable_guess(mesh, &chart, u) => old_spots(mesh, &chart, u)
                        .into_iter()
                        .map(|v| v.unwrap_or(Vec2::ZERO))
                        .collect(),
                    _ => flat.clone(),
                };
                match conformal(mesh, &chart, &guess) {
                    Some(at) => (at, true),
                    None => (flat, true),
                }
            }
        };
        let at = place(mesh, &chart, at, &was, density, align);
        laid.push((chart, at));
    }
    if !extra.is_empty() {
        let extra: Vec<EdgeKey> = extra.into_iter().collect();
        mesh::uvs::set_seams(mesh, &extra, true);
    }
    let changed = write(mesh, &laid);
    outcome(faces.len(), found, changed)
}

/// Straightens the pieces holding any of `faces` that are made only of
/// quads: laid out as a straight grid, at their own size, where they were.
/// Pieces with other polygons are skipped.
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
        let at = place(mesh, &chart, at, &was, density, false);
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
/// polygons stay put. A shape without UVs is left alone.
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
