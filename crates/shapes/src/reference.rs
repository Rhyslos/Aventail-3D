//! A reference image under the canvas: a photo or sketch to draw over,
//! shown at a chosen opacity. It is only a guide: it never changes what
//! the shape box gives.

use std::sync::Arc;
use words::text;

/// The widest or tallest a reference image is kept, in pixels: larger
/// ones are made smaller when brought in, so the project stays small.
pub const MAX_SIDE: u32 = 1024;
/// The longest name kept, in bytes.
pub const MAX_NAME: usize = 200;

/// A reference image: rows of pixels, four bytes each (red, green, blue,
/// alpha; sRGB), the top row first, stretched over the texture square.
#[derive(Clone, PartialEq)]
pub struct Reference {
    width: u32,
    height: u32,
    rgba: Arc<[u8]>,
    /// Its name (the file it came from), to show.
    pub name: String,
    /// How strongly it shows, 0 to 1.
    pub opacity: f32,
}

impl std::fmt::Debug for Reference {
    // Not the pixels: the size is enough to tell them apart in a list.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Reference")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("name", &self.name)
            .field("opacity", &self.opacity)
            .finish()
    }
}

impl Reference {
    /// A reference image of `width` × `height` pixels (at most
    /// [`MAX_SIDE`] each way, `rgba` four bytes each), or why it can't be
    /// one. The opacity starts at a half.
    pub fn new(width: u32, height: u32, rgba: Vec<u8>, name: &str) -> Result<Self, &'static str> {
        if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
            return Err(text!("the reference image is too large or has no pixels"));
        }
        if rgba.len() != width as usize * height as usize * 4 {
            return Err(text!("the reference image's pixels are damaged"));
        }
        let mut name = name.to_string();
        if name.len() > MAX_NAME {
            let mut cut = MAX_NAME;
            while !name.is_char_boundary(cut) {
                cut -= 1;
            }
            name.truncate(cut);
        }
        Ok(Self {
            width,
            height,
            rgba: rgba.into(),
            name,
            opacity: 0.5,
        })
    }

    /// A reference image from a picture of any size: made smaller (by
    /// averaging) until it fits within [`MAX_SIDE`]. `None` when `rgba`
    /// doesn't hold `width` × `height` pixels.
    pub fn fitted(width: u32, height: u32, rgba: &[u8], name: &str) -> Option<Self> {
        if width == 0 || height == 0 || rgba.len() != width as usize * height as usize * 4 {
            return None;
        }
        let step = width.max(height).div_ceil(MAX_SIDE).max(1);
        let (w, h) = (width.div_ceil(step), height.div_ceil(step));
        let mut out = Vec::with_capacity(w as usize * h as usize * 4);
        for y in 0..h {
            for x in 0..w {
                let mut sum = [0u32; 4];
                let mut count = 0u32;
                for sy in (y * step)..((y + 1) * step).min(height) {
                    for sx in (x * step)..((x + 1) * step).min(width) {
                        let at = (sy as usize * width as usize + sx as usize) * 4;
                        for (s, v) in sum.iter_mut().zip(&rgba[at..at + 4]) {
                            *s += u32::from(*v);
                        }
                        count += 1;
                    }
                }
                out.extend(sum.map(|s| (s / count.max(1)) as u8));
            }
        }
        Self::new(w, h, out, name).ok()
    }

    /// Pixels across.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Pixels down.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The pixels.
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    /// The same image at opacity `opacity` (kept from 0 to 1; not a real
    /// number is a half).
    pub fn with_opacity(&self, opacity: f32) -> Self {
        let mut out = self.clone();
        out.opacity = if opacity.is_finite() {
            opacity.clamp(0.0, 1.0)
        } else {
            0.5
        };
        out
    }

    /// Roughly how much memory it takes, in bytes.
    pub fn memory_size(&self) -> usize {
        self.rgba.len() + self.name.len() + 64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reference_image_checks_its_size_and_pixels() {
        assert!(Reference::new(2, 2, vec![0; 16], "leaf.png").is_ok());
        assert!(Reference::new(2, 2, vec![0; 15], "leaf.png").is_err());
        assert!(Reference::new(0, 2, vec![], "leaf.png").is_err());
        assert!(Reference::new(MAX_SIDE + 1, 1, vec![0; (MAX_SIDE as usize + 1) * 4], "").is_err());
        let long = "ä".repeat(MAX_NAME);
        let r = Reference::new(1, 1, vec![0; 4], &long).unwrap();
        assert!(r.name.len() <= MAX_NAME);
        assert_eq!(r.with_opacity(3.0).opacity, 1.0);
        assert_eq!(r.with_opacity(f32::NAN).opacity, 0.5);
    }

    #[test]
    fn big_pictures_are_made_smaller_by_averaging() {
        let (w, h) = (MAX_SIDE * 2 + 1, 3);
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for (i, px) in rgba.chunks_mut(4).enumerate() {
            let v = if i % 2 == 0 { 200 } else { 100 };
            px.copy_from_slice(&[v, v, v, 255]);
        }
        let r = Reference::fitted(w, h, &rgba, "big").unwrap();
        assert!(r.width() <= MAX_SIDE && r.height() >= 1);
        assert_eq!(r.rgba().len(), (r.width() * r.height() * 4) as usize);
        // Neighbouring pixels averaged: between the two greys.
        assert!((100..=200).contains(&r.rgba()[0]));
        assert!(Reference::fitted(2, 2, &[0; 3], "").is_none());
        let small = Reference::fitted(2, 1, &[1, 2, 3, 4, 5, 6, 7, 8], "small").unwrap();
        assert_eq!(small.rgba(), &[1, 2, 3, 4, 5, 6, 7, 8]);
    }
}
