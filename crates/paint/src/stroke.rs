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

use crate::brush::{Brush, BrushKind};
use crate::image::Image;
use crate::surface::{PaintSurface, barycentric};
use glam::{Mat4, Vec2, Vec3};
use std::collections::{BTreeSet, HashMap};

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
        map
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
                let Some(w) = barycentric(p, pts) else { continue };
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
        // the texels, so edges are given the benefit of the doubt.
        let mut farthest = f32::NEG_INFINITY;
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (x, y) = (cx + dx, cy + dy);
                if x < 0 || y < 0 || x >= self.width as i64 || y >= self.height as i64 {
                    continue;
                }
                let d = self.depth[y as usize * self.width + x as usize];
                if d.is_finite() {
                    farthest = farthest.max(d);
                } else {
                    return true;
                }
            }
        }
        !farthest.is_finite() || depth <= farthest + 1e-4 + farthest.abs() * 1e-4
    }
}

/// The layer image a stroke paints into, with how much each texel the
/// stroke has covered so far.
#[derive(Debug, Clone)]
pub struct StrokeCanvas {
    base: Image,
    out: Image,
    coverage: HashMap<usize, Vec<f32>>,
    touched: BTreeSet<usize>,
}

impl StrokeCanvas {
    /// A stroke on `image` as it is now.
    pub fn new(image: Image) -> Self {
        Self {
            out: image.clone(),
            base: image,
            coverage: HashMap::new(),
            touched: BTreeSet::new(),
        }
    }

    /// The image with the stroke so far.
    pub fn image(&self) -> &Image {
        &self.out
    }

    /// The tiles the stroke has changed so far.
    pub fn touched(&self) -> &BTreeSet<usize> {
        &self.touched
    }

    /// Covers texel (`x`, `y`) by `weight` (0 to 1) with `brush`: kept if
    /// the stroke already covered it more.
    pub fn cover(&mut self, x: u32, y: u32, weight: f32, brush: &Brush) {
        let size = self.out.size();
        if x >= size || y >= size || weight <= 0.0 {
            return;
        }
        let tile = ((y / crate::TILE) * self.out.tiles_across() + x / crate::TILE) as usize;
        let at = ((y % crate::TILE) * crate::TILE + x % crate::TILE) as usize;
        let cover = self
            .coverage
            .entry(tile)
            .or_insert_with(|| vec![0.0; (crate::TILE * crate::TILE) as usize]);
        let w = weight.min(1.0);
        if w <= cover[at] {
            return;
        }
        cover[at] = w;
        let amount = w * brush.flow.clamp(0.0, 1.0);
        let base = self.base.pixel(x, y);
        let pixel = match brush.kind {
            BrushKind::Paint => over(base, brush.color, amount),
            BrushKind::Erase => {
                let a = base[3] as f32 * (1.0 - amount);
                [base[0], base[1], base[2], a.round() as u8]
            }
        };
        self.out.set_pixel(x, y, pixel);
        self.touched.insert(tile);
    }
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

/// Paints one dab of `brush` on `canvas`.
pub fn paint_dab(canvas: &mut StrokeCanvas, target: &Target<'_>, brush: &Brush, dab: &Dab) {
    let size = canvas.image().size() as f32;
    let Some(inverse) = Some(target.model.inverse()).filter(|m| m.is_finite()) else {
        return;
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
        // The triangle in texel space (row 0 at the top, where V is 1).
        let texel = t.uvs.map(|uv| Vec2::new(uv.x * size, (1.0 - uv.y) * size));
        let lo = texel[0].min(texel[1]).min(texel[2]) - Vec2::splat(EDGE_REACH);
        let hi = texel[0].max(texel[1]).max(texel[2]) + Vec2::splat(EDGE_REACH);
        // Only the part the dab can reach.
        let (lo, hi) = match reach(&world, &texel, dab) {
            Some((a, b)) => (lo.max(a), hi.min(b)),
            None => (lo, hi),
        };
        let lo = lo.max(Vec2::ZERO);
        let hi = hi.min(Vec2::splat(size - 1.0));
        if lo.x > hi.x || lo.y > hi.y || !lo.is_finite() || !hi.is_finite() {
            continue;
        }
        let edges = edge_lines(&texel);
        for y in lo.y.floor() as u32..=hi.y.ceil() as u32 {
            for x in lo.x.floor() as u32..=hi.x.ceil() as u32 {
                let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                if !within(&edges, p) {
                    continue;
                }
                let Some(w) = barycentric(p, texel) else { continue };
                let w = clamp_weights(w);
                let at = world[0] * w[0] + world[1] * w[1] + world[2] * w[2];
                let Some((screen, depth)) = target.view.screen(at) else {
                    continue;
                };
                let mut weight = brush.falloff(screen.distance(dab.screen), dab.radius);
                if weight <= 0.0 {
                    continue;
                }
                if !brush.through {
                    let facing = normal.dot(target.view.toward_camera(at));
                    if facing <= 0.0 {
                        continue;
                    }
                    // Surfaces seen edge on take less.
                    weight *= (facing / 0.2).min(1.0);
                    if let Some(map) = target.depth
                        && !map.visible(screen, depth)
                    {
                        continue;
                    }
                }
                canvas.cover(x, y, weight, brush);
            }
        }
    }
}

/// The texel box the dab can reach on a triangle, from its middle's place
/// on the triangle and how many texels a world unit spans there.
fn reach(world: &[Vec3; 3], texel: &[Vec2; 3], dab: &Dab) -> Option<(Vec2, Vec2)> {
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
    let r = dab.world_radius * texels_per_unit * 1.2 + 2.0;
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
