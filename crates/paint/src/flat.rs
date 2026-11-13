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
    paint_flat_dab_turned(
        canvas,
        surface,
        slots,
        brush,
        (centre, radius, strength),
        (0.0, 0),
    );
}

/// [`paint_flat_dab`] for a dab turned to `angle` (radians, in texels:
/// x right, y down), numbered `seed` in its stroke (decision 0185).
pub fn paint_flat_dab_turned(
    canvas: &mut StrokeCanvas,
    surface: &PaintSurface,
    slots: &[u32],
    brush: &Brush,
    (centre, radius, strength): (Vec2, f32, f32),
    (angle, seed): (f32, u32),
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
                let offset = p - centre;
                let round = brush.falloff(offset.length(), radius);
                let weight = brush.tip.weight(offset, radius, angle, seed, (x, y), round)
                    * strength.clamp(0.0, 1.0);
                if weight > 0.0 {
                    canvas.cover(x, y, weight, brush);
                }
            }
        }
    }
}

/// Paints one dab of `brush` on a texture that repeats (a Painted box in
/// a material graph, decision 0282): the whole texture is paintable, and
/// a dab reaching past an edge comes back in on the other side, so the
/// paint stays seamless. `centre` is in texels (row 0 at the top; any
/// place, it wraps), `radius` in texels, `strength` 0 to 1, the dab
/// turned to `angle` and numbered `seed` in its stroke, as
/// [`paint_flat_dab_turned`].
pub fn paint_wrapped_dab(
    canvas: &mut StrokeCanvas,
    brush: &Brush,
    (centre, radius, strength): (Vec2, f32, f32),
    (angle, seed): (f32, u32),
) {
    if radius.is_nan() || radius <= 0.0 || !centre.is_finite() {
        return;
    }
    let size = canvas.image().size() as i64;
    let n = size as f32;
    // The dab's middle brought onto the texture.
    let centre = Vec2::new(centre.x.rem_euclid(n), centre.y.rem_euclid(n));
    let reach = (radius + 1.0).ceil() as i64;
    // A dab wider than the texture reaches every texel: each takes its
    // nearest copy of the dab (the one that covers it most).
    let whole = 2 * reach + 1 >= size;
    let (lo, hi) = if whole {
        ((0, 0), (size - 1, size - 1))
    } else {
        (
            (
                centre.x.floor() as i64 - reach,
                centre.y.floor() as i64 - reach,
            ),
            (
                centre.x.floor() as i64 + reach,
                centre.y.floor() as i64 + reach,
            ),
        )
    };
    let nearest = |d: f32| {
        if whole { d - (d / n).round() * n } else { d }
    };
    let strength = strength.clamp(0.0, 1.0);
    for y in lo.1..=hi.1 {
        for x in lo.0..=hi.0 {
            let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let offset = Vec2::new(nearest(p.x - centre.x), nearest(p.y - centre.y));
            let (wx, wy) = (x.rem_euclid(size) as u32, y.rem_euclid(size) as u32);
            let round = brush.falloff(offset.length(), radius);
            if round <= 0.0 {
                continue;
            }
            let weight = brush
                .tip
                .weight(offset, radius, angle, seed, (wx, wy), round)
                * strength;
            if weight > 0.0 {
                canvas.cover(wx, wy, weight, brush);
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

    fn wrapped(size: u32, centre: Vec2, radius: f32) -> Image {
        let mut canvas = StrokeCanvas::new(Image::new(size).unwrap());
        let brush = Brush {
            hardness: 1.0,
            ..Brush::default()
        };
        paint_wrapped_dab(&mut canvas, &brush, (centre, radius, 1.0), (0.0, 0));
        canvas.image().clone()
    }

    #[test]
    fn a_dab_over_a_corner_comes_back_in_every_corner() {
        let image = wrapped(256, Vec2::new(1.0, 1.0), 6.0);
        for (x, y) in [(0, 0), (255, 0), (0, 255), (255, 255), (3, 3)] {
            assert!(image.pixel(x, y)[3] > 0, "({x}, {y})");
        }
        assert_eq!(image.pixel(128, 128)[3], 0);
        // The same dab anywhere a whole texture away paints the same.
        assert_eq!(image, wrapped(256, Vec2::new(257.0, -255.0), 6.0));
    }

    #[test]
    fn a_dab_painted_across_the_join_matches_one_in_the_middle() {
        // Rolled half a texture, a dab on the join is a dab in the middle.
        let edge = wrapped(256, Vec2::new(0.0, 100.0), 10.0);
        let middle = wrapped(256, Vec2::new(128.0, 100.0), 10.0);
        for y in 0..256 {
            for x in 0..256 {
                assert_eq!(
                    edge.pixel(x, y),
                    middle.pixel((x + 128) % 256, y),
                    "({x}, {y})"
                );
            }
        }
    }

    #[test]
    fn a_dab_wider_than_the_texture_covers_it_all_once() {
        let image = wrapped(64, Vec2::new(10.0, 10.0), 500.0);
        for y in 0..64 {
            for x in 0..64 {
                assert_eq!(image.pixel(x, y)[3], 255);
            }
        }
        // Nothing for a dab of no size, or nowhere.
        assert!(wrapped(64, Vec2::splat(5.0), 0.0).is_empty());
        assert!(wrapped(64, Vec2::splat(f32::NAN), 5.0).is_empty());
    }
}
