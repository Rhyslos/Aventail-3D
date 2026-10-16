//! Packing: fitting pieces into the texture square as large as they can
//! go, with real-pixel padding between them ([decision 0101](../../../docs/decisions/0101-packing.md)).
//!
//! Pieces are packed by their **shapes**, not their boxes: each is drawn
//! into a grid of cells (a cell is a few texture pixels), grown by the
//! padding, and placed at the lowest, then leftmost, free spot where its
//! cells don't touch any other's. So a small piece can sit in the dent or
//! hole of a big one ("nesting"). Each piece is first turned so its box is
//! as small as can be, and may be placed a quarter turn round as well.
//! The largest scale at which every piece fits is found by halving the
//! gap between one that fits and one that doesn't, and every piece gets
//! the same scale, so their texel densities stay matched.
//!
//! Identical pieces can be **stacked**: packed once, the copies placed
//! exactly on top, sharing the same texture pixels.

use glam::{Affine2, Vec2};
use mesh::edit::Outcome;
use mesh::uvs::Uvs;
use mesh::{FaceId, Mesh};
use std::collections::BTreeSet;

/// A piece to pack: the UV outline of each of its polygons (triangles or
/// more), as it lies now.
#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    /// Each polygon's corners, in the texture.
    pub polygons: Vec<Vec<Vec2>>,
}

/// How to pack.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PackOptions {
    /// The gap between pieces, in texture widths (half of it to the
    /// square's edges).
    pub padding: f32,
    /// Whether pieces may be turned (to their smallest box, and a quarter
    /// turn at a time).
    pub rotate: bool,
    /// Whether identical pieces are stacked on one another.
    pub stack: bool,
    /// How many cells across the packing grid is (more is tighter and
    /// slower).
    pub cells: usize,
}

impl Default for PackOptions {
    fn default() -> Self {
        Self {
            padding: 8.0 / 2048.0,
            rotate: true,
            stack: false,
            cells: 256,
        }
    }
}

/// The most cells across the packing grid.
pub const MAX_CELLS: usize = 1024;

/// A grid of cells as rows of bits.
#[derive(Clone)]
struct Grid {
    width: usize,
    height: usize,
    words: usize,
    rows: Vec<u64>,
}

impl Grid {
    fn new(width: usize, height: usize) -> Self {
        let words = width.div_ceil(64).max(1);
        Self {
            width,
            height,
            words,
            rows: vec![0; words * height],
        }
    }

    fn set(&mut self, x: usize, y: usize) {
        if x < self.width && y < self.height {
            self.rows[y * self.words + x / 64] |= 1 << (x % 64);
        }
    }

    fn get(&self, x: usize, y: usize) -> bool {
        x < self.width
            && y < self.height
            && self.rows[y * self.words + x / 64] & (1 << (x % 64)) != 0
    }

    /// Whether `mask` placed with its corner at (`x`, `y`) touches any set
    /// cell (or runs off the grid).
    fn clashes(&self, mask: &Grid, x: usize, y: usize) -> bool {
        if x + mask.width > self.width || y + mask.height > self.height {
            return true;
        }
        let shift = x % 64;
        let first = x / 64;
        for r in 0..mask.height {
            let row = &self.rows[(y + r) * self.words..(y + r + 1) * self.words];
            let bits = &mask.rows[r * mask.words..(r + 1) * mask.words];
            for (k, &word) in bits.iter().enumerate() {
                if word == 0 {
                    continue;
                }
                let at = first + k;
                let low = word << shift;
                if at < self.words && row[at] & low != 0 {
                    return true;
                }
                if shift > 0 {
                    let high = word >> (64 - shift);
                    if high != 0 && at + 1 < self.words && row[at + 1] & high != 0 {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Sets every cell of `mask` placed at (`x`, `y`).
    fn stamp(&mut self, mask: &Grid, x: usize, y: usize) {
        for r in 0..mask.height {
            for c in 0..mask.width {
                if mask.get(c, r) {
                    self.set(x + c, y + r);
                }
            }
        }
    }

    /// The same grid, grown by `by` cells in every direction.
    fn grown(&self, by: usize) -> Self {
        let mut out = Self::new(self.width + 2 * by, self.height + 2 * by);
        for y in 0..self.height {
            for x in 0..self.width {
                if self.get(x, y) {
                    for dy in 0..=2 * by {
                        for dx in 0..=2 * by {
                            out.set(x + dx, y + dy);
                        }
                    }
                }
            }
        }
        out
    }
}

/// The corners of a piece, moved by `t`.
fn moved(piece: &Piece, t: Affine2) -> Vec<Vec<Vec2>> {
    piece
        .polygons
        .iter()
        .map(|p| p.iter().map(|v| t.transform_point2(*v)).collect())
        .collect()
}

fn bounds(polygons: &[Vec<Vec2>]) -> Option<(Vec2, Vec2)> {
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);
    for v in polygons.iter().flatten() {
        min = min.min(*v);
        max = max.max(*v);
    }
    min.is_finite().then_some((min, max))
}

fn area(polygons: &[Vec<Vec2>]) -> f32 {
    polygons
        .iter()
        .map(|p| {
            let mut sum = 0.0;
            for k in 0..p.len() {
                sum += p[k].perp_dot(p[(k + 1) % p.len()]);
            }
            (sum / 2.0).abs()
        })
        .sum()
}

/// Marks the cells (of size `cell`, from `origin`) a polygon covers:
/// its inside, and every cell its sides pass through, so nothing it
/// covers is missed.
fn draw(grid: &mut Grid, polygon: &[Vec2], origin: Vec2, cell: f32) {
    let to = |v: Vec2| (v - origin) / cell;
    let points: Vec<Vec2> = polygon.iter().map(|v| to(*v)).collect();
    if points.is_empty() {
        return;
    }
    // The sides, walked in small steps.
    for k in 0..points.len() {
        let (a, b) = (points[k], points[(k + 1) % points.len()]);
        let steps = ((b - a).abs().max_element() * 2.0).ceil().max(1.0) as usize;
        for i in 0..=steps {
            let p = a + (b - a) * (i as f32 / steps as f32);
            grid.set(p.x.max(0.0) as usize, p.y.max(0.0) as usize);
        }
    }
    // The inside: cell middles inside the polygon (even-odd).
    let (mut lo, mut hi) = (points[0], points[0]);
    for p in &points {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    for y in lo.y.floor().max(0.0) as usize..=hi.y.ceil().max(0.0) as usize {
        for x in lo.x.floor().max(0.0) as usize..=hi.x.ceil().max(0.0) as usize {
            let c = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let mut inside = false;
            for k in 0..points.len() {
                let (a, b) = (points[k], points[(k + 1) % points.len()]);
                if (a.y > c.y) != (b.y > c.y) && c.x < a.x + (c.y - a.y) / (b.y - a.y) * (b.x - a.x)
                {
                    inside = !inside;
                }
            }
            if inside {
                grid.set(x, y);
            }
        }
    }
}

/// A piece ready to place: its outline with its box's low corner at 0.
struct Shape {
    polygons: Vec<Vec<Vec2>>,
    size: Vec2,
}

/// `polygons` turned a quarter `k` times about 0, then moved so their
/// box's low corner is at 0, and the move doing it.
fn quarter(polygons: &[Vec<Vec2>], k: usize) -> (Shape, Affine2) {
    let turn = Affine2::from_angle(std::f32::consts::FRAC_PI_2 * k as f32);
    let turned = polygons
        .iter()
        .map(|p| {
            p.iter()
                .map(|v| turn.transform_point2(*v))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let (min, max) = bounds(&turned).unwrap_or((Vec2::ZERO, Vec2::ZERO));
    let shift = Affine2::from_translation(-min);
    let placed = turned
        .iter()
        .map(|p| p.iter().map(|v| *v - min).collect())
        .collect();
    (
        Shape {
            polygons: placed,
            size: max - min,
        },
        shift * turn,
    )
}

/// The cells `shape` (scaled by `scale`) covers, grown by `pad` cells.
fn mask(shape: &Shape, scale: f32, cell: f32, pad: usize) -> Grid {
    let size = shape.size * scale / cell;
    let mut grid = Grid::new(size.x.ceil() as usize + 1, size.y.ceil() as usize + 1);
    for p in &shape.polygons {
        let scaled: Vec<Vec2> = p.iter().map(|v| *v * scale).collect();
        draw(&mut grid, &scaled, Vec2::ZERO, cell);
    }
    grid.grown(pad)
}

/// Whether two pieces (already turned to their smallest boxes and moved
/// to the origin) are the same shape.
fn same_shape(a: &Shape, b: &Shape) -> bool {
    let tolerance = 1e-3 * a.size.max_element().max(1e-6);
    if a.polygons.len() != b.polygons.len() || (a.size - b.size).abs().max_element() > tolerance {
        return false;
    }
    let corners = |s: &Shape| {
        let mut list: Vec<(i64, i64)> = s
            .polygons
            .iter()
            .flatten()
            .map(|v| {
                (
                    (v.x / tolerance).round() as i64,
                    (v.y / tolerance).round() as i64,
                )
            })
            .collect();
        list.sort_unstable();
        list
    };
    let (x, y) = (corners(a), corners(b));
    x.len() == y.len()
        && x.iter()
            .zip(&y)
            .all(|(p, q)| (p.0 - q.0).abs() <= 1 && (p.1 - q.1).abs() <= 1)
}

/// Where each piece goes: one move per piece (turning, scaling and
/// moving it from where it lies now), or `None` if there's nothing to
/// pack (no piece has any area).
pub fn pack(pieces: &[Piece], options: &PackOptions) -> Option<Vec<Affine2>> {
    let cells = options.cells.clamp(16, MAX_CELLS);
    let cell = 1.0 / cells as f32;
    let padding = options.padding.max(0.0);
    let pad = (padding / cell / 2.0).ceil() as usize;
    // Each piece turned to its smallest box (when allowed), at the origin.
    let mut base: Vec<(Shape, Affine2)> = Vec::new();
    for piece in pieces {
        let points: Vec<Vec2> = piece.polygons.iter().flatten().copied().collect();
        let turn = if options.rotate {
            Affine2::from_angle(crate::unwrap::smallest_box_turn(&points))
        } else {
            Affine2::IDENTITY
        };
        let turned = moved(piece, turn);
        let (shape, shift) = quarter(&turned, 0);
        base.push((shape, shift * turn));
    }
    let total: f32 = base.iter().map(|(s, _)| area(&s.polygons)).sum();
    if total <= 1e-20 || base.is_empty() {
        return None;
    }
    // Copies of an earlier piece, when stacking.
    // (The copy's half turns: 0, or 2 when it lies the other way round.)
    let mut copy_of: Vec<Option<(usize, usize)>> = vec![None; base.len()];
    if options.stack {
        for i in 0..base.len() {
            if copy_of[i].is_some() {
                continue;
            }
            for j in i + 1..base.len() {
                if copy_of[j].is_some() {
                    continue;
                }
                if same_shape(&base[i].0, &base[j].0) {
                    copy_of[j] = Some((i, 0));
                } else if same_shape(&base[i].0, &quarter(&base[j].0.polygons, 2).0) {
                    copy_of[j] = Some((i, 2));
                }
            }
        }
    }
    let packed: Vec<usize> = (0..base.len()).filter(|i| copy_of[*i].is_none()).collect();
    let mut order = packed.clone();
    order.sort_by(|a, b| {
        area(&base[*b].0.polygons)
            .total_cmp(&area(&base[*a].0.polygons))
            .then(a.cmp(b))
    });
    let packed_area: f32 = packed.iter().map(|i| area(&base[*i].0.polygons)).sum();
    // Try a scale: where each packed piece goes (cell corner and quarter
    // turns), if all fit.
    let attempt = |scale: f32| -> Option<Vec<(usize, usize, usize)>> {
        // Each mask is grown by half the padding, so pieces end up a whole
        // padding apart, and half of one from the square's edges.
        let mut grid = Grid::new(cells, cells);
        let mut spots = vec![(0, 0, 0); base.len()];
        for &i in &order {
            let turns: &[usize] = if options.rotate { &[0, 1] } else { &[0] };
            let mut best: Option<(usize, usize, usize, Grid)> = None;
            for &k in turns {
                let (shape, _) = quarter(&base[i].0.polygons, k);
                let m = mask(&shape, scale, cell, pad);
                if m.width > cells || m.height > cells {
                    continue;
                }
                'search: for y in 0..=cells - m.height {
                    if best.as_ref().is_some_and(|b| y > b.1) {
                        break;
                    }
                    for x in 0..=cells - m.width {
                        if !grid.clashes(&m, x, y) {
                            let better = best.as_ref().is_none_or(|b| (y, x) < (b.1, b.0));
                            if better {
                                best = Some((x, y, k, m.clone()));
                            }
                            break 'search;
                        }
                    }
                }
            }
            let (x, y, k, m) = best?;
            grid.stamp(&m, x, y);
            spots[i] = (x, y, k);
        }
        Some(spots)
    };
    // The largest scale that fits, between one that fits and one that
    // doesn't.
    let mut high = (1.0 / packed_area).sqrt();
    let mut low = high;
    let mut fits = None;
    for _ in 0..40 {
        if let Some(s) = attempt(low) {
            fits = Some((low, s));
            break;
        }
        high = low;
        low *= 0.7;
    }
    let (mut scale, mut spots) = fits?;
    if (high - scale).abs() > 1e-9 {
        for _ in 0..10 {
            let mid = (scale + high) / 2.0;
            match attempt(mid) {
                Some(s) => {
                    scale = mid;
                    spots = s;
                }
                None => high = mid,
            }
        }
    }
    let mut out = vec![Affine2::IDENTITY; base.len()];
    for &i in &packed {
        let (x, y, k) = spots[i];
        let (_, turned) = quarter(&base[i].0.polygons, k);
        // The mask's cell (0, 0) is `pad` cells before the shape's box.
        let corner = Vec2::new((x + pad) as f32, (y + pad) as f32) * cell;
        out[i] = Affine2::from_translation(corner)
            * Affine2::from_scale(Vec2::splat(scale))
            * turned
            * base[i].1;
    }
    for i in 0..base.len() {
        if let Some((j, half)) = copy_of[i] {
            // The same place as the piece it copies (both at the origin,
            // turned to their smallest boxes, this one a half turn more if
            // it lay the other way round).
            let (x, y, k) = spots[j];
            let (flipped, half_turn) = quarter(&base[i].0.polygons, half);
            let (_, turned) = quarter(&flipped.polygons, k);
            let corner = Vec2::new((x + pad) as f32, (y + pad) as f32) * cell;
            out[i] = Affine2::from_translation(corner)
                * Affine2::from_scale(Vec2::splat(scale))
                * turned
                * half_turn
                * base[i].1;
        }
    }
    Some(out)
}

/// The UV islands of `mesh` holding any of `faces` (all of them, with
/// none given).
pub fn islands_of(mesh: &Mesh, faces: &[FaceId]) -> Vec<Vec<FaceId>> {
    let wanted: BTreeSet<FaceId> = faces.iter().copied().collect();
    mesh::uvs::islands(mesh)
        .into_iter()
        .filter(|island| wanted.is_empty() || island.iter().any(|f| wanted.contains(f)))
        .collect()
}

/// The piece `faces` of `mesh` make, by their current UVs.
pub fn piece_of(mesh: &Mesh, faces: &[FaceId]) -> Piece {
    let polygons = match Uvs::of(mesh) {
        Some(uvs) => faces
            .iter()
            .filter_map(|f| {
                let n = mesh.face(*f)?.len();
                Some((0..n).filter_map(|k| uvs.corner(mesh, *f, k)).collect())
            })
            .collect(),
        None => Vec::new(),
    };
    Piece { polygons }
}

/// Moves each island's corners (exactly those of its polygons, nothing
/// else) by its move, all at once.
pub fn place(mesh: &mut Mesh, islands: &[(Vec<FaceId>, Affine2)]) -> Outcome {
    let asked: usize = islands.iter().map(|(f, _)| f.len()).sum();
    let found: usize = islands
        .iter()
        .flat_map(|(f, _)| f)
        .filter(|f| mesh.face(**f).is_some())
        .count();
    let unchanged = Outcome {
        matched: 0,
        missing: asked - found,
        skipped: found,
    };
    let Some(uvs) = Uvs::of(mesh) else {
        return unchanged;
    };
    if islands.iter().any(|(_, t)| !t.is_finite()) {
        return unchanged;
    }
    let count = mesh.face_layout().1.len();
    let before = uvs.into_values(count);
    let mut values = before.clone();
    let (starts, _) = mesh.face_layout();
    for (faces, t) in islands {
        for f in faces {
            let Some(n) = mesh.face(*f).map(<[mesh::VertexId]>::len) else {
                continue;
            };
            let start = starts[f.get() as usize] as usize;
            for c in start..start + n {
                values[c] = t.transform_point2(before[c]);
            }
        }
    }
    if !mesh::uvs::set_uvs(mesh, values) {
        return unchanged;
    }
    Outcome {
        matched: found,
        missing: asked - found,
        skipped: 0,
    }
}

#[cfg(test)]
mod tests;
