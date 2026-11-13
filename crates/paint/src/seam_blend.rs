//! Seam blend layers ([decision 0293](../../../docs/decisions/0293-blend-seams.md)):
//! a layer that makes the texture on the two sides of chosen UV seams
//! meet smoothly, made from the layers under it.
//!
//! Each side's texels near a seam are blended towards the matching texels
//! on the other side: the same distance from the seam and the same way
//! along it, found through the two sides' edges on the texture, so pieces
//! turned or mirrored against each other still match. Right on the seam
//! both sides become the half-and-half mix of the two; the blend fades
//! out over the layer's width, so nothing changes farther from the seam.
//! The layer holds only the blended texels (the other side's colour, as
//! see-through as the blend is strong), so laid over the layers under it
//! with the Normal blend it gives the blended texture.
//!
//! Where two materials meet (decision 0296) each material's layer blends
//! its own side towards the other material's painting, so the two meet
//! in the middle as one material's seams do.

use crate::image::{Image, TILE, Tile};
use crate::seams::{Seam, SeamId, SeamSide};
use crate::surface::PaintSurface;
use glam::Vec2;
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

/// The narrowest blend, in texels each side of the seam.
pub const MIN_WIDTH: f32 = 1.0;

/// The widest blend, in texels each side of the seam.
pub const MAX_WIDTH: f32 = 64.0;

/// A new Seam blend layer's width, in texels each side of the seam.
pub const WIDTH: f32 = 8.0;

/// The most seams one layer blends.
pub const MAX_SEAMS: usize = 1 << 18;

/// How far outside a triangle (in texels) its texels still count as its
/// piece's, as when painting, so the texels along a seam are blended.
const EDGE_REACH: f32 = 0.75;

/// A Seam blend layer's settings.
#[derive(Debug, Clone, PartialEq)]
pub struct SeamBlend {
    /// The seams it blends, sorted, each once. A seam no longer on the
    /// model is kept (it may come back with an undo) and left out.
    pub seams: Vec<SeamId>,
    /// How far from each seam the blend reaches, in texels on each side:
    /// from [`MIN_WIDTH`] to [`MAX_WIDTH`].
    pub width: f32,
    /// What its pixels were last made from (the seams, the width, the
    /// model's UVs and the layers under it, hashed by whoever made them):
    /// 0 until made. When what it would be made from now hashes
    /// differently, it is made again.
    pub made: u64,
}

impl Default for SeamBlend {
    fn default() -> Self {
        Self {
            seams: Vec::new(),
            width: WIDTH,
            made: 0,
        }
    }
}

impl SeamBlend {
    /// `width` brought within [`MIN_WIDTH`] and [`MAX_WIDTH`] ([`WIDTH`]
    /// when it isn't a real number).
    pub fn usable_width(width: f32) -> f32 {
        if width.is_finite() {
            width.clamp(MIN_WIDTH, MAX_WIDTH)
        } else {
            WIDTH
        }
    }

    /// Marks `seams`: in place of those marked, or added to them (`more`).
    /// Kept sorted, each once, at most [`MAX_SEAMS`].
    pub fn mark(&mut self, seams: impl IntoIterator<Item = SeamId>, more: bool) {
        let mut all: BTreeSet<SeamId> = if more {
            self.seams.iter().copied().collect()
        } else {
            BTreeSet::new()
        };
        all.extend(seams);
        self.seams = all.into_iter().take(MAX_SEAMS).collect();
    }

    /// Unmarks `seams`.
    pub fn unmark(&mut self, seams: &[SeamId]) {
        self.seams.retain(|s| !seams.contains(s));
    }

    /// Whether all of `seams` are marked (and there are some).
    pub fn marks_all(&self, seams: &[SeamId]) -> bool {
        !seams.is_empty() && seams.iter().all(|s| self.seams.binary_search(s).is_ok())
    }

    /// Whether its width is in range and its seams sorted, each once,
    /// with no more than [`MAX_SEAMS`].
    pub fn is_valid(&self) -> bool {
        (MIN_WIDTH..=MAX_WIDTH).contains(&self.width)
            && self.seams.len() <= MAX_SEAMS
            && self.seams.windows(2).all(|w| w[0] < w[1])
    }
}

/// A shape whose seams can be blended: an object's surface, the slots of
/// the material, its seams ([`crate::seams::seams_of`]) and where it
/// meets other materials ([`Across`]).
#[derive(Debug, Clone, Copy)]
pub struct BlendShape<'a> {
    /// The object's id.
    pub id: u64,
    /// Its surface.
    pub surface: &'a PaintSurface,
    /// The material's slots on it.
    pub slots: &'a [u32],
    /// Its seams on the material, sorted.
    pub seams: &'a [Seam],
    /// Where it meets other materials, one for each other material.
    pub across: &'a [Across<'a>],
}

/// Where a shape's material meets another on it
/// ([decision 0296](../../../docs/decisions/0296-blending-seams-between-materials.md)):
/// the other material's slots on the shape and the seams between them
/// ([`crate::seams::seams_between`], this material's side first). The
/// other side is read from the other material's painting, given with the
/// blend as `others[other]`.
#[derive(Debug, Clone, Copy)]
pub struct Across<'a> {
    /// Which of the other paintings the other side is on.
    pub other: usize,
    /// The other material's slots on the shape.
    pub slots: &'a [u32],
    /// The seams between the two, sorted, this material's side first.
    pub seams: &'a [Seam],
}

/// A chosen seam found on the shapes.
#[derive(Debug, Clone, Copy)]
pub struct Found<'a> {
    /// Its shape's number.
    pub shape: usize,
    /// Which of the shape's [`Across`] it is on, when it is where two
    /// materials meet.
    pub across: Option<usize>,
    /// The seam.
    pub seam: &'a Seam,
}

/// The seams of `chosen` found on `shapes` (a seam of the material, or
/// one where it meets another); and how many weren't found (no longer on
/// the model).
pub fn resolve<'a>(shapes: &[BlendShape<'a>], chosen: &[SeamId]) -> (Vec<Found<'a>>, usize) {
    let mut found = Vec::new();
    let mut missing = 0;
    for id in chosen {
        let hit = shapes.iter().enumerate().find_map(|(n, shape)| {
            if shape.id != id.object {
                return None;
            }
            if let Some(i) = crate::seams::find(shape.seams, id.points) {
                return Some(Found {
                    shape: n,
                    across: None,
                    seam: &shape.seams[i],
                });
            }
            shape.across.iter().enumerate().find_map(|(k, across)| {
                crate::seams::find(across.seams, id.points).map(|i| Found {
                    shape: n,
                    across: Some(k),
                    seam: &across.seams[i],
                })
            })
        });
        match hit {
            Some(hit) => found.push(hit),
            None => missing += 1,
        }
    }
    (found, missing)
}

/// The tiles of a `size`-pixel texture the blend of `chosen` reads and
/// writes, `width` texels each side: what the layers under it are
/// worked out on, and what decides whether it is made again. Where two
/// materials meet, only this material's side is on its texture (the
/// other's tiles are [`other_tiles`]).
pub fn tiles_used(
    shapes: &[BlendShape<'_>],
    chosen: &[SeamId],
    width: f32,
    size: u32,
) -> Vec<usize> {
    let (found, _) = resolve(shapes, chosen);
    let s = size as f32;
    let mut tiles = BTreeSet::new();
    for f in found {
        let seam = f.seam;
        if f.across.is_some() {
            if let Some(edge) = Edge::of(&seam.sides[0], s) {
                add_tiles(&mut tiles, &edge, width + 2.0, size);
            }
            continue;
        }
        let [a, b] = seam.sides.map(|side| Edge::of(&side, s));
        let (Some(a), Some(b)) = (a, b) else {
            continue;
        };
        for (edge, other) in [(&a, &b), (&b, &a)] {
            let reach = width * (other.len / edge.len).max(1.0) + 2.0;
            add_tiles(&mut tiles, edge, reach, size);
        }
    }
    tiles.into_iter().collect()
}

/// The tiles of each other material's painting (`sizes[k]` pixels for
/// `others[k]`) the blend of `chosen` reads, for this material's
/// `size`-pixel texture blended `width` texels: what the other
/// paintings are worked out on, and part of what decides whether it is
/// made again (decision 0296).
pub fn other_tiles(
    shapes: &[BlendShape<'_>],
    chosen: &[SeamId],
    width: f32,
    size: u32,
    sizes: &[u32],
) -> Vec<Vec<usize>> {
    let (found, _) = resolve(shapes, chosen);
    let mut out = vec![BTreeSet::new(); sizes.len()];
    for f in found {
        let Some(across) = f.across.map(|k| shapes[f.shape].across[k]) else {
            continue;
        };
        let Some(&theirs) = sizes.get(across.other) else {
            continue;
        };
        let (Some(a), Some(b)) = (
            Edge::of(&f.seam.sides[0], size as f32),
            Edge::of(&f.seam.sides[1], theirs as f32),
        ) else {
            continue;
        };
        let reach = width * (b.len / a.len) + 2.0;
        add_tiles(&mut out[across.other], &b, reach, theirs);
    }
    out.into_iter().map(|t| t.into_iter().collect()).collect()
}

/// Adds the tiles of a `size`-pixel texture within `reach` texels of
/// `edge` to `tiles`.
fn add_tiles(tiles: &mut BTreeSet<usize>, edge: &Edge, reach: f32, size: u32) {
    let across = crate::image::tiles_across(size);
    let Some((min, max)) = edge.area(reach, size) else {
        return;
    };
    for ty in min[1] / TILE..=max[1] / TILE {
        for tx in min[0] / TILE..=max[0] / TILE {
            tiles.insert((ty * across + tx) as usize);
        }
    }
}

/// A seam's side on the texture, in texels (x right, y down, as the
/// image's rows run): where its edge starts, which way it runs and how
/// long it is, and which way its piece lies.
#[derive(Debug, Clone, Copy)]
struct Edge {
    start: Vec2,
    along: Vec2,
    inward: Vec2,
    len: f32,
}

impl Edge {
    fn of(side: &SeamSide, size: f32) -> Option<Self> {
        let texel = |uv: Vec2| Vec2::new(uv.x * size, (1.0 - uv.y) * size);
        let (start, end) = (texel(side.uvs[0]), texel(side.uvs[1]));
        let len = start.distance(end);
        if !len.is_finite() || len < 1e-3 {
            return None;
        }
        let along = (end - start) / len;
        let mut inward = along.perp();
        if (texel(side.inward) - start).dot(inward) < 0.0 {
            inward = -inward;
        }
        Some(Self {
            start,
            along,
            inward,
            len,
        })
    }

    /// The texels within `reach` of the edge, as the lowest and highest
    /// (inclusive), on a `size`-pixel texture; `None` off the texture.
    fn area(&self, reach: f32, size: u32) -> Option<([u32; 2], [u32; 2])> {
        let end = self.start + self.along * self.len;
        let lo = self.start.min(end) - Vec2::splat(reach);
        let hi = self.start.max(end) + Vec2::splat(reach);
        let s = size as f32;
        if hi.x < 0.0 || hi.y < 0.0 || lo.x >= s || lo.y >= s || !lo.is_finite() || !hi.is_finite()
        {
            return None;
        }
        let low = |v: f32| v.floor().clamp(0.0, s - 1.0) as u32;
        Some(([low(lo.x), low(lo.y)], [low(hi.x), low(hi.y)]))
    }
}

/// Which texels of an area belong to one piece.
struct Cover {
    min: [u32; 2],
    width: u32,
    height: u32,
    on: Vec<bool>,
}

impl Cover {
    /// The texels from `min` to `max` covered by `piece`'s triangles of
    /// `shape` (or reached by them, [`EDGE_REACH`]).
    fn of(
        surface: &PaintSurface,
        layout: &SeamLayout,
        piece: usize,
        (min, max): ([u32; 2], [u32; 2]),
        size: f32,
    ) -> Self {
        let (width, height) = (max[0] - min[0] + 1, max[1] - min[1] + 1);
        let mut cover = Self {
            min,
            width,
            height,
            on: vec![false; (width * height) as usize],
        };
        for t in layout.triangles_near(min, max) {
            if layout.pieces.get(t) != Some(&piece) {
                continue;
            }
            let uvs = surface.triangles()[t].uvs;
            let corners = uvs.map(|uv| Vec2::new(uv.x * size, (1.0 - uv.y) * size));
            cover.add(corners);
        }
        cover
    }

    fn add(&mut self, [a, b, c]: [Vec2; 3]) {
        let lo = a.min(b).min(c) - Vec2::splat(EDGE_REACH + 0.5);
        let hi = a.max(b).max(c) + Vec2::splat(EDGE_REACH + 0.5);
        let x0 = (lo.x.floor().max(self.min[0] as f32)) as u32;
        let y0 = (lo.y.floor().max(self.min[1] as f32)) as u32;
        let x1 = (hi.x.ceil().min((self.min[0] + self.width - 1) as f32)).max(0.0) as u32;
        let y1 = (hi.y.ceil().min((self.min[1] + self.height - 1) as f32)).max(0.0) as u32;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                if distance_to_triangle(p, [a, b, c]) <= EDGE_REACH
                    && let Some(i) = self.index(x, y)
                {
                    self.on[i] = true;
                }
            }
        }
    }

    fn index(&self, x: u32, y: u32) -> Option<usize> {
        let (dx, dy) = (x.checked_sub(self.min[0])?, y.checked_sub(self.min[1])?);
        (dx < self.width && dy < self.height).then(|| (dy * self.width + dx) as usize)
    }

    fn has(&self, x: i64, y: i64) -> bool {
        u32::try_from(x)
            .ok()
            .zip(u32::try_from(y).ok())
            .and_then(|(x, y)| self.index(x, y))
            .is_some_and(|i| self.on[i])
    }
}

/// How far `p` is from triangle `[a, b, c]` (0 inside).
fn distance_to_triangle(p: Vec2, [a, b, c]: [Vec2; 3]) -> f32 {
    let side = |u: Vec2, v: Vec2| (v - u).perp_dot(p - u);
    let (d0, d1, d2) = (side(a, b), side(b, c), side(c, a));
    let inside = (d0 >= 0.0 && d1 >= 0.0 && d2 >= 0.0) || (d0 <= 0.0 && d1 <= 0.0 && d2 <= 0.0);
    if inside {
        return 0.0;
    }
    let to_segment = |u: Vec2, v: Vec2| {
        let d = v - u;
        let t = if d.length_squared() > 0.0 {
            ((p - u).dot(d) / d.length_squared()).clamp(0.0, 1.0)
        } else {
            0.0
        };
        p.distance(u + d * t)
    };
    to_segment(a, b).min(to_segment(b, c)).min(to_segment(c, a))
}

/// Where a shape's triangles lie on a texture of one size: each
/// triangle's piece, and its triangles by the tile their UVs reach. Made
/// once and kept while the shape stays the same, to find which texels
/// near a seam belong to which side (the seam blend brush keeps one for
/// a stroke, [decision 0294](../../../docs/decisions/0294-seam-blend-brush.md)).
#[derive(Debug, Clone)]
pub struct SeamLayout {
    size: u32,
    pieces: Vec<usize>,
    by_tile: HashMap<u32, Vec<usize>>,
    across: u32,
}

impl SeamLayout {
    /// `surface`'s triangles in `slots` on a `size`-pixel texture.
    pub fn of(surface: &PaintSurface, slots: &[u32], size: u32) -> Self {
        let s = size as f32;
        let across = crate::image::tiles_across(size);
        let mut by_tile: HashMap<u32, Vec<usize>> = HashMap::new();
        for (i, t) in surface.triangles().iter().enumerate() {
            if !slots.contains(&t.slot) {
                continue;
            }
            let corners = t.uvs.map(|uv| Vec2::new(uv.x * s, (1.0 - uv.y) * s));
            let lo = corners[0].min(corners[1]).min(corners[2]) - Vec2::splat(EDGE_REACH + 1.0);
            let hi = corners[0].max(corners[1]).max(corners[2]) + Vec2::splat(EDGE_REACH + 1.0);
            if !lo.is_finite()
                || !hi.is_finite()
                || hi.x < 0.0
                || hi.y < 0.0
                || lo.x >= s
                || lo.y >= s
            {
                continue;
            }
            let tile = |v: f32| (v.max(0.0).min(s - 1.0) as u32) / TILE;
            for ty in tile(lo.y)..=tile(hi.y) {
                for tx in tile(lo.x)..=tile(hi.x) {
                    by_tile.entry(ty * across + tx).or_default().push(i);
                }
            }
        }
        Self {
            size,
            pieces: crate::fill::pieces(surface),
            by_tile,
            across,
        }
    }

    /// The texture size it was made for, in pixels.
    pub fn size(&self) -> u32 {
        self.size
    }

    /// The triangles whose UVs come near the texels from `min` to `max`,
    /// each once.
    fn triangles_near(&self, min: [u32; 2], max: [u32; 2]) -> Vec<usize> {
        let mut out = Vec::new();
        for ty in min[1] / TILE..=max[1] / TILE {
            for tx in min[0] / TILE..=max[0] / TILE {
                if let Some(list) = self.by_tile.get(&(ty * self.across + tx)) {
                    out.extend_from_slice(list);
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }
}

/// What blending one side of a seam writes, texel by texel: the seam
/// blend layer keeps the nearest seam's colour as see-through as its
/// strength; the brush keeps the colour to pull towards.
pub(crate) trait Sink {
    /// Whether texel (`x`, `y`) already has a seam nearer than `distance`.
    fn nearer(&self, x: u32, y: u32, distance: f32) -> bool;
    /// Texel (`x`, `y`), `distance` texels from the seam, is pulled
    /// towards `colour` (the other side's) by `strength` (0 to 0.5).
    fn put(&mut self, x: u32, y: u32, distance: f32, colour: [u8; 4], strength: f32);
}

/// The blend being made: per tile, each texel's nearest seam so far and
/// its pixel.
struct Blended {
    size: u32,
    tiles: HashMap<usize, (Vec<f32>, Vec<u8>)>,
}

impl Sink for Blended {
    /// Puts the other side's colour at texel (`x`, `y`), as see-through
    /// as the blend is weak, if `distance` is nearer a seam than what is
    /// there.
    fn put(&mut self, x: u32, y: u32, distance: f32, colour: [u8; 4], strength: f32) {
        let alpha = (strength * f32::from(colour[3])).round().clamp(0.0, 255.0) as u8;
        let rgba = [colour[0], colour[1], colour[2], alpha];
        let across = crate::image::tiles_across(self.size);
        let index = ((y / TILE) * across + x / TILE) as usize;
        let (near, pixels) = self.tiles.entry(index).or_insert_with(|| {
            (
                vec![f32::INFINITY; (TILE * TILE) as usize],
                vec![0; crate::image::TILE_BYTES],
            )
        });
        let at = ((y % TILE) * TILE + x % TILE) as usize;
        if distance < near[at] {
            near[at] = distance;
            pixels[at * 4..at * 4 + 4].copy_from_slice(&rgba);
        }
    }

    fn nearer(&self, x: u32, y: u32, distance: f32) -> bool {
        let across = crate::image::tiles_across(self.size);
        let index = ((y / TILE) * across + x / TILE) as usize;
        let at = ((y % TILE) * TILE + x % TILE) as usize;
        self.tiles
            .get(&index)
            .is_some_and(|(near, _)| near[at] <= distance)
    }
}

impl Blended {
    fn into_image(self) -> Option<Image> {
        let mut image = Image::new(self.size)?;
        for (index, (_, pixels)) in self.tiles {
            if pixels.chunks_exact(4).any(|p| p[3] > 0) {
                let tile: Tile = Arc::new(pixels);
                image.set_tile(index, Some(tile));
            }
        }
        Some(image)
    }
}

/// The Seam blend layer's pixels for `chosen` on `shapes`, `width` texels
/// each side, made from `below` (the layers under it mixed; only the
/// tiles [`tiles_used`] lists need be there). Where the material meets
/// another, its side is blended towards the other's painting in `others`
/// (only the tiles [`other_tiles`] lists need be there), and only its
/// side is written: the other material's own Seam blend layer blends
/// that one. `None` for an image of no usable size.
pub fn blend(
    below: &Image,
    others: &[&Image],
    shapes: &[BlendShape<'_>],
    chosen: &[SeamId],
    width: f32,
) -> Option<Image> {
    let size = below.size();
    let width = SeamBlend::usable_width(width);
    let mut out = Blended {
        size,
        tiles: HashMap::new(),
    };
    let (found, _) = resolve(shapes, chosen);
    let mut layouts: HashMap<(usize, Option<usize>), SeamLayout> = HashMap::new();
    for f in found {
        let shape = shapes[f.shape];
        layouts
            .entry((f.shape, None))
            .or_insert_with(|| SeamLayout::of(shape.surface, shape.slots, size));
        let Some(k) = f.across else {
            let layout = &layouts[&(f.shape, None)];
            for k in 0..2 {
                blend_side(
                    Side::on(below, shape.surface, layout, &f.seam.sides[k]),
                    Side::on(below, shape.surface, layout, &f.seam.sides[1 - k]),
                    width,
                    &mut out,
                );
            }
            continue;
        };
        let across = shape.across[k];
        let Some(&theirs) = others.get(across.other) else {
            continue;
        };
        layouts
            .entry((f.shape, Some(k)))
            .or_insert_with(|| SeamLayout::of(shape.surface, across.slots, theirs.size()));
        let (mine, other) = (&layouts[&(f.shape, None)], &layouts[&(f.shape, Some(k))]);
        blend_side(
            Side::on(below, shape.surface, mine, &f.seam.sides[0]),
            Side::on(theirs, shape.surface, other, &f.seam.sides[1]),
            width,
            &mut out,
        );
    }
    out.into_image()
}

/// One side of a seam as it is blended: the painting it is on, its
/// shape laid out on that painting, and the side.
#[derive(Clone, Copy)]
pub(crate) struct Side<'a> {
    image: &'a Image,
    surface: &'a PaintSurface,
    layout: &'a SeamLayout,
    side: &'a SeamSide,
}

impl<'a> Side<'a> {
    /// `side` of a seam of `surface` (laid out on `image` by `layout`).
    pub(crate) fn on(
        image: &'a Image,
        surface: &'a PaintSurface,
        layout: &'a SeamLayout,
        side: &'a SeamSide,
    ) -> Self {
        Self {
            image,
            surface,
            layout,
            side,
        }
    }
}

/// Blends the texels of side `from` of a seam towards side `to`'s,
/// `width` texels on `from`'s side: each painted texel of `from`'s piece
/// within the width, with the colour of `to`'s painting at the matching
/// place across the seam and how strongly it is pulled there, goes to
/// `out` (in `from`'s texels). The two sides may be on one painting or,
/// where two materials meet, on two of different sizes (decision 0296).
/// Shared by the Seam blend layer and the seam blend brush
/// ([decision 0294](../../../docs/decisions/0294-seam-blend-brush.md)).
pub(crate) fn blend_side(from: Side<'_>, to: Side<'_>, width: f32, out: &mut impl Sink) {
    let (size, theirs) = (from.image.size(), to.image.size());
    if from.layout.size() != size || to.layout.size() != theirs {
        return;
    }
    let (Some(a), Some(b)) = (
        Edge::of(from.side, size as f32),
        Edge::of(to.side, theirs as f32),
    ) else {
        return;
    };
    let scale = b.len / a.len;
    let (Some(area_a), Some(area_b)) = (
        a.area(width + 1.0, size),
        b.area(width * scale + 2.0, theirs),
    ) else {
        return;
    };
    let mine = Cover::of(
        from.surface,
        from.layout,
        from.side.piece,
        area_a,
        size as f32,
    );
    let other = Cover::of(to.surface, to.layout, to.side.piece, area_b, theirs as f32);
    let a_end = a.start + a.along * a.len;
    for y in area_a.0[1]..=area_a.1[1] {
        for x in area_a.0[0]..=area_a.1[0] {
            if !mine.has(i64::from(x), i64::from(y)) || from.image.pixel(x, y)[3] == 0 {
                continue;
            }
            let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let d = p - a.start;
            let (along, across) = (d.dot(a.along), d.dot(a.inward));
            if across < -1.0 {
                continue;
            }
            let distance = if along < 0.0 {
                d.length()
            } else if along > a.len {
                p.distance(a_end)
            } else {
                across.max(0.0)
            };
            if distance >= width || out.nearer(x, y, distance) {
                continue;
            }
            // The matching place on the other side, and its colour (nearer
            // the seam when the other piece is thinner there).
            let across = across.max(0.0);
            let colour = [1.0, 0.5, 0.0].into_iter().find_map(|f| {
                let depth = (across * f).max(0.5 / scale.max(1e-3));
                let q = b.start + (b.along * along + b.inward * depth) * scale;
                sample(to.image, &other, q)
            });
            let Some(colour) = colour else {
                continue;
            };
            let t = distance / width;
            let strength = 0.5 * (1.0 - t * t * (3.0 - 2.0 * t));
            out.put(x, y, distance, colour, strength);
        }
    }
}

/// `image`'s colour at `q` (in texels), mixed from the four texels round
/// it that `cover` holds and that are painted; `None` with none.
fn sample(image: &Image, cover: &Cover, q: Vec2) -> Option<[u8; 4]> {
    if !q.is_finite() {
        return None;
    }
    let base = q - Vec2::splat(0.5);
    let (x0, y0) = (base.x.floor(), base.y.floor());
    let (fx, fy) = (base.x - x0, base.y - y0);
    let size = i64::from(image.size());
    let mut sum = [0.0f32; 4];
    let mut weight = 0.0;
    for (dx, dy, w) in [
        (0, 0, (1.0 - fx) * (1.0 - fy)),
        (1, 0, fx * (1.0 - fy)),
        (0, 1, (1.0 - fx) * fy),
        (1, 1, fx * fy),
    ] {
        let (x, y) = (x0 as i64 + dx, y0 as i64 + dy);
        if w <= 0.0 || x < 0 || y < 0 || x >= size || y >= size || !cover.has(x, y) {
            continue;
        }
        let p = image.pixel(x as u32, y as u32);
        if p[3] == 0 {
            continue;
        }
        let a = f32::from(p[3]) * w;
        for c in 0..3 {
            sum[c] += f32::from(p[c]) * a;
        }
        sum[3] += a;
        weight += w;
    }
    if weight < 1e-4 || sum[3] <= 0.0 {
        return None;
    }
    let rgb = [0, 1, 2].map(|c| (sum[c] / sum[3]).round().clamp(0.0, 255.0) as u8);
    let alpha = (sum[3] / weight).round().clamp(0.0, 255.0) as u8;
    Some([rgb[0], rgb[1], rgb[2], alpha])
}

#[cfg(test)]
mod tests;
