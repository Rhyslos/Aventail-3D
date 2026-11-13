//! Maths the boxes turning a photo into a material share (milestone 21,
//! [decision 0283](../../../../docs/decisions/0283-image-to-material.md)):
//! lightness, a soft blur that keeps a photo's edges apart (for a picture
//! that doesn't tile yet), and stretching greys so the darkest and
//! lightest few hundredths reach 0 and 1.

use super::pattern::soft_blur;
use crate::image::{Grey, Image, Rgb, Size};
use crate::painting::lightness;
use crate::parallel;

/// How light each pixel of `colour` looks, 0 to 1.
pub(super) fn lightness_of(colour: &Rgb, threads: usize) -> Grey {
    let size = Size::nearest(colour.size());
    parallel::fill(size, threads, |x, y| {
        lightness(colour.at(i64::from(x), i64::from(y)))
    })
}

/// `image` blurred softly over about `radius` pixels: wrapping round its
/// edges when `wraps` (a picture that tiles), else as a photo, whose far
/// edges have nothing to do with each other: past an edge, the picture
/// carries on as it was heading (each pixel there is the edge's value
/// twice less the one as far inside), so a steady change of light across
/// the photo stays steady right up to its edges.
pub(super) fn soft_blur_at_edges(image: &Grey, radius: f32, wraps: bool, threads: usize) -> Grey {
    if wraps {
        return soft_blur(image, radius, threads);
    }
    if !radius.is_finite() || radius < 0.5 {
        return image.clone();
    }
    let r = (radius / 1.7).round().max(1.0) as usize;
    let mut out = image.clone();
    for _ in 0..3 {
        out = carried_rows(&out, r, threads);
        out = turned(&carried_rows(&turned(&out, threads), r, threads), threads);
    }
    out
}

/// `image` turned about its diagonal (rows become columns).
fn turned(image: &Grey, threads: usize) -> Grey {
    let size = Size::nearest(image.size());
    parallel::fill(size, threads, |x, y| image.at(i64::from(y), i64::from(x)))
}

/// Each row of `image` box-blurred `r` pixels each way, carried on past
/// its ends as [`soft_blur_at_edges`] says.
fn carried_rows(image: &Grey, r: usize, threads: usize) -> Grey {
    let size = Size::nearest(image.size());
    let n = image.size() as usize;
    let r = r.min(n.saturating_sub(3)).max(1);
    let width = (2 * r + 1) as f32;
    let px = image.pixels();
    parallel::rows(size, threads, |y, row| {
        let line = &px[y as usize * n..(y as usize + 1) * n];
        let at = |i: isize| -> f32 {
            if i < 0 {
                2.0 * line[0] - line[(-i) as usize]
            } else if i as usize >= n {
                let past = i as usize - (n - 1);
                2.0 * line[n - 1] - line[n - 1 - past]
            } else {
                line[i as usize]
            }
        };
        let r = r as isize;
        let mut sum: f32 = (-r..=r).map(at).sum();
        for (x, out) in row.iter_mut().enumerate() {
            *out = sum / width;
            let x = x as isize;
            sum += at(x + r + 1) - at(x - r);
        }
    })
}

/// The greys at `low` and `high` of the way through `image`'s values
/// from darkest to lightest (0 to 1 each), from a histogram of 4096
/// steps.
pub(super) fn spread(image: &Grey, low: f32, high: f32) -> (f32, f32) {
    let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
    for v in image.pixels().iter().filter(|v| v.is_finite()) {
        lo = lo.min(*v);
        hi = hi.max(*v);
    }
    if !lo.is_finite() || hi - lo < 1e-6 {
        let mid = if lo.is_finite() { lo } else { 0.0 };
        return (mid, mid);
    }
    const STEPS: usize = 4096;
    let mut counts = vec![0u32; STEPS];
    let scale = (STEPS - 1) as f32 / (hi - lo);
    let mut total = 0u64;
    for v in image.pixels().iter().filter(|v| v.is_finite()) {
        counts[((v - lo) * scale) as usize] += 1;
        total += 1;
    }
    let at = |share: f32| -> f32 {
        let wanted = (share.clamp(0.0, 1.0) as f64 * total as f64) as u64;
        let mut seen = 0u64;
        for (i, c) in counts.iter().enumerate() {
            seen += u64::from(*c);
            if seen > wanted {
                return lo + i as f32 / scale;
            }
        }
        hi
    };
    (at(low), at(high))
}

/// `image` stretched so its darkest and lightest two hundredths reach 0
/// and 1 (clamped beyond); a flat picture comes out mid grey.
pub(super) fn stretched(image: &Grey, threads: usize) -> Grey {
    let (lo, hi) = spread(image, 0.02, 0.98);
    let size = Size::nearest(image.size());
    if hi - lo < 1e-6 {
        return Image::filled(size, 0.5);
    }
    let k = 1.0 / (hi - lo);
    parallel::fill(size, threads, |x, y| {
        let v = image.at(i64::from(x), i64::from(y));
        if v.is_finite() {
            ((v - lo) * k).clamp(0.0, 1.0)
        } else {
            0.5
        }
    })
}

/// The average of `image`'s values.
pub(super) fn mean(image: &Grey) -> f32 {
    let px = image.pixels();
    let sum: f64 = px.iter().map(|v| f64::from(*v)).sum();
    (sum / px.len().max(1) as f64) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp(n: u32) -> Grey {
        let size = Size::new(n).unwrap();
        parallel::fill(size, 1, |x, _| x as f32 / (n - 1) as f32)
    }

    #[test]
    fn a_steady_ramp_stays_steady_up_to_the_edges() {
        let r = ramp(64);
        let blurred = soft_blur_at_edges(&r, 12.0, false, 2);
        for x in 0..64 {
            let want = r.at(x, 5);
            assert!((blurred.at(x, 5) - want).abs() < 1e-3, "{x}");
        }
        // Wrapping mixes the far edges instead.
        let wrapped = soft_blur_at_edges(&r, 12.0, true, 2);
        assert!((wrapped.at(0, 5) - r.at(0, 5)).abs() > 0.1);
    }

    #[test]
    fn a_tiny_radius_gives_the_picture_back() {
        let r = ramp(16);
        assert_eq!(soft_blur_at_edges(&r, 0.2, false, 1), r);
        assert_eq!(soft_blur_at_edges(&r, f32::NAN, false, 1), r);
    }

    #[test]
    fn stretching_reaches_black_and_white() {
        let size = Size::new(32).unwrap();
        let narrow: Grey = parallel::fill(size, 1, |x, _| 0.4 + x as f32 * 0.005);
        let s = stretched(&narrow, 2);
        assert!(s.pixels().iter().all(|v| (0.0..=1.0).contains(v)));
        assert_eq!(s.at(0, 0), 0.0);
        assert_eq!(s.at(31, 0), 1.0);
        let flat = Image::filled(size, 0.3);
        assert!(stretched(&flat, 1).pixels().iter().all(|v| *v == 0.5));
        let (lo, hi) = spread(&narrow, 0.0, 1.0);
        assert!(lo <= 0.4 + 1e-4 && hi >= 0.55 - 1e-3, "{lo} {hi}");
        assert!((mean(&flat) - 0.3).abs() < 1e-6);
    }
}
