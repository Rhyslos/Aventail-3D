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
    /// Which way its polygon faces.
    pub(crate) normal: Vec3,
    /// Which way the shape is shaded there: smooth across soft edges, as
    /// the viewport shades it (what light and facing go by).
    pub(crate) shading: Vec3,
    /// Where a ray toward the light starts: its place lifted off a
    /// flat-cut curve onto the curve the shading shows, so a low-poly
    /// round shape doesn't shadow itself in steps along its polygons.
    pub(crate) lifted: Vec3,
    /// Its polygon's material slot.
    pub(crate) slot: u32,
    /// A number of its own, for rays that differ texel to texel.
    pub(crate) seed: u32,
    /// Its triangle's index in the surface.
    pub(crate) triangle: u32,
}

/// Every texel of a `size`-pixel texture covered by `surface`'s
/// triangles in `slots`.
pub(crate) fn covered(surface: &PaintSurface, slots: &[u32], size: u32) -> Vec<Texel> {
    let side = size as f32;
    let mut out = Vec::new();
    for (index, t) in surface.triangles().iter().enumerate() {
        if !slots.contains(&t.slot) || t.normal == Vec3::ZERO {
            continue;
        }
        let whole = t.uvs.map(|uv| Vec2::new(uv.x * side, (1.0 - uv.y) * side));
        // Past the square, where the texture repeats (decision 0202).
        for shift in paint::surface::tile_shifts(&whole, side) {
            let texel = whole.map(|p| p + shift);
            cover_one(
                &mut out,
                t,
                surface.shading(index),
                index,
                texel,
                side,
                size,
            );
        }
    }
    out
}

/// The texels triangle `t` (number `index`, with shading normals
/// `shading` at its corners) covers at `texel` (texel space, `side` pixels
/// a square), added to `out`.
fn cover_one(
    out: &mut Vec<Texel>,
    t: &paint::surface::PaintTriangle,
    shading: [Vec3; 3],
    index: usize,
    texel: [Vec2; 3],
    side: f32,
    size: u32,
) {
    {
        let lo = (texel[0].min(texel[1]).min(texel[2]) - Vec2::splat(EDGE_REACH)).max(Vec2::ZERO);
        let hi = (texel[0].max(texel[1]).max(texel[2]) + Vec2::splat(EDGE_REACH))
            .min(Vec2::splat(side - 1.0));
        if lo.x > hi.x || lo.y > hi.y || !lo.is_finite() || !hi.is_finite() {
            return;
        }
        let area = (texel[1] - texel[0]).perp_dot(texel[2] - texel[0]);
        if area.abs() < 1e-9 {
            return;
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
                let at = t.points[0] * w[0] + t.points[1] * w[1] + t.points[2] * w[2];
                let smooth = (shading[0] * w[0] + shading[1] * w[1] + shading[2] * w[2])
                    .normalize_or(t.normal);
                out.push(Texel {
                    x,
                    y,
                    at,
                    normal: t.normal,
                    shading: smooth,
                    lifted: lifted(at, t.points, shading, w),
                    slot: t.slot,
                    seed: y.wrapping_mul(size).wrapping_add(x),
                    triangle: index as u32,
                });
            }
        }
    }
}

/// Point `at` (at weights `w` of a triangle with corners `points` and
/// shading normals `shading`) moved out onto the curved surface the
/// shading suggests: each corner's flat plane lifts it where it lies
/// below, blended by the weights. A flat triangle leaves it where it is.
/// (After Hanika's "Hacking the shadow terminator", 2021.)
fn lifted(at: Vec3, points: [Vec3; 3], shading: [Vec3; 3], w: [f32; 3]) -> Vec3 {
    let mut out = at;
    for i in 0..3 {
        let below = (at - points[i]).dot(shading[i]).min(0.0);
        out -= shading[i] * (below * w[i]);
    }
    if out.is_finite() { out } else { at }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flat_triangle_lifts_nothing() {
        let points = [Vec3::ZERO, Vec3::X, Vec3::Y];
        let at = Vec3::new(0.25, 0.25, 0.0);
        let lift = lifted(at, points, [Vec3::Z; 3], [0.5, 0.25, 0.25]);
        assert!(lift.distance(at) < 1e-6);
    }

    #[test]
    fn a_curved_shading_lifts_the_middle_outward() {
        // Corners leaning out, as on a round shape cut flat.
        let points = [Vec3::new(-1.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), Vec3::Y];
        let lean = |x: f32| Vec3::new(x, 0.0, 1.0).normalize();
        let shading = [lean(-0.5), lean(0.5), Vec3::Z];
        let at = Vec3::new(0.0, 0.0, 0.0);
        let lift = lifted(at, points, shading, [0.5, 0.5, 0.0]);
        assert!(lift.z > 0.1, "{lift}");
        // Never pushed inward, and nothing broken stays put.
        assert!(lift.z >= 0.0);
        let broken = [Vec3::NAN; 3];
        assert_eq!(lifted(at, points, broken, [0.5, 0.5, 0.0]), at);
    }
}
