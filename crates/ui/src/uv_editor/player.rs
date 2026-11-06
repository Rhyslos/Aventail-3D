//! Texels per screen pixel in the UV Editor ([decision 0247](../../../../docs/decisions/0247-player-view-uvs.md)):
//! each polygon coloured by how many texture pixels land on one screen
//! pixel from the player's nearest camera: blue where the texture will look
//! blurry (under half), green where it is about right, red where texture
//! is wasted (over two), grey where the player never sees it.

use super::UvEditor;
use super::painting::add_polygon;
use super::shape::UvShape;
use bake::player::Seen;
use egui::{Color32, Pos2, Rect};
use scene::FaceId;
use std::collections::BTreeMap;

/// Texels per screen pixel for a polygon of UV area `flat` (texture
/// square units) and real area `real` (square metres), on a texture
/// `size` pixels across, seen at `pixels_per_metre`; `None` when unseen
/// or without size.
pub(crate) fn texels_per_pixel(flat: f32, real: f32, size: u32, pixels_per_metre: f32) -> Option<f32> {
    if pixels_per_metre <= 0.0 || real <= 1e-12 || !(flat.is_finite() && real.is_finite()) {
        return None;
    }
    let texels_per_metre = (flat.max(0.0) / real).sqrt() * size as f32;
    Some(texels_per_metre / pixels_per_metre)
}

/// The colour for a ratio: blue under half, green about one, red over two.
pub(crate) fn ratio_color(ratio: Option<f32>) -> Color32 {
    let Some(r) = ratio else {
        return Color32::from_gray(110);
    };
    let t = r.max(1e-6).log2().clamp(-1.0, 1.0);
    let green = Color32::from_rgb(70, 190, 90);
    if t < 0.0 {
        lerp(green, Color32::from_rgb(60, 110, 230), -t)
    } else {
        lerp(green, Color32::from_rgb(225, 70, 60), t)
    }
}

fn lerp(a: Color32, b: Color32, t: f32) -> Color32 {
    let mix = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
    Color32::from_rgb(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()))
}

impl UvEditor {
    /// Whether polygons are coloured by texels per screen pixel.
    pub(crate) fn player_texels(&self) -> bool {
        self.player_texels
    }

    /// Switches the colouring on or off.
    pub(crate) fn set_player_texels(&mut self, on: bool) {
        self.player_texels = on;
    }

    /// Colours `shape`'s polygons by texels per screen pixel.
    pub(super) fn paint_player(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        shape: &UvShape,
        seen: &BTreeMap<FaceId, Seen>,
        size: u32,
    ) {
        let mut tint = egui::Mesh::default();
        for ((face, points), (flat, real)) in shape.polygons.iter().zip(&shape.areas) {
            let ratio = seen
                .get(face)
                .and_then(|s| texels_per_pixel(*flat, *real, size, s.pixels_per_metre));
            let corners: Vec<Pos2> = points.iter().map(|(_, uv)| self.to_screen(rect, *uv)).collect();
            add_polygon(&mut tint, &corners, ratio_color(ratio).gamma_multiply(0.55));
        }
        painter.add(tint);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ratio_is_texture_pixels_over_screen_pixels() {
        // A square metre with the whole of a 1024 texture, seen at 512
        // screen pixels a metre: two texels per screen pixel.
        assert_eq!(texels_per_pixel(1.0, 1.0, 1024, 512.0), Some(2.0));
        assert_eq!(texels_per_pixel(0.25, 1.0, 1024, 512.0), Some(1.0));
        assert_eq!(texels_per_pixel(1.0, 1.0, 1024, 0.0), None);
        assert_eq!(texels_per_pixel(1.0, 0.0, 1024, 100.0), None);
        assert_eq!(texels_per_pixel(f32::NAN, 1.0, 1024, 100.0), None);
    }

    #[test]
    fn colours_run_blue_green_red() {
        let blurry = ratio_color(Some(0.25));
        let right = ratio_color(Some(1.0));
        let wasted = ratio_color(Some(4.0));
        assert!(blurry.b() > blurry.r() && blurry.b() > blurry.g());
        assert!(right.g() > right.r() && right.g() > right.b());
        assert!(wasted.r() > wasted.g() && wasted.r() > wasted.b());
        assert_eq!(ratio_color(None), Color32::from_gray(110));
    }
}
