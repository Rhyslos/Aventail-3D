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
//!
//! The grid is laid over the space packed into (the square, a region of
//! it, a row of UDIM tiles) in texture units, so the padding is always
//! the same texture pixels.

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
    /// A region of the square to pack into (its two corners, either way
    /// round; decision 0240), instead of the whole square: see
    /// [`region_of`].
    pub region: Option<(Vec2, Vec2)>,
    /// UDIM tiles across (1: the square alone): pieces fill tile 1001 (the
    /// square) and the ones to its right, never across a tile's side.
    pub tiles: u32,
}

impl Default for PackOptions {
    fn default() -> Self {
        Self {
            padding: 8.0 / 2048.0,
            rotate: true,
            stack: false,
            cells: 256,
            region: None,
            tiles: 1,
        }
    }
}

/// The most UDIM tiles across.
pub const MAX_TILES: u32 = 10;

/// What a piece asks of packing (decision 0240).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PieceAsks {
    /// Keeping its grain: never turned (only moved and sized).
    pub grain: bool,
    /// How much texture it gets against the rest, as a factor of length.
    pub scale: f32,
}

impl Default for PieceAsks {
    fn default() -> Self {
        Self {
            grain: false,
            scale: 1.0,
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

    /// Sets every cell of `mask` placed at (`x`, `y`) (cells that would
    /// fall off the grid are left out).
    fn stamp(&mut self, mask: &Grid, x: usize, y: usize) {
        for r in 0..mask.height.min(self.height.saturating_sub(y)) {
            let row = &mut self.rows[(y + r) * self.words..(y + r + 1) * self.words];
            or_shifted(row, &mask.rows[r * mask.words..(r + 1) * mask.words], x);
        }
    }

    /// The same grid, grown by `by` cells in every direction (every cell
    /// within `by` cells across and up of a set one is set, so the result
    /// is `2 * by` cells wider and taller).
    ///
    /// Done in two passes, across then up, each by doubling: a run of
    /// `2 * by + 1` cells takes about log2(by) word-wide steps per row
    /// instead of a square of cells per set cell, so a lot of padding
    /// at a fine grid stays quick.
    fn grown(&self, by: usize) -> Self {
        let mut out = Self::new(self.width + 2 * by, self.height + 2 * by);
        let want = 2 * by + 1;
        // Across: each row, smeared over 2 by + 1 cells to its right.
        let mut copy = vec![0u64; out.words];
        for y in 0..self.height {
            let row = &mut out.rows[y * out.words..(y + 1) * out.words];
            or_shifted(row, &self.rows[y * self.words..(y + 1) * self.words], 0);
            let mut span = 1;
            while span < want {
                let step = span.min(want - span);
                copy.copy_from_slice(row);
                or_shifted(row, &copy, step);
                span += step;
            }
        }
        // Up: each row of the result takes in the 2 by rows below it.
        // Going from the top down, the rows read are not yet changed.
        let mut span = 1;
        while span < want {
            let step = span.min(want - span);
            for y in (step..out.height).rev() {
                let (low, high) = out.rows.split_at_mut(y * out.words);
                let from = &low[(y - step) * out.words..(y - step + 1) * out.words];
                for (to, bits) in high[..out.words].iter_mut().zip(from) {
                    *to |= *bits;
                }
            }
            span += step;
        }
        out
    }
}

/// Ors the bits of `bits` into `row`, moved `shift` bits along (to higher
/// cells); bits moved past the end of `row` are dropped.
fn or_shifted(row: &mut [u64], bits: &[u64], shift: usize) {
    let (first, s) = (shift / 64, shift % 64);
    for (k, &word) in bits.iter().enumerate() {
        if word == 0 {
            continue;
        }
        let at = first + k;
        if at < row.len() {
            row[at] |= word << s;
        }
        if s > 0 && at + 1 < row.len() {
            row[at + 1] |= word >> (64 - s);
        }
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
            let p = (a + (b - a) * (i as f32 / steps as f32)).floor();
            // Parts before the grid's start are off it (not on its edge).
            if p.x >= 0.0 && p.y >= 0.0 {
                grid.set(p.x as usize, p.y as usize);
            }
        }
    }
    // The inside: cell middles inside the polygon (even-odd), within the
    // grid.
    let (mut lo, mut hi) = (points[0], points[0]);
    for p in &points {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    let rows = lo.y.floor().max(0.0) as usize..=(hi.y.ceil().max(0.0) as usize).min(grid.height);
    for y in rows {
        for x in lo.x.floor().max(0.0) as usize..=(hi.x.ceil().max(0.0) as usize).min(grid.width) {
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
    pack_with(pieces, &[], &[], options)
}

/// The region packed into, low corner then high: the one asked for with
/// its ends put in order (one typed From 0.8 To 0.2 is the same as From
/// 0.2 To 0.8), or the whole square when none is asked or it has no size.
pub fn region_of(options: &PackOptions) -> (Vec2, Vec2) {
    match options.region {
        Some((a, b)) if a.is_finite() && b.is_finite() => {
            let (lo, hi) = (
                a.min(b).clamp(Vec2::ZERO, Vec2::ONE),
                a.max(b).clamp(Vec2::ZERO, Vec2::ONE),
            );
            if (hi - lo).min_element() > 1e-6 {
                (lo, hi)
            } else {
                (Vec2::ZERO, Vec2::ONE)
            }
        }
        _ => (Vec2::ZERO, Vec2::ONE),
    }
}

/// The packing grid laid over the texture: cells of one size in texture
/// units, so padding counts the same texture pixels in a region as in the
/// whole square, and a region keeps its own shape (decision 0240).
struct Space {
    /// Where the grid's cell (0, 0) starts in the texture.
    origin: Vec2,
    /// Cells per texture width.
    per_texture: usize,
    /// Cells across (the region in each tile, and the tiles between).
    width: usize,
    /// Cells up.
    height: usize,
    /// Columns no piece may cover: those outside the region in a tile, and
    /// a tile's sides between tiles.
    blocked: Vec<usize>,
}

/// Lays the grid over the region (or the square), for each UDIM tile:
/// the region's long side gets about `cells` cells.
fn space(options: &PackOptions, cells: usize, tiles: u32) -> Space {
    let (lo, hi) = region_of(options);
    let long = (hi - lo).max_element();
    let per_texture = ((cells as f32 / long).round() as usize)
        .clamp(cells, (16 * MAX_CELLS / tiles as usize).max(cells));
    // (Fewer extra cells with more tiles, so the grid stays a sensible size.)
    let n = per_texture as f32;
    // The region's cells within a tile: whole cells inside it only.
    let first = ((lo * n) - 1e-3).ceil().max(Vec2::ZERO);
    let last = ((hi * n) + 1e-3).floor();
    let (c0, r0) = (first.x as usize, first.y as usize);
    let (c1, r1) = ((last.x as usize).max(c0 + 1), (last.y as usize).max(r0 + 1));
    let tiles = tiles as usize;
    let width = (tiles - 1) * per_texture + (c1 - c0);
    let mut blocked = Vec::new();
    if tiles > 1 {
        for x in 0..width {
            let (tile, local) = ((c0 + x) / per_texture, (c0 + x) % per_texture);
            let outside = local < c0 || local >= c1;
            let side = (tile > 0 && local == c0) || (tile + 1 < tiles && local + 1 == c1);
            if outside || side {
                blocked.push(x);
            }
        }
    }
    Space {
        origin: Vec2::new(c0 as f32, r0 as f32) / n,
        per_texture,
        width,
        height: r1 - r0,
        blocked,
    }
}

/// [`pack`], with what each piece asks (`asks`, by piece; missing ones
/// ask nothing), and pieces that stay where they are (`filled`: locked
/// pieces, decision 0112), whose space is kept free of the rest.
pub fn pack_with(
    pieces: &[Piece],
    asks: &[PieceAsks],
    filled: &[Piece],
    options: &PackOptions,
) -> Option<Vec<Affine2>> {
    let cells = options.cells.clamp(16, MAX_CELLS);
    let tiles = options.tiles.clamp(1, MAX_TILES);
    let room = space(options, cells, tiles);
    let (width, height) = (room.width, room.height);
    let cell = 1.0 / room.per_texture as f32;
    let padding = options.padding.max(0.0);
    // The padding in texture widths: the same pixels whatever the region.
    let pad = (padding / cell / 2.0).ceil() as usize;
    let to_texture = Affine2::from_translation(room.origin);
    let from_texture = to_texture.inverse();
    let ask = |i: usize| asks.get(i).copied().unwrap_or_default();
    // Each piece turned to its smallest box (when allowed), sized as it
    // asks, at the origin.
    let mut base: Vec<(Shape, Affine2)> = Vec::new();
    for (i, piece) in pieces.iter().enumerate() {
        let points: Vec<Vec2> = piece.polygons.iter().flatten().copied().collect();
        let a = ask(i);
        let turn = if options.rotate && !a.grain {
            Affine2::from_angle(crate::unwrap::smallest_box_turn(&points))
        } else {
            Affine2::IDENTITY
        };
        let size = if a.scale.is_finite() && a.scale > 0.0 {
            a.scale
        } else {
            1.0
        };
        let sized = Affine2::from_scale(Vec2::splat(size)) * turn;
        let turned = moved(piece, sized);
        let (shape, shift) = quarter(&turned, 0);
        base.push((shape, shift * sized));
    }
    // What's taken before anything is placed: the locked pieces, and the
    // sides between UDIM tiles.
    let mut taken = Grid::new(width, height);
    for piece in filled {
        for polygon in &piece.polygons {
            let inside: Vec<Vec2> = polygon
                .iter()
                .map(|v| from_texture.transform_point2(*v))
                .collect();
            draw(&mut taken, &inside, Vec2::ZERO, cell);
        }
    }
    let taken = taken.grown(pad);
    let mut start = Grid::new(width, height);
    for y in 0..height {
        for x in 0..width {
            if taken.get(x + pad, y + pad) {
                start.set(x, y);
            }
        }
        for &x in &room.blocked {
            start.set(x, y);
        }
    }
    let total: f32 = base.iter().map(|(s, _)| area(&s.polygons)).sum();
    if total <= 1e-20 || base.is_empty() {
        return None;
    }
    // Pieces that are never turned: turning is off, or they keep their
    // grain.
    let fixed = |i: usize| !options.rotate || ask(i).grain;
    // Copies of an earlier piece, when stacking.
    // (The copy's quarter turns to match: 0, or 2 when it lies the other
    // way round, or 1 or 3; never any for a piece that isn't turned, since
    // lettering would end upside down. Such a piece stacks only on another
    // that isn't turned, so it isn't given its twin's turn either.)
    let mut copy_of: Vec<Option<(usize, usize)>> = vec![None; base.len()];
    if options.stack {
        for i in 0..base.len() {
            if copy_of[i].is_some() {
                continue;
            }
            for j in i + 1..base.len() {
                if copy_of[j].is_some() || fixed(i) != fixed(j) {
                    continue;
                }
                // Free pieces may be turned by quarters, as packing would.
                let turns: &[usize] = if fixed(j) { &[0] } else { &[0, 2, 1, 3] };
                if let Some(&k) = turns
                    .iter()
                    .find(|&&k| same_shape(&base[i].0, &quarter(&base[j].0.polygons, k).0))
                {
                    copy_of[j] = Some((i, k));
                }
            }
        }
    }
    let packed: Vec<usize> = (0..base.len()).filter(|i| copy_of[*i].is_none()).collect();
    // Biggest first. Areas are rounded (to a ten-thousandth of the
    // biggest) so pieces of the same size go in the order they were given,
    // not by rounding noise: packing a packed layout again gives the same
    // layout.
    let areas: Vec<f32> = base.iter().map(|(s, _)| area(&s.polygons)).collect();
    let biggest = areas.iter().copied().fold(0.0f32, f32::max).max(1e-30);
    let rounded = |i: usize| (areas[i] / biggest * 1e4).round() as i64;
    let mut order = packed.clone();
    order.sort_by(|a, b| rounded(*b).cmp(&rounded(*a)).then(a.cmp(b)));
    let packed_area: f32 = packed.iter().map(|i| area(&base[*i].0.polygons)).sum();
    // Try a scale: where each packed piece goes (cell corner and quarter
    // turns), if all fit.
    let attempt = |scale: f32| -> Option<Vec<(usize, usize, usize)>> {
        // Each mask is grown by half the padding, so pieces end up a whole
        // padding apart, and half of one from the square's edges.
        let mut grid = start.clone();
        let mut spots = vec![(0, 0, 0); base.len()];
        for &i in &order {
            let turns: &[usize] = if options.rotate && !ask(i).grain {
                &[0, 1]
            } else {
                &[0]
            };
            let mut best: Option<(usize, usize, usize, Grid)> = None;
            for &k in turns {
                let (shape, _) = quarter(&base[i].0.polygons, k);
                let m = mask(&shape, scale, cell, pad);
                if m.width > width || m.height > height {
                    continue;
                }
                'search: for y in 0..=height - m.height {
                    if best.as_ref().is_some_and(|b| y > b.1) {
                        break;
                    }
                    for x in 0..=width - m.width {
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
    let room_area = (width * height) as f32 * cell * cell;
    let mut high = (room_area / packed_area).sqrt();
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
        out[i] = to_texture
            * Affine2::from_translation(corner)
            * Affine2::from_scale(Vec2::splat(scale))
            * turned
            * base[i].1;
    }
    for i in 0..base.len() {
        if let Some((j, extra)) = copy_of[i] {
            // The same place as the piece it copies (both at the origin,
            // turned to their smallest boxes, this one by the quarter turns
            // that make it lie the same way).
            let (x, y, k) = spots[j];
            let (matched, match_turn) = quarter(&base[i].0.polygons, extra);
            let (_, turned) = quarter(&matched.polygons, k);
            let corner = Vec2::new((x + pad) as f32, (y + pad) as f32) * cell;
            out[i] = to_texture
                * Affine2::from_translation(corner)
                * Affine2::from_scale(Vec2::splat(scale))
                * turned
                * match_turn
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
