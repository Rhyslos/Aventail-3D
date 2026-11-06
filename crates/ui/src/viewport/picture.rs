//! A texture picture of your own (decision 0244), shown on the model by
//! its UVs and behind the UVs in the UV Editor, with a mip level to see it
//! as a game shows it far away.

/// A picture, made square (a power of two across) when chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    /// Its file's name, for the menus.
    pub name: String,
    /// Pixels across (and down).
    pub size: u32,
    /// Rows of RGBA pixels, the top row first.
    pub rgba: Vec<u8>,
}

/// The biggest picture kept, in pixels across.
pub const MAX_PICTURE: u32 = 2048;
/// The deepest mip level offered.
pub const MAX_MIP: u32 = 6;

impl Picture {
    /// `rgba` (`width` by `height`) resampled into a square a power of two
    /// across (the larger side's, at most [`MAX_PICTURE`]), as a texture
    /// stretches over the square. `None` for an empty or broken picture.
    pub fn square(name: &str, width: u32, height: u32, rgba: &[u8]) -> Option<Self> {
        if width == 0 || height == 0 || rgba.len() != (width * height * 4) as usize {
            return None;
        }
        let size = width.max(height).next_power_of_two().min(MAX_PICTURE);
        let mut out = vec![0u8; (size * size * 4) as usize];
        for y in 0..size {
            for x in 0..size {
                // Bilinear from the picture.
                let fx = ((x as f32 + 0.5) / size as f32 * width as f32 - 0.5).max(0.0);
                let fy = ((y as f32 + 0.5) / size as f32 * height as f32 - 0.5).max(0.0);
                let (x0, y0) = (fx as u32, fy as u32);
                let (x1, y1) = ((x0 + 1).min(width - 1), (y0 + 1).min(height - 1));
                let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
                for c in 0..4 {
                    let at = |px: u32, py: u32| f32::from(rgba[((py * width + px) * 4 + c) as usize]);
                    let top = at(x0, y0) * (1.0 - tx) + at(x1, y0) * tx;
                    let bottom = at(x0, y1) * (1.0 - tx) + at(x1, y1) * tx;
                    out[((y * size + x) * 4 + c) as usize] =
                        (top * (1.0 - ty) + bottom * ty).round() as u8;
                }
            }
        }
        Some(Self {
            name: name.to_owned(),
            size,
            rgba: out,
        })
    }

    /// The picture at mip `level` (0 itself; each level half as wide,
    /// every pixel the average of four), as a game shows it from further
    /// away: its size and pixels.
    pub fn mip(&self, level: u32) -> (u32, Vec<u8>) {
        let mut size = self.size;
        let mut rgba = self.rgba.clone();
        for _ in 0..level.min(MAX_MIP) {
            if size <= 1 {
                break;
            }
            let half = size / 2;
            let mut next = vec![0u8; (half * half * 4) as usize];
            for y in 0..half {
                for x in 0..half {
                    for c in 0..4 {
                        let at = |px: u32, py: u32| u32::from(rgba[((py * size + px) * 4 + c) as usize]);
                        let sum = at(2 * x, 2 * y)
                            + at(2 * x + 1, 2 * y)
                            + at(2 * x, 2 * y + 1)
                            + at(2 * x + 1, 2 * y + 1);
                        next[((y * half + x) * 4 + c) as usize] = ((sum + 2) / 4) as u8;
                    }
                }
            }
            size = half;
            rgba = next;
        }
        (size, rgba)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_is_made_square_a_power_of_two_across() {
        let rgba = vec![200u8; 300 * 100 * 4];
        let p = Picture::square("wall.png", 300, 100, &rgba).unwrap();
        assert_eq!(p.size, 512);
        assert_eq!(p.rgba.len(), 512 * 512 * 4);
        assert!(p.rgba.iter().all(|c| *c == 200));
        assert!(Picture::square("x", 0, 10, &[]).is_none());
        assert!(Picture::square("x", 2, 2, &[0; 3]).is_none());
        let huge = Picture::square("x", 4096, 1, &vec![0; 4096 * 4]).unwrap();
        assert_eq!(huge.size, MAX_PICTURE);
    }

    #[test]
    fn each_mip_level_is_half_as_wide_and_averages_four_pixels() {
        // A 4 × 4 checker of black and white: level 1 is grey.
        let mut rgba = Vec::new();
        for y in 0..4u32 {
            for x in 0..4u32 {
                let v = if (x + y) % 2 == 0 { 255 } else { 0 };
                rgba.extend([v, v, v, 255]);
            }
        }
        let p = Picture::square("c", 4, 4, &rgba).unwrap();
        let (size, one) = p.mip(1);
        assert_eq!(size, 2);
        assert!((127..=128).contains(&one[0]), "{}", one[0]);
        assert_eq!(p.mip(0), (4, p.rgba.clone()));
        assert_eq!(p.mip(99).0, 1, "never below one pixel");
    }
}
