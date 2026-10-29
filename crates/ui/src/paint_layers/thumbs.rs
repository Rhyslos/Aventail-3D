//! Small pictures of each layer and its mask for the Paint layers list
//! ([decision 0227](../../../../docs/decisions/0227-paint-layers-on-top.md)),
//! as paint programs show them: what the layer holds at a glance.
//!
//! A picture is made again only when its image changes. Images share
//! their tiles until painted (decision 0188), so comparing the tiles'
//! addresses tells cheaply whether anything was painted since.

use egui::{Color32, ColorImage, TextureHandle, TextureOptions};
use paint::{Image, LayerPath};
use std::collections::HashMap;

/// Pixels across one picture.
pub(crate) const SIDE: usize = 28;

/// Which picture: of a material's layer, or of its mask.
type Key = (String, LayerPath, bool);

/// The pictures shown, each with the tiles it was made from.
#[derive(Default)]
pub(crate) struct Thumbs {
    made: HashMap<Key, (Vec<usize>, TextureHandle)>,
    /// The pictures asked for this frame: the others are let go.
    used: Vec<Key>,
}

impl std::fmt::Debug for Thumbs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Thumbs")
            .field("pictures", &self.made.len())
            .finish()
    }
}

impl Thumbs {
    /// The picture of `image` (the layer at `path` of `material`, or its
    /// mask when `mask` gives the mask's base and inversion), made again
    /// if it changed.
    pub(crate) fn picture(
        &mut self,
        ctx: &egui::Context,
        material: &str,
        path: &[usize],
        image: &Image,
        mask: Option<(bool, bool)>,
    ) -> egui::TextureId {
        let key = (material.to_string(), path.to_vec(), mask.is_some());
        let tiles = fingerprint(image);
        self.used.push(key.clone());
        if let Some((was, handle)) = self.made.get(&key)
            && *was == tiles
        {
            return handle.id();
        }
        let picture = draw(image, mask);
        match self.made.get_mut(&key) {
            Some((was, handle)) => {
                handle.set(picture, TextureOptions::LINEAR);
                *was = tiles;
                handle.id()
            }
            None => {
                let handle =
                    ctx.load_texture("paint layer picture", picture, TextureOptions::LINEAR);
                let id = handle.id();
                self.made.insert(key, (tiles, handle));
                id
            }
        }
    }

    /// Lets go of the pictures not asked for since the last call.
    pub(crate) fn end_frame(&mut self) {
        let used = std::mem::take(&mut self.used);
        self.made.retain(|key, _| used.contains(key));
    }

    /// How many pictures are kept (for tests).
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.made.len()
    }
}

/// The tiles' addresses: the same while nothing is painted.
fn fingerprint(image: &Image) -> Vec<usize> {
    image
        .tiles()
        .iter()
        .map(|tile| {
            tile.as_ref()
                .map_or(0, |t| std::sync::Arc::as_ptr(t) as *const u8 as usize)
        })
        .collect()
}

/// The picture: the image's pixels at `SIDE` points across, over a dark
/// checker where it is clear; for a mask (`Some((white, inverted))`),
/// how much it shows, white to black.
pub(crate) fn draw(image: &Image, mask: Option<(bool, bool)>) -> ColorImage {
    let size = image.size().max(1);
    let mut pixels = Vec::with_capacity(SIDE * SIDE);
    for y in 0..SIDE {
        for x in 0..SIDE {
            let px = ((x as u32 * 2 + 1) * size) / (SIDE as u32 * 2);
            let py = ((y as u32 * 2 + 1) * size) / (SIDE as u32 * 2);
            let at = image.pixel(px, py);
            pixels.push(match mask {
                Some((white, inverted)) => {
                    let v = (paint::set::amount(at, white, inverted) * 255.0).round() as u8;
                    Color32::from_gray(v)
                }
                None => {
                    let back = if (x / 7 + y / 7) % 2 == 0 { 58.0 } else { 44.0 };
                    let a = at[3] as f32 / 255.0;
                    let mix = |c: u8| (c as f32 * a + back * (1.0 - a)).round() as u8;
                    Color32::from_rgb(mix(at[0]), mix(at[1]), mix(at[2]))
                }
            });
        }
    }
    ColorImage::new([SIDE, SIDE], pixels)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clear_image_shows_the_checker_and_paint_shows_its_colour() {
        let mut image = Image::new(64).unwrap();
        let clear = draw(&image, None);
        assert_ne!(clear.pixels[0], clear.pixels[7], "a checker behind");
        for y in 0..64 {
            for x in 0..64 {
                image.set_pixel(x, y, [200, 40, 30, 255]);
            }
        }
        let red = draw(&image, None);
        assert!(
            red.pixels
                .iter()
                .all(|p| *p == Color32::from_rgb(200, 40, 30))
        );
    }

    #[test]
    fn a_mask_shows_its_base_and_what_is_painted_on_it() {
        let mut image = Image::new(64).unwrap();
        assert!(
            draw(&image, Some((true, false)))
                .pixels
                .iter()
                .all(|p| *p == Color32::WHITE)
        );
        assert!(
            draw(&image, Some((false, false)))
                .pixels
                .iter()
                .all(|p| *p == Color32::BLACK)
        );
        assert!(
            draw(&image, Some((false, true)))
                .pixels
                .iter()
                .all(|p| *p == Color32::WHITE)
        );
        for y in 0..64 {
            for x in 0..64 {
                image.set_pixel(x, y, [255, 255, 255, 255]);
            }
        }
        assert!(
            draw(&image, Some((false, false)))
                .pixels
                .iter()
                .all(|p| *p == Color32::WHITE)
        );
    }

    #[test]
    fn a_tiny_or_odd_sized_image_still_makes_a_whole_picture() {
        let image = Image::new(1).unwrap_or_else(|| Image::new(64).unwrap());
        let picture = draw(&image, None);
        assert_eq!(picture.pixels.len(), SIDE * SIDE);
    }

    #[test]
    fn a_picture_is_made_again_only_when_the_image_changes_and_let_go_when_unused() {
        let ctx = egui::Context::default();
        let mut thumbs = Thumbs::default();
        let mut image = Image::new(64).unwrap();
        let first = thumbs.picture(&ctx, "Clay", &[0], &image, None);
        thumbs.end_frame();
        let again = thumbs.picture(&ctx, "Clay", &[0], &image, None);
        assert_eq!(first, again);
        let tiles = fingerprint(&image);
        image.set_pixel(3, 3, [1, 2, 3, 255]);
        assert_ne!(
            tiles,
            fingerprint(&image),
            "painting changes the fingerprint"
        );
        let painted = thumbs.picture(&ctx, "Clay", &[0], &image, None);
        assert_eq!(first, painted, "the same texture, updated in place");
        thumbs.end_frame();
        assert_eq!(thumbs.len(), 1);
        thumbs.end_frame();
        assert_eq!(
            thumbs.len(),
            0,
            "a layer no longer listed loses its picture"
        );
    }
}
