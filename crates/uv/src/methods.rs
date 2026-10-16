//! Ways of laying a piece flat
//! ([decision 0109](../../../docs/decisions/0109-pins-and-unwrap-methods.md)).
//!
//! Every way works on a chart's triangles, each given as its own flat
//! shape (its corners in 2D), and ends with a layout of the chart's
//! wedges:
//!
//! - **Keep shapes** ([`UnwrapMethod::Conformal`]): least squares
//!   conformal maps (LSCM), on the triangles' real shapes. Every small
//!   piece of surface keeps its angles; sizes may drift across a curved
//!   piece. Fast, and what the app always did.
//! - **Keep shapes, smoother** ([`UnwrapMethod::AngleBased`]): the angles
//!   are first evened out over the whole piece (linear angle-based
//!   flattening, Zayer, Rössl and Seidel 2007: the corners round each
//!   point made to add up to a full turn and fit together, as near to the
//!   real angles as can be), then laid out by LSCM from those angles.
//!   Less drift on curved pieces.
//! - **Keep sizes** ([`UnwrapMethod::SizeKeeping`]): "as rigid as
//!   possible" (Liu et al. 2008), started from LSCM, then each triangle in
//!   turn made as close to its real size and shape as it can be while
//!   joined to the others. A texture's pixels come out the same size all
//!   over the piece, at the cost of a little angle change.
//! - **Automatic** ([`UnwrapMethod::Automatic`]): tries all three and
//!   keeps the one that lies most evenly (by area-weighted stretch and
//!   size change); one must be clearly better (by 1%) to beat an earlier
//!   one, in the order keep shapes, smoother, keep sizes. Always the same
//!   choice for the same piece: nothing is random.
//!
//! Wedges with a spot in `pins` stay there. Without at least two pins
//! apart, two far-apart wedges are held where the method puts them, and
//! the result is placed afterwards like any unwrap.

use crate::chart::Chart;
use crate::solve::Normal;
use glam::{Vec2, Vec3};
use mesh::Mesh;
use std::f64::consts::PI;

/// How a piece is laid flat. See the module notes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum UnwrapMethod {
    /// Least squares conformal maps on the real triangles.
    #[default]
    Conformal,
    /// Angles evened out first (linear angle-based flattening), then LSCM.
    AngleBased,
    /// As rigid as possible: sizes kept.
    SizeKeeping,
    /// The one of the three that lies most evenly.
    Automatic,
}

impl UnwrapMethod {
    /// Every method, in the order offered.
    pub const ALL: [UnwrapMethod; 4] = [
        Self::Automatic,
        Self::Conformal,
        Self::AngleBased,
        Self::SizeKeeping,
    ];
}

/// How many rounds "as rigid as possible" takes.
const RIGID_ROUNDS: usize = 12;

/// A triangle's corners, flat.
pub(crate) type Shape = [[f64; 2]; 3];

/// The flat shape of each of the chart's triangles (`None` for one with
/// no area), and the triangles themselves.
pub(crate) fn real_shapes(mesh: &Mesh, chart: &Chart) -> (Vec<[usize; 3]>, Vec<Option<Shape>>) {
    let p = |w: usize| mesh.position(chart.points[w]).unwrap_or(Vec3::ZERO);
    let triangles = chart.triangles(mesh);
    let shapes = triangles
        .iter()
        .map(|t| {
            let (p0, p1, p2) = (p(t[0]), p(t[1]), p(t[2]));
            let e1 = p1 - p0;
            let e2 = p2 - p0;
            let twice = e1.cross(e2).length();
            if twice < 1e-12 || e1.length() < 1e-9 {
                return None;
            }
            let x = e1 / e1.length();
            let y = e1.cross(e2).normalize().cross(x);
            Some([
                [0.0, 0.0],
                [f64::from(e1.length()), 0.0],
                [f64::from(e2.dot(x)), f64::from(e2.dot(y))],
            ])
        })
        .collect();
    (triangles, shapes)
}

fn area(s: &Shape) -> f64 {
    ((s[1][0] - s[0][0]) * (s[2][1] - s[0][1]) - (s[2][0] - s[0][0]) * (s[1][1] - s[0][1])) / 2.0
}

/// The chart laid out by least squares conformal maps on `shapes`, with
/// the pinned wedges held; `None` if it can't be (no free wedge, no
/// triangle with area, or no real result).
pub(crate) fn conformal_shapes(
    n: usize,
    triangles: &[[usize; 3]],
    shapes: &[Option<Shape>],
    pins: &[Option<Vec2>],
    start: &[Vec2],
) -> Option<Vec<Vec2>> {
    if pins.len() != n {
        return None;
    }
    let pinned =
        |w: usize| -> Option<[f64; 2]> { pins[w].map(|v| [f64::from(v.x), f64::from(v.y)]) };
    // Unknowns: u then v of each wedge that isn't pinned.
    let mut slot = vec![usize::MAX; n];
    let mut count = 0;
    for (w, s) in slot.iter_mut().enumerate() {
        if pinned(w).is_none() {
            *s = count;
            count += 1;
        }
    }
    if count == 0 {
        return None;
    }
    let mut normal = Normal::new(2 * count);
    let mut rhs = vec![0.0f64; 2 * count];
    let mut any = false;
    for (t, shape) in triangles.iter().zip(shapes) {
        let Some(q) = shape else {
            continue;
        };
        let twice = 2.0 * area(q).abs();
        if twice < 1e-24 {
            continue;
        }
        any = true;
        let s = 1.0 / twice.sqrt();
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
    let mut x = vec![0.0f64; 2 * count];
    if start.len() == n {
        for w in 0..n {
            if slot[w] != usize::MAX {
                x[2 * slot[w]] = f64::from(start[w].x);
                x[2 * slot[w] + 1] = f64::from(start[w].y);
            }
        }
    }
    let steps = (8 * count).clamp(400, 6_000);
    normal.solve(&rhs, &mut x, steps);
    finish(n, &slot, &x, pins)
}

fn finish(n: usize, slot: &[usize], x: &[f64], pins: &[Option<Vec2>]) -> Option<Vec<Vec2>> {
    let out: Vec<Vec2> = (0..n)
        .map(|w| match pins[w] {
            Some(v) => v,
            None => {
                let k = slot[w];
                Vec2::new(x[2 * k] as f32, x[2 * k + 1] as f32)
            }
        })
        .collect();
    out.iter().all(|v| v.is_finite()).then_some(out)
}

/// The corner angles of a flat triangle.
fn angles(s: &Shape) -> [f64; 3] {
    let corner = |j: usize| {
        let (a, b, c) = (s[j], s[(j + 1) % 3], s[(j + 2) % 3]);
        let u = [b[0] - a[0], b[1] - a[1]];
        let v = [c[0] - a[0], c[1] - a[1]];
        let cross = u[0] * v[1] - u[1] * v[0];
        let dot = u[0] * v[0] + u[1] * v[1];
        cross.abs().atan2(dot)
    };
    [corner(0), corner(1), corner(2)]
}

/// The chart's wedges that lie inside it (not on its border): those whose
/// every side is shared by two of its triangles.
fn inner_wedges(n: usize, triangles: &[[usize; 3]]) -> Vec<bool> {
    let mut uses: std::collections::HashMap<(usize, usize), u32> = Default::default();
    for t in triangles {
        for j in 0..3 {
            let (a, b) = (t[j], t[(j + 1) % 3]);
            *uses.entry((a.min(b), a.max(b))).or_default() += 1;
        }
    }
    let mut inner = vec![true; n];
    let mut seen = vec![false; n];
    for t in triangles {
        for &w in t {
            seen[w] = true;
        }
    }
    for ((a, b), count) in uses {
        if count != 2 {
            inner[a] = false;
            inner[b] = false;
        }
    }
    for w in 0..n {
        inner[w] &= seen[w];
    }
    inner
}

/// The triangles' shapes with their angles evened out over the chart by
/// linear angle-based flattening: as near to the real angles as can be
/// while each triangle's add up to a half turn and those round each inner
/// point to a full turn and fit together. `None` if it can't be solved.
pub(crate) fn angle_based_shapes(
    n: usize,
    triangles: &[[usize; 3]],
    shapes: &[Option<Shape>],
) -> Option<Vec<Option<Shape>>> {
    let t_count = triangles.len();
    // The aimed-for angles: the real ones, those round an inner point
    // scaled to make a full turn.
    let mut beta: Vec<[f64; 3]> = shapes
        .iter()
        .map(|s| s.as_ref().map_or([PI / 3.0; 3], angles))
        .collect();
    let inner = inner_wedges(n, triangles);
    let mut around = vec![0.0f64; n];
    for (t, b) in triangles.iter().zip(&beta) {
        for j in 0..3 {
            around[t[j]] += b[j];
        }
    }
    for (t, b) in triangles.iter().zip(beta.iter_mut()) {
        for j in 0..3 {
            if inner[t[j]] && around[t[j]] > 1e-9 {
                b[j] *= 2.0 * PI / around[t[j]];
            }
        }
    }
    for b in &mut beta {
        for a in b.iter_mut() {
            *a = a.clamp(1e-3, PI - 1e-3);
        }
    }
    // The constraints, one row each: per triangle, then per inner point
    // its full turn and its wheel (the sides' lengths round it agreeing).
    let inner_index: Vec<usize> = {
        let mut next = 0;
        (0..n)
            .map(|w| {
                if inner[w] {
                    next += 1;
                    next - 1
                } else {
                    usize::MAX
                }
            })
            .collect()
    };
    let inner_count = inner.iter().filter(|i| **i).count();
    let m = t_count + 2 * inner_count;
    // For each angle (3 t + j), its weight in each constraint (the
    // constraint matrix A, by column).
    let mut columns: Vec<Vec<(usize, f64)>> = vec![Vec::new(); 3 * t_count];
    let mut b = vec![0.0f64; m];
    for (ti, (t, bt)) in triangles.iter().zip(&beta).enumerate() {
        for j in 0..3 {
            columns[3 * ti + j].push((ti, 1.0));
        }
        b[ti] = PI - (bt[0] + bt[1] + bt[2]);
        for j in 0..3 {
            let w = t[j];
            if !inner[w] {
                continue;
            }
            let full = t_count + 2 * inner_index[w];
            let wheel = full + 1;
            columns[3 * ti + j].push((full, 1.0));
            b[full] += 2.0 * PI;
            b[full] -= bt[j];
            let (next, prev) = ((j + 1) % 3, (j + 2) % 3);
            let cot = |a: f64| a.cos() / a.sin();
            columns[3 * ti + next].push((wheel, cot(bt[next])));
            columns[3 * ti + prev].push((wheel, -cot(bt[prev])));
            b[wheel] -= bt[next].sin().ln() - bt[prev].sin().ln();
        }
    }
    // The full-turn rows were given one 2π per triangle round the point:
    // only one is wanted.
    for w in 0..n {
        if inner[w] {
            let count = triangles.iter().flatten().filter(|v| **v == w).count() as f64;
            b[t_count + 2 * inner_index[w]] -= 2.0 * PI * (count - 1.0);
        }
    }
    // Least change, weighted by 1/β²: with B = A·diag(β), the change is
    // e = diag(β)·Bᵀ·λ where (B·Bᵀ)·λ = b.
    let mut normal = Normal::new(m);
    for (k, column) in columns.iter().enumerate() {
        let beta_k = beta[k / 3][k % 3];
        let entries: Vec<(usize, f64)> = column.iter().map(|(i, a)| (*i, a * beta_k)).collect();
        normal.add_row(&entries);
    }
    let mut lambda = vec![0.0f64; m];
    normal.solve(&b, &mut lambda, (4 * m).clamp(400, 8_000));
    let mut out = Vec::with_capacity(t_count);
    for (ti, shape) in shapes.iter().enumerate() {
        let Some(real) = shape else {
            out.push(None);
            continue;
        };
        let mut alpha = [0.0f64; 3];
        for j in 0..3 {
            let k = 3 * ti + j;
            let f: f64 = columns[k]
                .iter()
                .map(|(i, a)| a * beta[ti][j] * lambda[*i])
                .sum();
            alpha[j] = (beta[ti][j] + beta[ti][j] * f).clamp(1e-3, PI - 1e-3);
        }
        let sum = alpha[0] + alpha[1] + alpha[2];
        for a in &mut alpha {
            *a *= PI / sum;
        }
        // The triangle with those angles (sides as the sines of the angles
        // across), as big as the real one.
        let (s01, s02) = (alpha[2].sin(), alpha[1].sin());
        let mut q = [
            [0.0, 0.0],
            [s01, 0.0],
            [s02 * alpha[0].cos(), s02 * alpha[0].sin()],
        ];
        let made = area(&q).abs();
        let wanted = area(real).abs();
        if made < 1e-30 || !made.is_finite() {
            out.push(Some(*real));
            continue;
        }
        let k = (wanted / made).sqrt();
        for c in &mut q {
            c[0] *= k;
            c[1] *= k;
        }
        out.push(Some(q));
    }
    out.iter()
        .flatten()
        .all(|q| q.iter().all(|c| c[0].is_finite() && c[1].is_finite()))
        .then_some(out)
}

/// The chart laid out "as rigid as possible" on `shapes` (scaled by
/// `scale`, to the layout's units), starting from `start` (one spot per
/// wedge), with the pinned wedges held (or, with none, the first wedge
/// held where it starts).
pub(crate) fn as_rigid_as_possible(
    n: usize,
    triangles: &[[usize; 3]],
    shapes: &[Option<Shape>],
    pins: &[Option<Vec2>],
    start: &[Vec2],
    scale: f64,
) -> Option<Vec<Vec2>> {
    if start.len() != n || pins.len() != n || n < 3 {
        return None;
    }
    let mut pins = pins.to_vec();
    if pins.iter().all(Option::is_none) {
        pins[0] = Some(start[0]);
    }
    let mut slot = vec![usize::MAX; n];
    let mut count = 0;
    for (w, s) in slot.iter_mut().enumerate() {
        if pins[w].is_none() {
            *s = count;
            count += 1;
        }
    }
    if count == 0 {
        return None;
    }
    // Each triangle's corners at the layout's scale, and its sides'
    // weights (half the cotangent across, kept positive).
    let scaled: Vec<Option<Shape>> = shapes
        .iter()
        .map(|s| s.map(|q| q.map(|c| [c[0] * scale, c[1] * scale])))
        .collect();
    let weights: Vec<[f64; 3]> = scaled
        .iter()
        .map(|s| match s {
            Some(q) => {
                let a = angles(q);
                a.map(|x| (0.5 * x.cos() / x.sin().max(1e-6)).clamp(1e-4, 1e4))
            }
            None => [0.0; 3],
        })
        .collect();
    // The matrix is the same every round.
    let mut normal = Normal::new(2 * count);
    for (t, w) in triangles.iter().zip(&weights) {
        for j in 0..3 {
            let (a, b) = (t[(j + 1) % 3], t[(j + 2) % 3]);
            let r = w[j].sqrt();
            if r == 0.0 {
                continue;
            }
            for axis in 0..2 {
                let mut entries = Vec::with_capacity(2);
                if slot[a] != usize::MAX {
                    entries.push((2 * slot[a] + axis, r));
                }
                if slot[b] != usize::MAX {
                    entries.push((2 * slot[b] + axis, -r));
                }
                normal.add_row(&entries);
            }
        }
    }
    let mut at: Vec<Vec2> = start.to_vec();
    for (w, p) in pins.iter().enumerate() {
        if let Some(p) = p {
            at[w] = *p;
        }
    }
    let mut x = vec![0.0f64; 2 * count];
    for w in 0..n {
        if slot[w] != usize::MAX {
            x[2 * slot[w]] = f64::from(at[w].x);
            x[2 * slot[w] + 1] = f64::from(at[w].y);
        }
    }
    for _ in 0..RIGID_ROUNDS {
        // Each triangle's best turn from its real shape to where it lies.
        let turns: Vec<(f64, f64)> = triangles
            .iter()
            .zip(&scaled)
            .zip(&weights)
            .map(|((t, s), w)| {
                let Some(q) = s else {
                    return (1.0, 0.0);
                };
                let (mut s00, mut s01, mut s10, mut s11) = (0.0, 0.0, 0.0, 0.0);
                for (j, weight) in w.iter().enumerate() {
                    let (a, b) = ((j + 1) % 3, (j + 2) % 3);
                    let du = at[t[a]] - at[t[b]];
                    let dq = [q[a][0] - q[b][0], q[a][1] - q[b][1]];
                    s00 += weight * f64::from(du.x) * dq[0];
                    s01 += weight * f64::from(du.x) * dq[1];
                    s10 += weight * f64::from(du.y) * dq[0];
                    s11 += weight * f64::from(du.y) * dq[1];
                }
                let angle = (s10 - s01).atan2(s00 + s11);
                (angle.cos(), angle.sin())
            })
            .collect();
        let mut rhs = vec![0.0f64; 2 * count];
        for ((t, s), (w, (c, sn))) in triangles
            .iter()
            .zip(&scaled)
            .zip(weights.iter().zip(&turns))
        {
            let Some(q) = s else {
                continue;
            };
            for (j, weight) in w.iter().enumerate() {
                let (ia, ib) = ((j + 1) % 3, (j + 2) % 3);
                let (a, b) = (t[ia], t[ib]);
                let dq = [q[ia][0] - q[ib][0], q[ia][1] - q[ib][1]];
                let target = [c * dq[0] - sn * dq[1], sn * dq[0] + c * dq[1]];
                let r = weight.sqrt();
                for axis in 0..2 {
                    let pinned =
                        |v: usize| pins[v].map(|p| f64::from(if axis == 0 { p.x } else { p.y }));
                    // Row: r·(x_a − x_b) = r·target.
                    let mut constant = r * target[axis];
                    if let Some(p) = pinned(a) {
                        constant -= r * p;
                    }
                    if let Some(p) = pinned(b) {
                        constant += r * p;
                    }
                    if slot[a] != usize::MAX {
                        rhs[2 * slot[a] + axis] += r * constant;
                    }
                    if slot[b] != usize::MAX {
                        rhs[2 * slot[b] + axis] -= r * constant;
                    }
                }
            }
        }
        normal.solve(&rhs, &mut x, (4 * count).clamp(200, 3_000));
        at = finish(n, &slot, &x, &pins)?;
    }
    Some(at)
}

/// How unevenly a layout lies over the triangles' shapes, area-weighted:
/// how much each is stretched one way more than the other, plus how far
/// its size is from the layout's own average, plus a heavy price for any
/// triangle turned over. 0 is perfect.
pub(crate) fn distortion(triangles: &[[usize; 3]], shapes: &[Option<Shape>], at: &[Vec2]) -> f64 {
    let flat_area = |t: &[usize; 3]| {
        let (a, b, c) = (at[t[0]], at[t[1]], at[t[2]]);
        f64::from((b - a).perp_dot(c - a)) / 2.0
    };
    let total_real: f64 = shapes.iter().flatten().map(|q| area(q).abs()).sum();
    let total_flat: f64 = triangles
        .iter()
        .zip(shapes)
        .filter(|(_, s)| s.is_some())
        .map(|(t, _)| flat_area(t).abs())
        .sum();
    if total_real <= 0.0 || total_flat <= 0.0 {
        return f64::INFINITY;
    }
    let average = total_flat / total_real;
    let mut sum = 0.0;
    for (t, s) in triangles.iter().zip(shapes) {
        let Some(q) = s else {
            continue;
        };
        let weight = area(q).abs();
        if weight <= 0.0 {
            continue;
        }
        // The map from the real triangle to the flat one.
        let (q1, q2) = (
            [q[1][0] - q[0][0], q[1][1] - q[0][1]],
            [q[2][0] - q[0][0], q[2][1] - q[0][1]],
        );
        let det = q1[0] * q2[1] - q2[0] * q1[1];
        if det.abs() < 1e-30 {
            continue;
        }
        let (u1, u2) = (at[t[1]] - at[t[0]], at[t[2]] - at[t[0]]);
        let (u1, u2) = (
            [f64::from(u1.x), f64::from(u1.y)],
            [f64::from(u2.x), f64::from(u2.y)],
        );
        let inv = [[q2[1] / det, -q2[0] / det], [-q1[1] / det, q1[0] / det]];
        // J = U · Q⁻¹, columns j0 and j1.
        let j0 = [
            u1[0] * inv[0][0] + u2[0] * inv[1][0],
            u1[1] * inv[0][0] + u2[1] * inv[1][0],
        ];
        let j1 = [
            u1[0] * inv[0][1] + u2[0] * inv[1][1],
            u1[1] * inv[0][1] + u2[1] * inv[1][1],
        ];
        let flipped = j0[0] * j1[1] - j0[1] * j1[0] < 0.0;
        let (aa, ab, bb) = (
            j0[0] * j0[0] + j0[1] * j0[1],
            j0[0] * j1[0] + j0[1] * j1[1],
            j1[0] * j1[0] + j1[1] * j1[1],
        );
        let trace = aa + bb;
        let d = (aa * bb - ab * ab).max(0.0);
        let spread = (trace * trace / 4.0 - d).max(0.0).sqrt();
        let most = (trace / 2.0 + spread).max(0.0).sqrt();
        let least = (trace / 2.0 - spread).max(0.0).sqrt();
        let uneven = if least > 1e-12 {
            (most / least).ln()
        } else {
            10.0
        };
        let size = if most * least > 1e-24 {
            ((most * least) / average).ln().abs()
        } else {
            10.0
        };
        sum += weight * (uneven + size + if flipped { 10.0 } else { 0.0 });
    }
    sum / total_real
}

/// The chart laid out by `method` (not `Automatic`), with `pins` held
/// (each wedge's spot, or none), starting from `start`, with `scale`
/// taking real lengths to the layout's units for keeping sizes. Keeping
/// sizes starts from keeping shapes held by `start_pins`.
#[allow(clippy::too_many_arguments)]
fn by(
    method: UnwrapMethod,
    n: usize,
    triangles: &[[usize; 3]],
    real: &[Option<Shape>],
    pins: &[Option<Vec2>],
    start_pins: &[Option<Vec2>],
    start: &[Vec2],
    scale: f64,
) -> Option<Vec<Vec2>> {
    match method {
        UnwrapMethod::Conformal | UnwrapMethod::Automatic => {
            conformal_shapes(n, triangles, real, pins, start)
        }
        UnwrapMethod::AngleBased => {
            let even = angle_based_shapes(n, triangles, real)?;
            conformal_shapes(n, triangles, &even, pins, start)
        }
        UnwrapMethod::SizeKeeping => {
            // Started from keeping shapes, held by two pins at least (one
            // pin leaves it free to shrink to nothing).
            let first = conformal_shapes(n, triangles, real, start_pins, start)?;
            as_rigid_as_possible(n, triangles, real, pins, &first, scale)
        }
    }
}

/// The chart laid out by `method`, with the wedges that have a spot in
/// `pins` held there (at least two apart for the result to keep the
/// texture's place; else it is laid out on its own, at real size). The
/// method used is returned with the layout (for `Automatic`, the one
/// chosen). `None` if no method could lay it out.
pub(crate) fn lay_flat(
    mesh: &Mesh,
    chart: &Chart,
    method: UnwrapMethod,
    pins: &[Option<Vec2>],
    guess: &[Vec2],
) -> Option<(Vec<Vec2>, UnwrapMethod)> {
    let n = chart.len();
    if !(3..=crate::unwrap::SOLVE_UP_TO).contains(&n) {
        return None;
    }
    let (triangles, real) = real_shapes(mesh, chart);
    let held: Vec<Vec2> = pins.iter().flatten().copied().collect();
    let (lo, hi) = held
        .iter()
        .fold((Vec2::INFINITY, Vec2::NEG_INFINITY), |(lo, hi), v| {
            (lo.min(*v), hi.max(*v))
        });
    let pinned = held.len() >= 2 && (hi - lo).length() > 1e-7;
    // With the pins: the layout's units are the texture's; their scale
    // to real lengths is read from the guess (the piece's UVs).
    let (pins, start, scale) = if pinned {
        let scale = if guess.len() == n {
            let flat: f64 = triangles
                .iter()
                .map(|t| {
                    let (a, b, c) = (guess[t[0]], guess[t[1]], guess[t[2]]);
                    f64::from((b - a).perp_dot(c - a)).abs() / 2.0
                })
                .sum();
            let total: f64 = real.iter().flatten().map(|q| area(q).abs()).sum();
            if flat > 0.0 && total > 0.0 {
                (flat / total).sqrt()
            } else {
                1.0
            }
        } else {
            1.0
        };
        (pins.to_vec(), guess.to_vec(), scale)
    } else {
        // Two far-apart wedges held, at their real distance.
        let (pa, pb) = crate::unwrap::far_pair(mesh, chart)?;
        let p = |w: usize| mesh.position(chart.points[w]).unwrap_or(Vec3::ZERO);
        let mut two = vec![None; n];
        two[pa] = Some(Vec2::ZERO);
        two[pb] = Some(Vec2::new(p(pa).distance(p(pb)), 0.0));
        let start = aligned_guess(guess, pa, pb, p(pa).distance(p(pb)));
        (two, start, 1.0)
    };
    let one = |m: UnwrapMethod| -> Option<Vec<Vec2>> {
        let mut held = pins.clone();
        if m == UnwrapMethod::SizeKeeping && !pinned {
            // One wedge held is enough; the second would bend it.
            if let Some(second) = held.iter().rposition(Option::is_some) {
                held[second] = None;
            }
        }
        by(m, n, &triangles, &real, &held, &pins, &start, scale)
    };
    match method {
        UnwrapMethod::Automatic => {
            let mut best: Option<(f64, Vec<Vec2>, UnwrapMethod)> = None;
            for m in [
                UnwrapMethod::Conformal,
                UnwrapMethod::AngleBased,
                UnwrapMethod::SizeKeeping,
            ] {
                if let Some(at) = one(m) {
                    let score = distortion(&triangles, &real, &at);
                    // Only clearly better counts, so near-ties go to the
                    // first (and rounding never decides).
                    if best
                        .as_ref()
                        .is_none_or(|(s, _, _)| score < *s * 0.99 - 1e-4)
                    {
                        best = Some((score, at, m));
                    }
                }
            }
            best.map(|(_, at, m)| (at, m))
        }
        m => one(m).map(|at| (at, m)),
    }
}

/// `guess` moved, turned and scaled so wedge `pa` is at 0 and `pb` at
/// (`length`, 0); empty when it can't be.
fn aligned_guess(guess: &[Vec2], pa: usize, pb: usize, length: f32) -> Vec<Vec2> {
    if guess.len() <= pa.max(pb) {
        return Vec::new();
    }
    let (ga, gb) = (guess[pa], guess[pb]);
    let from = gb - ga;
    if from.length() <= 1e-9 {
        return Vec::new();
    }
    let k = length / from.length();
    let turn = -from.y.atan2(from.x);
    let (sin, cos) = turn.sin_cos();
    guess
        .iter()
        .map(|g| {
            let d = (*g - ga) * k;
            Vec2::new(cos * d.x - sin * d.y, sin * d.x + cos * d.y)
        })
        .collect()
}

#[cfg(test)]
mod tests;
