//! The polygon fill tool ([decision 0182](../../../docs/decisions/0182-masks-and-polygon-fill.md)):
//! filling whole parts of a shape at once, by polygon, UV piece or
//! material, as Substance Painter's polygon fill does, instead of
//! painting them with the brush.
//!
//! The parts are found among a [`PaintSurface`]'s triangles and filled
//! fully in texture space, with the same thin rim outside their edges as
//! the brush paints, so neighbouring parts meet without a gap.

use crate::brush::Brush;
use crate::stroke::StrokeCanvas;
use crate::surface::PaintSurface;
use glam::Vec2;
use std::collections::HashMap;

/// How far outside a triangle's edge (in texels) a texel is still filled.
const EDGE_REACH: f32 = 0.75;

/// What one click of the polygon fill tool fills.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum FillBy {
    /// The polygon clicked.
    #[default]
    Polygon,
    /// The whole UV piece holding it.
    Piece,
    /// Every polygon with its material.
    Material,
}

impl FillBy {
    /// Every way, in the order Attributes lists them.
    pub const ALL: [FillBy; 3] = [Self::Polygon, Self::Piece, Self::Material];
}

/// The triangles of `surface` that a click on triangle `at` fills `by`.
/// Only triangles whose slot is one of `slots` are filled.
pub fn fill_area(surface: &PaintSurface, at: usize, by: FillBy, slots: &[u32]) -> Vec<usize> {
    let triangles = surface.triangles();
    let Some(start) = triangles.get(at) else {
        return Vec::new();
    };
    let mut out: Vec<usize> = match by {
        FillBy::Polygon => (0..triangles.len())
            .filter(|i| triangles[*i].face == start.face)
            .collect(),
        FillBy::Material => (0..triangles.len())
            .filter(|i| slots.contains(&triangles[*i].slot))
            .collect(),
        FillBy::Piece => {
            let pieces = pieces(surface);
            (0..triangles.len())
                .filter(|i| pieces[*i] == pieces[at])
                .collect()
        }
    };
    out.retain(|i| slots.contains(&triangles[*i].slot));
    out
}

/// Each triangle's UV piece, as a number: triangles sharing an edge whose
/// corners match in place and in UVs are in the same piece.
pub fn pieces(surface: &PaintSurface) -> Vec<usize> {
    let triangles = surface.triangles();
    let mut parent: Vec<usize> = (0..triangles.len()).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    type Corner = ([u32; 3], [u32; 2]);
    let corner = |t: usize, k: usize| -> Corner {
        let tri = &triangles[t];
        (
            tri.points[k].to_array().map(f32::to_bits),
            tri.uvs[k].to_array().map(f32::to_bits),
        )
    };
    let mut edges: HashMap<(Corner, Corner), usize> = HashMap::new();
    for t in 0..triangles.len() {
        for (a, b) in [(0, 1), (1, 2), (2, 0)] {
            let (p, q) = (corner(t, a), corner(t, b));
            let key = if p <= q { (p, q) } else { (q, p) };
            match edges.get(&key) {
                Some(&other) => {
                    let (x, y) = (root(&mut parent, t), root(&mut parent, other));
                    parent[x] = y;
                }
                None => {
                    edges.insert(key, t);
                }
            }
        }
    }
    (0..triangles.len()).map(|t| root(&mut parent, t)).collect()
}

/// Fills `triangles` of `surface` fully with `brush` (its colour, or
/// taking paint away for the eraser).
pub fn fill_triangles(
    canvas: &mut StrokeCanvas,
    surface: &PaintSurface,
    triangles: &[usize],
    brush: &Brush,
) {
    let size = canvas.image().size() as f32;
    for &i in triangles {
        let Some(t) = surface.triangles().get(i) else {
            continue;
        };
        let texel = t.uvs.map(|uv| Vec2::new(uv.x * size, (1.0 - uv.y) * size));
        let lo = (texel[0].min(texel[1]).min(texel[2]) - Vec2::splat(EDGE_REACH)).max(Vec2::ZERO);
        let hi = (texel[0].max(texel[1]).max(texel[2]) + Vec2::splat(EDGE_REACH))
            .min(Vec2::splat(size - 1.0));
        if lo.x > hi.x || lo.y > hi.y || !lo.is_finite() || !hi.is_finite() {
            continue;
        }
        let area = (texel[1] - texel[0]).perp_dot(texel[2] - texel[0]);
        let edges: Vec<(Vec2, Vec2)> = if area.abs() < 1e-9 {
            // No area: just the texels along it.
            Vec::new()
        } else {
            let sign = area.signum();
            [(0, 1), (1, 2), (2, 0)]
                .iter()
                .map(|&(a, b)| {
                    let e = texel[b] - texel[a];
                    (texel[a], Vec2::new(-e.y, e.x).normalize_or_zero() * sign)
                })
                .collect()
        };
        if edges.is_empty() {
            continue;
        }
        for y in lo.y.floor() as u32..=hi.y.ceil() as u32 {
            for x in lo.x.floor() as u32..=hi.x.ceil() as u32 {
                let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                if edges
                    .iter()
                    .all(|(on, inside)| (p - *on).dot(*inside) >= -EDGE_REACH)
                {
                    canvas.cover(x, y, 1.0, brush);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::Image;
    use mesh::PrimitiveKind;

    fn cube() -> PaintSurface {
        PaintSurface::of(&PrimitiveKind::Cube.standard().mesh())
    }

    #[test]
    fn a_polygon_is_its_triangles_and_a_piece_or_material_more() {
        let surface = cube();
        let polygon = fill_area(&surface, 0, FillBy::Polygon, &[0]);
        assert_eq!(polygon.len(), 2, "a quad is two triangles");
        let piece = fill_area(&surface, 0, FillBy::Piece, &[0]);
        assert!(piece.len() >= polygon.len());
        assert!(polygon.iter().all(|t| piece.contains(t)));
        let material = fill_area(&surface, 0, FillBy::Material, &[0]);
        assert_eq!(material.len(), surface.triangles().len());
        // Other slots are left alone, and a click off the shape fills nothing.
        assert!(fill_area(&surface, 0, FillBy::Material, &[3]).is_empty());
        assert!(fill_area(&surface, 9999, FillBy::Polygon, &[0]).is_empty());
    }

    #[test]
    fn pieces_join_across_shared_uv_edges_only() {
        let surface = cube();
        let pieces = pieces(&surface);
        let mut distinct = pieces.clone();
        distinct.sort_unstable();
        distinct.dedup();
        // The standard cube's six sides are laid out apart (six pieces).
        assert_eq!(distinct.len(), 6);
    }

    #[test]
    fn filling_covers_the_polygon_fully_and_nothing_far_from_it() {
        let surface = cube();
        let mut canvas = StrokeCanvas::new(Image::new(256).unwrap());
        let brush = Brush {
            hardness: 0.0,
            ..Brush::default()
        };
        let polygon = fill_area(&surface, 0, FillBy::Polygon, &[0]);
        fill_triangles(&mut canvas, &surface, &polygon, &brush);
        let image = canvas.image();
        let t = surface.triangles()[0];
        let middle = (t.uvs[0] + t.uvs[1] + t.uvs[2]) / 3.0;
        let at = |uv: Vec2| image.pixel((uv.x * 256.0) as u32, ((1.0 - uv.y) * 256.0) as u32);
        assert_eq!(at(middle)[3], 255, "fully covered, even with a soft brush");
        // Another side's middle is untouched.
        let other = surface
            .triangles()
            .iter()
            .find(|o| o.face != t.face)
            .unwrap();
        let far = (other.uvs[0] + other.uvs[1] + other.uvs[2]) / 3.0;
        assert_eq!(at(far)[3], 0);
        // Odd input fills nothing.
        let mut empty = StrokeCanvas::new(Image::new(64).unwrap());
        fill_triangles(&mut empty, &surface, &[12345], &brush);
        assert!(empty.image().is_empty());
    }
}
