//! Changing a texture set's size ([decision 0191](../../../docs/decisions/0191-texture-size-without-blur.md)):
//! layers painted only with the brush and the polygon fill are painted
//! again at the new size from their kept strokes, crisp; anything else
//! is resized as a picture (averaged when smaller, smoothly enlarged when
//! bigger).
//!
//! Whether a layer can be painted again is found out by doing it: its
//! strokes are painted again at the old size first, and only if that
//! comes out as the layer is now (nothing imported, moved, merged or
//! blurred since) are they trusted at the new size.

use crate::brush::{Brush, BrushKind};
use crate::follow::Shape;
use crate::image::Image;
use crate::set::{Content, DabRecord, Layer, StrokeRecord, TextureSet};
use crate::stroke::{Dab, DabJob, StrokeCanvas, Target, View};
use glam::{Mat4, Vec2, Vec3};
use std::sync::Arc;

/// `set` at `size` pixels across, painted on `shapes` (the objects using
/// its material): each paint layer and mask painted again from its
/// strokes where that gives what it shows now, otherwise resized as a
/// picture. `None` for a size out of range.
pub fn resized(set: &TextureSet, size: u32, shapes: &[Shape<'_>]) -> Option<TextureSet> {
    let mut out = TextureSet::new(size)?;
    out.layers = set
        .layers
        .iter()
        .map(|layer| resize_layer(layer, size, shapes))
        .collect::<Option<Vec<_>>>()?;
    Some(out)
}

fn resize_layer(layer: &Layer, size: u32, shapes: &[Shape<'_>]) -> Option<Layer> {
    let mut out = layer.clone();
    match &layer.content {
        Content::Paint(image) => {
            out.content = Content::Paint(resize_image(image, &layer.strokes, size, shapes)?);
        }
        Content::Group(children) => {
            out.content = Content::Group(
                children
                    .iter()
                    .map(|c| resize_layer(c, size, shapes))
                    .collect::<Option<Vec<_>>>()?,
            );
        }
        Content::Fill(_) => {}
    }
    if let (Some(mask), Some(was)) = (&mut out.mask, &layer.mask) {
        mask.image = resize_image(&was.image, &was.strokes, size, shapes)?;
    }
    Some(out)
}

/// `image` at `size`: painted again from `strokes` if they make it,
/// else resized as a picture.
fn resize_image(
    image: &Image,
    strokes: &[Arc<StrokeRecord>],
    size: u32,
    shapes: &[Shape<'_>],
) -> Option<Image> {
    if image.size() == size {
        return Some(image.clone());
    }
    if let Some(again) = repaint_if_same(image, strokes, size, shapes) {
        return Some(again);
    }
    resample(image, size)
}

/// `strokes` painted again at `size`, if painting them again at the
/// image's own size gives the image.
fn repaint_if_same(
    image: &Image,
    strokes: &[Arc<StrokeRecord>],
    size: u32,
    shapes: &[Shape<'_>],
) -> Option<Image> {
    // Dabs are in one object's own space: with several objects sharing
    // the material, which one is unknown.
    let [shape] = shapes else {
        return None;
    };
    if strokes.is_empty() || image.is_empty() {
        return None;
    }
    let again = repaint(strokes, image.size(), shape)?;
    if !alike(&again, image) {
        return None;
    }
    repaint(strokes, size, shape)
}

/// `strokes` painted on an empty image `size` pixels across, on `shape`;
/// `None` if one of them can't be painted again (projection, clone,
/// smudge, blur, blend and hue shift depend on more than is kept).
pub fn repaint(strokes: &[Arc<StrokeRecord>], size: u32, shape: &Shape<'_>) -> Option<Image> {
    let mut image = Image::new(size)?;
    for stroke in strokes {
        if !matches!(stroke.kind, BrushKind::Paint | BrushKind::Erase) {
            return None;
        }
        let mut canvas = StrokeCanvas::new(image);
        let brush = Brush {
            kind: stroke.kind,
            color: stroke.color,
            hardness: stroke.hardness,
            flow: stroke.flow,
            through: stroke.through,
            tip: stroke.tip,
            ..Brush::default()
        };
        for fill in &stroke.fills {
            let at = shape
                .surface
                .triangles()
                .iter()
                .position(|t| t.face == fill.face && shape.slots.contains(&t.slot))?;
            let area = crate::fill::fill_area(shape.surface, at, fill.by, shape.slots);
            let full = Brush {
                hardness: 1.0,
                flow: 1.0,
                ..brush
            };
            crate::fill::fill_triangles(&mut canvas, shape.surface, &area, &full);
        }
        for (seed, dab) in stroke.dabs.iter().enumerate() {
            repaint_dab(&mut canvas, shape, brush, dab, seed as u32);
        }
        image = canvas.image().clone();
    }
    Some(image)
}

/// One kept dab painted again.
fn repaint_dab(
    canvas: &mut StrokeCanvas,
    shape: &Shape<'_>,
    brush: Brush,
    dab: &DabRecord,
    seed: u32,
) {
    let size = canvas.image().size() as f32;
    if let Some(uv) = dab.flat_uv() {
        let centre = Vec2::new(uv.x * size, (1.0 - uv.y) * size);
        let radius = dab.radius * size;
        let brush = Brush { radius, ..brush };
        crate::flat::paint_flat_dab_turned(
            canvas,
            shape.surface,
            shape.slots,
            &brush,
            (centre, radius, dab.strength),
            (0.0, seed),
        );
        return;
    }
    // Seen straight on from the way the camera was, one point a unit.
    let reach = (dab.radius * 4.0).max(1.0);
    let view = flat_view(dab.at, dab.toward, reach);
    let Some((screen, _)) = view.screen(dab.at) else {
        return;
    };
    let brush = Brush {
        radius: dab.radius,
        ..brush
    };
    let job = DabJob {
        target: Target {
            surface: shape.surface,
            slots: shape.slots,
            model: Mat4::IDENTITY,
            view: &view,
            depth: None,
        },
        brush,
        dab: Dab {
            screen,
            world: dab.at,
            radius: dab.radius,
            world_radius: dab.radius,
            strength: dab.strength,
            angle: 0.0,
            seed,
        },
    };
    crate::stroke::paint_dabs(canvas, &[job]);
}

/// An orthographic view looking at `at` from along `toward`, one screen
/// point a unit, `reach` units each way.
fn flat_view(at: Vec3, toward: Vec3, reach: f32) -> View {
    let toward = toward.normalize_or_zero();
    let up = if toward.z.abs() < 0.9 {
        Vec3::Z
    } else {
        Vec3::Y
    };
    let eye = at + toward * reach;
    // Across and up on screen, and depth from the eye, each a row.
    let right = up.cross(toward).normalize_or_zero();
    let upward = toward.cross(right);
    let row = |axis: Vec3, scale: f32, from: f32| (axis * scale).extend(from * scale);
    let view_projection = Mat4::from_cols(
        row(right, 1.0 / reach, -at.dot(right)),
        row(upward, 1.0 / reach, -at.dot(upward)),
        row(-toward, 0.5 / reach, eye.dot(toward)),
        glam::Vec4::W,
    )
    .transpose();
    View {
        view_projection,
        size: Vec2::splat(reach * 2.0),
        eye,
        forward: -toward,
        flat: true,
    }
}

/// Whether two images of one size show the same, give or take what
/// painting again from kept numbers changes: nearly every texel within a
/// few levels.
fn alike(a: &Image, b: &Image) -> bool {
    if a.size() != b.size() {
        return false;
    }
    let (mut painted, mut off) = (0usize, 0usize);
    for (ta, tb) in a.tiles().iter().zip(b.tiles()) {
        match (ta, tb) {
            (None, None) => continue,
            (Some(x), Some(y)) if Arc::ptr_eq(x, y) => continue,
            _ => {}
        }
        let empty = [0u8; crate::TILE_BYTES];
        let x = ta.as_deref().map_or(&empty[..], |v| &v[..]);
        let y = tb.as_deref().map_or(&empty[..], |v| &v[..]);
        for (p, q) in x.chunks_exact(4).zip(y.chunks_exact(4)) {
            if p[3] == 0 && q[3] == 0 {
                continue;
            }
            painted += 1;
            let far = (0..4).any(|c| p[c].abs_diff(q[c]) > 24);
            off += usize::from(far);
        }
    }
    off * 100 <= painted
}

/// `image` resized to `size` as a picture: each new texel the average of
/// the old ones it covers when smaller, read smoothly when bigger
/// (weighted by coverage either way).
pub fn resample(image: &Image, size: u32) -> Option<Image> {
    let mut out = Image::new(size)?;
    let ratio = image.size() as f32 / size as f32;
    let across = (ratio.ceil() as u32).clamp(1, 16);
    for index in 0..image.tile_count() {
        // Only where the old image is painted.
        if image.tiles()[index].is_none() {
            continue;
        }
        let (x0, y0, w, h) = image.tile_area(index);
        let lo = (
            (x0 as f32 / ratio).floor() as u32,
            (y0 as f32 / ratio).floor() as u32,
        );
        let hi = (
            (((x0 + w) as f32 / ratio).ceil() as u32).min(size),
            (((y0 + h) as f32 / ratio).ceil() as u32).min(size),
        );
        for y in lo.1..hi.1 {
            for x in lo.0..hi.0 {
                let mut sum = [0.0f32; 4];
                let n = across * across;
                for k in 0..n {
                    let sx = (x as f32 + ((k % across) as f32 + 0.5) / across as f32) * ratio - 0.5;
                    let sy = (y as f32 + ((k / across) as f32 + 0.5) / across as f32) * ratio - 0.5;
                    let p = crate::follow::sample(image, Vec2::new(sx, sy));
                    let a = p[3] as f32 / 255.0;
                    for c in 0..3 {
                        sum[c] += p[c] as f32 * a;
                    }
                    sum[3] += a;
                }
                if sum[3] <= 1e-6 {
                    continue;
                }
                out.set_pixel(
                    x,
                    y,
                    [
                        (sum[0] / sum[3]).round().clamp(0.0, 255.0) as u8,
                        (sum[1] / sum[3]).round().clamp(0.0, 255.0) as u8,
                        (sum[2] / sum[3]).round().clamp(0.0, 255.0) as u8,
                        (sum[3] / n as f32 * 255.0).round().clamp(0.0, 255.0) as u8,
                    ],
                );
            }
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests;
