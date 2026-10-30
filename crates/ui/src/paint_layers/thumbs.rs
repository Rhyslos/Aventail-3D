//! Small pictures of each layer and its mask for the Paint layers list
//! ([decision 0227](../../../../docs/decisions/0227-paint-layers-on-top.md)),
//! as paint programs show them: what the layer holds at a glance.
//!
//! A picture is made again only when its image changes. Images share
//! their tiles until painted (decision 0188), so comparing the tiles of
//! the image it was made from (kept, so its tiles stay alive and can't be
//! confused with new ones at the same address) with the image now tells
//! cheaply, without copying anything, whether anything was painted since.
//! A mask's picture also follows its white or black start and Invert.

use egui::{Color32, ColorImage, TextureHandle, TextureOptions};
use paint::{Image, LayerPath};

/// Pixels across one picture.
pub(crate) const SIDE: usize = 28;

/// One picture shown: whose it is, what it was made from, and whether it
/// was asked for since the last [`Thumbs::end_frame`].
struct Shown {
    material: String,
    path: LayerPath,
    /// For a mask's picture, the mask's (white, inverted); `None` for the
    /// layer's own.
    mask: Option<(bool, bool)>,
    image: Image,
    handle: TextureHandle,
    used: bool,
}

/// The pictures shown.
#[derive(Default)]
pub(crate) struct Thumbs {
    shown: Vec<Shown>,
}

impl std::fmt::Debug for Thumbs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Thumbs")
            .field("pictures", &self.shown.len())
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
        let found = self.shown.iter_mut().find(|s| {
            s.material == material
                && s.path.as_slice() == path
                && s.mask.is_some() == mask.is_some()
        });
        match found {
            Some(shown) => {
                shown.used = true;
                if shown.mask != mask || !same_tiles(&shown.image, image) {
                    shown.handle.set(draw(image, mask), TextureOptions::LINEAR);
                    shown.image = image.clone();
                    shown.mask = mask;
                }
                shown.handle.id()
            }
            None => {
                let handle = ctx.load_texture(
                    "paint layer picture",
                    draw(image, mask),
                    TextureOptions::LINEAR,
                );
                let id = handle.id();
                self.shown.push(Shown {
                    material: material.to_string(),
                    path: path.to_vec(),
                    mask,
                    image: image.clone(),
                    handle,
                    used: true,
                });
                id
            }
        }
    }

    /// Lets go of the pictures not asked for since the last call.
    pub(crate) fn end_frame(&mut self) {
        self.shown.retain(|s| s.used);
        for shown in &mut self.shown {
            shown.used = false;
        }
    }

    /// How many pictures are kept (for tests).
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.shown.len()
    }
}

/// Whether `a` and `b` are the same pixels for certain: the same size and
/// the very same tiles.
fn same_tiles(a: &Image, b: &Image) -> bool {
    a.size() == b.size()
        && a.tiles().iter().zip(b.tiles()).all(|pair| match pair {
            (Some(x), Some(y)) => std::sync::Arc::ptr_eq(x, y),
            (None, None) => true,
            _ => false,
        })
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
        let before = image.clone();
        image.set_pixel(3, 3, [1, 2, 3, 255]);
        assert!(!same_tiles(&before, &image), "painting changes a tile");
        assert!(
            same_tiles(&image, &image.clone()),
            "a copy shares its tiles"
        );
        assert!(thumbs.shown[0].image == before, "the old picture's image");
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

    #[test]
    fn a_mask_picture_follows_invert_and_its_start() {
        // A review found the picture kept the old state after Invert.
        let ctx = egui::Context::default();
        let mut thumbs = Thumbs::default();
        let image = Image::new(64).unwrap();
        let white = thumbs.picture(&ctx, "Clay", &[0], &image, Some((true, false)));
        assert_eq!(thumbs.shown[0].mask, Some((true, false)));
        let inverted = thumbs.picture(&ctx, "Clay", &[0], &image, Some((true, true)));
        assert_eq!(white, inverted, "the same picture, made again");
        assert_eq!(thumbs.shown[0].mask, Some((true, true)));
        // A black mask moving into a white one's place (after a delete).
        thumbs.picture(
            &ctx,
            "Clay",
            &[0],
            &Image::new(64).unwrap(),
            Some((false, false)),
        );
        assert_eq!(thumbs.shown[0].mask, Some((false, false)));
        // The layer's own picture and its mask's are kept apart.
        thumbs.picture(&ctx, "Clay", &[0], &image, None);
        assert_eq!(thumbs.len(), 2);
    }
}
