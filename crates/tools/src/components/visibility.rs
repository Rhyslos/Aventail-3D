//! Which of many points can be seen, quickly: for a box or lasso drag
//! with Select only visible over a heavy shape, where looking along each
//! point's own line of sight (one ray per point) took most of a second.
//!
//! The screen area is sampled once per pixel: one ray through each
//! pixel's middle, keeping how far it went before it hit a surface (its
//! *depth*). Where a point lands on the screen, the nearest surface's
//! depth is read between the four pixels' middles round it (blended, as
//! the surface is smooth at that scale), with how far that reading could
//! be off, judged from how much the depths bend over the 4 × 4 pixels
//! round it. On or in front of that surface, the point can be seen; well
//! behind it, it can't. Where the depths jump (an outline, where one
//! surface passes in front of another, or nothing is hit) or the reading
//! doesn't settle it, the point gets its own exact ray, as
//! [`PickScene::can_see`] gives every point when there are few. So the
//! answer is the exact one everywhere but for something thinner than a
//! pixel in front of a point, which the screen can't show either (and the
//! odd exact ray that slips between two triangles).

use super::PickScene;
use crate::camera::Camera;
use glam::{Vec2, Vec3};

/// From this many points, sampling the screen can be quicker than a ray
/// per point.
const SAMPLE_FROM: usize = 256;

/// The depths of the pixels in a box of the screen: how far the ray
/// through each pixel's middle goes before hitting a surface (infinite
/// where it hits none).
struct DepthGrid {
    /// The first pixel's column and row.
    x0: i64,
    y0: i64,
    width: usize,
    height: usize,
    depths: Vec<f32>,
}

impl DepthGrid {
    /// The grid over the pixels of `area` (clamped to the screen) and one
    /// more round it, or `None` if that's more pixels than `most`.
    fn pixels(area: (Vec2, Vec2), size: Vec2, most: usize) -> Option<(i64, i64, usize, usize)> {
        let (lo, hi) = area;
        if !(lo.is_finite() && hi.is_finite() && size.is_finite()) {
            return None;
        }
        let x0 = (lo.x.floor() as i64 - 1).max(0);
        let y0 = (lo.y.floor() as i64 - 1).max(0);
        let x1 = (hi.x.ceil() as i64 + 1).min(size.x.ceil() as i64);
        let y1 = (hi.y.ceil() as i64 + 1).min(size.y.ceil() as i64);
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        let (width, height) = ((x1 - x0) as usize, (y1 - y0) as usize);
        (width.saturating_mul(height) <= most).then_some((x0, y0, width, height))
    }

    /// The depth of pixel (`x`, `y`), if it is in the grid.
    fn at(&self, x: i64, y: i64) -> Option<f32> {
        let (dx, dy) = (x - self.x0, y - self.y0);
        if dx < 0 || dy < 0 || dx as usize >= self.width || dy as usize >= self.height {
            return None;
        }
        self.depths
            .get(dy as usize * self.width + dx as usize)
            .copied()
    }

    /// The nearest surface's depth at `screen`, blended from the four
    /// pixels' middles round it, and how much the depths bend from one
    /// pixel to the next over the 4 × 4 pixels round it (the most), which
    /// says how far the blend could be off. `None`
    /// if any of them is outside the grid or hits nothing (an outline).
    fn surface_at(&self, screen: Vec2) -> Option<(f32, f32)> {
        // Pixel (x, y)'s middle is at (x + 0.5, y + 0.5).
        let (u, v) = (screen.x - 0.5, screen.y - 0.5);
        let (x0, y0) = (u.floor() as i64, v.floor() as i64);
        let (fx, fy) = (u - u.floor(), v - v.floor());
        let mut block = [[0.0f32; 4]; 4];
        for (row, y) in (y0 - 1..=y0 + 2).enumerate() {
            for (column, x) in (x0 - 1..=x0 + 2).enumerate() {
                block[row][column] = self.at(x, y).filter(|d| d.is_finite())?;
            }
        }
        let bend = |a: f32, b: f32, c: f32| (a - 2.0 * b + c).abs();
        let mut off: f32 = 0.0;
        for row in &block {
            for three in row.windows(3) {
                off = off.max(bend(three[0], three[1], three[2]));
            }
        }
        for rows in block.windows(3) {
            for ((a, b), c) in rows[0].iter().zip(&rows[1]).zip(&rows[2]) {
                off = off.max(bend(*a, *b, *c));
            }
        }
        let (c00, c10, c01, c11) = (block[1][1], block[1][2], block[2][1], block[2][2]);
        off = off.max((c00 - c10 - c01 + c11).abs());
        let top = c00 + (c10 - c00) * fx;
        let bottom = c01 + (c11 - c01) * fx;
        Some((top + (bottom - top) * fy, off))
    }
}

/// What the pixels round a point say about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Seen,
    Hidden,
    /// Too close to call: look along its own line of sight.
    Unsure,
}

/// The verdict on a point at `depth` along its line of sight, where the
/// nearest surface is `surface` deep, with depths bending by up to `bend`
/// from one pixel to the next round it. Reading between pixels is off by
/// less than that (an eighth of it each way across, on a smooth surface);
/// the whole bend is allowed, as a shape's tiny facets and creases bend
/// less evenly (half was too little in the tests). `tolerance` is how far behind a
/// surface a point may be and still count as on it, as
/// [`PickScene::can_see`] allows; a quarter of it more is left either way
/// for rounding.
fn verdict(depth: f32, (surface, bend): (f32, f32), tolerance: f32) -> Verdict {
    let margin = bend + 0.25 * tolerance;
    if !(depth.is_finite() && surface.is_finite() && margin.is_finite()) {
        Verdict::Unsure
    } else if depth - tolerance <= surface - margin {
        Verdict::Seen
    } else if depth - tolerance > surface + margin {
        Verdict::Hidden
    } else {
        Verdict::Unsure
    }
}

/// `work` done on each part of `items` on a thread of its own (as many as
/// the computer has cores, up to 8), the answers put back in order. Rays
/// are independent of one another, so this is a plain split.
fn in_parts<T: Sync, A: Send>(items: &[T], work: impl Fn(&[T]) -> Vec<A> + Sync) -> Vec<A> {
    let threads = std::thread::available_parallelism()
        .map_or(1, std::num::NonZeroUsize::get)
        .min(8);
    if threads <= 1 || items.len() < 2 * SAMPLE_FROM {
        return work(items);
    }
    let part = items.len().div_ceil(threads);
    std::thread::scope(|scope| {
        let handles: Vec<_> = items
            .chunks(part)
            .map(|chunk| scope.spawn(|| work(chunk)))
            .collect();
        // A part whose thread failed is done again here, so every answer
        // stays in its place.
        handles
            .into_iter()
            .zip(items.chunks(part))
            .flat_map(|(handle, chunk)| handle.join().unwrap_or_else(|_| work(chunk)))
            .collect()
    })
}

impl PickScene {
    /// For each of `points` (in the world, seen inside the screen box
    /// `area`), whether it can be seen: the same answers as
    /// [`Self::can_see`] for each, worked out from one ray per pixel of
    /// `area` when that's fewer rays than one per point (see the module
    /// notes).
    pub(super) fn seen_all(
        &self,
        camera: &Camera,
        size: Vec2,
        area: (Vec2, Vec2),
        points: &[Vec3],
    ) -> Vec<bool> {
        let exact = || {
            points
                .iter()
                .map(|p| self.can_see(camera, size, *p))
                .collect()
        };
        if points.len() < SAMPLE_FROM {
            return exact();
        }
        // Sampling costs a ray per pixel: worth it only with fewer pixels
        // than points.
        let Some((x0, y0, width, height)) = DepthGrid::pixels(area, size, points.len() / 2) else {
            return exact();
        };
        let grid = self.depth_grid(camera, size, (x0, y0, width, height));
        in_parts(points, |part| {
            part.iter()
                .map(|p| self.seen_with(&grid, camera, size, *p))
                .collect()
        })
    }

    /// The depths of the pixels `x0, y0, width, height`.
    fn depth_grid(
        &self,
        camera: &Camera,
        size: Vec2,
        (x0, y0, width, height): (i64, i64, usize, usize),
    ) -> DepthGrid {
        let pixels: Vec<usize> = (0..width * height).collect();
        let depths = in_parts(&pixels, |part| {
            part.iter()
                .map(|i| {
                    let middle = Vec2::new(
                        (x0 + (i % width) as i64) as f32 + 0.5,
                        (y0 + (i / width) as i64) as f32 + 0.5,
                    );
                    let ray = camera.ray_through(middle, size);
                    self.first_hit(&ray).map_or(f32::INFINITY, |(d, ..)| d)
                })
                .collect()
        });
        DepthGrid {
            x0,
            y0,
            width,
            height,
            depths,
        }
    }

    /// Whether `point` can be seen, from `grid` where it settles it, else
    /// from its own ray.
    fn seen_with(&self, grid: &DepthGrid, camera: &Camera, size: Vec2, point: Vec3) -> bool {
        let Some(screen) = camera.project(point, size) else {
            return false;
        };
        let ray = camera.ray_through(screen, size);
        let depth = (point - ray.origin).dot(ray.direction);
        let tolerance = super::seen_tolerance(depth);
        let verdict = grid.surface_at(screen).map_or(Verdict::Unsure, |surface| {
            verdict(depth, surface, tolerance)
        });
        match verdict {
            Verdict::Seen => true,
            Verdict::Hidden => false,
            Verdict::Unsure => self.can_see(camera, size, point),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_on_the_nearest_surface_is_seen_and_one_behind_it_hidden() {
        let tolerance = 0.1;
        assert_eq!(verdict(100.0, (100.0, 0.01), tolerance), Verdict::Seen);
        assert_eq!(verdict(150.0, (100.0, 0.2), tolerance), Verdict::Hidden);
        // Just behind it, where the reading could be off: its own ray.
        assert_eq!(verdict(100.15, (100.0, 0.05), tolerance), Verdict::Unsure);
        assert_eq!(verdict(100.0, (99.0, 2.0), tolerance), Verdict::Unsure);
        assert_eq!(verdict(f32::NAN, (99.0, 0.0), tolerance), Verdict::Unsure);
    }

    #[test]
    fn the_grid_covers_the_area_and_a_pixel_more_within_the_screen() {
        let size = Vec2::new(100.0, 80.0);
        let pixels = DepthGrid::pixels((Vec2::new(10.2, 20.0), Vec2::new(19.5, 30.0)), size, 1_000);
        assert_eq!(pixels, Some((9, 19, 12, 12)));
        // Clamped to the screen.
        let pixels = DepthGrid::pixels((Vec2::new(-50.0, -50.0), Vec2::new(5.0, 5.0)), size, 1_000);
        assert_eq!(pixels, Some((0, 0, 6, 6)));
        // Too many pixels, nothing on screen, or not real numbers.
        assert_eq!(DepthGrid::pixels((Vec2::ZERO, size), size, 100), None);
        assert_eq!(
            DepthGrid::pixels((Vec2::splat(200.0), Vec2::splat(300.0)), size, 1_000),
            None
        );
        assert_eq!(
            DepthGrid::pixels((Vec2::splat(f32::NAN), Vec2::ONE), size, 1_000),
            None
        );
    }

    /// A 5 × 5 grid of depths `depth(x, y)` at the pixels' middles.
    fn grid_of(depth: impl Fn(f32, f32) -> f32) -> DepthGrid {
        let depths = (0..25)
            .map(|i| depth((i % 5) as f32 + 0.5, (i / 5) as f32 + 0.5))
            .collect();
        DepthGrid {
            x0: 0,
            y0: 0,
            width: 5,
            height: 5,
            depths,
        }
    }

    #[test]
    fn a_sloping_surface_is_read_between_the_pixels() {
        let grid = grid_of(|x, y| 100.0 + 2.0 * x + 3.0 * y);
        let (surface, off) = grid.surface_at(Vec2::new(2.25, 2.75)).unwrap();
        assert!((surface - (100.0 + 4.5 + 8.25)).abs() < 1e-4, "{surface}");
        assert!(off < 1e-4, "flat: {off}");
        // A bent one says it could be off.
        let grid = grid_of(|x, y| 100.0 + x * x + y);
        let (_, off) = grid.surface_at(Vec2::new(2.25, 2.75)).unwrap();
        assert!((off - 2.0).abs() < 1e-3, "{off}");
    }

    #[test]
    fn an_outline_or_the_grid_s_edge_settles_nothing() {
        let mut grid = grid_of(|_, _| 1.0);
        assert_eq!(grid.surface_at(Vec2::new(2.5, 2.5)), Some((1.0, 0.0)));
        assert_eq!(
            grid.surface_at(Vec2::new(0.6, 0.6)),
            None,
            "at the grid's edge"
        );
        grid.depths[18] = f32::INFINITY;
        assert_eq!(
            grid.surface_at(Vec2::new(2.5, 2.5)),
            None,
            "a ray hits nothing"
        );
    }
}
