//! Brush strokes on a shape ([decision 0179](../../../docs/decisions/0179-the-round-brush.md)).
//!
//! **Each dab is painted onto the nearby triangles in texture space:**
//! every texel a triangle covers near the dab is placed on the surface,
//! seen from the camera, and weighed by its distance from the dab's middle
//! on screen. Both sides of a UV seam sit at the same place on the surface,
//! so both are painted and the join doesn't show. Texels hidden from the
//! camera (behind another part of the shape) or facing away are left alone
//! unless the brush paints through.
//!
//! Within one stroke, coverage takes the largest dab weight (paint doesn't
//! pile up where dabs overlap); the stroke is laid on its layer as it was
//! when the stroke began, with the brush's flow.

use crate::brush::{Alignment, Brush, BrushKind};
use crate::image::Image;
use crate::surface::{PaintSurface, barycentric};
use glam::{Mat4, Vec2, Vec3};
use std::collections::BTreeSet;

/// The camera a stroke is painted through.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// World to clip space.
    pub view_projection: Mat4,
    /// The view's size in points.
    pub size: Vec2,
    /// Where the camera is.
    pub eye: Vec3,
    /// Which way it looks (unit length).
    pub forward: Vec3,
    /// Whether it looks flat (orthographic): every ray the same way.
    pub flat: bool,
}

impl View {
    /// Where `world` lands on screen (in points, from the top left) and
    /// its depth, or `None` behind the camera.
    pub fn screen(&self, world: Vec3) -> Option<(Vec2, f32)> {
        let clip = self.view_projection * world.extend(1.0);
        if clip.w <= 1e-6 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        Some((
            Vec2::new(
                (ndc.x + 1.0) * 0.5 * self.size.x,
                (1.0 - ndc.y) * 0.5 * self.size.y,
            ),
            ndc.z,
        ))
    }

    /// The world place seen at `screen` (points) at depth `depth` (as
    /// [`Self::screen`] gives it), or `None` if the camera can't say.
    pub fn unproject(&self, screen: Vec2, depth: f32) -> Option<Vec3> {
        let inverse = self.view_projection.inverse();
        if !inverse.is_finite() || self.size.x <= 0.0 || self.size.y <= 0.0 {
            return None;
        }
        let ndc = Vec3::new(
            screen.x / self.size.x * 2.0 - 1.0,
            1.0 - screen.y / self.size.y * 2.0,
            depth,
        );
        let world = inverse * ndc.extend(1.0);
        (world.w.abs() > 1e-12).then(|| world.truncate() / world.w)
    }

    /// The way from `world` towards the camera (unit length).
    pub fn toward_camera(&self, world: Vec3) -> Vec3 {
        if self.flat {
            -self.forward
        } else {
            (self.eye - world).normalize_or_zero()
        }
    }
}

/// How deep the shape is at each spot of the screen, seen from a stroke's
/// camera, to leave hidden texels alone. Made once when a stroke begins.
#[derive(Debug, Clone)]
pub struct DepthMap {
    width: usize,
    height: usize,
    /// Map pixels per screen point.
    scale: f32,
    depth: Vec<f32>,
    /// For each spot, the farthest depth among it and its neighbours
    /// (infinite if any of them is empty): what [`Self::visible`] tests
    /// against, worked out once (decision 0188).
    farthest: Vec<f32>,
}

/// Each spot's largest value among it and its eight neighbours on a
/// `width` × `height` map (an empty spot, infinite, wins): across the
/// rows, then down the columns.
fn window_max(values: &[f32], width: usize, height: usize) -> Vec<f32> {
    let mut across = vec![f32::NEG_INFINITY; values.len()];
    for y in 0..height {
        let row = &values[y * width..(y + 1) * width];
        for x in 0..width {
            let lo = x.saturating_sub(1);
            let hi = (x + 1).min(width - 1);
            across[y * width + x] = row[lo..=hi]
                .iter()
                .copied()
                .fold(f32::NEG_INFINITY, f32::max);
        }
    }
    let mut out = vec![f32::NEG_INFINITY; values.len()];
    for y in 0..height {
        let lo = y.saturating_sub(1);
        let hi = (y + 1).min(height - 1);
        for x in 0..width {
            out[y * width + x] = (lo..=hi)
                .map(|r| across[r * width + x])
                .fold(f32::NEG_INFINITY, f32::max);
        }
    }
    out
}

/// The widest the depth map is made.
const DEPTH_WIDTH: f32 = 1024.0;

impl DepthMap {
    /// The depth of `surface` placed by `model`, seen through `view`.
    pub fn of(surface: &PaintSurface, model: Mat4, view: &View) -> Self {
        let scale = (DEPTH_WIDTH / view.size.x.max(1.0)).min(1.0);
        let width = ((view.size.x * scale).ceil() as usize).clamp(1, 4096);
        let height = ((view.size.y * scale).ceil() as usize).clamp(1, 4096);
        let mut map = Self {
            width,
            height,
            scale,
            depth: vec![f32::INFINITY; width * height],
            farthest: Vec::new(),
        };
        for t in surface.triangles() {
            let mut corners = [(Vec2::ZERO, 0.0f32); 3];
            let mut seen = true;
            for (k, p) in t.points.iter().enumerate() {
                match view.screen(model.transform_point3(*p)) {
                    Some((s, z)) => corners[k] = (s * scale, z),
                    None => seen = false,
                }
            }
            if seen {
                map.fill(corners);
            }
        }
        map.farthest = window_max(&map.depth, width, height);
        map
    }

    /// The farthest depth of spot (`x`, `y`) and its neighbours on the
    /// map; infinite if any is empty, and minus infinity off the map.
    fn farthest_around(&self, x: i64, y: i64) -> f32 {
        let mut farthest = f32::NEG_INFINITY;
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (x, y) = (x + dx, y + dy);
                if x < 0 || y < 0 || x >= self.width as i64 || y >= self.height as i64 {
                    continue;
                }
                let d = self.depth[y as usize * self.width + x as usize];
                if !d.is_finite() {
                    return f32::INFINITY;
                }
                farthest = farthest.max(d);
            }
        }
        farthest
    }

    fn fill(&mut self, corners: [(Vec2, f32); 3]) {
        let pts = [corners[0].0, corners[1].0, corners[2].0];
        let lo = pts[0].min(pts[1]).min(pts[2]).max(Vec2::ZERO);
        let hi = pts[0]
            .max(pts[1])
            .max(pts[2])
            .min(Vec2::new(self.width as f32 - 1.0, self.height as f32 - 1.0));
        if lo.x > hi.x || lo.y > hi.y || !lo.is_finite() || !hi.is_finite() {
            return;
        }
        for y in lo.y.floor() as usize..=hi.y.ceil() as usize {
            for x in lo.x.floor() as usize..=hi.x.ceil() as usize {
                if x >= self.width || y >= self.height {
                    continue;
                }
                let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                let Some(w) = barycentric(p, pts) else {
                    continue;
                };
                if w.iter().any(|v| *v < -0.01) {
                    continue;
                }
                let z = w[0] * corners[0].1 + w[1] * corners[1].1 + w[2] * corners[2].1;
                let at = y * self.width + x;
                if z < self.depth[at] {
                    self.depth[at] = z;
                }
            }
        }
    }

    /// Whether a point at `screen` (in points) and `depth` is the nearest
    /// thing there, give or take a little.
    pub fn visible(&self, screen: Vec2, depth: f32) -> bool {
        let p = screen * self.scale;
        let (cx, cy) = (p.x.floor() as i64, p.y.floor() as i64);
        // The farthest of the neighbouring spots: the map is coarser than
        // the texels, so edges are given the benefit of the doubt. Just
        // off the map, its edge spots still count.
        let inside = cx >= 0 && cy >= 0 && cx < self.width as i64 && cy < self.height as i64;
        let farthest = if inside {
            self.farthest[cy as usize * self.width + cx as usize]
        } else {
            self.farthest_around(cx, cy)
        };
        !farthest.is_finite() || depth <= farthest + 1e-4 + farthest.abs() * 1e-4
    }
}

/// A picture laid over the view for projection painting (decision 0186):
/// its pixels and where it lies on screen.
#[derive(Debug, Clone, PartialEq)]
pub struct Stencil {
    /// Its width in pixels.
    pub width: u32,
    /// Its height in pixels.
    pub height: u32,
    /// Its pixels, RGBA (sRGB, straight alpha), row 0 at the top.
    pub rgba: std::sync::Arc<Vec<u8>>,
    /// Where its middle lies on screen, in points from the view's top left.
    pub centre: Vec2,
    /// How many screen points one of its pixels covers.
    pub scale: f32,
}

impl Stencil {
    /// Its pixel at screen point `at`; transparent outside it.
    pub fn at_screen(&self, at: Vec2) -> [u8; 4] {
        if self.scale.is_nan() || self.scale <= 0.0 {
            return [0; 4];
        }
        let p = (at - self.centre) / self.scale
            + Vec2::new(self.width as f32, self.height as f32) * 0.5;
        self.pixel(p.x, p.y)
    }

    /// Its pixel at `uv` when it is stretched over the texture square
    /// (painting flat in the UV Editor); `u` and `v` from 0 to 1, v up.
    pub fn at_uv(&self, uv: Vec2) -> [u8; 4] {
        self.pixel(uv.x * self.width as f32, (1.0 - uv.y) * self.height as f32)
    }

    fn pixel(&self, x: f32, y: f32) -> [u8; 4] {
        if !x.is_finite() || !y.is_finite() || x < 0.0 || y < 0.0 {
            return [0; 4];
        }
        let (x, y) = (x as u32, y as u32);
        if x >= self.width || y >= self.height {
            return [0; 4];
        }
        let i = ((y * self.width + x) * 4) as usize;
        match self.rgba.get(i..i + 4) {
            Some(p) => [p[0], p[1], p[2], p[3]],
            None => [0; 4],
        }
    }
}

/// Where effect brushes take their colours from (decision 0186), set for a
/// stroke.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum Source {
    /// Nothing more: the brush's own colour, or the texel itself.
    #[default]
    Own,
    /// The picture laid over the view (projection).
    Stencil(std::sync::Arc<Stencil>),
    /// The texel this many texels away (clone).
    Offset(i32, i32),
}

/// The layer image a stroke paints into, with how much each texel the
/// stroke has covered so far.
#[derive(Debug, Clone)]
pub struct StrokeCanvas {
    base: Image,
    out: Image,
    /// How much each texel is covered, tile by tile (none for a tile the
    /// stroke hasn't reached).
    coverage: Vec<Option<Box<[f32]>>>,
    /// Where effect brushes take colours from.
    source: Source,
    /// The colour a smudge carries, picked up as it goes.
    carried: [u8; 4],
    /// The texel rows the stroke may change: all, or one trim row's
    /// (decision 0202).
    keep: std::ops::Range<u32>,
}

impl StrokeCanvas {
    /// A stroke on `image` as it is now.
    pub fn new(image: Image) -> Self {
        Self {
            out: image.clone(),
            coverage: vec![None; image.tile_count()],
            base: image,
            source: Source::Own,
            carried: [0; 4],
            keep: 0..u32::MAX,
        }
    }

    /// Keeps the stroke to texel rows `rows` (one trim row's, decision
    /// 0202), or (`None`) lets it reach every row.
    pub fn keep_to_rows(&mut self, rows: Option<std::ops::Range<u32>>) {
        self.keep = rows.unwrap_or(0..u32::MAX);
    }

    /// Sets where effect brushes take colours from, for this stroke.
    pub fn set_source(&mut self, source: Source) {
        self.source = source;
    }

    /// The colour a smudge carries now.
    pub fn carried(&self) -> [u8; 4] {
        self.carried
    }

    /// Sets the colour a smudge carries.
    pub fn set_carried(&mut self, colour: [u8; 4]) {
        self.carried = colour;
    }

    /// The average of the stroke's image so far over the texels within
    /// `radius` of (`x`, `y`), alpha-weighted: what a smudge picks up.
    pub fn average(&self, x: f32, y: f32, radius: f32) -> [u8; 4] {
        average(&self.out, x, y, radius.clamp(1.0, 24.0))
    }

    /// The image with the stroke so far.
    pub fn image(&self) -> &Image {
        &self.out
    }

    /// The tiles the stroke has reached so far.
    pub fn touched(&self) -> BTreeSet<usize> {
        (self.coverage.iter().enumerate())
            .filter_map(|(i, c)| c.is_some().then_some(i))
            .collect()
    }

    /// Covers texel (`x`, `y`) by `weight` (0 to 1) with `brush`: kept if
    /// the stroke already covered it more.
    pub fn cover(&mut self, x: u32, y: u32, weight: f32, brush: &Brush) {
        self.cover_at(x, y, weight, brush, None);
    }

    /// [`cover`](Self::cover) for a texel seen at `screen` (for
    /// projection; `None` when painting flat).
    pub fn cover_at(&mut self, x: u32, y: u32, weight: f32, brush: &Brush, screen: Option<Vec2>) {
        let rows = self.out.tiles_across();
        self.band(0..rows).cover_at(x, y, weight, brush, screen);
    }

    /// Tile rows `rows` of the canvas, to change on their own.
    fn band(&mut self, rows: std::ops::Range<u32>) -> Band<'_> {
        let across = self.out.tiles_across() as usize;
        let size = self.out.size();
        let first = (rows.start as usize * across).min(self.coverage.len());
        let end = (rows.end as usize * across).clamp(first, self.coverage.len());
        Band {
            first,
            tiles: &mut self.out.tiles_mut()[first..end],
            coverage: &mut self.coverage[first..end],
            base: &self.base,
            source: &self.source,
            carried: self.carried,
            size,
            across: across as u32,
            rows: rows.start * crate::TILE..rows.end * crate::TILE,
            keep: self.keep.clone(),
        }
    }

    /// The canvas split into bands of whole tile rows, `bounds` apart
    /// (tile rows, rising, from 0 to the last), each changed on its own:
    /// several threads can paint a big dab at once (decision 0188).
    fn bands(&mut self, bounds: &[u32]) -> Vec<Band<'_>> {
        let across = self.out.tiles_across() as usize;
        let size = self.out.size();
        let mut tiles = self.out.tiles_mut();
        let mut coverage = self.coverage.as_mut_slice();
        let mut out = Vec::new();
        for pair in bounds.windows(2) {
            let rows = pair[1].saturating_sub(pair[0]) as usize;
            let count = (rows * across).min(tiles.len());
            let (t, rest) = std::mem::take(&mut tiles).split_at_mut(count);
            tiles = rest;
            let (c, rest) = std::mem::take(&mut coverage).split_at_mut(count);
            coverage = rest;
            out.push(Band {
                first: pair[0] as usize * across,
                tiles: t,
                coverage: c,
                base: &self.base,
                source: &self.source,
                carried: self.carried,
                size,
                across: across as u32,
                rows: pair[0] * crate::TILE..pair[1] * crate::TILE,
                keep: self.keep.clone(),
            });
        }
        out
    }
}

/// Some whole tile rows of a [`StrokeCanvas`], painted on their own.
struct Band<'a> {
    /// The index of its first tile.
    first: usize,
    tiles: &'a mut [Option<crate::Tile>],
    coverage: &'a mut [Option<Box<[f32]>>],
    base: &'a Image,
    source: &'a Source,
    carried: [u8; 4],
    /// The image's size, in pixels.
    size: u32,
    /// Tiles in a row.
    across: u32,
    /// The texel rows it holds.
    rows: std::ops::Range<u32>,
    /// The texel rows the stroke may change.
    keep: std::ops::Range<u32>,
}

impl Band<'_> {
    /// How much the stroke has covered texel (`x`, `y`) so far (0 outside
    /// the band).
    fn covered(&self, x: u32, y: u32) -> f32 {
        let index = ((y / crate::TILE) * self.across + x / crate::TILE) as usize;
        let at = ((y % crate::TILE) * crate::TILE + x % crate::TILE) as usize;
        index
            .checked_sub(self.first)
            .and_then(|t| self.coverage.get(t))
            .and_then(Option::as_ref)
            .map_or(0.0, |c| c[at])
    }

    /// [`StrokeCanvas::cover_at`] within the band (texels outside it are
    /// left alone).
    fn cover_at(&mut self, x: u32, y: u32, weight: f32, brush: &Brush, screen: Option<Vec2>) {
        let size = self.size;
        if x >= size
            || y >= size
            || weight <= 0.0
            || !self.rows.contains(&y)
            || !self.keep.contains(&y)
        {
            return;
        }
        let Some(tile) = (((y / crate::TILE) * self.across + x / crate::TILE) as usize)
            .checked_sub(self.first)
            .filter(|t| *t < self.tiles.len())
        else {
            return;
        };
        let at = ((y % crate::TILE) * crate::TILE + x % crate::TILE) as usize;
        let cover = self.coverage[tile]
            .get_or_insert_with(|| vec![0.0; (crate::TILE * crate::TILE) as usize].into());
        let w = weight.min(1.0);
        if w <= cover[at] {
            return;
        }
        cover[at] = w;
        let amount = w * brush.flow.clamp(0.0, 1.0);
        let base = self.base.pixel(x, y);
        let size_f = size as f32;
        let pixel = match brush.kind {
            BrushKind::Paint => over(base, brush.color, amount),
            BrushKind::Erase => {
                let a = base[3] as f32 * (1.0 - amount);
                [base[0], base[1], base[2], a.round() as u8]
            }
            BrushKind::Smudge => mix(base, self.carried, amount),
            BrushKind::Blur => mix(
                base,
                average(self.base, x as f32, y as f32, BLUR_REACH),
                amount,
            ),
            BrushKind::Blend => mix(
                base,
                average(self.base, x as f32, y as f32, BLEND_REACH),
                amount,
            ),
            BrushKind::HueShift => {
                let [r, g, b, a] = base;
                let [h, s, v] = crate::palette::to_hsv([r, g, b]);
                let [r, g, b] = crate::palette::from_hsv([h + brush.hue_shift, s, v]);
                mix(base, [r, g, b, a], amount)
            }
            BrushKind::Projection => {
                let colour = match (self.source, screen) {
                    (Source::Stencil(stencil), Some(at)) => stencil.at_screen(at),
                    (Source::Stencil(stencil), None) => stencil.at_uv(Vec2::new(
                        (x as f32 + 0.5) / size_f,
                        1.0 - (y as f32 + 0.5) / size_f,
                    )),
                    _ => [0; 4],
                };
                over(base, colour, amount)
            }
            BrushKind::Clone => match self.source {
                Source::Offset(dx, dy) => {
                    let (sx, sy) = (x as i64 + *dx as i64, y as i64 + *dy as i64);
                    if sx < 0 || sy < 0 || sx >= size as i64 || sy >= size as i64 {
                        return;
                    }
                    mix(base, self.base.pixel(sx as u32, sy as u32), amount)
                }
                _ => return,
            },
        };
        let pixels =
            self.tiles[tile].get_or_insert_with(|| std::sync::Arc::new(vec![0; crate::TILE_BYTES]));
        let offset = at * 4;
        std::sync::Arc::make_mut(pixels)[offset..offset + 4].copy_from_slice(&pixel);
    }
}

/// `base` moved toward `target` by `amount`, colour and coverage alike
/// (mixed premultiplied, so empty texels don't bring their colour).
fn mix(base: [u8; 4], target: [u8; 4], amount: f32) -> [u8; 4] {
    let t = amount.clamp(0.0, 1.0);
    let (ba, ta) = (base[3] as f32 / 255.0, target[3] as f32 / 255.0);
    let a = ba + (ta - ba) * t;
    if a <= 0.0 {
        return [0; 4];
    }
    let mut out = [0u8; 4];
    for c in 0..3 {
        let p = base[c] as f32 * ba + (target[c] as f32 * ta - base[c] as f32 * ba) * t;
        out[c] = (p / a).round().clamp(0.0, 255.0) as u8;
    }
    out[3] = (a * 255.0).round().clamp(0.0, 255.0) as u8;
    out
}

/// How far round blur averages, in texels, each pass.
const BLUR_REACH: f32 = 4.0;

/// How far round blend averages, in texels.
const BLEND_REACH: f32 = 9.0;

/// The alpha-weighted average of `image` over texels within `radius` of
/// (`x`, `y`), sampled about seven times across whatever the radius.
fn average(image: &Image, x: f32, y: f32, radius: f32) -> [u8; 4] {
    let size = image.size() as i64;
    let step = ((radius / 3.0).floor() as i64).max(1);
    let r = radius.ceil() as i64;
    let (cx, cy) = (x.floor() as i64, y.floor() as i64);
    let mut sum = [0.0f32; 4];
    let mut count = 0.0f32;
    let mut dy = -r;
    while dy <= r {
        let mut dx = -r;
        while dx <= r {
            let (px, py) = (cx + dx, cy + dy);
            if (dx * dx + dy * dy) as f32 <= radius * radius
                && px >= 0
                && py >= 0
                && px < size
                && py < size
            {
                let p = image.pixel(px as u32, py as u32);
                let a = p[3] as f32 / 255.0;
                for c in 0..3 {
                    sum[c] += p[c] as f32 * a;
                }
                sum[3] += a;
                count += 1.0;
            }
            dx += step;
        }
        dy += step;
    }
    if count <= 0.0 || sum[3] <= 0.0 {
        return [0; 4];
    }
    [
        (sum[0] / sum[3]).round().clamp(0.0, 255.0) as u8,
        (sum[1] / sum[3]).round().clamp(0.0, 255.0) as u8,
        (sum[2] / sum[3]).round().clamp(0.0, 255.0) as u8,
        (sum[3] / count * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

/// `color` laid over `base` by `amount` (straight alpha).
fn over(base: [u8; 4], color: [u8; 4], amount: f32) -> [u8; 4] {
    let sa = color[3] as f32 / 255.0 * amount.clamp(0.0, 1.0);
    let da = base[3] as f32 / 255.0;
    let out_a = sa + da * (1.0 - sa);
    if out_a <= 0.0 {
        return [0; 4];
    }
    let mut out = [0u8; 4];
    for c in 0..3 {
        let v = (color[c] as f32 * sa + base[c] as f32 * da * (1.0 - sa)) / out_a;
        out[c] = v.round().clamp(0.0, 255.0) as u8;
    }
    out[3] = (out_a * 255.0).round() as u8;
    out
}

/// One dab of a stroke.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dab {
    /// Its middle on screen, in points.
    pub screen: Vec2,
    /// Its middle on the surface, in the world.
    pub world: Vec3,
    /// Its radius on screen, in points.
    pub radius: f32,
    /// Its radius on the surface, in world units (centimetres).
    pub world_radius: f32,
    /// How strongly it covers, 0 to 1 (pen pressure on the flow,
    /// decision 0184; 1 for a mouse).
    pub strength: f32,
    /// Which way the stroke goes on screen, in radians (tips that follow
    /// the stroke turn to it, decision 0185).
    pub angle: f32,
    /// Its number in the stroke, for tips that scatter.
    pub seed: u32,
}

/// What a dab paints on: the shape's surface, which of its material slots
/// belong to the texture set, how it is placed in the world, and the
/// camera.
#[derive(Debug, Clone, Copy)]
pub struct Target<'a> {
    /// The shape.
    pub surface: &'a PaintSurface,
    /// The material slots painted (those whose material owns the
    /// texture set).
    pub slots: &'a [u32],
    /// Shape space to the world.
    pub model: Mat4,
    /// The camera.
    pub view: &'a View,
    /// What the camera sees, to leave hidden texels alone.
    pub depth: Option<&'a DepthMap>,
}

/// How far outside a triangle (in texels) a texel is still painted, so
/// the texels along a UV piece's edge are painted too.
const EDGE_REACH: f32 = 0.75;

/// Dabs covering more texels than this are painted on several threads
/// (decision 0188): fewer aren't worth starting threads for.
const TEXELS_A_THREAD: u64 = 16_384;

/// One triangle a dab reaches, ready to paint: in the world and in texel
/// space, which way it faces, and the texel box to go over.
struct Piece {
    world: [Vec3; 3],
    normal: Vec3,
    texel: [Vec2; 3],
    edges: [(Vec2, Vec2); 3],
    /// The first and last texel column and row.
    columns: (u32, u32),
    rows: (u32, u32),
}

/// One dab to paint: what on, with which brush, and where.
#[derive(Debug, Clone, Copy)]
pub struct DabJob<'a> {
    /// What it paints on.
    pub target: Target<'a>,
    /// The brush (its radius as painted).
    pub brush: Brush,
    /// The dab.
    pub dab: Dab,
}

/// Paints one dab of `brush` on `canvas`. A big dab is shared out over
/// the processor's cores, each painting its own rows of tiles.
pub fn paint_dab(canvas: &mut StrokeCanvas, target: &Target<'_>, brush: &Brush, dab: &Dab) {
    paint_dabs(
        canvas,
        &[DabJob {
            target: *target,
            brush: *brush,
            dab: *dab,
        }],
    );
}

/// Paints `jobs` on `canvas`, in order, as [`paint_dab`] would one by
/// one; many or big dabs are shared out over the processor's cores at
/// once, each core painting all the dabs on its own rows of tiles, so
/// threads are started once a frame rather than once a dab (decision
/// 0188).
pub fn paint_dabs(canvas: &mut StrokeCanvas, jobs: &[DabJob<'_>]) {
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
    paint_dabs_on(canvas, jobs, cores);
}

/// [`paint_dab`] on at most `cores` threads.
#[cfg(test)]
fn paint_dab_on(
    canvas: &mut StrokeCanvas,
    target: &Target<'_>,
    brush: &Brush,
    dab: &Dab,
    cores: usize,
) {
    let job = DabJob {
        target: *target,
        brush: *brush,
        dab: *dab,
    };
    paint_dabs_on(canvas, &[job], cores);
}

/// The triangles a dab reaches, ready to paint, on a texture `size`
/// texels across.
fn pieces_of(job: &DabJob<'_>, size: f32) -> Vec<Piece> {
    let (target, dab) = (&job.target, &job.dab);
    let Some(inverse) = Some(target.model.inverse()).filter(|m| m.is_finite()) else {
        return Vec::new();
    };
    let scale = target
        .model
        .to_scale_rotation_translation()
        .0
        .abs()
        .max_element()
        .max(1e-6);
    let mirrored = target.model.determinant() < 0.0;
    let centre = inverse.transform_point3(dab.world);
    let mut near = Vec::new();
    target
        .surface
        .near(centre, dab.world_radius / scale * 1.05, &mut near);
    // A dab seen from the camera stretches on a surface turned away, up to
    // where turned surfaces stop taking paint (a fifth facing): where the
    // surface under it turns away, it may reach that much further
    // (decision 0217).
    let toward = target.view.toward_camera(dab.world);
    let stretch = if job.brush.alignment == Alignment::Camera {
        let least = near
            .iter()
            .map(|i| {
                let t = &target.surface.triangles()[*i];
                let n = target.model.transform_vector3(t.normal).normalize_or_zero();
                n.dot(toward).abs()
            })
            .fold(1.0f32, f32::min);
        1.0 / least.max(0.2)
    } else {
        1.0
    };
    if stretch > 1.05 {
        near.clear();
        target
            .surface
            .near(centre, dab.world_radius * stretch / scale * 1.05, &mut near);
    }
    let mut pieces = Vec::new();
    for i in near {
        let t = &target.surface.triangles()[i];
        if !target.slots.contains(&t.slot) {
            continue;
        }
        let world = t.points.map(|p| target.model.transform_point3(p));
        let mut normal = (world[1] - world[0])
            .cross(world[2] - world[0])
            .normalize_or_zero();
        if mirrored {
            normal = -normal;
        }
        // A triangle facing away at its three corners faces away all
        // over (unless painting through): nothing to paint there.
        if !job.brush.through
            && world
                .iter()
                .all(|p| normal.dot(target.view.toward_camera(*p)) <= 0.0)
        {
            continue;
        }
        // The triangle in texel space (row 0 at the top, where V is 1).
        let whole = t.uvs.map(|uv| Vec2::new(uv.x * size, (1.0 - uv.y) * size));
        // A triangle past the square (a trim strip running along its row,
        // decision 0202) paints where the texture repeats: each square
        // it reaches, moved back onto the texture.
        for shift in crate::surface::tile_shifts(&whole, size) {
            let texel = whole.map(|p| p + shift);
            let lo = texel[0].min(texel[1]).min(texel[2]) - Vec2::splat(EDGE_REACH);
            let hi = texel[0].max(texel[1]).max(texel[2]) + Vec2::splat(EDGE_REACH);
            // Only the part the dab can reach.
            let stretched = if stretch > 1.0 {
                (1.0 / normal.dot(toward).abs().max(0.2)).min(stretch)
            } else {
                1.0
            };
            let (lo, hi) = match reach(&world, &texel, dab, stretched) {
                Some((a, b)) => (lo.max(a), hi.min(b)),
                None => (lo, hi),
            };
            let lo = lo.max(Vec2::ZERO);
            let hi = hi.min(Vec2::splat(size - 1.0));
            if lo.x > hi.x || lo.y > hi.y || !lo.is_finite() || !hi.is_finite() {
                continue;
            }
            pieces.push(Piece {
                world,
                normal,
                texel,
                edges: edge_lines(&texel),
                columns: (lo.x.floor() as u32, hi.x.ceil() as u32),
                rows: (lo.y.floor() as u32, hi.y.ceil() as u32),
            });
        }
    }
    pieces
}

/// How a dab's shape is laid on the surface, worked out once a dab
/// (decision 0217): where its middle is, and which way is across and down
/// in it.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Frame {
    /// Seen from the camera: offsets on screen.
    Camera,
    /// On the surface where the dab lands: offsets along its flat
    /// (`across` and `down` follow the screen's), wrapped round or fading
    /// off the flat.
    Surface {
        centre: Vec3,
        normal: Vec3,
        across: Vec3,
        down: Vec3,
        wrap: bool,
    },
    /// On the texture: offsets in texels from the middle's texel,
    /// `radius` texels across.
    Uv { centre: Vec2, radius: f32 },
}

/// The frame for `job`'s dab, from the triangles it reaches.
fn frame_of(job: &DabJob<'_>, pieces: &[Piece]) -> Frame {
    let dab = &job.dab;
    match job.brush.alignment {
        Alignment::Camera => Frame::Camera,
        Alignment::Wrap | Alignment::Planar => {
            surface_frame(job, pieces, job.brush.alignment == Alignment::Wrap)
                .unwrap_or(Frame::Camera)
        }
        Alignment::Uv => {
            // The triangle the middle lies on (the nearest), and how many
            // texels a world unit spans there.
            let mut best: Option<(f32, Vec2, f32)> = None;
            for piece in pieces {
                let Some((weights, gap)) = on_triangle(&piece.world, dab.world) else {
                    continue;
                };
                let outside = weights.iter().map(|w| (-w).max(0.0)).sum::<f32>();
                let score = gap + outside * dab.world_radius.max(1e-3) * 4.0;
                if best.is_some_and(|b| b.0 <= score) {
                    continue;
                }
                let texel = piece.texel;
                let middle = texel[0] * weights[0] + texel[1] * weights[1] + texel[2] * weights[2];
                best = Some((score, middle, texels_per_unit(&piece.world, &texel)));
            }
            match best {
                Some((_, centre, per_unit)) if per_unit > 0.0 => Frame::Uv {
                    centre,
                    radius: dab.world_radius * per_unit,
                },
                _ => Frame::Camera,
            }
        }
    }
}

/// The surface frame: the surface's facing round the dab's middle (the
/// triangles near it, the larger and nearer counting for more), turned
/// to the camera, with across and down along the screen's.
fn surface_frame(job: &DabJob<'_>, pieces: &[Piece], wrap: bool) -> Option<Frame> {
    let dab = &job.dab;
    let near = (dab.world_radius * 0.5).max(1e-4);
    let mut sum = Vec3::ZERO;
    let mut nearest: Option<(f32, Vec3)> = None;
    for piece in pieces {
        let Some((_, gap)) = on_triangle(&piece.world, dab.world) else {
            continue;
        };
        let area = (piece.world[1] - piece.world[0])
            .cross(piece.world[2] - piece.world[0])
            .length();
        let middle = (piece.world[0] + piece.world[1] + piece.world[2]) / 3.0;
        let closeness = (1.0 - middle.distance(dab.world) / (2.0 * near)).max(0.0);
        sum += piece.normal * area * closeness;
        if nearest.is_none_or(|n| gap < n.0) {
            nearest = Some((gap, piece.normal));
        }
    }
    let mut normal = sum.try_normalize().or(nearest.map(|n| n.1))?;
    let toward = job.target.view.toward_camera(dab.world);
    if normal.dot(toward) < 0.0 {
        normal = -normal;
    }
    let view = job.target.view;
    let (screen, depth) = view.screen(dab.world)?;
    let along = |by: Vec2| -> Option<Vec3> {
        let to = view.unproject(screen + by, depth)? - dab.world;
        (to - normal * to.dot(normal)).try_normalize()
    };
    let across = along(Vec2::X).unwrap_or_else(|| normal.any_orthonormal_vector());
    let mut down = normal.cross(across);
    if let Some(d) = along(Vec2::Y)
        && d.dot(down) < 0.0
    {
        down = -down;
    }
    Some(Frame::Surface {
        centre: dab.world,
        normal,
        across,
        down,
        wrap,
    })
}

/// Where `point` falls on the plane of triangle `world`: its weights and
/// how far it lies off the plane.
fn on_triangle(world: &[Vec3; 3], point: Vec3) -> Option<([f32; 3], f32)> {
    let (e1, e2) = (world[1] - world[0], world[2] - world[0]);
    let d = point - world[0];
    let (d00, d01, d11) = (e1.dot(e1), e1.dot(e2), e2.dot(e2));
    let (d20, d21) = (d.dot(e1), d.dot(e2));
    let den = d00 * d11 - d01 * d01;
    if den.abs() < 1e-12 {
        return None;
    }
    let v = (d11 * d20 - d01 * d21) / den;
    let w = (d00 * d21 - d01 * d20) / den;
    let normal = e1.cross(e2).normalize_or_zero();
    Some(([1.0 - v - w, v, w], d.dot(normal).abs()))
}

/// How many texels a world unit spans on a triangle, on average over its
/// area.
fn texels_per_unit(world: &[Vec3; 3], texel: &[Vec2; 3]) -> f32 {
    let area = (world[1] - world[0]).cross(world[2] - world[0]).length();
    let flat = (texel[1] - texel[0]).perp_dot(texel[2] - texel[0]).abs();
    if area <= 1e-12 {
        return 0.0;
    }
    (flat / area).sqrt()
}

/// A texel's offset from the dab's middle and the dab's radius, in the
/// frame's units, and how much of it stays (a planar dab fading where the
/// surface leaves its flat).
fn offset_in(frame: &Frame, texel: Vec2, at: Vec3, screen: Vec2, dab: &Dab) -> (Vec2, f32, f32) {
    match *frame {
        Frame::Camera => (screen - dab.screen, dab.radius, 1.0),
        Frame::Surface {
            centre,
            normal,
            across,
            down,
            wrap,
        } => {
            let d = at - centre;
            let flat = Vec2::new(d.dot(across), d.dot(down));
            let r = dab.world_radius;
            if wrap {
                // Measured along the surface (the straight way there, which
                // follows a curve closely within a dab): the dab bends
                // round instead of stretching.
                (flat.normalize_or_zero() * d.length(), r, 1.0)
            } else {
                let off = d.dot(normal).abs();
                let t = ((off - 0.25 * r) / (0.5 * r).max(1e-6)).clamp(0.0, 1.0);
                (flat, r, 1.0 - t * t * (3.0 - 2.0 * t))
            }
        }
        Frame::Uv { centre, radius } => (texel - centre, radius, 1.0),
    }
}

/// [`paint_dabs`] on at most `cores` threads.
fn paint_dabs_on(canvas: &mut StrokeCanvas, jobs: &[DabJob<'_>], cores: usize) {
    let size = canvas.image().size() as f32;
    let prepared: Vec<(&DabJob<'_>, Vec<Piece>, Frame)> = jobs
        .iter()
        .map(|job| {
            let pieces = pieces_of(job, size);
            let frame = frame_of(job, &pieces);
            (job, pieces, frame)
        })
        .collect();
    let all = || prepared.iter().flat_map(|(_, pieces, _)| pieces);
    let Some(first_row) = all().map(|p| p.rows.0).min() else {
        return;
    };
    let last_row = all().map(|p| p.rows.1).max().unwrap_or(first_row);
    let texels: u64 = all()
        .map(|p| u64::from(p.columns.1 - p.columns.0 + 1) * u64::from(p.rows.1 - p.rows.0 + 1))
        .sum();
    let tile_rows = canvas.image().tiles_across();
    let (top, bottom) = (first_row / crate::TILE, last_row / crate::TILE + 1);
    let threads = cores
        .min((texels / TEXELS_A_THREAD) as usize)
        .min((bottom - top) as usize)
        .max(1);
    let paint_all = |band: &mut Band<'_>| {
        for (job, pieces, frame) in &prepared {
            for piece in pieces {
                paint_piece(band, piece, frame, &job.target, &job.brush, &job.dab);
            }
        }
    };
    if threads == 1 {
        paint_all(&mut canvas.band(0..tile_rows));
        return;
    }
    // Rows of tiles shared out evenly over the rows the dabs reach.
    let mut bounds = vec![0];
    for k in 1..threads as u32 {
        bounds.push(top + (bottom - top) * k / threads as u32);
    }
    bounds.push(tile_rows);
    let bands = canvas.bands(&bounds);
    let paint_all = &paint_all;
    std::thread::scope(|scope| {
        for mut band in bands {
            scope.spawn(move || paint_all(&mut band));
        }
    });
}

/// Paints the part of `piece` within `band`. The weights, world place
/// and screen place of each texel are worked out along the rows from
/// sums made once per piece (they change evenly across a triangle), and
/// each row only goes over the texels inside the triangle (decision
/// 0188).
fn paint_piece(
    band: &mut Band<'_>,
    piece: &Piece,
    frame: &Frame,
    target: &Target<'_>,
    brush: &Brush,
    dab: &Dab,
) {
    let first = piece.rows.0.max(band.rows.start);
    let last = piece.rows.1.min(band.rows.end.saturating_sub(1));
    if band.rows.is_empty() || first > last {
        return;
    }
    let world = &piece.world;
    // The weights as sums of x and y: v = vx·x + vy·y + vc, and so w.
    let [a, b, c] = piece.texel;
    let (v0, v1) = (b - a, c - a);
    let den = v0.x * v1.y - v1.x * v0.y;
    if den.abs() < 1e-14 {
        return;
    }
    let weights_v = glam::Vec3::new(v1.y, -v1.x, v1.x * a.y - a.x * v1.y) / den;
    let weights_w = glam::Vec3::new(-v0.y, v0.x, a.x * v0.y - v0.x * a.y) / den;
    // Each corner in clip space: a texel's is the same weighted sum.
    let clip = world.map(|p| target.view.view_projection * p.extend(1.0));
    let size = target.view.size;
    // The most this dab can cover a texel: where the stroke has covered
    // it that much already, there's nothing to work out.
    let most = dab.strength.clamp(0.0, 1.0);
    for y in first..=last {
        let py = y as f32 + 0.5;
        let Some((from, to)) = row_span(&piece.edges, py, piece.columns) else {
            continue;
        };
        for x in from..=to {
            let p = Vec2::new(x as f32 + 0.5, py);
            if !within(&piece.edges, p) || band.covered(x, y) >= most {
                continue;
            }
            let at3 = glam::Vec3::new(p.x, p.y, 1.0);
            let (v, w) = (weights_v.dot(at3), weights_w.dot(at3));
            let mut weights = [1.0 - v - w, v, w];
            if weights.iter().any(|k| *k < 0.0) {
                weights = clamp_weights(weights);
            }
            let at = world[0] * weights[0] + world[1] * weights[1] + world[2] * weights[2];
            // How squarely the surface faces the camera (1 when painting
            // through): first, as it is the quickest way out.
            let facing = if brush.through {
                1.0
            } else {
                piece.normal.dot(target.view.toward_camera(at))
            };
            if facing <= 0.0 {
                continue;
            }
            let point = clip[0] * weights[0] + clip[1] * weights[1] + clip[2] * weights[2];
            if point.w <= 1e-6 {
                continue;
            }
            let ndc = point.truncate() / point.w;
            let screen = Vec2::new((ndc.x + 1.0) * 0.5 * size.x, (1.0 - ndc.y) * 0.5 * size.y);
            let (offset, radius, kept) = offset_in(frame, p, at, screen, dab);
            let round = brush.falloff(offset.length(), radius);
            let mut weight = brush
                .tip
                .weight(offset, radius, dab.angle, dab.seed, (x, y), round)
                * kept;
            if weight <= 0.0 {
                continue;
            }
            if !brush.through {
                // Surfaces seen edge on take less.
                weight *= (facing / 0.2).min(1.0);
                if let Some(map) = target.depth
                    && !map.visible(screen, ndc.z)
                {
                    continue;
                }
            }
            band.cover_at(x, y, weight * most, brush, Some(screen));
        }
    }
}

/// The columns of row `py` (a texel middle's height) that can be within
/// a triangle of `edges`, inside `columns`: a little wide, so the texels
/// at its ends are still tested one by one.
fn row_span(edges: &[(Vec2, Vec2); 3], py: f32, columns: (u32, u32)) -> Option<(u32, u32)> {
    let (mut lo, mut hi) = (columns.0 as f32, columns.1 as f32);
    for (on, inside) in edges {
        // Within: inside.x · (x + 0.5) + rest ≥ −EDGE_REACH.
        let rest = inside.x * 0.5 + inside.y * py - on.dot(*inside);
        let bound = (-EDGE_REACH - rest) / inside.x;
        if inside.x > 1e-6 {
            lo = lo.max(bound.floor() - 1.0);
        } else if inside.x < -1e-6 {
            hi = hi.min(bound.ceil() + 1.0);
        } else if rest < -EDGE_REACH - 1e-3 {
            return None;
        }
    }
    if lo.is_nan() || hi.is_nan() || lo > hi {
        return None;
    }
    Some((
        lo.max(columns.0 as f32) as u32,
        hi.min(columns.1 as f32) as u32,
    ))
}

/// The texel box the dab can reach on a triangle, from its middle's place
/// on the triangle and how many texels a world unit spans there,
/// `stretched` times as far (a camera dab on a turned surface).
fn reach(world: &[Vec3; 3], texel: &[Vec2; 3], dab: &Dab, stretched: f32) -> Option<(Vec2, Vec2)> {
    let mut texels_per_unit = 0.0f32;
    for (a, b) in [(0, 1), (1, 2), (2, 0)] {
        let length = world[a].distance(world[b]);
        if length > 1e-6 {
            texels_per_unit = texels_per_unit.max(texel[a].distance(texel[b]) / length);
        }
    }
    if texels_per_unit <= 0.0 {
        return None;
    }
    // The dab's middle placed on the triangle's plane, in its weights.
    let (e1, e2) = (world[1] - world[0], world[2] - world[0]);
    let d = dab.world - world[0];
    let (d00, d01, d11) = (e1.dot(e1), e1.dot(e2), e2.dot(e2));
    let (d20, d21) = (d.dot(e1), d.dot(e2));
    let den = d00 * d11 - d01 * d01;
    if den.abs() < 1e-12 {
        return None;
    }
    let v = (d11 * d20 - d01 * d21) / den;
    let w = (d00 * d21 - d01 * d20) / den;
    let middle = texel[0] * (1.0 - v - w) + texel[1] * v + texel[2] * w;
    let r = dab.world_radius * stretched * texels_per_unit * 1.2 + 2.0;
    Some((middle - Vec2::splat(r), middle + Vec2::splat(r)))
}

/// A triangle's three edges as lines: a point on each, and the unit
/// normal pointing inside.
fn edge_lines(t: &[Vec2; 3]) -> [(Vec2, Vec2); 3] {
    let area = (t[1] - t[0]).perp_dot(t[2] - t[0]);
    let sign = if area >= 0.0 { 1.0 } else { -1.0 };
    [(0, 1), (1, 2), (2, 0)].map(|(a, b)| {
        let e = t[b] - t[a];
        let inside = Vec2::new(-e.y, e.x).normalize_or_zero() * sign;
        (t[a], inside)
    })
}

/// Whether `p` is inside the triangle or within [`EDGE_REACH`] texels of
/// it.
fn within(edges: &[(Vec2, Vec2); 3], p: Vec2) -> bool {
    edges
        .iter()
        .all(|(on, inside)| (p - *on).dot(*inside) >= -EDGE_REACH)
}

/// Weights moved inside the triangle (for texels just outside its edge).
fn clamp_weights(w: [f32; 3]) -> [f32; 3] {
    let w = w.map(|v| v.max(0.0));
    let sum = w[0] + w[1] + w[2];
    if sum <= 0.0 {
        return [1.0 / 3.0; 3];
    }
    w.map(|v| v / sum)
}

/// Where the dabs of a stroke go as the mouse moves from `from` to `to`,
/// every `step` points. `since` is how far the mouse has gone since the
/// last dab (`None` at a stroke's start: a dab at once at `from`).
pub fn dabs_along(from: Vec2, to: Vec2, step: f32, since: &mut Option<f32>) -> Vec<Vec2> {
    let step = step.max(0.5);
    let mut out = Vec::new();
    let gone = match since {
        Some(gone) => *gone,
        None => {
            out.push(from);
            0.0
        }
    };
    let length = from.distance(to);
    let mut at = step - gone;
    while at <= length {
        out.push(from.lerp(to, at / length.max(1e-6)));
        at += step;
    }
    *since = Some(length - (at - step));
    out
}

#[cfg(test)]
mod tests;
