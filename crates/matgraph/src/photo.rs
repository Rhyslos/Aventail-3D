//! A photo or a painting brought in on an Image box (Phase 12 milestone
//! 21, [decision 0284](../../../docs/decisions/0284-image-to-material.md)):
//! the picture a material is made from.
//!
//! A [`Photo`] keeps its pixels as sRGB bytes, three a pixel (red, green,
//! blue: a material's colour has no see-through parts), row by row from
//! the top, at most [`MAX_PHOTO_SIDE`] pixels across or down. It may be
//! any shape: the Image box takes a square part of it, chosen by its
//! settings ([`Photo::square`]), and scales that to whatever size the
//! graph is worked out at ([`Photo::sampled`]).
//!
//! The crate never reads image files itself. Whoever brings a photo in
//! (`fileio`) decodes it and hands over the pixels with the bytes it will
//! save (`packed`, a JPEG), so saving never packs the photo again and a
//! photo saved and opened again has exactly the same pixels. The
//! [`key`](Photo::key) is a hash of the pixels, so a box's result is only
//! worked out again when its photo changes.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// The widest or tallest a photo is kept, in pixels: bigger ones are made
/// smaller when brought in, so projects stay small.
pub const MAX_PHOTO_SIDE: u32 = 2048;

/// The longest name kept with a photo (its file's name), in bytes.
pub const MAX_PHOTO_NAME: usize = 200;

/// The most bytes a photo's packed form may take (16 MB).
pub const MAX_PHOTO_PACKED: usize = 16 << 20;

/// The smallest share of a photo's shorter side the square can be (the
/// Image box's Square size).
pub const MIN_PART: f32 = 0.05;

/// A photo or painting brought in on an Image box.
#[derive(Clone)]
pub struct Photo {
    width: u32,
    height: u32,
    rgb: Arc<Vec<u8>>,
    packed: Arc<Vec<u8>>,
    name: String,
    key: u64,
}

impl std::fmt::Debug for Photo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Photo")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("packed", &self.packed.len())
            .field("name", &self.name)
            .field("key", &self.key)
            .finish()
    }
}

impl PartialEq for Photo {
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width
            && self.height == other.height
            && self.key == other.key
            && self.name == other.name
            && (Arc::ptr_eq(&self.rgb, &other.rgb) || self.rgb == other.rgb)
    }
}

impl Photo {
    /// A photo `width` × `height` pixels from sRGB bytes `rgb` (three a
    /// pixel, row by row from the top), with the bytes it is saved as
    /// (`packed`, which may be empty in tests) and its file's `name`
    /// (cut to [`MAX_PHOTO_NAME`]); or why it can't be one, as a file's
    /// would be checked.
    pub fn new(
        width: u32,
        height: u32,
        rgb: Vec<u8>,
        packed: Vec<u8>,
        name: &str,
    ) -> Result<Self, &'static str> {
        if width == 0 || height == 0 || width > MAX_PHOTO_SIDE || height > MAX_PHOTO_SIDE {
            return Err(words::text!(
                "a picture on an Image box is bigger than the app keeps, or has no pixels"
            ));
        }
        if rgb.len() != width as usize * height as usize * 3 {
            return Err(words::text!(
                "a picture on an Image box has the wrong number of pixels"
            ));
        }
        if packed.len() > MAX_PHOTO_PACKED {
            return Err(words::text!(
                "a picture on an Image box takes more room than the app keeps"
            ));
        }
        let mut hasher = DefaultHasher::new();
        (width, height).hash(&mut hasher);
        rgb.hash(&mut hasher);
        Ok(Self {
            width,
            height,
            rgb: Arc::new(rgb),
            packed: Arc::new(packed),
            name: cut(name, MAX_PHOTO_NAME),
            key: hasher.finish(),
        })
    }

    /// Pixels across.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Pixels down.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Its pixels: sRGB bytes, three a pixel, row by row from the top.
    pub fn rgb(&self) -> &[u8] {
        &self.rgb
    }

    /// The bytes it is saved as (a JPEG, from whoever brought it in).
    pub fn packed(&self) -> &[u8] {
        &self.packed
    }

    /// Its file's name, for saying which picture a box holds.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// A hash of its size and pixels: the same for the same pixels,
    /// wherever they come from.
    pub fn key(&self) -> u64 {
        self.key
    }

    /// Roughly how much memory it takes, in bytes.
    pub fn memory_size(&self) -> usize {
        96 + self.rgb.len() + self.packed.len() + self.name.len()
    }

    /// Whether `a` and `b` share their pixels in memory (an undo step
    /// changing one into the other holds on to no more for them).
    pub fn shares(a: &Photo, b: &Photo) -> bool {
        Arc::ptr_eq(&a.rgb, &b.rgb)
    }

    /// The pixel at column `x`, row `y` (kept inside the picture), sRGB
    /// 0 to 1.
    pub fn pixel(&self, x: i64, y: i64) -> [f32; 3] {
        let x = x.clamp(0, i64::from(self.width) - 1) as usize;
        let y = y.clamp(0, i64::from(self.height) - 1) as usize;
        let at = (y * self.width as usize + x) * 3;
        [
            f32::from(self.rgb[at]) / 255.0,
            f32::from(self.rgb[at + 1]) / 255.0,
            f32::from(self.rgb[at + 2]) / 255.0,
        ]
    }

    /// The square part of the picture taken: `part` of its shorter side
    /// across (from [`MIN_PART`] to 1), its left edge `across` of the way
    /// from the far left to the far right it can go, its top `down` of the
    /// way from the top to the bottom (0 to 1 each). Its left, top and
    /// side, in pixels.
    pub fn square(&self, part: f32, across: f32, down: f32) -> (f32, f32, f32) {
        let tidy = |v: f32, lo: f32, standard: f32| {
            if v.is_finite() {
                v.clamp(lo, 1.0)
            } else {
                standard
            }
        };
        let (w, h) = (self.width as f32, self.height as f32);
        let side = w.min(h) * tidy(part, MIN_PART, 1.0);
        let left = (w - side) * tidy(across, 0.0, 0.5);
        let top = (h - side) * tidy(down, 0.0, 0.5);
        (left, top, side)
    }

    /// The square part `square` (left, top, side in pixels, from
    /// [`Self::square`]) at `size` pixels across, row by row from the top,
    /// sRGB 0 to 1. Each pixel is the average of the part of the picture
    /// it covers (up to 8 × 8 blended samples), so a large photo made
    /// small doesn't shimmer.
    pub fn sampled(&self, square: (f32, f32, f32), size: u32) -> Vec<[f32; 3]> {
        let n = size.max(1) as usize;
        let (left, top, side) = square;
        let step = side / n as f32;
        let samples = (step.ceil() as usize).clamp(1, 8);
        let share = 1.0 / (samples * samples) as f32;
        let mut out = vec![[0.0f32; 3]; n * n];
        for (i, pixel) in out.iter_mut().enumerate() {
            let (px, py) = ((i % n) as f32, (i / n) as f32);
            let mut sum = [0.0f32; 3];
            for sy in 0..samples {
                for sx in 0..samples {
                    let fx = left + (px + (sx as f32 + 0.5) / samples as f32) * step - 0.5;
                    let fy = top + (py + (sy as f32 + 0.5) / samples as f32) * step - 0.5;
                    let c = self.blended(fx, fy);
                    for k in 0..3 {
                        sum[k] += c[k];
                    }
                }
            }
            *pixel = sum.map(|v| (v * share).clamp(0.0, 1.0));
        }
        out
    }

    /// The colour at `(x, y)` in pixels (0 the first pixel's middle),
    /// blended between the four nearest, kept inside the picture.
    fn blended(&self, x: f32, y: f32) -> [f32; 3] {
        let (x0, y0) = (x.floor(), y.floor());
        let (tx, ty) = (x - x0, y - y0);
        let (x0, y0) = (x0 as i64, y0 as i64);
        let (a, b, c, d) = (
            self.pixel(x0, y0),
            self.pixel(x0 + 1, y0),
            self.pixel(x0, y0 + 1),
            self.pixel(x0 + 1, y0 + 1),
        );
        [0, 1, 2].map(|k| {
            let top = a[k] + (b[k] - a[k]) * tx;
            let bottom = c[k] + (d[k] - c[k]) * tx;
            top + (bottom - top) * ty
        })
    }
}

/// `text` cut to at most `most` bytes, on a character boundary.
fn cut(text: &str, most: usize) -> String {
    if text.len() <= most {
        return text.to_string();
    }
    let mut end = most;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_string()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A photo `w` × `h` whose red rises left to right and green top to
    /// bottom.
    pub(crate) fn ramp(w: u32, h: u32) -> Photo {
        let mut rgb = Vec::with_capacity((w * h * 3) as usize);
        for y in 0..h {
            for x in 0..w {
                rgb.push((x * 255 / (w - 1).max(1)) as u8);
                rgb.push((y * 255 / (h - 1).max(1)) as u8);
                rgb.push(100);
            }
        }
        Photo::new(w, h, rgb, Vec::new(), "ramp.png").unwrap()
    }

    #[test]
    fn damaged_photos_are_refused() {
        assert!(Photo::new(0, 4, Vec::new(), Vec::new(), "x").is_err());
        assert!(Photo::new(MAX_PHOTO_SIDE + 1, 1, vec![0; 3 * 2049], Vec::new(), "x").is_err());
        assert!(Photo::new(2, 2, vec![0; 11], Vec::new(), "x").is_err());
        assert!(Photo::new(2, 2, vec![0; 12], vec![0; MAX_PHOTO_PACKED + 1], "x").is_err());
        assert!(Photo::new(2, 2, vec![0; 12], Vec::new(), "x").is_ok());
    }

    #[test]
    fn long_names_are_cut_on_a_character() {
        let name = "é".repeat(150);
        let p = Photo::new(1, 1, vec![1, 2, 3], Vec::new(), &name).unwrap();
        assert!(p.name().len() <= MAX_PHOTO_NAME);
        assert!(p.name().chars().all(|c| c == 'é'));
    }

    #[test]
    fn the_key_follows_the_pixels() {
        let a = ramp(8, 4);
        assert_eq!(a.key(), ramp(8, 4).key());
        assert_eq!(a, ramp(8, 4));
        assert_ne!(a.key(), ramp(4, 8).key());
        let mut rgb = a.rgb().to_vec();
        rgb[5] ^= 1;
        let b = Photo::new(8, 4, rgb, Vec::new(), "ramp.png").unwrap();
        assert_ne!(a.key(), b.key());
        assert_ne!(a, b);
        assert!(Photo::shares(&a, &a.clone()));
        assert!(!Photo::shares(&a, &ramp(8, 4)));
    }

    #[test]
    fn the_square_sits_where_asked_inside_the_photo() {
        let p = ramp(300, 100);
        assert_eq!(p.square(1.0, 0.0, 0.5), (0.0, 0.0, 100.0));
        assert_eq!(p.square(1.0, 1.0, 0.5), (200.0, 0.0, 100.0));
        assert_eq!(p.square(1.0, 0.5, 0.5), (100.0, 0.0, 100.0));
        let (l, t, s) = p.square(0.5, 1.0, 1.0);
        assert_eq!((l, t, s), (250.0, 50.0, 50.0));
        // Out of range or not a number: kept in.
        let (_, _, s) = p.square(0.0, f32::NAN, 7.0);
        assert!((s - 100.0 * MIN_PART).abs() < 1e-3);
        let (l, t, s) = p.square(f32::NAN, -3.0, 2.0);
        assert_eq!((l, t, s), (0.0, 0.0, 100.0));
    }

    #[test]
    fn sampling_scales_the_square_to_any_size() {
        let p = ramp(64, 32);
        // The left square: red rises across it, green down.
        let s = p.sampled(p.square(1.0, 0.0, 0.0), 16);
        assert_eq!(s.len(), 256);
        assert!(s[0][0] < 0.1 && s[0][1] < 0.1);
        assert!(s[15][0] > 0.4 && s[15][0] < 0.55, "{:?}", s[15]);
        assert!(s[255][1] > 0.9);
        // Larger than the photo: blended, never outside 0 to 1.
        let big = p.sampled(p.square(1.0, 1.0, 0.0), 128);
        assert!(big.iter().flatten().all(|v| (0.0..=1.0).contains(v)));
        assert!(big[127][0] > 0.95);
        // The average is kept when made small.
        let mean = |v: &[[f32; 3]]| v.iter().map(|c| c[1]).sum::<f32>() / v.len() as f32;
        let whole = p.sampled(p.square(1.0, 0.5, 0.5), 32);
        let small = p.sampled(p.square(1.0, 0.5, 0.5), 4);
        assert!((mean(&whole) - mean(&small)).abs() < 0.02);
    }
}
