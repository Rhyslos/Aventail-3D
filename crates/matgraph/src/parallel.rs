//! Making a picture on several cores, in tiles.
//!
//! The picture is cut into bands of [`TILE`] rows; each core takes the next
//! band left until none are. Every pixel is worked out by the same pure
//! function of its place, so the picture is exactly the same however many
//! cores share it.

use crate::image::{Image, Size};
use std::sync::Mutex;

/// Rows in one tile band.
pub(crate) const TILE: usize = 64;

/// The cores to share work over, as the system reports them.
pub(crate) fn cores() -> usize {
    std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get)
}

/// A picture of `size` with `f(x, y)` at every pixel, made on up to
/// `threads` cores.
pub(crate) fn fill<P: Copy + Default + Send>(
    size: Size,
    threads: usize,
    f: impl Fn(u32, u32) -> P + Sync,
) -> Image<P> {
    rows(size, threads, |y, row| {
        for (x, px) in row.iter_mut().enumerate() {
            *px = f(x as u32, y);
        }
    })
}

/// A picture of `size` made row by row: `f(y, row)` fills row `y` (any
/// order, on up to `threads` cores; each row from its number alone).
pub(crate) fn rows<P: Copy + Default + Send>(
    size: Size,
    threads: usize,
    f: impl Fn(u32, &mut [P]) + Sync,
) -> Image<P> {
    let n = size.get() as usize;
    let mut pixels = vec![P::default(); n * n];
    let bands: Vec<(usize, &mut [P])> = pixels.chunks_mut(TILE * n).enumerate().collect();
    let threads = threads.clamp(1, bands.len().max(1));
    let work = |band: usize, rows: &mut [P]| {
        for (i, row) in rows.chunks_mut(n).enumerate() {
            f((band * TILE + i) as u32, row);
        }
    };
    if threads == 1 {
        for (band, rows) in bands {
            work(band, rows);
        }
    } else {
        let queue = Mutex::new(bands.into_iter());
        std::thread::scope(|scope| {
            for _ in 0..threads {
                scope.spawn(|| {
                    loop {
                        // A poisoned lock means another core panicked; stop.
                        let next = match queue.lock() {
                            Ok(mut q) => q.next(),
                            Err(_) => None,
                        };
                        let Some((band, rows)) = next else { break };
                        work(band, rows);
                    }
                });
            }
        });
    }
    // The pixels were made for exactly this size.
    Image::from_pixels(size, pixels).unwrap_or_else(|| Image::filled(size, P::default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_picture_is_the_same_on_one_core_or_many() {
        let size = Size::new(256).unwrap();
        let f = |x: u32, y: u32| (x * 7 + y * 13) as f32;
        let one = fill(size, 1, f);
        for threads in [2, 3, 8, 100] {
            assert_eq!(fill(size, threads, f), one);
        }
        assert_eq!(one.at(5, 200), (5 * 7 + 200 * 13) as f32);
    }
}
