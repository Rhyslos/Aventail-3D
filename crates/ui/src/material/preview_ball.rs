//! The pictures the preview draws itself, on the processor: the ball
//! for when there is no graphics card, Flat's texture (averaged down when
//! zoomed out, decision 0291) and the checker under an alpha card.

use super::pictures::{byte, colour_at, colour_image};
use crate::theme;
use egui::{Color32, ColorImage};
use glam::Vec3;
use matgraph::Value;

/// How many times the ball wraps the texture round.
const WRAPS: f32 = 2.0;

/// A grey checker filling `rect`, under an alpha card (decision 0276) so
/// its cut-away parts show as see-through.
pub(crate) fn checker(painter: &egui::Painter, rect: egui::Rect) {
    const CELLS: usize = 16;
    painter.rect_filled(rect, 0.0, theme::FIELD);
    let cell = rect.width() / CELLS as f32;
    for y in 0..CELLS {
        for x in (y % 2..CELLS).step_by(2) {
            let min = rect.min + egui::vec2(x as f32, y as f32) * cell;
            painter.rect_filled(
                egui::Rect::from_min_size(min, egui::vec2(cell, cell)),
                0.0,
                theme::TEXT_STRONG.gamma_multiply(0.12),
            );
        }
    }
}

/// `value` as Flat's picture `side` pixels across (at most its own
/// size): every pixel the average of the block it covers, so a texture
/// repeated many times over (zoomed out on Flat) shows what it really
/// looks like from afar, with no shimmer. At its own size or more it is
/// [`colour_image`]'s picture.
pub(crate) fn averaged(value: &Value, side: u32) -> ColorImage {
    let size = value.size().max(1);
    let side = side.clamp(1, size);
    if side >= size || side >= 512 {
        return colour_image(value, side);
    }
    let block = i64::from((size / side).max(1));
    let count = (block * block) as f32;
    let mut pixels = Vec::with_capacity((side * side) as usize);
    for y in 0..i64::from(side) {
        for x in 0..i64::from(side) {
            let mut sum = [0.0f32; 4];
            for dy in 0..block {
                for dx in 0..block {
                    let (px, py) = (x * block + dx, y * block + dy);
                    let [r, g, b] = colour_at(value, px, py);
                    let a = value.material().map_or(1.0, |m| m.opacity_at(px, py));
                    for (s, v) in sum.iter_mut().zip([r, g, b, a]) {
                        *s += v;
                    }
                }
            }
            let [r, g, b, a] = sum.map(|v| byte(v / count));
            pixels.push(Color32::from_rgba_unmultiplied(r, g, b, a));
        }
    }
    ColorImage::new([side as usize, side as usize], pixels)
}

/// `value` on a lit ball `side` pixels across, drawn on the processor for
/// when there is no graphics card: wrapped round it twice, lit from the
/// top left, its bumps from the material's normal, shinier where it is
/// smooth.
pub(crate) fn ball(value: &Value, side: u32) -> ColorImage {
    let size = value.size().max(1) as f32;
    let light = Vec3::new(-0.5, 0.6, 0.7).normalize();
    let half = (light + Vec3::Z).normalize();
    let mut pixels = Vec::with_capacity((side * side) as usize);
    for y in 0..side {
        for x in 0..side {
            let nx = (x as f32 + 0.5) / side as f32 * 2.0 - 1.0;
            let ny = 1.0 - (y as f32 + 0.5) / side as f32 * 2.0;
            let r2 = nx * nx + ny * ny;
            if r2 > 1.0 {
                pixels.push(Color32::TRANSPARENT);
                continue;
            }
            let n = Vec3::new(nx, ny, (1.0 - r2).sqrt());
            let u = 0.5 + n.x.atan2(n.z) / std::f32::consts::TAU;
            let v = n.y.clamp(-1.0, 1.0).acos() / std::f32::consts::PI;
            let (px, py) = ((u * WRAPS * size) as i64, (v * size) as i64);
            let (colour, normal, rough, metal, open) = match value {
                Value::Material(m) => (
                    Vec3::from(m.colour.at(px, py)),
                    Vec3::from(m.normal.at(px, py)),
                    m.roughness.at(px, py),
                    m.metal.at(px, py),
                    m.occlusion.at(px, py),
                ),
                other => (Vec3::from(colour_at(other, px, py)), Vec3::Z, 0.6, 0.0, 1.0),
            };
            // An alpha card is cut out where it is see-through.
            let solid = value.material().map_or(1.0, |m| m.opacity_at(px, py));
            if solid < 0.5 {
                pixels.push(Color32::TRANSPARENT);
                continue;
            }
            let tangent = Vec3::new(n.z, 0.0, -n.x).normalize_or(Vec3::X);
            let up = n.cross(tangent);
            let bent = (tangent * normal.x + up * normal.y + n * normal.z).normalize_or(n);
            let diffuse = bent.dot(light).max(0.0);
            let power = 4.0 + (1.0 - rough) * 60.0;
            let shine = bent.dot(half).max(0.0).powf(power) * (1.0 - rough) * 0.6;
            let base = colour * (0.22 * open + 0.85 * diffuse) * (1.0 - 0.5 * metal);
            let tint = Vec3::ONE.lerp(colour, metal);
            let out = base + tint * shine;
            // Smooth the ball's rim over a pixel.
            let edge = ((1.0 - r2.sqrt()) * side as f32 / 2.0).clamp(0.0, 1.0);
            let alpha = byte(edge);
            pixels.push(Color32::from_rgba_unmultiplied(
                byte(out.x),
                byte(out.y),
                byte(out.z),
                alpha,
            ));
        }
    }
    ColorImage::new([side as usize, side as usize], pixels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use matgraph::{Size, ValueKind};

    #[test]
    fn the_ball_is_round_and_lit_from_the_top_left() {
        let value = Value::flat_grey(ValueKind::Height, Size::nearest(64), 1.0).unwrap();
        let image = ball(&value, 64);
        let at = |x: usize, y: usize| image.pixels[y * 64 + x];
        assert_eq!(at(0, 0), Color32::TRANSPARENT, "corners are outside");
        let top_left = at(20, 20).r();
        let bottom_right = at(44, 44).r();
        assert!(top_left > bottom_right, "{top_left} > {bottom_right}");
    }

    #[test]
    fn a_zoomed_out_picture_is_the_average_of_its_blocks() {
        // Stripes one pixel wide: averaged, an even grey.
        let size = Size::nearest(64);
        let stripes = (0..64 * 64).map(|i| (i % 2) as f32).collect();
        let image = matgraph::Image::from_pixels(size, stripes).unwrap();
        let value = Value::Height(std::sync::Arc::new(image));
        let small = averaged(&value, 16);
        assert_eq!(small.size, [16, 16]);
        for p in &small.pixels {
            assert!((i32::from(p.r()) - 128).abs() <= 1, "{p:?}");
        }
        // Taking every few pixels instead would show only one stripe.
        let picked = colour_image(&value, 16);
        assert!(picked.pixels.iter().all(|p| p.r() == 255 || p.r() == 0));
        // At its own size, the picture as it is.
        assert_eq!(averaged(&value, 64).pixels, colour_image(&value, 64).pixels);
        assert_eq!(averaged(&value, 0).size, [1, 1]);
    }
}
