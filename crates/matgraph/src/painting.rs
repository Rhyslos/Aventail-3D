//! What is painted on a Painted box (Phase 12 milestone 20,
//! [decision 0282](../../../docs/decisions/0282-painted-box.md)): a square
//! picture kept with the box, painted with the painter's brushes in the
//! Material editor.
//!
//! A [`Painting`] is stored as the painter stores its layers: RGBA bytes
//! (sRGB, straight alpha), in square tiles of [`PAINT_TILE`] pixels, each
//! shared (`Arc`) or missing (nothing painted there). A stroke copies only
//! the tiles it touches, so undo steps and saving stay small. Its size is
//! one of [`PAINTING_SIZES`]; the box scales it to whatever size the
//! graph is worked out at ([`Painting::sampled`]), wrapping round its
//! edges so the texture stays seamless.
//!
//! A painting's [`key`](Painting::key) is a hash of what is painted (not
//! of where its tiles live in memory), so the same painting opened from a
//! file has the same key, and a box's result is only worked out again
//! when the painting changes.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Pixels across one tile (the painter's tile, `paint::TILE`).
pub const PAINT_TILE: u32 = 64;

/// Bytes in one tile.
pub const PAINT_TILE_BYTES: usize = (PAINT_TILE * PAINT_TILE * 4) as usize;

/// The sizes a painting can be, in pixels across.
pub const PAINTING_SIZES: [u32; 4] = [256, 512, 1024, 2048];

/// The size a new painting is.
pub const STANDARD_PAINTING_SIZE: u32 = 1024;

/// One tile's pixels: RGBA bytes, row by row.
pub type PaintTile = Arc<Vec<u8>>;

/// A picture painted on a Painted box.
#[derive(Clone, PartialEq)]
pub struct Painting {
    size: u32,
    tiles: Vec<Option<PaintTile>>,
    key: u64,
}

impl std::fmt::Debug for Painting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Painting")
            .field("size", &self.size)
            .field("painted_tiles", &self.painted_tiles())
            .field("key", &self.key)
            .finish()
    }
}

/// Tiles across a painting `size` pixels across.
fn across(size: u32) -> usize {
    size.div_ceil(PAINT_TILE) as usize
}

impl Painting {
    /// An empty painting `size` pixels across; `None` for a size not in
    /// [`PAINTING_SIZES`].
    pub fn new(size: u32) -> Option<Self> {
        Self::from_tiles(size, vec![None; across(size) * across(size)]).ok()
    }

    /// A painting from its tiles, row by row from the top left; or why it
    /// can't be one (as a file's would be checked). Tiles with nothing
    /// painted are left out, so two paintings of the same pixels are the
    /// same.
    pub fn from_tiles(size: u32, mut tiles: Vec<Option<PaintTile>>) -> Result<Self, &'static str> {
        if !PAINTING_SIZES.contains(&size) {
            return Err(words::text!(
                "a painting's size isn't one a painting can be"
            ));
        }
        if tiles.len() != across(size) * across(size) {
            return Err(words::text!("a painting has the wrong number of tiles"));
        }
        for tile in &mut tiles {
            match tile {
                Some(t) if t.len() != PAINT_TILE_BYTES => {
                    return Err(words::text!("a painted tile is the wrong size"));
                }
                Some(t) if t.chunks_exact(4).all(|p| p[3] == 0) => *tile = None,
                _ => {}
            }
        }
        let key = key_of(size, &tiles);
        Ok(Self { size, tiles, key })
    }

    /// A painting `size` pixels across from RGBA bytes row by row (sRGB,
    /// straight alpha); `None` for a size it can't be or the wrong number
    /// of bytes.
    pub fn from_rgba(size: u32, rgba: &[u8]) -> Option<Self> {
        let n = size as usize;
        if !PAINTING_SIZES.contains(&size) || rgba.len() != n * n * 4 {
            return None;
        }
        let a = across(size);
        let t = PAINT_TILE as usize;
        let mut tiles = Vec::with_capacity(a * a);
        for ty in 0..a {
            for tx in 0..a {
                let mut tile = vec![0u8; PAINT_TILE_BYTES];
                for row in 0..t {
                    let from = ((ty * t + row) * n + tx * t) * 4;
                    tile[row * t * 4..(row + 1) * t * 4].copy_from_slice(&rgba[from..from + t * 4]);
                }
                tiles.push(Some(Arc::new(tile)));
            }
        }
        Self::from_tiles(size, tiles).ok()
    }

    /// Its pixels as RGBA bytes row by row from the top (sRGB, straight
    /// alpha; transparent where nothing is painted).
    pub fn to_rgba(&self) -> Vec<u8> {
        let n = self.size as usize;
        let t = PAINT_TILE as usize;
        let a = across(self.size);
        let mut out = vec![0u8; n * n * 4];
        for (index, tile) in self.tiles.iter().enumerate() {
            let Some(tile) = tile else { continue };
            let (tx, ty) = (index % a, index / a);
            for row in 0..t {
                let to = ((ty * t + row) * n + tx * t) * 4;
                out[to..to + t * 4].copy_from_slice(&tile[row * t * 4..(row + 1) * t * 4]);
            }
        }
        out
    }

    /// Pixels across (and down).
    pub fn size(&self) -> u32 {
        self.size
    }

    /// Its tiles, row by row from the top left (`None`: nothing painted).
    pub fn tiles(&self) -> &[Option<PaintTile>] {
        &self.tiles
    }

    /// How many tiles have paint.
    pub fn painted_tiles(&self) -> usize {
        self.tiles.iter().filter(|t| t.is_some()).count()
    }

    /// Whether nothing is painted.
    pub fn is_empty(&self) -> bool {
        self.tiles.iter().all(Option::is_none)
    }

    /// A hash of its size and what is painted: the same for the same
    /// pixels, wherever they come from.
    pub fn key(&self) -> u64 {
        self.key
    }

    /// The pixel at column `x`, row `y`, wrapping round past any edge.
    pub fn pixel(&self, x: i64, y: i64) -> [u8; 4] {
        let n = i64::from(self.size);
        let (x, y) = (x.rem_euclid(n) as u32, y.rem_euclid(n) as u32);
        let index = (y / PAINT_TILE) as usize * across(self.size) + (x / PAINT_TILE) as usize;
        match &self.tiles[index] {
            Some(tile) => {
                let at = (((y % PAINT_TILE) * PAINT_TILE + x % PAINT_TILE) * 4) as usize;
                [tile[at], tile[at + 1], tile[at + 2], tile[at + 3]]
            }
            None => [0; 4],
        }
    }

    /// Roughly how much memory it takes, in bytes.
    pub fn memory_size(&self) -> usize {
        64 + self.tiles.len() * 8 + self.painted_tiles() * PAINT_TILE_BYTES
    }

    /// The bytes of tiles `a` and `b` don't share, place by place: what an
    /// undo step changing one into the other holds on to beyond them.
    pub fn bytes_apart(a: Option<&Painting>, b: Option<&Painting>) -> usize {
        match (a, b) {
            (Some(a), Some(b)) if a.size == b.size => a
                .tiles
                .iter()
                .zip(&b.tiles)
                .map(|(x, y)| match (x, y) {
                    (Some(x), Some(y)) if Arc::ptr_eq(x, y) => 0,
                    _ => (usize::from(x.is_some()) + usize::from(y.is_some())) * PAINT_TILE_BYTES,
                })
                .sum(),
            _ => a.map_or(0, Painting::memory_size) + b.map_or(0, Painting::memory_size),
        }
    }

    /// The painting at `size` pixels across (each pixel premultiplied:
    /// red, green and blue times alpha, then alpha, all 0 to 1), row by
    /// row from the top. A smaller size averages each block of pixels; a
    /// larger one blends between them, wrapping round the edges, so the
    /// result tiles as the painting does. `size` is a power of two.
    pub fn sampled(&self, size: u32) -> Vec<[f32; 4]> {
        let n = size.max(1) as usize;
        let own = self.size as usize;
        let premultiplied = |p: [u8; 4]| {
            let a = f32::from(p[3]) / 255.0;
            [
                f32::from(p[0]) / 255.0 * a,
                f32::from(p[1]) / 255.0 * a,
                f32::from(p[2]) / 255.0 * a,
                a,
            ]
        };
        let mut out = vec![[0.0f32; 4]; n * n];
        if self.is_empty() {
            return out;
        }
        if n <= own {
            // Each pixel the average of the block it covers.
            let block = (own / n).max(1);
            let share = 1.0 / (block * block) as f32;
            for (i, pixel) in out.iter_mut().enumerate() {
                let (x, y) = ((i % n) * block, (i / n) * block);
                let mut sum = [0.0f32; 4];
                for dy in 0..block {
                    for dx in 0..block {
                        let p = premultiplied(self.pixel((x + dx) as i64, (y + dy) as i64));
                        for c in 0..4 {
                            sum[c] += p[c];
                        }
                    }
                }
                *pixel = sum.map(|v| v * share);
            }
        } else {
            // Blended between the four nearest, wrapping round.
            let scale = own as f32 / n as f32;
            for (i, pixel) in out.iter_mut().enumerate() {
                let fx = ((i % n) as f32 + 0.5) * scale - 0.5;
                let fy = ((i / n) as f32 + 0.5) * scale - 0.5;
                let (x0, y0) = (fx.floor(), fy.floor());
                let (tx, ty) = (fx - x0, fy - y0);
                let (x0, y0) = (x0 as i64, y0 as i64);
                let at = |x, y| premultiplied(self.pixel(x, y));
                let (a, b, c, d) = (
                    at(x0, y0),
                    at(x0 + 1, y0),
                    at(x0, y0 + 1),
                    at(x0 + 1, y0 + 1),
                );
                for k in 0..4 {
                    let top = a[k] + (b[k] - a[k]) * tx;
                    let bottom = c[k] + (d[k] - c[k]) * tx;
                    pixel[k] = top + (bottom - top) * ty;
                }
            }
        }
        out
    }

    /// The same painting at another of [`PAINTING_SIZES`] (smoothly
    /// scaled, wrapping round); `None` for a size it can't be.
    pub fn resized(&self, size: u32) -> Option<Painting> {
        if !PAINTING_SIZES.contains(&size) {
            return None;
        }
        if size == self.size {
            return Some(self.clone());
        }
        let rgba: Vec<u8> = self
            .sampled(size)
            .into_iter()
            .flat_map(|[r, g, b, a]| {
                let straight = |v: f32| {
                    if a > 1e-6 {
                        (v / a * 255.0).round().clamp(0.0, 255.0) as u8
                    } else {
                        0
                    }
                };
                [
                    straight(r),
                    straight(g),
                    straight(b),
                    (a * 255.0).round().clamp(0.0, 255.0) as u8,
                ]
            })
            .collect();
        Self::from_rgba(size, &rgba)
    }
}

/// The key of a painting `size` across with `tiles`.
fn key_of(size: u32, tiles: &[Option<PaintTile>]) -> u64 {
    let mut hasher = DefaultHasher::new();
    size.hash(&mut hasher);
    for (index, tile) in tiles.iter().enumerate() {
        if let Some(tile) = tile {
            index.hash(&mut hasher);
            tile.as_slice().hash(&mut hasher);
        }
    }
    hasher.finish()
}

/// How light an sRGB colour (0 to 1 each) looks, 0 to 1: what a mask or
/// height painted with it holds (the painter's masks read paint the same
/// way, by lightness).
pub fn lightness([r, g, b]: [f32; 3]) -> f32 {
    (0.2126 * r + 0.7152 * g + 0.0722 * b).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dotted(size: u32) -> Painting {
        let n = size as usize;
        let mut rgba = vec![0u8; n * n * 4];
        for (i, p) in rgba.chunks_exact_mut(4).enumerate() {
            if (i % n).is_multiple_of(7) && (i / n).is_multiple_of(5) {
                p.copy_from_slice(&[200, 100, 50, 255]);
            }
        }
        Painting::from_rgba(size, &rgba).unwrap()
    }

    #[test]
    fn only_the_allowed_sizes_are_made() {
        for size in PAINTING_SIZES {
            let p = Painting::new(size).unwrap();
            assert!(p.is_empty());
            assert_eq!(p.size(), size);
            assert_eq!(p.tiles().len(), across(size) * across(size));
        }
        for size in [0, 16, 100, 300, 4096, 8192] {
            assert!(Painting::new(size).is_none(), "{size}");
        }
    }

    #[test]
    fn damaged_tiles_are_refused() {
        assert!(Painting::from_tiles(256, vec![None; 3]).is_err());
        let mut tiles = vec![None; 16];
        tiles[2] = Some(Arc::new(vec![1u8; 10]));
        assert!(Painting::from_tiles(256, tiles).is_err());
        assert!(Painting::from_rgba(256, &[0; 12]).is_none());
    }

    #[test]
    fn rgba_round_trips_and_empty_tiles_are_dropped() {
        let p = dotted(256);
        let back = Painting::from_rgba(256, &p.to_rgba()).unwrap();
        assert_eq!(p, back);
        assert_eq!(p.key(), back.key());
        // A tile painted with nothing see-through is left out.
        let mut tiles = vec![None; 16];
        tiles[5] = Some(Arc::new(
            vec![9u8; PAINT_TILE_BYTES]
                .iter()
                .enumerate()
                .map(|(i, v)| if i % 4 == 3 { 0 } else { *v })
                .collect(),
        ));
        let p = Painting::from_tiles(256, tiles).unwrap();
        assert!(p.is_empty());
        assert_eq!(p.key(), Painting::new(256).unwrap().key());
    }

    #[test]
    fn the_key_follows_the_pixels_not_the_memory() {
        let a = dotted(512);
        let b = dotted(512);
        assert_eq!(a.key(), b.key());
        let mut rgba = a.to_rgba();
        rgba[7] = 255;
        let c = Painting::from_rgba(512, &rgba).unwrap();
        assert_ne!(a.key(), c.key());
        assert_ne!(
            Painting::new(256).unwrap().key(),
            Painting::new(512).unwrap().key()
        );
    }

    #[test]
    fn pixels_wrap_round() {
        let p = dotted(256);
        assert_eq!(p.pixel(0, 0), [200, 100, 50, 255]);
        assert_eq!(p.pixel(256, -256), p.pixel(0, 0));
        assert_eq!(p.pixel(-1, 0), p.pixel(255, 0));
    }

    #[test]
    fn sampling_keeps_the_average_and_wraps() {
        let p = dotted(512);
        let mean = |v: &[[f32; 4]]| v.iter().map(|p| p[3]).sum::<f32>() / v.len() as f32;
        let own = mean(&p.sampled(512));
        for size in [16, 64, 256, 1024] {
            let s = p.sampled(size);
            assert_eq!(s.len(), (size * size) as usize);
            assert!(
                (mean(&s) - own).abs() < 0.02,
                "{size}: {} {}",
                mean(&s),
                own
            );
        }
        // Bigger: the far edge blends into the near one (it wraps).
        let mut rgba = vec![0u8; 256 * 256 * 4];
        for y in 0..256 {
            let at = (y * 256 + 255) * 4;
            rgba[at..at + 4].copy_from_slice(&[255, 255, 255, 255]);
        }
        let edge = Painting::from_rgba(256, &rgba).unwrap();
        let s = edge.sampled(1024);
        assert!(s[0][3] > 0.1, "the left edge picks up the right one");
        assert!(
            Painting::new(256)
                .unwrap()
                .sampled(64)
                .iter()
                .all(|p| p[3] == 0.0)
        );
    }

    #[test]
    fn resizing_keeps_the_picture() {
        let p = dotted(512);
        let up = p.resized(1024).unwrap();
        assert_eq!(up.size(), 1024);
        let back = up.resized(512).unwrap();
        // Mostly the same where painted.
        let (mut same, mut all) = (0, 0);
        for y in 0..512 {
            for x in 0..512 {
                if p.pixel(x, y)[3] > 0 {
                    all += 1;
                    if back.pixel(x, y)[3] > 100 {
                        same += 1;
                    }
                }
            }
        }
        assert!(same * 10 >= all * 9, "{same} of {all}");
        assert_eq!(p.resized(512).unwrap(), p);
        assert!(p.resized(300).is_none());
    }

    #[test]
    fn bytes_apart_counts_only_unshared_tiles() {
        let a = dotted(256);
        assert_eq!(Painting::bytes_apart(Some(&a), Some(&a.clone())), 0);
        let mut tiles = a.tiles().to_vec();
        tiles[0] = Some(Arc::new(vec![255u8; PAINT_TILE_BYTES]));
        let b = Painting::from_tiles(256, tiles).unwrap();
        assert_eq!(
            Painting::bytes_apart(Some(&a), Some(&b)),
            2 * PAINT_TILE_BYTES
        );
        assert!(Painting::bytes_apart(None, Some(&b)) >= b.painted_tiles() * PAINT_TILE_BYTES);
    }

    #[test]
    fn lightness_runs_from_black_to_white() {
        assert_eq!(lightness([0.0; 3]), 0.0);
        assert!((lightness([1.0; 3]) - 1.0).abs() < 1e-5);
        assert!(lightness([0.0, 1.0, 0.0]) > lightness([1.0, 0.0, 0.0]));
    }
}
