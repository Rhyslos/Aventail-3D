//! A box blur that wraps round the edges, so a blurred texture still
//! tiles. It costs the same whatever the radius (running sums), and is
//! shared over the cores row by row.

use crate::image::{Grey, Size};
use crate::parallel;

/// `image` blurred by a square `radius` pixels from each pixel's middle,
/// on up to `threads` cores. A radius of 0 gives the image back.
pub fn box_blur(image: &Grey, radius: u32, threads: usize) -> Grey {
    let Ok(size) = Size::new(image.size()) else {
        return image.clone();
    };
    if radius == 0 {
        return image.clone();
    }
    let across = along_rows(image, size, radius, threads);
    let turned = parallel::fill(size, threads, |x, y| across.at(i64::from(y), i64::from(x)));
    let down = along_rows(&turned, size, radius, threads);
    parallel::fill(size, threads, |x, y| down.at(i64::from(y), i64::from(x)))
}

/// Each row blurred along itself.
fn along_rows(image: &Grey, size: Size, radius: u32, threads: usize) -> Grey {
    let n = i64::from(size.get());
    let r = i64::from(radius).min(n);
    let width = (2 * r + 1) as f32;
    parallel::rows(size, threads, |y, row| {
        let y = i64::from(y);
        let mut sum: f32 = (-r..=r).map(|dx| image.at(dx, y)).sum();
        for (x, px) in row.iter_mut().enumerate() {
            *px = sum / width;
            let x = x as i64;
            sum += image.at(x + r + 1, y) - image.at(x - r, y);
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::Image;

    #[test]
    fn blurring_keeps_the_average_and_wraps_round() {
        let size = Size::new(32).unwrap();
        let mut pixels = vec![0.0; 32 * 32];
        pixels[0] = 1.0;
        let image = Image::from_pixels(size, pixels).unwrap();
        let blurred = box_blur(&image, 2, 3);
        let total: f32 = blurred.pixels().iter().sum();
        assert!((total - 1.0).abs() < 1e-4, "{total}");
        // The corner's light reaches round to the opposite edges.
        assert!(blurred.at(31, 31) > 0.0 && blurred.at(-2, 0) > 0.0);
        assert_eq!(blurred.at(5, 5), 0.0);
        assert_eq!(box_blur(&image, 0, 1), image);
        // The same on one core.
        assert_eq!(box_blur(&image, 2, 1), blurred);
    }
}
