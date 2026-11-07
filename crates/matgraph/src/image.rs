//! The square pictures boxes make, and the sizes a graph is worked out at.
//!
//! Every image **wraps round**: reading past its right edge reads its left
//! edge, and past the bottom reads the top ([`Image::at`]). Boxes that look
//! at neighbouring pixels (a blur, a normal from a height) read through
//! it, so nothing they make has a seam, and a graph's textures tile
//! seamlessly by default.
//!
//! Row 0 is the top of the texture, as in the painter. Colours are sRGB,
//! 0 to 1, as picked (the hand-painted look mixes them as seen).

use crate::refusal::{Fix, Refusal};
use words::trf;

/// A square picture of `P` pixels, row by row from the top.
#[derive(Debug, Clone, PartialEq)]
pub struct Image<P> {
    size: u32,
    pixels: Vec<P>,
}

/// Greys, 0 (black) to 1 (white): shapes and heights.
pub type Grey = Image<f32>;
/// Colours (sRGB red, green and blue, 0 to 1), or directions (normals).
pub type Rgb = Image<[f32; 3]>;

impl<P: Copy> Image<P> {
    /// An image of one value everywhere.
    pub fn filled(size: Size, value: P) -> Self {
        let n = size.get() as usize;
        Self {
            size: size.get(),
            pixels: vec![value; n * n],
        }
    }

    /// An image from its pixels, row by row from the top; `None` unless
    /// there are exactly `size × size` of them.
    pub fn from_pixels(size: Size, pixels: Vec<P>) -> Option<Self> {
        let n = size.get() as usize;
        (pixels.len() == n * n).then_some(Self {
            size: size.get(),
            pixels,
        })
    }

    /// Pixels across (and down).
    pub fn size(&self) -> u32 {
        self.size
    }

    /// Every pixel, row by row from the top.
    pub fn pixels(&self) -> &[P] {
        &self.pixels
    }

    /// The pixel at column `x`, row `y`, wrapping round past any edge, so
    /// any whole numbers can be asked for.
    pub fn at(&self, x: i64, y: i64) -> P {
        let n = i64::from(self.size);
        let (x, y) = (x.rem_euclid(n), y.rem_euclid(n));
        self.pixels[(y * n + x) as usize]
    }

    /// How much memory its pixels take, in bytes.
    pub fn memory_size(&self) -> usize {
        self.pixels.len() * std::mem::size_of::<P>()
    }

    /// The same picture with `f` applied to every pixel.
    pub fn map<Q: Copy>(&self, f: impl Fn(P) -> Q) -> Image<Q> {
        Image {
            size: self.size,
            pixels: self.pixels.iter().map(|p| f(*p)).collect(),
        }
    }
}

impl Grey {
    /// The grey at a place on the texture (`u` across, `v` down, a whole
    /// texture each, wrapping), blended between the four nearest pixels.
    pub fn sample(&self, u: f32, v: f32) -> f32 {
        let n = self.size as f32;
        let (x, y) = (u * n - 0.5, v * n - 0.5);
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let (x0, y0) = (x0 as i64, y0 as i64);
        let top = self.at(x0, y0) * (1.0 - fx) + self.at(x0 + 1, y0) * fx;
        let bottom = self.at(x0, y0 + 1) * (1.0 - fx) + self.at(x0 + 1, y0 + 1) * fx;
        top * (1.0 - fy) + bottom * fy
    }
}

impl Rgb {
    /// The colour at a place on the texture, as [`Grey::sample`].
    pub fn sample_rgb(&self, u: f32, v: f32) -> [f32; 3] {
        let n = self.size as f32;
        let (x, y) = (u * n - 0.5, v * n - 0.5);
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let (x0, y0) = (x0 as i64, y0 as i64);
        let mix = |a: [f32; 3], b: [f32; 3], t: f32| {
            [
                a[0] + (b[0] - a[0]) * t,
                a[1] + (b[1] - a[1]) * t,
                a[2] + (b[2] - a[2]) * t,
            ]
        };
        let top = mix(self.at(x0, y0), self.at(x0 + 1, y0), fx);
        let bottom = mix(self.at(x0, y0 + 1), self.at(x0 + 1, y0 + 1), fx);
        mix(top, bottom, fy)
    }
}

/// The size a graph is worked out at: pixels across a square texture, a
/// power of two from [`Size::MIN`] to [`Size::MAX`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Size(u32);

impl Size {
    /// The smallest size: below it a pattern can't be seen at all.
    pub const MIN: u32 = 16;
    /// The largest size: one grey picture at it takes 64 MB, and a whole
    /// material about 1 GB.
    pub const MAX: u32 = 4096;
    /// The size the boxes' pictures are worked out at while you work.
    pub const PREVIEW: Size = Size(256);

    /// `pixels` across, or why it can't be used, with the nearest size
    /// that can be offered as a fix.
    pub fn new(pixels: u32) -> Result<Self, Refusal> {
        let nearest = Self::nearest(pixels).get();
        let reason = if pixels < Self::MIN {
            trf!(
                "{} pixels across is too small to show a pattern: textures start at {} pixels.",
                pixels,
                Self::MIN
            )
        } else if pixels > Self::MAX {
            trf!(
                "{} pixels across would take more memory than a graph may use: the largest is {} \
                 pixels.",
                pixels,
                Self::MAX
            )
        } else if !pixels.is_power_of_two() {
            trf!(
                "Textures are a power of two across (256, 512, 1024 and so on), so they fit \
                 game engines; {} isn't one.",
                pixels
            )
        } else {
            return Ok(Self(pixels));
        };
        Err(Refusal::new(reason).with_fixes([Fix::UseSize(nearest)]))
    }

    /// The allowed size nearest `pixels` (a power of two, kept in range).
    pub fn nearest(pixels: u32) -> Self {
        let clamped = pixels.clamp(Self::MIN, Self::MAX);
        let up = clamped.next_power_of_two();
        let down = up / 2;
        let best = if down >= Self::MIN && clamped - down < up - clamped {
            down
        } else {
            up
        };
        Self(best.min(Self::MAX))
    }

    /// Pixels across.
    pub fn get(self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_outside_the_range_or_not_powers_of_two_are_refused_with_the_nearest() {
        assert_eq!(Size::new(512).unwrap().get(), 512);
        for (asked, nearest) in [(1, 16), (0, 16), (100_000, 4096), (300, 256), (400, 512)] {
            let refusal = Size::new(asked).unwrap_err();
            assert!(!refusal.reason.is_empty());
            assert_eq!(refusal.fixes, vec![Fix::UseSize(nearest)], "{asked}");
        }
        assert_eq!(Size::nearest(u32::MAX).get(), Size::MAX);
    }

    #[test]
    fn reading_past_an_edge_wraps_round() {
        let size = Size::new(16).unwrap();
        let pixels: Vec<f32> = (0..256).map(|i| i as f32).collect();
        let image = Image::from_pixels(size, pixels).unwrap();
        assert_eq!(image.at(-1, 0), image.at(15, 0));
        assert_eq!(image.at(16, 3), image.at(0, 3));
        assert_eq!(image.at(2, -1), image.at(2, 15));
        assert_eq!(
            image.at(i64::MIN / 4, 0),
            image.at((i64::MIN / 4).rem_euclid(16), 0)
        );
        assert!(Image::from_pixels(size, vec![0.0; 3]).is_none());
    }

    #[test]
    fn sampling_wraps_round_too() {
        let size = Size::new(16).unwrap();
        let mut pixels = vec![0.0; 256];
        pixels[0] = 1.0;
        let image = Image::from_pixels(size, pixels).unwrap();
        // Halfway between the last and first column.
        let edge = image.sample(0.0, 0.5 / 16.0);
        assert!((edge - 0.5).abs() < 1e-5, "{edge}");
        assert!((image.sample(1.0, 0.5 / 16.0) - edge).abs() < 1e-6);
    }
}
