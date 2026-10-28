//! The texels a shape's polygons cover on the texture, with their place
//! and facing on the shape.

use glam::{Vec2, Vec3};
use paint::PaintSurface;
use paint::surface::barycentric;

/// How far outside a triangle (in texels) a texel is still baked, as
/// painting does, so seams stay covered.
const EDGE_REACH: f32 = 0.75;

/// One covered texel.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Texel {
    pub(crate) x: u32,
    pub(crate) y: u32,
    /// Its place on the shape.
    pub(crate) at: Vec3,
    /// Which way the shape faces there.
    pub(crate) normal: Vec3,
    /// Its polygon's material slot.
    pub(crate) slot: u32,
    /// A number of its own, for rays that differ texel to texel.
    pub(crate) seed: u32,
}

/// Every texel of a `size`-pixel texture covered by `surface`'s
/// triangles in `slots`.
pub(crate) fn covered(surface: &PaintSurface, slots: &[u32], size: u32) -> Vec<Texel> {
    let side = size as f32;
    let mut out = Vec::new();
    for t in surface.triangles() {
        if !slots.contains(&t.slot) || t.normal == Vec3::ZERO {
            continue;
        }
        let texel = t.uvs.map(|uv| Vec2::new(uv.x * side, (1.0 - uv.y) * side));
        let lo = (texel[0].min(texel[1]).min(texel[2]) - Vec2::splat(EDGE_REACH)).max(Vec2::ZERO);
        let hi = (texel[0].max(texel[1]).max(texel[2]) + Vec2::splat(EDGE_REACH))
            .min(Vec2::splat(side - 1.0));
        if lo.x > hi.x || lo.y > hi.y || !lo.is_finite() || !hi.is_finite() {
            continue;
        }
        let area = (texel[1] - texel[0]).perp_dot(texel[2] - texel[0]);
        if area.abs() < 1e-9 {
            continue;
        }
        let sign = area.signum();
        let edges = [(0, 1), (1, 2), (2, 0)].map(|(a, b)| {
            let e = texel[b] - texel[a];
            (texel[a], Vec2::new(-e.y, e.x).normalize_or_zero() * sign)
        });
        for y in lo.y.floor() as u32..=hi.y.ceil() as u32 {
            for x in lo.x.floor() as u32..=hi.x.ceil() as u32 {
                let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                if !edges
                    .iter()
                    .all(|(on, inside)| (p - *on).dot(*inside) >= -EDGE_REACH)
                {
                    continue;
                }
                let Some(w) = barycentric(p, texel) else {
                    continue;
                };
                let w = w.map(|v| v.max(0.0));
                let sum = (w[0] + w[1] + w[2]).max(1e-9);
                let w = w.map(|v| v / sum);
                out.push(Texel {
                    x,
                    y,
                    at: t.points[0] * w[0] + t.points[1] * w[1] + t.points[2] * w[2],
                    normal: t.normal,
                    slot: t.slot,
                    seed: y.wrapping_mul(size).wrapping_add(x),
                });
            }
        }
    }
    out
}
