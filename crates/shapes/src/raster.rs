//! Turning a drawing into pictures at any size: the filled shape (a sharp
//! mask, smooth along its edges) and its lines.
//!
//! The canvas's texture square becomes the picture, so a drawing comes
//! out the same at 32 pixels or 4096, never blurred by being made larger.
//! **Everything wraps round:** a piece crossing an edge of the square
//! comes back in on the other side, so the picture tiles seamlessly (as
//! every material texture does). Filling is `tiny-skia`'s, with
//! antialiasing; sides crossing over count as filled (non-zero winding).

use crate::drawing::{Drawing, Piece, PieceKind};
use crate::path::Path;
use glam::Vec2;
use tiny_skia::{FillRule, LineCap, LineJoin, Mask, PathBuilder, Stroke, Transform};

mod veins;

pub use veins::{Veins, veins};

/// The largest picture made, in pixels across.
pub const MAX_SIZE: u32 = 8192;

/// The drawing's filled shape at `size` × `size` pixels, row by row from
/// the top: how much of each pixel is inside, 0 to 1. Pieces are laid in
/// order: each Add adds its inside, each Cut takes its inside out of
/// everything under it; lines are left out. Empty for a size of 0 or
/// over [`MAX_SIZE`].
pub fn mask(drawing: &Drawing, size: u32) -> Vec<f32> {
    if !allowed(size) {
        return Vec::new();
    }
    let mut out = vec![0.0f32; (size as usize).pow(2)];
    let filled = fill_pieces(drawing, size, |piece, cover| match piece.kind {
        PieceKind::Add => {
            for (o, c) in out.iter_mut().zip(cover) {
                let c = f32::from(*c) / 255.0;
                *o += c * (1.0 - *o);
            }
        }
        PieceKind::Cut => {
            for (o, c) in out.iter_mut().zip(cover) {
                *o *= 1.0 - f32::from(*c) / 255.0;
            }
        }
        PieceKind::Line => {}
    });
    if filled { out } else { Vec::new() }
}

/// A drawing's pieces as pictures with their colours and heights: what
/// [`layers`] gives.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Layers {
    /// The filled shape, as [`mask`] gives it.
    pub mask: Vec<f32>,
    /// Each piece's colour (shifted when its copies vary) laid over black
    /// in order (each piece covering
    /// the ones under it, cut-outs taking colour away as they take the
    /// shape away): black outside the shape.
    pub colours: Vec<[f32; 3]>,
    /// Each piece's height laid the same way: 0 outside the shape.
    pub heights: Vec<f32>,
}

/// The drawing at `size` × `size` pixels: its mask, and its pieces'
/// colours and heights laid in order over black (the top piece covering
/// those under it). Empty for a size of 0 or over [`MAX_SIZE`].
pub fn layers(drawing: &Drawing, size: u32) -> Layers {
    if !allowed(size) {
        return Layers::default();
    }
    let n = (size as usize).pow(2);
    let mut mask = vec![0.0f32; n];
    let mut colours = vec![[0.0f32; 3]; n];
    let mut heights = vec![0.0f32; n];
    let filled = fill_pieces(drawing, size, |piece, cover| {
        let pixels = mask
            .iter_mut()
            .zip(colours.iter_mut())
            .zip(heights.iter_mut())
            .zip(cover);
        match piece.kind {
            PieceKind::Add => {
                let (colour, height) = (drawing.drawn_colour(piece), piece.height);
                for (((o, rgb), h), c) in pixels {
                    let c = f32::from(*c) / 255.0;
                    *o += c * (1.0 - *o);
                    for (v, p) in rgb.iter_mut().zip(colour) {
                        *v += (p - *v) * c;
                    }
                    *h += (height - *h) * c;
                }
            }
            PieceKind::Cut => {
                for (((o, rgb), h), c) in pixels {
                    let keep = 1.0 - f32::from(*c) / 255.0;
                    *o *= keep;
                    *h *= keep;
                    for v in rgb.iter_mut() {
                        *v *= keep;
                    }
                }
            }
            PieceKind::Line => {}
        }
    });
    if !filled {
        return Layers::default();
    }
    Layers {
        mask,
        colours,
        heights,
    }
}

/// Fills each Add and Cut piece in order (with its mirror image, and a
/// copy wherever it crosses an edge) and hands its coverage to `laid`.
/// False when `size` isn't allowed.
fn fill_pieces(drawing: &Drawing, size: u32, mut laid: impl FnMut(&Piece, &[u8])) -> bool {
    let Some(mut piece) = canvas(size) else {
        return false;
    };
    let n = size as f32;
    for p in drawing.pieces() {
        if p.kind == PieceKind::Line {
            continue;
        }
        piece.clear();
        for path in drawing.drawn_paths(p) {
            if let Some(built) = built(&path) {
                for offset in wraps(&path, 0.0) {
                    let ts = Transform::from_row(n, 0.0, 0.0, n, offset.x * n, offset.y * n);
                    piece.fill_path(&built, FillRule::Winding, true, ts);
                }
            }
        }
        laid(p, piece.data());
    }
    true
}

/// The drawing's lines (its open paths, the Line pieces) at `size` ×
/// `size` pixels, drawn `width` pixels wide with round ends: how much of
/// each pixel they cover, 0 to 1. Empty for a size of 0 or over
/// [`MAX_SIZE`].
pub fn lines(drawing: &Drawing, size: u32, width: f32) -> Vec<f32> {
    let Some(mut all) = canvas(size) else {
        return Vec::new();
    };
    let n = size as f32;
    let width = if width.is_finite() {
        width.clamp(0.5, n)
    } else {
        1.0
    };
    let stroke = Stroke {
        width,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Stroke::default()
    };
    for p in drawing
        .pieces()
        .iter()
        .filter(|p| p.kind == PieceKind::Line)
    {
        for path in drawing.drawn_paths(p) {
            let Some(built) = built(&path) else {
                continue;
            };
            for offset in wraps(&path, width / n) {
                let ts = Transform::from_row(n, 0.0, 0.0, n, offset.x * n, offset.y * n);
                let stroked = built
                    .clone()
                    .transform(ts)
                    .and_then(|placed| placed.stroke(&stroke, 1.0));
                if let Some(stroked) = stroked {
                    all.fill_path(&stroked, FillRule::Winding, true, Transform::identity());
                }
            }
        }
    }
    all.data().iter().map(|c| f32::from(*c) / 255.0).collect()
}

/// Whether a picture `size` pixels square may be made.
fn allowed(size: u32) -> bool {
    size > 0 && size <= MAX_SIZE
}

/// A clear picture `size` pixels square, if that size is allowed.
fn canvas(size: u32) -> Option<Mask> {
    if !allowed(size) {
        return None;
    }
    Mask::new(size, size)
}

/// `path` in `tiny-skia`'s terms, in canvas units; `None` with fewer than
/// two points.
fn built(path: &Path) -> Option<tiny_skia::Path> {
    let knots = path.knots();
    let first = knots.first()?;
    if knots.len() < 2 {
        return None;
    }
    let mut b = PathBuilder::new();
    b.move_to(first.at.x, first.at.y);
    for i in 0..path.sides() {
        let (a, z) = (knots[i], knots[(i + 1) % knots.len()]);
        if a.out == Vec2::ZERO && z.inn == Vec2::ZERO {
            b.line_to(z.at.x, z.at.y);
        } else {
            let (c1, c2) = (a.at + a.out, z.at + z.inn);
            b.cubic_to(c1.x, c1.y, c2.x, c2.y, z.at.x, z.at.y);
        }
    }
    if path.is_closed() {
        b.close();
    }
    b.finish()
}

/// The whole-texture moves that bring a copy of `path` (reaching `pad`
/// further each way) over the texture square: (0, 0) for a path inside
/// it, more for one crossing an edge or lying outside.
fn wraps(path: &Path, pad: f32) -> Vec<Vec2> {
    let Some((lo, hi)) = path.bounds() else {
        return Vec::new();
    };
    let (lo, hi) = (lo - Vec2::splat(pad), hi + Vec2::splat(pad));
    let range = |lo: f32, hi: f32| ((-hi).floor() as i32)..=((1.0 - lo).ceil() as i32);
    let mut out = Vec::new();
    for ky in range(lo.y, hi.y) {
        for kx in range(lo.x, hi.x) {
            let k = Vec2::new(kx as f32, ky as f32);
            let (a, b) = (lo + k, hi + k);
            if a.x < 1.0 && b.x > 0.0 && a.y < 1.0 && b.y > 0.0 {
                out.push(k);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;
