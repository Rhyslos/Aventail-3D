//! Baked maps as files ([decision 0250](../../../docs/decisions/0250-the-baker.md)):
//! a bake's pixels spread past the edges of the UV pieces (padding, so a
//! game's smaller copies of the texture don't pull in the empty space
//! round them), the rest filled with a neutral colour, as a PNG.

use crate::export::textures::{TextureFormat, encode, pad};

/// The PNG of `image` (a bake: texels off the pieces see-through), its
/// pieces spread `padding` texels outward and everything else `background`.
pub fn bake_png(image: &paint::Image, padding: u32, background: [u8; 3]) -> Option<Vec<u8>> {
    let size = image.size();
    let mut rgba = image.to_rgba();
    let mut covered: Vec<bool> = rgba.chunks_exact(4).map(|p| p[3] > 0).collect();
    pad(&mut rgba, &mut covered, size, padding);
    for (pixel, on) in rgba.chunks_exact_mut(4).zip(&covered) {
        if !*on {
            pixel[..3].copy_from_slice(&background);
        }
        pixel[3] = 255;
    }
    encode(&rgba, size, TextureFormat::Png)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pieces_are_padded_and_the_rest_filled() {
        let mut image = paint::Image::new(16).unwrap();
        image.set_pixel(8, 8, [200, 10, 10, 255]);
        let png = bake_png(&image, 2, [128, 128, 255]).unwrap();
        let read = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(read.get_pixel(8, 8).0, [200, 10, 10, 255]);
        // Two texels out: the piece's colour.
        assert_eq!(read.get_pixel(10, 8).0, [200, 10, 10, 255]);
        // Further: the background, opaque.
        assert_eq!(read.get_pixel(0, 0).0, [128, 128, 255, 255]);
    }

    #[test]
    fn an_empty_bake_is_all_background() {
        let image = paint::Image::new(16).unwrap();
        let png = bake_png(&image, 4, [0, 0, 0]).unwrap();
        let read = image::load_from_memory(&png).unwrap().to_rgba8();
        assert!(read.pixels().all(|p| p.0 == [0, 0, 0, 255]));
    }
}
