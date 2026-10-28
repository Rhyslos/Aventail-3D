//! Painting flat in the UV Editor ([decision 0180](../../../docs/decisions/0180-painting-in-the-uv-editor.md)):
//! a dab is a circle on the texture itself, painting only the texels of
//! the shape's UV pieces that belong to the texture set (and those just
//! outside their edges, as on the model), so paint never lands in the
//! empty space between pieces.

use crate::brush::Brush;
use crate::stroke::StrokeCanvas;
use crate::surface::{PaintSurface, barycentric};
use glam::Vec2;

/// How far outside a piece's edge (in texels) a texel is still painted.
const EDGE_REACH: f32 = 0.75;

/// Paints one dab of `brush` at `centre` (in texels, row 0 at the top)
/// with a radius of `radius` texels and `strength` (0 to 1, the pen's
/// pressure on the flow), on the parts of `surface`'s pieces whose
/// material slot is one of `slots`.
pub fn paint_flat_dab(
    canvas: &mut StrokeCanvas,
    surface: &PaintSurface,
    slots: &[u32],
    brush: &Brush,
    (centre, radius, strength): (Vec2, f32, f32),
) {
    if radius.is_nan() || radius <= 0.0 || !centre.is_finite() {
        return;
    }
    let size = canvas.image().size() as f32;
    let reach = Vec2::splat(radius + 1.0);
    for t in surface.triangles() {
        if !slots.contains(&t.slot) {
            continue;
        }
        let texel = t.uvs.map(|uv| Vec2::new(uv.x * size, (1.0 - uv.y) * size));
        let lo = (texel[0].min(texel[1]).min(texel[2]) - Vec2::splat(EDGE_REACH))
            .max(centre - reach)
            .max(Vec2::ZERO);
        let hi = (texel[0].max(texel[1]).max(texel[2]) + Vec2::splat(EDGE_REACH))
            .min(centre + reach)
            .min(Vec2::splat(size - 1.0));
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
                let weight = brush.falloff(p.distance(centre), radius) * strength.clamp(0.0, 1.0);
                if weight > 0.0 {
                    canvas.cover(x, y, weight, brush);
                }
            }
        }
    }
}

/// The triangle (by index) whose UVs cover `uv` on `surface`, if any.
pub fn triangle_at(surface: &PaintSurface, uv: Vec2) -> Option<usize> {
    surface
        .triangles()
        .iter()
        .position(|t| barycentric(uv, t.uvs).is_some_and(|w| w.iter().all(|v| *v >= -1e-4)))
}

/// The material slot of the piece under `uv` on `surface`, if any.
pub fn slot_at(surface: &PaintSurface, uv: Vec2) -> Option<u32> {
    triangle_at(surface, uv).map(|i| surface.triangles()[i].slot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::Image;
    use mesh::PrimitiveKind;

    #[test]
    fn a_flat_dab_paints_only_inside_the_pieces() {
        let surface = PaintSurface::of(&PrimitiveKind::Cube.standard().mesh());
        let mut canvas = StrokeCanvas::new(Image::new(256).unwrap());
        let brush = Brush::default();
        // A dab over the whole texture: only the cube's pieces are painted.
        paint_flat_dab(
            &mut canvas,
            &surface,
            &[0],
            &brush,
            (Vec2::splat(128.0), 400.0, 1.0),
        );
        let image = canvas.image();
        let size = image.size() as f32;
        let mut inside = 0;
        let mut outside = 0;
        for y in 0..image.size() {
            for x in 0..image.size() {
                let uv = Vec2::new((x as f32 + 0.5) / size, 1.0 - (y as f32 + 0.5) / size);
                let painted = image.pixel(x, y)[3] > 0;
                let in_piece = slot_at(&surface, uv).is_some();
                match (painted, in_piece) {
                    (true, true) => inside += 1,
                    (true, false) => outside += 1,
                    _ => {}
                }
            }
        }
        assert!(inside > 1000);
        // Only a thin rim just outside the pieces' edges.
        assert!(outside < inside / 10, "{outside} outside, {inside} inside");
    }

    #[test]
    fn a_small_flat_dab_is_a_disc() {
        let surface = PaintSurface::of(&PrimitiveKind::Cube.standard().mesh());
        // A point well inside the first piece.
        let t = surface.triangles()[0];
        let uv = (t.uvs[0] + t.uvs[1] + t.uvs[2]) / 3.0;
        let mut canvas = StrokeCanvas::new(Image::new(512).unwrap());
        let brush = Brush {
            hardness: 1.0,
            ..Brush::default()
        };
        let centre = Vec2::new(uv.x * 512.0, (1.0 - uv.y) * 512.0);
        paint_flat_dab(&mut canvas, &surface, &[0], &brush, (centre, 5.0, 1.0));
        let image = canvas.image();
        let count = (0..512u32)
            .flat_map(|y| (0..512u32).map(move |x| (x, y)))
            .filter(|&(x, y)| image.pixel(x, y)[3] > 0)
            .count() as f32;
        let disc = std::f32::consts::PI * 25.0;
        assert!((count - disc).abs() < disc * 0.3, "{count}");
    }

    #[test]
    fn other_slots_and_odd_input_paint_nothing() {
        let surface = PaintSurface::of(&PrimitiveKind::Cube.standard().mesh());
        let mut canvas = StrokeCanvas::new(Image::new(64).unwrap());
        let brush = Brush::default();
        paint_flat_dab(
            &mut canvas,
            &surface,
            &[7],
            &brush,
            (Vec2::splat(32.0), 50.0, 1.0),
        );
        paint_flat_dab(
            &mut canvas,
            &surface,
            &[0],
            &brush,
            (Vec2::splat(f32::NAN), 5.0, 1.0),
        );
        paint_flat_dab(
            &mut canvas,
            &surface,
            &[0],
            &brush,
            (Vec2::splat(32.0), 0.0, 1.0),
        );
        assert!(canvas.image().is_empty());
        assert!(slot_at(&surface, Vec2::splat(-5.0)).is_none());
    }

    #[test]
    fn a_weaker_dab_covers_less() {
        let surface = PaintSurface::of(&PrimitiveKind::Cube.standard().mesh());
        let t = surface.triangles()[0];
        let uv = (t.uvs[0] + t.uvs[1] + t.uvs[2]) / 3.0;
        let centre = Vec2::new(uv.x * 256.0, (1.0 - uv.y) * 256.0);
        let brush = Brush {
            hardness: 1.0,
            ..Brush::default()
        };
        let alpha = |strength: f32| {
            let mut canvas = StrokeCanvas::new(Image::new(256).unwrap());
            paint_flat_dab(&mut canvas, &surface, &[0], &brush, (centre, 4.0, strength));
            canvas.image().pixel(centre.x as u32, centre.y as u32)[3]
        };
        assert!(alpha(0.25) < alpha(1.0));
        assert!(alpha(0.25) > 0);
        assert_eq!(alpha(0.0), 0);
    }
}
