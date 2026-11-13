//! The health check's measures of a worked-out material: how much of it
//! is too dark or too bright, how its light and dark fade when it is seen
//! smaller, how big patches differ from one part of a repeat to another,
//! and where lines run across it.
//!
//! Everything reads plain pictures and is the same on any number of
//! cores. Not-a-number pixels count as nothing (they are left out).

use crate::image::Grey;
use crate::value::MaterialMaps;

/// Below this light (linear, 0 to 1) a colour is darker than charcoal,
/// the darkest real surfaces: about 30 in 255.
pub(super) const DARK: f32 = 0.012;
/// Above this light a colour is brighter than fresh snow: about 240 in
/// 255.
pub(super) const BRIGHT: f32 = 0.87;
/// Below this light bare metal is darker than any real metal (about 180
/// in 255): metal reflects at least half the light.
pub(super) const METAL_DARK: f32 = 0.45;
/// Where a pixel counts as metal.
const METAL: f32 = 0.5;

/// How much of a material's colour lies outside what lights well, each a
/// share of its solid area (0 to 1).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(super) struct ColourRange {
    /// Non-metal colour darker than charcoal.
    pub dark: f32,
    /// Non-metal colour brighter than snow.
    pub bright: f32,
    /// Of the metal parts, how much is darker than real metal.
    pub metal_dark: f32,
    /// How much of the material is metal.
    pub metal: f32,
}

/// A colour's light as the eye sums it (linear, 0 to 1), from sRGB.
pub(super) fn light_of(c: [f32; 3]) -> f32 {
    let [r, g, b] = c.map(linear);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

fn linear(c: f32) -> f32 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// How much of `m`'s colour is too dark or too bright to light well, by
/// its solid area (an alpha card's see-through parts don't count).
pub(super) fn colour_range(m: &MaterialMaps) -> ColourRange {
    let n = i64::from(m.size());
    let (mut solid, mut dark, mut bright, mut metal, mut metal_dark) =
        (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for y in 0..n {
        for x in 0..n {
            let w = m.opacity_at(x, y);
            let light = light_of(m.colour.at(x, y));
            let is_metal = m.metal.at(x, y);
            if !(w.is_finite() && light.is_finite() && is_metal.is_finite()) || w <= 0.0 {
                continue;
            }
            let w = f64::from(w.min(1.0));
            solid += w;
            if is_metal >= METAL {
                metal += w;
                if light < METAL_DARK {
                    metal_dark += w;
                }
            } else if light < DARK {
                dark += w;
            } else if light > BRIGHT {
                bright += w;
            }
        }
    }
    if solid <= 0.0 {
        return ColourRange::default();
    }
    ColourRange {
        dark: (dark / solid) as f32,
        bright: (bright / solid) as f32,
        metal_dark: if metal > 0.0 {
            (metal_dark / metal) as f32
        } else {
            0.0
        },
        metal: (metal / solid) as f32,
    }
}

/// What the material looks like in light and dark, one grey a pixel: its
/// colour's lightness, shaded by its normals under a light from the top
/// left (so detail made only of bumps counts as well).
pub(super) fn look(m: &MaterialMaps) -> Vec<f32> {
    let n = m.size() as usize;
    let sun = glam::Vec3::new(-0.45, 0.45, 0.77).normalize();
    let mut out = Vec::with_capacity(n * n);
    for (c, d) in m.colour.pixels().iter().zip(m.normal.pixels()) {
        let lightness = linear_to_lightness(light_of(*c));
        let normal = glam::Vec3::from(*d);
        let facing = if normal.is_finite() && normal.length_squared() > 1e-6 {
            normal.normalize().dot(sun).clamp(0.0, 1.0)
        } else {
            sun.z
        };
        // Lit from one side, softly: flat ground gives about 0.9.
        let shade = 0.35 + 0.65 * facing;
        let v = lightness * shade;
        out.push(if v.is_finite() { v } else { 0.0 });
    }
    out
}

/// Light as the eye sees steps in it (perceived lightness, 0 to 1).
fn linear_to_lightness(y: f32) -> f32 {
    y.clamp(0.0, 1.0).cbrt()
}

/// How much light and dark a square picture `n` across keeps when it is
/// seen smaller: its spread (standard deviation) as it is, then averaged
/// in blocks of 2, 4, 8… pixels, down to 2 × 2 blocks across.
pub(super) fn spreads(pixels: &[f32], n: usize) -> Vec<f32> {
    let mut out = Vec::new();
    if n < 2 || pixels.len() != n * n || !n.is_power_of_two() {
        return out;
    }
    let mut level = pixels.to_vec();
    let mut side = n;
    loop {
        out.push(spread(&level));
        if side <= 2 {
            break;
        }
        level = halved(&level, side);
        side /= 2;
    }
    out
}

/// A square picture `side` across averaged in blocks of 2 × 2.
fn halved(pixels: &[f32], side: usize) -> Vec<f32> {
    let half = side / 2;
    let mut out = Vec::with_capacity(half * half);
    for y in 0..half {
        for x in 0..half {
            let at = |dx: usize, dy: usize| pixels[(2 * y + dy) * side + 2 * x + dx];
            out.push((at(0, 0) + at(1, 0) + at(0, 1) + at(1, 1)) * 0.25);
        }
    }
    out
}

/// The standard deviation of `values`.
fn spread(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let count = values.len() as f64;
    let mean = values.iter().map(|v| f64::from(*v)).sum::<f64>() / count;
    let var = values
        .iter()
        .map(|v| (f64::from(*v) - mean).powi(2))
        .sum::<f64>()
        / count;
    var.sqrt() as f32
}

/// How a picture's big patches differ across one repeat: its average in
/// each of `cells` × `cells` squares, and which stands out most (to say
/// where in the repeat to look).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Patches {
    /// The spread of the squares' averages.
    pub spread: f32,
    /// The square that stands out most: its column and row.
    pub most: (usize, usize),
    /// Whether that square is lighter than the rest (else darker).
    pub lighter: bool,
    /// Squares across.
    pub cells: usize,
}

/// `pixels` (a square picture `n` across) averaged in `cells` × `cells`
/// squares; `None` when `n` isn't a whole number of squares.
pub(super) fn patches(pixels: &[f32], n: usize, cells: usize) -> Option<Patches> {
    if cells < 2 || n < cells || !n.is_multiple_of(cells) || pixels.len() != n * n {
        return None;
    }
    let step = n / cells;
    let mut means = vec![0.0f64; cells * cells];
    for y in 0..n {
        for x in 0..n {
            means[(y / step) * cells + x / step] += f64::from(pixels[y * n + x]);
        }
    }
    let per = (step * step) as f64;
    let means: Vec<f32> = means.iter().map(|m| (m / per) as f32).collect();
    let mean = means.iter().sum::<f32>() / means.len() as f32;
    let (at, most) = means
        .iter()
        .enumerate()
        .max_by(|a, b| (a.1 - mean).abs().total_cmp(&(b.1 - mean).abs()))?;
    Some(Patches {
        spread: spread(&means),
        most: (at % cells, at / cells),
        lighter: *most > mean,
        cells,
    })
}

/// How strongly a square picture `n` across changes light and dark once
/// across one repeat: the size (from the average to the top of the wave)
/// of its four slowest waves together, across, down and both diagonals.
/// A patch that stands out once a repeat makes these strong, so the
/// repeat shows as a grid; many small patches don't.
pub(super) fn slowest_waves(pixels: &[f32], n: usize) -> f32 {
    if n < 2 || pixels.len() != n * n {
        return 0.0;
    }
    let turn: Vec<(f64, f64)> = (0..n)
        .map(|i| {
            let a = std::f64::consts::TAU * i as f64 / n as f64;
            (a.cos(), a.sin())
        })
        .collect();
    let waves: [(usize, usize); 4] = [(1, 0), (0, 1), (1, 1), (1, n - 1)];
    let mut sums = [(0.0f64, 0.0f64); 4];
    for y in 0..n {
        for x in 0..n {
            let v = f64::from(pixels[y * n + x]);
            if !v.is_finite() {
                continue;
            }
            for (sum, (u, w)) in sums.iter_mut().zip(waves) {
                let (c, s) = turn[(u * x + w * y) % n];
                sum.0 += v * c;
                sum.1 += v * s;
            }
        }
    }
    let count = (n * n) as f64;
    let power: f64 = sums
        .iter()
        .map(|(c, s)| (2.0 * c / count).powi(2) + (2.0 * s / count).powi(2))
        .sum();
    power.sqrt() as f32
}

/// Rows of a picture where a line runs right across it (the gap between
/// rows of planks or bricks): runs of rows, each from its first row to
/// its last (the last may be past the bottom: the run wraps to the top).
/// Lines are low places in `height`, else dark places in `look` when the
/// height is flat; `None` when nothing runs across.
pub(super) fn lines_across(height: &Grey, look: &[f32]) -> Option<Vec<(usize, usize)>> {
    let n = height.size() as usize;
    if n < 4 || look.len() != n * n {
        return None;
    }
    let source: &[f32] = if spread(height.pixels()) > 0.02 {
        height.pixels()
    } else {
        look
    };
    let rows: Vec<f32> = source
        .chunks_exact(n)
        .map(|row| row.iter().sum::<f32>() / n as f32)
        .collect();
    if rows.iter().any(|r| !r.is_finite()) {
        return None;
    }
    let mut sorted = rows.clone();
    sorted.sort_by(f32::total_cmp);
    let (least, middle) = (sorted[0], sorted[n / 2]);
    // A line is a clear dip below the usual row: deeper than rows of
    // noise could dip by chance (the pixels' spread over the square root
    // of a row's length, six times over).
    let chance = 6.0 * spread(source) / (n as f32).sqrt();
    if middle - least < 0.08f32.max(chance) {
        return None;
    }
    let cut = least + (middle - least) * 0.5;
    let low: Vec<bool> = rows.iter().map(|r| *r < cut).collect();
    // Start just after a row that isn't low, so a line over the edge is
    // one run.
    let start = (0..n).find(|y| !low[*y])?;
    let mut runs = Vec::new();
    let mut run: Option<usize> = None;
    for i in 1..=n {
        let y = start + i;
        match (low[y % n], run) {
            (true, None) => run = Some(y),
            (false, Some(first)) => {
                runs.push((first % n, first % n + (y - 1 - first)));
                run = None;
            }
            _ => {}
        }
    }
    Some(runs)
}

/// How many rows from row `y` to the nearest of `runs` (0 inside one),
/// in a picture `n` tall, going down (`down`) or up.
pub(super) fn rows_to_line(runs: &[(usize, usize)], y: usize, n: usize, down: bool) -> usize {
    let n = n.max(1);
    let mut best = n;
    for &(first, last) in runs {
        let len = last - first + 1;
        for k in 0..len {
            let row = (first + k) % n;
            let d = if down {
                (row + n - y % n) % n
            } else {
                (y % n + n - row) % n
            };
            best = best.min(d);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::{Image, Size};
    use std::sync::Arc;

    fn material(size: u32, colour: impl Fn(u32, u32) -> [f32; 3] + Sync) -> MaterialMaps {
        let s = Size::new(size).unwrap();
        let flat = Arc::new(Image::filled(s, 0.5f32));
        MaterialMaps {
            colour: Arc::new(crate::parallel::fill(s, 1, colour)),
            height: flat.clone(),
            roughness: flat.clone(),
            metal: Arc::new(Image::filled(s, 0.0)),
            normal: Arc::new(Image::filled(s, [0.0, 0.0, 1.0])),
            occlusion: flat.clone(),
            curvature: flat,
            opacity: None,
            translucency: None,
        }
    }

    #[test]
    fn light_follows_srgb_and_the_eye() {
        assert!(light_of([0.0; 3]) < 1e-6);
        assert!((light_of([1.0; 3]) - 1.0).abs() < 1e-4);
        // 30 in 255 is about the charcoal line; 240 the snow line.
        assert!((light_of([30.0 / 255.0; 3]) - DARK).abs() < 0.002);
        assert!((light_of([240.0 / 255.0; 3]) - BRIGHT).abs() < 0.01);
        // Green looks lighter than blue.
        assert!(light_of([0.0, 1.0, 0.0]) > light_of([0.0, 0.0, 1.0]));
    }

    #[test]
    fn shares_of_dark_and_bright_are_by_area() {
        // A quarter black, a quarter white, half mid grey.
        let m = material(16, |x, _| match x / 4 {
            0 => [0.0; 3],
            1 => [1.0; 3],
            _ => [0.5; 3],
        });
        let range = colour_range(&m);
        assert!((range.dark - 0.25).abs() < 1e-6, "{range:?}");
        assert!((range.bright - 0.25).abs() < 1e-6, "{range:?}");
        assert_eq!(range.metal, 0.0);
        // Dark metal counts apart, as a share of the metal.
        let s = Size::new(16).unwrap();
        let m = MaterialMaps {
            metal: Arc::new(crate::parallel::fill(
                s,
                1,
                |x, _| if x < 8 { 1.0 } else { 0.0 },
            )),
            ..material(16, |x, _| if x < 4 { [0.2; 3] } else { [0.9; 3] })
        };
        let range = colour_range(&m);
        assert!((range.metal - 0.5).abs() < 1e-6);
        assert!((range.metal_dark - 0.5).abs() < 1e-6, "{range:?}");
        assert_eq!(range.dark, 0.0);
    }

    #[test]
    fn see_through_parts_and_broken_numbers_dont_count() {
        let s = Size::new(16).unwrap();
        let m = MaterialMaps {
            opacity: Some(Arc::new(crate::parallel::fill(s, 1, |x, _| {
                if x < 4 { 0.0 } else { 1.0 }
            }))),
            ..material(16, |x, _| if x < 4 { [0.0; 3] } else { [0.5; 3] })
        };
        assert_eq!(colour_range(&m).dark, 0.0);
        let broken = material(16, |_, _| [f32::NAN; 3]);
        assert_eq!(colour_range(&broken), ColourRange::default());
        let all_gone = MaterialMaps {
            opacity: Some(Arc::new(Image::filled(s, 0.0))),
            ..material(16, |_, _| [0.0; 3])
        };
        assert_eq!(colour_range(&all_gone), ColourRange::default());
    }

    #[test]
    fn spreads_halve_for_fine_noise_and_stay_for_big_shapes() {
        let n = 64usize;
        // Pixel-sized checks vanish at the first averaging.
        let fine: Vec<f32> = (0..n * n)
            .map(|i| {
                if (i % n + i / n).is_multiple_of(2) {
                    1.0
                } else {
                    0.0
                }
            })
            .collect();
        let s = spreads(&fine, n);
        assert!((s[0] - 0.5).abs() < 1e-6);
        assert!(s[1] < 1e-6);
        // Halves left and right keep their spread down to 2 × 2.
        let big: Vec<f32> = (0..n * n)
            .map(|i| if i % n < n / 2 { 1.0 } else { 0.0 })
            .collect();
        let s = spreads(&big, n);
        assert_eq!(s.len(), 6);
        assert!(s.iter().all(|v| (v - 0.5).abs() < 1e-6), "{s:?}");
        // Not square or not a power of two: nothing.
        assert!(spreads(&big, 63).is_empty());
        assert!(spreads(&[0.5], 1).is_empty());
    }

    #[test]
    fn the_patch_that_stands_out_is_found() {
        let n = 32usize;
        // A dark patch in the top right quarter's corner square.
        let pixels: Vec<f32> = (0..n * n)
            .map(|i| if i % n >= 24 && i / n < 8 { 0.1 } else { 0.6 })
            .collect();
        let p = patches(&pixels, n, 4).unwrap();
        assert_eq!(p.most, (3, 0));
        assert!(!p.lighter);
        assert!(p.spread > 0.1);
        let even = vec![0.4; n * n];
        assert!(patches(&even, n, 4).unwrap().spread < 1e-6);
        assert!(patches(&even, n, 3).is_none());
        assert!(patches(&even, 2, 4).is_none());
    }

    #[test]
    fn slow_waves_are_measured_by_their_size() {
        let n = 64usize;
        let wave = |amp: f32| -> Vec<f32> {
            (0..n * n)
                .map(|i| {
                    let x = (i % n) as f32 / n as f32;
                    0.5 + amp * (std::f32::consts::TAU * x).cos()
                })
                .collect()
        };
        assert!((slowest_waves(&wave(0.2), n) - 0.2).abs() < 1e-3);
        assert!((slowest_waves(&wave(0.05), n) - 0.05).abs() < 1e-3);
        // Fine checks and flat pictures have none.
        let fine: Vec<f32> = (0..n * n)
            .map(|i| {
                if (i % n + i / n).is_multiple_of(2) {
                    1.0
                } else {
                    0.0
                }
            })
            .collect();
        assert!(slowest_waves(&fine, n) < 1e-6);
        assert!(slowest_waves(&vec![0.3; n * n], n) < 1e-6);
        assert_eq!(slowest_waves(&[0.5], 1), 0.0);
        // Broken numbers are left out.
        let mut broken = wave(0.2);
        broken[5] = f32::NAN;
        assert!(slowest_waves(&broken, n).is_finite());
    }

    #[test]
    fn lines_across_are_found_also_over_the_edge() {
        let n = 32u32;
        let s = Size::new(n).unwrap();
        // Grooves at rows 0–1 and 30–31 (one line over the edge) and
        // 15–16.
        let height = Image::from_pixels(
            s,
            (0..n * n)
                .map(|i| match i / n {
                    0 | 1 | 15 | 16 | 30 | 31 => 0.0,
                    _ => 1.0,
                })
                .collect(),
        )
        .unwrap();
        let look = vec![0.5; (n * n) as usize];
        let runs = lines_across(&height, &look).unwrap();
        assert_eq!(runs.len(), 2, "{runs:?}");
        assert!(runs.contains(&(15, 16)));
        assert!(runs.contains(&(30, 33)), "{runs:?}");
        assert_eq!(rows_to_line(&runs, 0, 32, true), 0);
        assert_eq!(rows_to_line(&runs, 8, 32, true), 7);
        assert_eq!(rows_to_line(&runs, 8, 32, false), 7);
        assert_eq!(rows_to_line(&runs, 20, 32, false), 4);
        // Noise has no lines; nor does a flat picture.
        let noisy = Image::from_pixels(
            s,
            (0..n * n)
                .map(|i| crate::seed::unit(crate::seed::mix(u64::from(i), 7)))
                .collect(),
        )
        .unwrap();
        assert!(lines_across(&noisy, &look).is_none());
        assert!(lines_across(&Image::filled(s, 0.5), &look).is_none());
    }

    #[test]
    fn a_flat_height_falls_back_to_the_look() {
        let n = 16u32;
        let s = Size::new(n).unwrap();
        let look: Vec<f32> = (0..n * n)
            .map(|i| if i / n == 8 { 0.0 } else { 0.8 })
            .collect();
        let runs = lines_across(&Image::filled(s, 0.5), &look).unwrap();
        assert_eq!(runs, vec![(8, 8)]);
    }
}
