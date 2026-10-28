//! Paint that follows the model ([decision 0189](../../../docs/decisions/0189-paint-follows-the-model.md)):
//! when modelling or UV work changes where a shape's polygons lie on the
//! texture, the paint moves with them.
//!
//! The texture is painted through the UVs, so paint belongs to the
//! polygons. After a change, every triangle whose UVs moved (or that is
//! new) takes its texels from where the same place on the shape was
//! before:
//! - a triangle of a polygon that is still there, cut into as many
//!   triangles as before, takes them from the same triangle of that
//!   polygon, point for point: the paint stays on the polygon however
//!   its UVs moved or the polygon was reshaped;
//! - any other (a new polygon, or one cut differently) takes them from
//!   the nearest point of the shape as it was.
//!
//! Texels of triangles whose UVs didn't change are left exactly as they
//! are, so paint doesn't soften edit after edit.

use crate::image::Image;
use crate::set::{Content, Layer, TextureSet};
use crate::surface::{PaintSurface, barycentric};
use glam::{Vec2, Vec3};
use std::collections::HashMap;

/// One object's shape as the paint lies on it: its id, its surface, and
/// which of its material slots use the texture set.
#[derive(Debug, Clone, Copy)]
pub struct Shape<'a> {
    /// The object's id (any number that stays the same for the object).
    pub id: u64,
    /// Its shape.
    pub surface: &'a PaintSurface,
    /// Its slots whose material owns the texture set.
    pub slots: &'a [u32],
}

/// A number standing for where `shapes`' polygons lie on the texture:
/// the same when nothing that matters to the paint changed.
pub fn layout_key(shapes: &[Shape<'_>]) -> u64 {
    // FNV-1a: small, and the same on every run.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut add = |value: u64| {
        for byte in value.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    };
    for shape in shapes {
        add(shape.id);
        for t in shape.surface.triangles() {
            if !shape.slots.contains(&t.slot) {
                continue;
            }
            add(u64::from(t.face));
            for uv in t.uvs {
                add(u64::from(uv.x.to_bits()) << 32 | u64::from(uv.y.to_bits()));
            }
        }
    }
    hash
}

/// How far outside a triangle (in texels) its texels still reach, as
/// when painting, so seams stay covered.
const EDGE_REACH: f32 = 0.75;

/// `set` with its paint moved from `before` to `after` (the same
/// objects' shapes before and after a change), or `None` when no
/// triangle's UVs changed. Every paint layer and mask moves; fill layers
/// and the strokes kept with layers stay as they are.
pub fn follow(set: &TextureSet, before: &[Shape<'_>], after: &[Shape<'_>]) -> Option<TextureSet> {
    let size = set.size as f32;
    let mut out = set.clone();
    let old = images(&set.layers);
    let mut moved = false;
    {
        let mut new = images_mut(&mut out.layers);
        for shape in after {
            let Some(was) = before.iter().find(|b| b.id == shape.id) else {
                continue;
            };
            // The old triangles of each polygon, in order.
            let mut by_face: HashMap<u32, Vec<usize>> = HashMap::new();
            for (i, t) in was.surface.triangles().iter().enumerate() {
                if was.slots.contains(&t.slot) {
                    by_face.entry(t.face).or_default().push(i);
                }
            }
            let mut now_by_face: HashMap<u32, Vec<usize>> = HashMap::new();
            for (i, t) in shape.surface.triangles().iter().enumerate() {
                if shape.slots.contains(&t.slot) {
                    now_by_face.entry(t.face).or_default().push(i);
                }
            }
            for (face, now) in &now_by_face {
                let old_tris = by_face.get(face).filter(|o| o.len() == now.len());
                for (k, &i) in now.iter().enumerate() {
                    let t = &shape.surface.triangles()[i];
                    let same = old_tris.map(|o| &was.surface.triangles()[o[k]]);
                    if same.is_some_and(|o| o.uvs == t.uvs) {
                        continue;
                    }
                    moved |= move_triangle(&old, &mut new, size, t, same, was);
                }
            }
        }
    }
    moved.then_some(out)
}

/// Fills the texels of triangle `t` in every image of `new` from where
/// they were in `old`: from `same` (the same triangle before) point for
/// point, or else from the nearest place on `was`. Whether any texel was
/// filled.
fn move_triangle(
    old: &[&Image],
    new: &mut [&mut Image],
    size: f32,
    t: &crate::surface::PaintTriangle,
    same: Option<&crate::surface::PaintTriangle>,
    was: &Shape<'_>,
) -> bool {
    let texel = t.uvs.map(|uv| Vec2::new(uv.x * size, (1.0 - uv.y) * size));
    let lo = (texel[0].min(texel[1]).min(texel[2]) - Vec2::splat(EDGE_REACH)).max(Vec2::ZERO);
    let hi = (texel[0].max(texel[1]).max(texel[2]) + Vec2::splat(EDGE_REACH))
        .min(Vec2::splat(size - 1.0));
    if lo.x > hi.x || lo.y > hi.y || !lo.is_finite() || !hi.is_finite() {
        return false;
    }
    let edges = edge_lines(&texel);
    // Where a place in this triangle (in texels) was on the old texture
    // (in texels, middles at whole numbers).
    let from = |p: Vec2| -> Option<Vec2> {
        let w = clamp_weights(barycentric(p, texel)?);
        let uv = match same {
            Some(o) => o.uvs[0] * w[0] + o.uvs[1] * w[1] + o.uvs[2] * w[2],
            None => {
                let at: Vec3 = t.points[0] * w[0] + t.points[1] * w[1] + t.points[2] * w[2];
                let (i, v) = was.surface.closest(at, was.slots)?;
                let o = &was.surface.triangles()[i];
                o.uvs[0] * v[0] + o.uvs[1] * v[1] + o.uvs[2] * v[2]
            }
        };
        Some(Vec2::new(uv.x * size - 0.5, (1.0 - uv.y) * size - 0.5))
    };
    // A polygon made smaller on the texture (packed tighter) covers more
    // old texels with each new one: those are averaged over, a few
    // samples across each way, rather than one picked.
    let across = same.map_or(1, |o| {
        let area = |uvs: &[Vec2; 3]| ((uvs[1] - uvs[0]).perp_dot(uvs[2] - uvs[0])).abs();
        let shrink = (area(&o.uvs) / area(&t.uvs).max(1e-12)).sqrt();
        if shrink.is_finite() {
            (shrink.ceil() as usize).clamp(1, MAX_SAMPLES_ACROSS)
        } else {
            1
        }
    });
    let mut any = false;
    for y in lo.y.floor() as u32..=hi.y.ceil() as u32 {
        for x in lo.x.floor() as u32..=hi.x.ceil() as u32 {
            let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            if !edges
                .iter()
                .all(|(on, inside)| (p - *on).dot(*inside) >= -EDGE_REACH)
            {
                continue;
            }
            let mut places = [Vec2::ZERO; MAX_SAMPLES_ACROSS * MAX_SAMPLES_ACROSS];
            let mut count = 0;
            let step = 1.0 / across as f32;
            for k in 0..across * across {
                let sub = if across == 1 {
                    p
                } else {
                    Vec2::new(
                        x as f32 + ((k % across) as f32 + 0.5) * step,
                        y as f32 + ((k / across) as f32 + 0.5) * step,
                    )
                };
                if let Some(at) = from(sub) {
                    places[count] = at;
                    count += 1;
                }
            }
            if count == 0 {
                continue;
            }
            let places = &places[..count];
            for (old, new) in old.iter().zip(new.iter_mut()) {
                let colour = average(places.iter().map(|at| sample(old, *at)));
                if new.pixel(x, y) != colour {
                    new.set_pixel(x, y, colour);
                }
            }
            any = true;
        }
    }
    any
}

/// The most samples taken across a texel each way, when a polygon was
/// made smaller on the texture.
const MAX_SAMPLES_ACROSS: usize = 4;

/// The average of `colours`, weighted by their coverage.
fn average(colours: impl Iterator<Item = [u8; 4]>) -> [u8; 4] {
    let mut sum = [0.0f32; 4];
    let mut count = 0.0f32;
    for c in colours {
        let a = c[3] as f32 / 255.0;
        for k in 0..3 {
            sum[k] += c[k] as f32 * a;
        }
        sum[3] += a;
        count += 1.0;
    }
    if sum[3] <= 1e-6 || count <= 0.0 {
        return [0; 4];
    }
    [
        (sum[0] / sum[3]).round().clamp(0.0, 255.0) as u8,
        (sum[1] / sum[3]).round().clamp(0.0, 255.0) as u8,
        (sum[2] / sum[3]).round().clamp(0.0, 255.0) as u8,
        (sum[3] / count * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

/// `image` at texel place `at` (texel middles at whole numbers), mixed
/// from the four texels around it with their coverage, so empty texels
/// don't darken the edge of paint.
fn sample(image: &Image, at: Vec2) -> [u8; 4] {
    let max = image.size() as f32 - 1.0;
    let at = at.clamp(Vec2::ZERO, Vec2::splat(max));
    let (x0, y0) = (at.x.floor(), at.y.floor());
    let (fx, fy) = (at.x - x0, at.y - y0);
    let (x0, y0) = (x0 as u32, y0 as u32);
    let (x1, y1) = ((x0 + 1).min(max as u32), (y0 + 1).min(max as u32));
    let mut sum = [0.0f32; 4];
    for (x, y, k) in [
        (x0, y0, (1.0 - fx) * (1.0 - fy)),
        (x1, y0, fx * (1.0 - fy)),
        (x0, y1, (1.0 - fx) * fy),
        (x1, y1, fx * fy),
    ] {
        let p = image.pixel(x, y);
        let a = p[3] as f32 / 255.0 * k;
        for c in 0..3 {
            sum[c] += p[c] as f32 * a;
        }
        sum[3] += a;
    }
    if sum[3] <= 1e-6 {
        return [0; 4];
    }
    [
        (sum[0] / sum[3]).round().clamp(0.0, 255.0) as u8,
        (sum[1] / sum[3]).round().clamp(0.0, 255.0) as u8,
        (sum[2] / sum[3]).round().clamp(0.0, 255.0) as u8,
        (sum[3] * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

/// Every paint layer's and mask's image in `layers`, groups included, in
/// a set order.
fn images(layers: &[Layer]) -> Vec<&Image> {
    let mut out = Vec::new();
    for layer in layers {
        match &layer.content {
            Content::Paint(image) => out.push(image),
            Content::Group(children) => out.extend(images(children)),
            Content::Fill(_) => {}
        }
        if let Some(mask) = &layer.mask {
            out.push(&mask.image);
        }
    }
    out
}

/// [`images`], to change, in the same order.
fn images_mut(layers: &mut [Layer]) -> Vec<&mut Image> {
    let mut out = Vec::new();
    for layer in layers {
        let Layer { content, mask, .. } = layer;
        match content {
            Content::Paint(image) => out.push(image),
            Content::Group(children) => out.extend(images_mut(children)),
            Content::Fill(_) => {}
        }
        if let Some(mask) = mask {
            out.push(&mut mask.image);
        }
    }
    out
}

fn edge_lines(t: &[Vec2; 3]) -> [(Vec2, Vec2); 3] {
    let area = (t[1] - t[0]).perp_dot(t[2] - t[0]);
    let sign = if area >= 0.0 { 1.0 } else { -1.0 };
    [(0, 1), (1, 2), (2, 0)].map(|(a, b)| {
        let e = t[b] - t[a];
        (t[a], Vec2::new(-e.y, e.x).normalize_or_zero() * sign)
    })
}

fn clamp_weights(w: [f32; 3]) -> [f32; 3] {
    let w = w.map(|v| v.max(0.0));
    let sum = w[0] + w[1] + w[2];
    if sum <= 0.0 {
        return [1.0 / 3.0; 3];
    }
    w.map(|v| v / sum)
}

#[cfg(test)]
mod tests;
