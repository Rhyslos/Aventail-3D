//! The remesh grid's values kept only near the surface (decision 0312):
//! the grid is cut into blocks of 8 × 8 × 8 points, and only blocks the
//! surface passes near hold values. Everywhere else a point only needs to
//! know which side of the surface it is on, which its row's crossings
//! tell ([`Rows`]). So a grid of a billion points costs what its surface
//! costs, and the blocks are worked out on every core.
//!
//! A point's value is its distance to the surface (at most
//! [`BAND`] steps, past which every point is that far), negative inside:
//! the sign says the side, even at no distance (`-0.0` is inside).

use super::field::{BAND, FILLED, NO_TRIANGLE, Rows, cell_index, fill_for, segment_distance};
use super::{Grid, Progress, RemeshProblem, Soup};
use crate::bvh::closest_on_triangle;
use glam::Vec3;

/// Points along each side of a block.
pub(super) const SIDE: usize = 8;
/// Points in a block.
pub(super) const BLOCK: usize = SIDE * SIDE * SIDE;
/// No block holds values here.
const NONE: u32 = u32::MAX;
/// The most blocks a grid may have (held or not): a table of this many
/// takes 128 MB. Pieces far apart for their voxel size would pass it.
pub(super) const MAX_BLOCKS: u64 = 32_000_000;
/// The most points made inside to clean up corner-only touches: real
/// shapes need a handful.
const MOST_FILLS: usize = 10_000_000;
/// Blocks each thread takes at least, when the work is shared.
const BLOCKS_PER_THREAD: usize = 64;

/// The grid's values near the surface.
pub(super) struct Sparse {
    pub(super) grid: Grid,
    /// Blocks along X, Y and Z.
    dims: [usize; 3],
    /// Each block's slot in `values`, or [`NONE`].
    slot_of: Vec<u32>,
    /// Each slot's block.
    pub(super) blocks: Vec<usize>,
    /// [`BLOCK`] values a slot: the signed distance (see the module notes).
    pub(super) values: Vec<f32>,
    /// The triangle nearest each point within [`BAND`] steps of the
    /// surface, or [`NO_TRIANGLE`].
    pub(super) nearest: Vec<u32>,
    rows: Rows,
    /// How far a point far from the surface is.
    far: f32,
}

/// How many blocks along each axis a grid of `size` points has.
pub(super) fn block_dims(size: [usize; 3]) -> [usize; 3] {
    size.map(|n| n.div_ceil(SIDE))
}

impl Sparse {
    /// The values of `grid` round `soup`, cleaned so the surface between
    /// inside and outside is a clean solid; `progress` goes from `from`
    /// to `to` of the whole job.
    pub(super) fn new(
        grid: Grid,
        soup: &Soup,
        progress: &Progress,
        (from, to): (f32, f32),
    ) -> Result<Self, RemeshProblem> {
        let dims = block_dims(grid.size);
        let total = dims.iter().map(|d| *d as u64).product::<u64>();
        if total > MAX_BLOCKS {
            return Err(RemeshProblem::TooSpread);
        }
        let mut slot_of = vec![NONE; total as usize];
        let band = BAND * grid.step;
        // Which blocks the surface passes near, and the triangles near
        // each (in triangle order, so ties go as they always have).
        let reach = |t: &[Vec3; 3]| -> Option<[[usize; 3]; 2]> {
            let low = t[0].min(t[1]).min(t[2]) - Vec3::splat(band);
            let high = t[0].max(t[1]).max(t[2]) + Vec3::splat(band);
            let a = cell_index(&grid, low, f32::ceil);
            let b = cell_index(&grid, high, f32::floor);
            (a[0] <= b[0] && a[1] <= b[1] && a[2] <= b[2]).then_some([a, b])
        };
        let block_of = |b: [usize; 3]| b[0] + dims[0] * (b[1] + dims[1] * b[2]);
        let each_block = |[a, b]: [[usize; 3]; 2], f: &mut dyn FnMut(usize)| {
            for bk in a[2] / SIDE..=b[2] / SIDE {
                for bj in a[1] / SIDE..=b[1] / SIDE {
                    for bi in a[0] / SIDE..=b[0] / SIDE {
                        f(block_of([bi, bj, bk]));
                    }
                }
            }
        };
        for r in soup.triangles.iter().filter_map(reach) {
            each_block(r, &mut |b| slot_of[b] = 0);
        }
        let mut blocks = Vec::new();
        for (b, slot) in slot_of.iter_mut().enumerate() {
            if *slot == 0 {
                *slot = u32::try_from(blocks.len()).map_err(|_| RemeshProblem::TooBig)?;
                blocks.push(b);
            }
        }
        if blocks.len() as u64 * BLOCK as u64 >= u64::from(u32::MAX) {
            return Err(RemeshProblem::TooBig);
        }
        // Each slot's triangles, packed: `starts[s]..starts[s + 1]`.
        let mut starts = vec![0u32; blocks.len() + 1];
        for r in soup.triangles.iter().filter_map(reach) {
            each_block(r, &mut |b| starts[slot_of[b] as usize + 1] += 1);
        }
        for s in 1..starts.len() {
            starts[s] += starts[s - 1];
        }
        let mut filled = starts.clone();
        let mut near = vec![0u32; *starts.last().unwrap_or(&0) as usize];
        for (t, triangle) in soup.triangles.iter().enumerate() {
            if let Some(r) = reach(triangle) {
                each_block(r, &mut |b| {
                    let s = slot_of[b] as usize;
                    near[filled[s] as usize] = t as u32;
                    filled[s] += 1;
                });
            }
        }
        let share = |x: f32| from + (to - from) * x;
        progress.set(share(0.1));
        let rows = Rows::new(&grid, soup);
        progress.check()?;
        progress.set(share(0.25));
        let mut field = Self {
            grid,
            dims,
            slot_of,
            blocks,
            values: Vec::new(),
            nearest: Vec::new(),
            rows,
            far: band,
        };
        field.fill_values(soup, &starts, &near, progress, (share(0.25), share(0.85)))?;
        drop(near);
        progress.set(share(0.85));
        field.clean_corners()?;
        progress.set(to);
        Ok(field)
    }

    /// Where block `b` starts, in grid points.
    fn block_start(&self, b: usize) -> [usize; 3] {
        let [dx, dy, _] = self.dims;
        [b % dx, (b / dx) % dy, b / (dx * dy)].map(|c| c * SIDE)
    }

    /// Works out every held block's values, on every core.
    fn fill_values(
        &mut self,
        soup: &Soup,
        starts: &[u32],
        near: &[u32],
        progress: &Progress,
        (from, to): (f32, f32),
    ) -> Result<(), RemeshProblem> {
        let count = self.blocks.len();
        let done = std::sync::atomic::AtomicUsize::new(0);
        let this = &*self;
        let parts = crate::parallel::in_parallel_from(count, BLOCKS_PER_THREAD, |range| {
            let mut values = Vec::with_capacity(range.len() * BLOCK);
            let mut nearest = Vec::with_capacity(range.len() * BLOCK);
            for s in range {
                if progress.cancelled() {
                    break;
                }
                let triangles = &near[starts[s] as usize..starts[s + 1] as usize];
                let (v, n) = this.block_values(this.blocks[s], soup, triangles);
                values.extend_from_slice(&v);
                nearest.extend_from_slice(&n);
                let d = done.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if d.is_multiple_of(256) {
                    progress.set(from + (to - from) * d as f32 / count.max(1) as f32);
                }
            }
            (values, nearest)
        });
        progress.check()?;
        let mut values = Vec::with_capacity(count * BLOCK);
        let mut nearest = Vec::with_capacity(count * BLOCK);
        for (v, n) in parts {
            values.extend(v);
            nearest.extend(n);
        }
        self.values = values;
        self.nearest = nearest;
        Ok(())
    }

    /// Block `b`'s values from the triangles near it.
    fn block_values(
        &self,
        b: usize,
        soup: &Soup,
        triangles: &[u32],
    ) -> ([f32; BLOCK], [u32; BLOCK]) {
        let grid = &self.grid;
        let start = self.block_start(b);
        let end = [0, 1, 2].map(|a| (start[a] + SIDE).min(grid.size[a]) - 1);
        let mut distance = [f32::INFINITY; BLOCK];
        let mut nearest = [NO_TRIANGLE; BLOCK];
        let band = BAND * grid.step;
        for &t in triangles {
            let [a, b, c] = soup.triangles[t as usize];
            let low = cell_index(grid, a.min(b).min(c) - Vec3::splat(band), f32::ceil);
            let high = cell_index(grid, a.max(b).max(c) + Vec3::splat(band), f32::floor);
            let lo = [0, 1, 2].map(|x| low[x].max(start[x]));
            let hi = [0, 1, 2].map(|x| high[x].min(end[x]));
            if lo[0] > hi[0] || lo[1] > hi[1] || lo[2] > hi[2] {
                continue;
            }
            let normal = (b - a).cross(c - a);
            let flat = normal.length_squared() <= 1e-12 * (b - a).length_squared().max(1e-12);
            let unit = if flat { Vec3::ZERO } else { normal.normalize() };
            for k in lo[2]..=hi[2] {
                for j in lo[1]..=hi[1] {
                    for i in lo[0]..=hi[0] {
                        let p = grid.point(i, j, k);
                        let at = local(i - start[0], j - start[1], k - start[2]);
                        // Quick skip: the triangle's plane is too far, or
                        // farther than a triangle already found.
                        if !flat && unit.dot(p - a).abs() >= band.min(distance[at]) {
                            continue;
                        }
                        let d = if flat {
                            segment_distance(p, a, b)
                                .min(segment_distance(p, b, c))
                                .min(segment_distance(p, c, a))
                        } else {
                            p.distance(closest_on_triangle(p, a, b, c))
                        };
                        if d < distance[at] {
                            distance[at] = d;
                            nearest[at] = t;
                        }
                    }
                }
            }
        }
        let mut values = [self.far; BLOCK];
        let mut inside = [false; SIDE];
        for k in 0..SIDE {
            self.rows
                .rows_along(grid, start[1], start[2] + k, SIDE, |j, hits| {
                    Rows::inside(grid, hits, start[0], &mut inside);
                    for (i, side) in inside.iter().enumerate() {
                        let d = distance[local(i, j, k)].min(self.far);
                        values[local(i, j, k)] = if *side { -d } else { d };
                    }
                });
        }
        (values, nearest)
    }

    /// Where point (i, j, k)'s value is kept, if its block is held.
    fn place(&self, i: usize, j: usize, k: usize) -> Option<usize> {
        let b = i / SIDE + self.dims[0] * (j / SIDE + self.dims[1] * (k / SIDE));
        let slot = *self.slot_of.get(b)?;
        (slot != NONE).then(|| slot as usize * BLOCK + local(i % SIDE, j % SIDE, k % SIDE))
    }

    /// Point (i, j, k)'s value: kept, or from its row when its block isn't
    /// held (its distance is then [`BAND`] steps). Outside the grid, it is
    /// outside.
    pub(super) fn value(&self, i: usize, j: usize, k: usize) -> f32 {
        let [nx, ny, nz] = self.grid.size;
        if i >= nx || j >= ny || k >= nz {
            return self.far;
        }
        if let Some(at) = self.place(i, j, k) {
            return self.values[at];
        }
        let mut inside = [false];
        Rows::inside(&self.grid, self.rows.row(&self.grid, j, k), i, &mut inside);
        if inside[0] { -self.far } else { self.far }
    }

    /// Whether point (i, j, k) is inside.
    pub(super) fn inside(&self, i: usize, j: usize, k: usize) -> bool {
        self.value(i, j, k).is_sign_negative()
    }

    /// The triangle nearest point (i, j, k), if it is near the surface.
    pub(super) fn nearest(&self, i: usize, j: usize, k: usize) -> u32 {
        self.place(i, j, k)
            .map_or(NO_TRIANGLE, |at| self.nearest[at])
    }

    /// Point (i, j, k) made inside, just under the new surface: its block
    /// is held from now on if it wasn't.
    fn fill_in(&mut self, i: usize, j: usize, k: usize) -> Result<(), RemeshProblem> {
        if self.place(i, j, k).is_none() {
            self.hold_block(i / SIDE, j / SIDE, k / SIDE)?;
        }
        let value = -FILLED * self.grid.step;
        if let Some(at) = self.place(i, j, k) {
            self.values[at] = value;
        }
        Ok(())
    }

    /// Holds block (bi, bj, bk), its values from the rows.
    fn hold_block(&mut self, bi: usize, bj: usize, bk: usize) -> Result<(), RemeshProblem> {
        let b = bi + self.dims[0] * (bj + self.dims[1] * bk);
        let slot = u32::try_from(self.blocks.len()).map_err(|_| RemeshProblem::TooBig)?;
        let start = self.block_start(b);
        let mut values = [self.far; BLOCK];
        let mut inside = [false; SIDE];
        let grid = &self.grid;
        for k in 0..SIDE {
            self.rows
                .rows_along(grid, start[1], start[2] + k, SIDE, |j, hits| {
                    Rows::inside(grid, hits, start[0], &mut inside);
                    for (i, side) in inside.iter().enumerate() {
                        if *side {
                            values[local(i, j, k)] = -self.far;
                        }
                    }
                });
        }
        self.slot_of[b] = slot;
        self.blocks.push(b);
        self.values.extend_from_slice(&values);
        self.nearest.extend_from_slice(&[NO_TRIANGLE; BLOCK]);
        Ok(())
    }

    /// Whether cell (i, j, k)'s corners are on both sides, and the corner
    /// to fill in if they touch only along a diagonal.
    fn cell_fill(&self, i: usize, j: usize, k: usize) -> Option<usize> {
        let corners: [bool; 8] =
            std::array::from_fn(|b| self.inside(i + (b & 1), j + ((b >> 1) & 1), k + (b >> 2)));
        fill_for(&corners)
    }

    /// Fills in outside points where inside points touch only along a
    /// square's diagonal or a cube's long diagonal: there the new surface
    /// would pinch to a line or a point, and edges would be shared by four
    /// quads. Every held block's cells are looked at on every core; then
    /// each fill looks again at the cells round the point filled. Only
    /// points are filled in, so it always ends.
    fn clean_corners(&mut self) -> Result<(), RemeshProblem> {
        let [nx, ny, nz] = self.grid.size;
        if nx < 2 || ny < 2 || nz < 2 {
            return Ok(());
        }
        let this = &*self;
        let found = crate::parallel::in_parallel_from(self.blocks.len(), BLOCKS_PER_THREAD, |r| {
            let mut cells = Vec::new();
            for s in r {
                let start = this.slot_start(s);
                let cube = this.cube(s);
                this.each_cell(start, |i, j, k| {
                    let at = [i - start[0], j - start[1], k - start[2]];
                    let corners: [bool; 8] = std::array::from_fn(|b| {
                        cube.inside(at[0] + (b & 1), at[1] + ((b >> 1) & 1), at[2] + (b >> 2))
                    });
                    if fill_for(&corners).is_some() {
                        cells.push([i, j, k]);
                    }
                });
            }
            cells
        });
        let mut waiting: Vec<[usize; 3]> = found.into_iter().flatten().collect();
        waiting.reverse();
        let mut fills = 0;
        while let Some([i, j, k]) = waiting.pop() {
            let Some(b) = self.cell_fill(i, j, k) else {
                continue;
            };
            let at = [i + (b & 1), j + ((b >> 1) & 1), k + (b >> 2)];
            self.fill_in(at[0], at[1], at[2])?;
            fills += 1;
            if fills > MOST_FILLS {
                break;
            }
            // The eight cells with that point as a corner.
            for c in 0..8 {
                let cell = [
                    at[0].checked_sub(c & 1),
                    at[1].checked_sub((c >> 1) & 1),
                    at[2].checked_sub(c >> 2),
                ];
                if let [Some(ci), Some(cj), Some(ck)] = cell
                    && ci + 1 < nx
                    && cj + 1 < ny
                    && ck + 1 < nz
                {
                    waiting.push([ci, cj, ck]);
                }
            }
        }
        Ok(())
    }

    /// Calls `f` with each cell (by its lowest corner) whose lowest corner
    /// is in the block starting at `start`.
    pub(super) fn each_cell(&self, start: [usize; 3], mut f: impl FnMut(usize, usize, usize)) {
        let [nx, ny, nz] = self.grid.size;
        for k in start[2]..(start[2] + SIDE).min(nz - 1) {
            for j in start[1]..(start[1] + SIDE).min(ny - 1) {
                for i in start[0]..(start[0] + SIDE).min(nx - 1) {
                    f(i, j, k);
                }
            }
        }
    }

    /// Slot `s`'s block's values and nearest triangles, with one more
    /// point along +X, +Y and +Z (from the blocks past it, or their rows
    /// when they aren't held): everything its cells need.
    pub(super) fn cube(&self, s: usize) -> Cube {
        let start = self.slot_start(s);
        let [nx, ny, nz] = self.grid.size;
        let mut cube = Cube {
            values: [self.far; CUBE * CUBE * CUBE],
            nearest: [NO_TRIANGLE; CUBE * CUBE * CUBE],
        };
        let mut inside = [false; CUBE];
        for k in 0..CUBE {
            for j in 0..CUBE {
                let (gj, gk) = (start[1] + j, start[2] + k);
                if gj >= ny || gk >= nz {
                    continue;
                }
                let mut row: Option<&[super::field::Hit]> = None;
                // This block's eight points, then the next block's first.
                for (from, count) in [(0, SIDE), (SIDE, 1)] {
                    let gi = start[0] + from;
                    if gi >= nx {
                        continue;
                    }
                    let count = count.min(nx - gi);
                    if let Some(at) = self.place(gi, gj, gk) {
                        for n in 0..count {
                            cube.values[cube_at(from + n, j, k)] = self.values[at + n];
                            cube.nearest[cube_at(from + n, j, k)] = self.nearest[at + n];
                        }
                    } else {
                        let hits = *row.get_or_insert_with(|| self.rows.row(&self.grid, gj, gk));
                        Rows::inside(&self.grid, hits, gi, &mut inside[..count]);
                        for (n, _) in inside[..count].iter().enumerate().filter(|(_, i)| **i) {
                            cube.values[cube_at(from + n, j, k)] = -self.far;
                        }
                    }
                }
            }
        }
        cube
    }

    /// Where slot `s`'s block starts.
    pub(super) fn slot_start(&self, s: usize) -> [usize; 3] {
        self.block_start(self.blocks[s])
    }

    /// The slot holding point (i, j, k)'s block, if it is held.
    pub(super) fn slot(&self, i: usize, j: usize, k: usize) -> Option<usize> {
        self.place(i, j, k).map(|at| at / BLOCK)
    }
}

/// Points along each side of a [`Cube`].
pub(super) const CUBE: usize = SIDE + 1;

/// A block's values with one more point along each axis.
pub(super) struct Cube {
    /// The values (see the module notes).
    pub(super) values: [f32; CUBE * CUBE * CUBE],
    /// The nearest triangles.
    pub(super) nearest: [u32; CUBE * CUBE * CUBE],
}

impl Cube {
    /// Whether point (i, j, k) of the cube is inside.
    pub(super) fn inside(&self, i: usize, j: usize, k: usize) -> bool {
        self.values[cube_at(i, j, k)].is_sign_negative()
    }
}

/// A point's place within a [`Cube`].
pub(super) fn cube_at(i: usize, j: usize, k: usize) -> usize {
    i + CUBE * (j + CUBE * k)
}

/// A point's place within its block.
pub(super) fn local(i: usize, j: usize, k: usize) -> usize {
    i + SIDE * (j + SIDE * k)
}
