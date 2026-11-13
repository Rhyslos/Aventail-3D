//! Maths the pattern boxes share: soft blurs, distances to a shape's
//! edge, and random numbers per cell. Everything wraps round the texture's
//! edges, so what it makes tiles seamlessly.
//!
//! The distance to an edge is the exact distance transform of
//! Felzenszwalb and Huttenlocher ("Distance Transforms of Sampled
//! Functions", 2012), worked out on rows that are lengthened by wrapping
//! round. Nothing is taken from other programs.

use super::blur::box_blur;
use crate::image::{Grey, Image, Size};
use crate::parallel;
use crate::seed::{lattice, unit};

/// `image` blurred softly (three box blurs, close to a Gaussian) over
/// about `radius` pixels, wrapping round. Under half a pixel gives it back.
pub(crate) fn soft_blur(image: &Grey, radius: f32, threads: usize) -> Grey {
    if !radius.is_finite() || radius < 0.5 {
        return image.clone();
    }
    // Three passes of radius r spread about as far as one of radius 1.7 r.
    let r = (radius / 1.7).round().max(1.0) as u32;
    let once = box_blur(image, r, threads);
    let twice = box_blur(&once, r, threads);
    box_blur(&twice, r, threads)
}

/// A random number from 0 to 1 for cell `(x, y)` of a grid `across` by
/// `down` cells that repeats across the texture (cells past an edge are
/// the ones on the other side), on layer `layer` with `seed`.
pub(crate) fn cell_random(seed: u64, layer: u32, x: i64, y: i64, across: i64, down: i64) -> f32 {
    unit(lattice(
        seed,
        layer,
        x.rem_euclid(across.max(1)),
        y.rem_euclid(down.max(1)),
    ))
}

/// How many whole pieces `piece` pixels wide fit across a texture `n`
/// pixels wide, from 1 to `most`: patterns sized in centimetres repeat a
/// whole number of times, so they still tile.
pub(crate) fn whole_repeats(n: f32, piece: f32, most: i64) -> i64 {
    if !piece.is_finite() || piece <= 0.0 {
        return most.max(1);
    }
    ((n / piece).round() as i64).clamp(1, most.max(1))
}

/// The smooth step from 0 at `a` to 1 at `b`.
pub(crate) fn smooth(a: f32, b: f32, t: f32) -> f32 {
    if b <= a {
        return if t < a { 0.0 } else { 1.0 };
    }
    let k = ((t - a) / (b - a)).clamp(0.0, 1.0);
    k * k * (3.0 - 2.0 * k)
}

/// A large number standing for "no edge near".
const FAR: f64 = 1e30;

/// For every pixel inside `shape` (lighter than half grey), how many
/// pixels it is from the nearest pixel outside, wrapping round; 0 outside.
/// Distances past `most` are `most`. Inside pixels next to the edge are
/// half a pixel from it (the edge runs between pixels).
pub(crate) fn distance_inside(shape: &Grey, most: f32, threads: usize) -> Grey {
    let Ok(size) = Size::new(shape.size()) else {
        return shape.map(|_| 0.0);
    };
    let most = if most.is_finite() { most.max(0.0) } else { 0.0 };
    let start: Image<f64> = shape.map(|v| if v > 0.5 { FAR } else { 0.0 });
    // Down the columns (turned into rows), then along the rows.
    let margin = (most.ceil() as usize + 2).min(size.get() as usize);
    let turned = transposed(&start, size, threads);
    let down = along_rows(&turned, size, margin, threads);
    let back = transposed(&down, size, threads);
    let both = along_rows(&back, size, margin, threads);
    let squared: Image<f32> = parallel::fill(size, threads, |x, y| {
        let d2 = both.at(i64::from(x), i64::from(y));
        if d2 <= 0.0 {
            0.0
        } else {
            ((d2.sqrt() - 0.5) as f32).clamp(0.0, most)
        }
    });
    squared
}

/// `image` turned about its diagonal (rows become columns).
fn transposed(image: &Image<f64>, size: Size, threads: usize) -> Image<f64> {
    parallel::fill(size, threads, |x, y| image.at(i64::from(y), i64::from(x)))
}

/// The squared distance transform of each row of `f`, the row lengthened
/// by `margin` pixels each side by wrapping round.
fn along_rows(f: &Image<f64>, size: Size, margin: usize, threads: usize) -> Image<f64> {
    let n = size.get() as usize;
    parallel::rows(size, threads, |y, row| {
        let long = n + 2 * margin;
        let line: Vec<f64> = (0..long)
            .map(|i| f.at(i as i64 - margin as i64, i64::from(y)))
            .collect();
        let out = transform_1d(&line);
        row.copy_from_slice(&out[margin..margin + n]);
    })
}

/// The one-dimensional squared distance transform of `f` (Felzenszwalb
/// and Huttenlocher's lower envelope of parabolas).
fn transform_1d(f: &[f64]) -> Vec<f64> {
    let n = f.len();
    let mut out = vec![0.0; n];
    if n == 0 {
        return out;
    }
    let mut v = vec![0usize; n];
    let mut z = vec![0.0f64; n + 1];
    let mut k = 0usize;
    z[0] = f64::NEG_INFINITY;
    z[1] = f64::INFINITY;
    let sq = |q: usize| (q * q) as f64;
    for q in 1..n {
        let mut s;
        loop {
            let p = v[k];
            s = ((f[q] + sq(q)) - (f[p] + sq(p))) / (2.0 * (q - p) as f64);
            // z[0] is minus infinity, so this stops at the first parabola.
            if s <= z[k] && k > 0 {
                k -= 1;
            } else {
                break;
            }
        }
        k += 1;
        v[k] = q;
        z[k] = s;
        z[k + 1] = f64::INFINITY;
    }
    let mut k = 0usize;
    for (q, o) in out.iter_mut().enumerate() {
        while z[k + 1] < q as f64 {
            k += 1;
        }
        let p = v[k];
        let d = q as f64 - p as f64;
        *o = (d * d + f[p]).min(FAR);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(n: u32, from: u32, to: u32) -> Grey {
        let size = Size::new(n).unwrap();
        let pixels = (0..n * n)
            .map(|i| {
                let (x, y) = (i % n, i / n);
                if (from..to).contains(&x) && (from..to).contains(&y) {
                    1.0
                } else {
                    0.0
                }
            })
            .collect();
        Image::from_pixels(size, pixels).unwrap()
    }

    #[test]
    fn the_distance_inside_is_exact_and_capped() {
        let shape = square(32, 8, 24);
        let d = distance_inside(&shape, 100.0, 2);
        assert_eq!(d.at(0, 0), 0.0);
        assert_eq!(d.at(8, 16), 0.5);
        assert_eq!(d.at(10, 16), 2.5);
        assert_eq!(d.at(15, 15), 7.5);
        let capped = distance_inside(&shape, 3.0, 1);
        assert_eq!(capped.at(15, 15), 3.0);
        assert_eq!(capped.at(8, 16), 0.5);
    }

    #[test]
    fn the_distance_wraps_round_the_edges() {
        // Everything inside but one column: the far side is near it.
        let size = Size::new(16).unwrap();
        let pixels = (0..256)
            .map(|i| if i % 16 == 5 { 0.0 } else { 1.0 })
            .collect();
        let shape = Image::from_pixels(size, pixels).unwrap();
        let d = distance_inside(&shape, 50.0, 1);
        assert_eq!(d.at(4, 3), 0.5);
        assert_eq!(d.at(6, 3), 0.5);
        // Column 13 is 8 from column 5 either way round.
        assert_eq!(d.at(13, 3), 7.5);
        // Nothing outside at all: every pixel is as far as allowed.
        let full = Image::filled(size, 1.0);
        assert_eq!(distance_inside(&full, 4.0, 1).at(3, 3), 4.0);
        let empty = Image::filled(size, 0.0);
        assert_eq!(distance_inside(&empty, 4.0, 1).at(3, 3), 0.0);
    }

    #[test]
    fn the_one_dimensional_transform_matches_a_slow_search() {
        let f: Vec<f64> = [0.0, FAR, FAR, 4.0, FAR, FAR, FAR, 0.0, FAR, 1.0]
            .into_iter()
            .collect();
        let fast = transform_1d(&f);
        for (q, got) in fast.iter().enumerate() {
            let slow = f
                .iter()
                .enumerate()
                .map(|(p, fp)| (q as f64 - p as f64).powi(2) + fp)
                .fold(FAR, f64::min);
            assert_eq!(*got, slow, "{q}");
        }
        assert!(transform_1d(&[]).is_empty());
        assert_eq!(transform_1d(&[FAR]), vec![FAR]);
    }

    #[test]
    fn soft_blur_keeps_the_average() {
        let shape = square(32, 8, 24);
        let blurred = soft_blur(&shape, 4.0, 2);
        let a: f32 = shape.pixels().iter().sum();
        let b: f32 = blurred.pixels().iter().sum();
        assert!((a - b).abs() < 1e-2, "{a} {b}");
        assert_eq!(soft_blur(&shape, 0.2, 1), shape);
        assert_eq!(soft_blur(&shape, f32::NAN, 1), shape);
    }

    #[test]
    fn cells_repeat_round_the_grid() {
        assert_eq!(
            cell_random(3, 0, -1, 0, 4, 4),
            cell_random(3, 0, 3, 0, 4, 4)
        );
        assert_eq!(cell_random(3, 0, 2, 9, 4, 4), cell_random(3, 0, 2, 1, 4, 4));
        assert_eq!(cell_random(3, 0, 2, 9, 0, 0), cell_random(3, 0, 0, 0, 1, 1));
    }
}
