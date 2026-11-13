//! Veins (Phase 12 milestone 13, decision 0275): each Line piece's groove
//! or ridge for the Height, and its darker or lighter line for the
//! Colours.
//!
//! Each line is followed in short straight steps, and every pixel near a
//! step gets a rounded profile from its exact distance to it (1 on the
//! line, falling to 0 at its edge), so a vein is smooth at any size. Where
//! veins meet, the strongest wins rather than adding up. Pixels wrap round
//! the texture's edges, so veins tile.

use super::allowed;
use crate::drawing::{Drawing, PieceKind};
use crate::sheet::Sheet;
use glam::Vec2;

/// What [`veins`] gives: both empty when no line is a vein.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Veins {
    /// How much each pixel is raised (above 0) or sunk, -1 to 1: add it to
    /// the height, times the full height.
    pub raise: Vec<f32>,
    /// How much lighter (above 0) or darker each pixel is, -1 to 1.
    pub shade: Vec<f32>,
}

impl Veins {
    /// Whether there are no veins.
    pub fn is_empty(&self) -> bool {
        self.raise.is_empty()
    }
}

/// The drawing's veins at `size` × `size` pixels, row by row from the top.
/// Empty when no Line piece is a vein, or for a size of 0 or over
/// [`super::MAX_SIZE`].
pub fn veins(drawing: &Drawing, size: u32) -> Veins {
    veins_on(drawing, None, size)
}

/// [`veins`], laid out on `sheet` when one is given: each vein moved with
/// its card, as wide as the card is sized.
pub fn veins_on(drawing: &Drawing, sheet: Option<&Sheet>, size: u32) -> Veins {
    let lines: Vec<_> = drawing
        .pieces()
        .iter()
        .filter(|p| p.kind == PieceKind::Line)
        .filter(|p| sheet.is_none_or(|s| s.change(p.id).is_some()))
        .filter_map(|p| Some((p, p.vein.fitted().filter(|v| !v.is_none())?)))
        .collect();
    if lines.is_empty() || !allowed(size) {
        return Veins::default();
    }
    let n = size as usize;
    // The strongest ridge, groove, lightening and darkening at each pixel.
    let mut most = vec![[0.0f32; 4]; n * n];
    let px = size as f32;
    for (piece, vein) in lines {
        let scale = sheet.map_or(1.0, |s| s.scale_of(piece.id));
        let half = (vein.width * scale * px * 0.5).max(0.75);
        let strengths = [
            vein.depth.max(0.0),
            (-vein.depth).max(0.0),
            vein.shade.max(0.0),
            (-vein.shade).max(0.0),
        ];
        for path in super::drawn(drawing, sheet, piece) {
            let mut points: Vec<Vec2> = path
                .flattened(0.25 / f64::from(px))
                .into_iter()
                .map(|p| p * px)
                .collect();
            if path.is_closed()
                && let Some(first) = points.first().copied()
            {
                points.push(first);
            }
            for pair in points.windows(2) {
                for (a, b) in steps(pair[0], pair[1], half.max(8.0)) {
                    lay(&mut most, n, a, b, half, strengths);
                }
            }
        }
    }
    Veins {
        raise: most.iter().map(|m| m[0] - m[1]).collect(),
        shade: most.iter().map(|m| m[2] - m[3]).collect(),
    }
}

/// The line from `a` to `b` cut into steps no longer than `longest`.
fn steps(a: Vec2, b: Vec2, longest: f32) -> impl Iterator<Item = (Vec2, Vec2)> {
    let count = ((a.distance(b) / longest).ceil() as usize).clamp(1, 4096);
    (0..count).map(move |i| {
        let t0 = i as f32 / count as f32;
        let t1 = (i + 1) as f32 / count as f32;
        (a.lerp(b, t0), a.lerp(b, t1))
    })
}

/// Lays the rounded profile of the step from `a` to `b` (in pixels), half
/// `half` wide, into `most` at its four strengths.
fn lay(most: &mut [[f32; 4]], n: usize, a: Vec2, b: Vec2, half: f32, strengths: [f32; 4]) {
    if !(a.is_finite() && b.is_finite()) {
        return;
    }
    let lo = (a.min(b) - Vec2::splat(half + 1.0)).floor();
    let hi = (a.max(b) + Vec2::splat(half + 1.0)).ceil();
    let along = b - a;
    let long = along.length_squared();
    let side = n as i64;
    for y in lo.y as i64..hi.y as i64 {
        for x in lo.x as i64..hi.x as i64 {
            let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let t = if long > 0.0 {
                ((p - a).dot(along) / long).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let d = p.distance(a + along * t);
            if d >= half {
                continue;
            }
            let r = d / half;
            let profile = 1.0 - r * r;
            let at = (y.rem_euclid(side) * side + x.rem_euclid(side)) as usize;
            for (m, s) in most[at].iter_mut().zip(strengths) {
                *m = m.max(s * profile);
            }
        }
    }
}
