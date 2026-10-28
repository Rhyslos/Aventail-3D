//! "Why does this look wrong?" ([decision 0203](../../../docs/decisions/0203-why-does-this-look-wrong.md)):
//! the usual reasons a painted model looks off, found on one object's
//! surface and its material's mixed texture. Each finding names the
//! polygons concerned, so the app can show them and explain.

use crate::image::Image;
use crate::surface::PaintSurface;
use glam::{Vec2, Vec3};
use std::collections::{BTreeSet, HashMap};

/// What looks wrong, with the polygons (face numbers) it's on.
#[derive(Debug, Clone, PartialEq)]
pub enum Finding {
    /// Painted lines or colours break where the UVs are cut (seams): the
    /// two sides of the cut were painted differently. The polygons on
    /// either side, and how many seam edges.
    Seams {
        /// Polygons beside the broken seams.
        faces: Vec<u32>,
        /// How many seam edges break.
        edges: usize,
    },
    /// Pixels stretched one way: the UVs squash the polygons, so the
    /// texture's pixels look like streaks. How much, at worst.
    Stretched {
        /// The stretched polygons.
        faces: Vec<u32>,
        /// The worst stretch: how many times longer one way than the other.
        worst: f32,
    },
    /// Too few pixels for the size: the texture looks soft or blocky.
    Blurry {
        /// The polygons with too few pixels.
        faces: Vec<u32>,
        /// Their pixels per metre, on average.
        pixels_per_metre: f32,
        /// The pixels per metre aimed for.
        aim: f32,
    },
    /// The painting has hardly any light and dark: it looks flat.
    FlatLight {
        /// How far apart its lights and darks are (0 to 1 lightness).
        spread: f32,
    },
}

/// How differently the two sides of a seam may be painted (0 to 1 of the
/// colour range) before it shows.
const SEAM_DIFFERENCE: f32 = 0.16;
/// How many times longer one way than the other a polygon's pixels may
/// be before it shows.
const STRETCH: f32 = 2.0;
/// The share of the aimed-for pixels per metre below which it looks soft.
const BLURRY: f32 = 0.5;
/// The least spread of lightness (between the darkest and lightest tenth
/// of the painting) for it to read as lit.
const FLAT: f32 = 0.12;

/// The findings on `surface`'s polygons in material `slots`, painted with
/// `image` (the material's mixed texture), placed at `scale` (its world
/// size per unit), with `aim` pixels per metre aimed for. Most important
/// first: seams, blurry, stretched, flat light.
pub fn look_for(
    surface: &PaintSurface,
    slots: &[u32],
    image: &Image,
    scale: f32,
    aim: f32,
) -> Vec<Finding> {
    let mut out = Vec::new();
    let size = image.size() as f32;
    let triangles: Vec<usize> = (0..surface.triangles().len())
        .filter(|i| slots.contains(&surface.triangles()[*i].slot))
        .collect();
    if triangles.is_empty() {
        return out;
    }
    if let Some(seams) = seams(surface, &triangles, image) {
        out.push(seams);
    }
    // Pixels per metre and stretch, triangle by triangle.
    let mut blurry = BTreeSet::new();
    let mut blurry_area = 0.0;
    let mut blurry_density = 0.0;
    let mut stretched = BTreeSet::new();
    let mut worst: f32 = 1.0;
    for &i in &triangles {
        let t = &surface.triangles()[i];
        let Some((most, least)) = stretches(t.points.map(|p| p * scale), t.uvs) else {
            continue;
        };
        // Texture widths per centimetre, as pixels per metre.
        let density = (most * least).sqrt() * size * 100.0;
        let area = triangle_area(t.points.map(|p| p * scale));
        if aim > 0.0 && density < aim * BLURRY {
            blurry.insert(t.face);
            blurry_area += area;
            blurry_density += density * area;
        }
        let ratio = most / least.max(most * 1e-6);
        if ratio > STRETCH {
            stretched.insert(t.face);
            worst = worst.max(ratio);
        }
    }
    if !blurry.is_empty() && blurry_area > 0.0 {
        out.push(Finding::Blurry {
            faces: blurry.into_iter().collect(),
            pixels_per_metre: blurry_density / blurry_area,
            aim,
        });
    }
    if !stretched.is_empty() {
        out.push(Finding::Stretched {
            faces: stretched.into_iter().collect(),
            worst,
        });
    }
    if let Some(spread) = light_spread(surface, &triangles, image)
        && spread < FLAT
    {
        out.push(Finding::FlatLight { spread });
    }
    out
}

/// An edge's two ends, rounded to a hundredth of a unit.
type EdgePlace = ([i64; 3], [i64; 3]);

/// Seams painted differently on their two sides.
fn seams(surface: &PaintSurface, triangles: &[usize], image: &Image) -> Option<Finding> {
    let size = image.size() as f32;
    let key = |p: Vec3| [p.x, p.y, p.z].map(|v| (v * 100.0).round() as i64);
    // Each edge's place, and the triangles and UVs on it.
    let mut edges: HashMap<EdgePlace, Vec<(usize, Vec2, Vec2)>> = HashMap::new();
    for &i in triangles {
        let t = &surface.triangles()[i];
        for k in 0..3 {
            let (a, b) = (k, (k + 1) % 3);
            let (ka, kb) = (key(t.points[a]), key(t.points[b]));
            let (ka, kb, ua, ub) = if ka <= kb {
                (ka, kb, t.uvs[a], t.uvs[b])
            } else {
                (kb, ka, t.uvs[b], t.uvs[a])
            };
            edges.entry((ka, kb)).or_default().push((i, ua, ub));
        }
    }
    let mut faces = BTreeSet::new();
    let mut broken = 0;
    for sides in edges.values() {
        let [(i, ua, ub), (j, va, vb)] = sides.as_slice() else {
            continue;
        };
        let near = |a: Vec2, b: Vec2| a.distance(b) * size < 0.5;
        if near(*ua, *va) && near(*ub, *vb) {
            continue; // not cut: one piece on both sides
        }
        let centre = |n: usize| {
            let t = &surface.triangles()[n];
            (t.uvs[0] + t.uvs[1] + t.uvs[2]) / 3.0
        };
        let (ci, cj) = (centre(*i), centre(*j));
        let mut differ = 0;
        let mut compared = 0;
        for f in [0.25, 0.5, 0.75] {
            // A texel and a half in from the edge on each side.
            let inward = |a: Vec2, b: Vec2, c: Vec2| {
                let on = a.lerp(b, f);
                on + (c - on).normalize_or_zero() * (1.5 / size)
            };
            let x = sample(image, inward(*ua, *ub, ci));
            let y = sample(image, inward(*va, *vb, cj));
            if x[3] < 128 || y[3] < 128 {
                continue;
            }
            compared += 1;
            let d = (0..3)
                .map(|c| (f32::from(x[c]) - f32::from(y[c])).abs())
                .sum::<f32>()
                / (3.0 * 255.0);
            if d > SEAM_DIFFERENCE {
                differ += 1;
            }
        }
        if compared >= 2 && differ * 2 > compared {
            broken += 1;
            faces.insert(surface.triangles()[*i].face);
            faces.insert(surface.triangles()[*j].face);
        }
    }
    (broken > 0).then(|| Finding::Seams {
        faces: faces.into_iter().collect(),
        edges: broken,
    })
}

/// How far apart the painting's lights and darks are: the lightness of
/// its lightest tenth less its darkest tenth, sampled across the
/// triangles; `None` with too little painted to judge.
fn light_spread(surface: &PaintSurface, triangles: &[usize], image: &Image) -> Option<f32> {
    let mut values = Vec::new();
    for &i in triangles {
        let t = &surface.triangles()[i];
        for w in [
            [1.0 / 3.0; 3],
            [0.6, 0.2, 0.2],
            [0.2, 0.6, 0.2],
            [0.2, 0.2, 0.6],
        ] {
            let uv = t.uvs[0] * w[0] + t.uvs[1] * w[1] + t.uvs[2] * w[2];
            let c = sample(image, uv);
            if c[3] >= 128 {
                values.push(
                    (0.2126 * f32::from(c[0])
                        + 0.7152 * f32::from(c[1])
                        + 0.0722 * f32::from(c[2]))
                        / 255.0,
                );
            }
        }
    }
    if values.len() < 16 {
        return None;
    }
    values.sort_by(f32::total_cmp);
    let at = |q: f32| values[((values.len() - 1) as f32 * q).round() as usize];
    Some(at(0.9) - at(0.1))
}

/// The texel at `uv` (where the texture repeats).
fn sample(image: &Image, uv: Vec2) -> [u8; 4] {
    let size = image.size();
    let s = size as f32;
    let x = (uv.x.rem_euclid(1.0) * s).floor().clamp(0.0, s - 1.0) as u32;
    let y = ((1.0 - uv.y.rem_euclid(1.0)) * s)
        .floor()
        .clamp(0.0, s - 1.0) as u32;
    image.pixel(x.min(size - 1), y.min(size - 1))
}

fn triangle_area(p: [Vec3; 3]) -> f32 {
    (p[1] - p[0]).cross(p[2] - p[0]).length() / 2.0
}

/// How much one unit of surface stretches on the texture, most and least
/// (texture widths per unit of length), for a triangle at `p` with UVs
/// `uv`; `None` for one with no area.
fn stretches(p: [Vec3; 3], uv: [Vec2; 3]) -> Option<(f32, f32)> {
    let e1 = p[1] - p[0];
    let e2 = p[2] - p[0];
    let across = e1.cross(e2);
    if across.length() < 1e-12 || e1.length() < 1e-9 {
        return None;
    }
    let x = e1 / e1.length();
    let y = across.normalize().cross(x);
    let a = Vec2::new(e1.length(), 0.0);
    let b = Vec2::new(e2.dot(x), e2.dot(y));
    let det = a.x * b.y - b.x * a.y;
    if det.abs() < 1e-12 {
        return None;
    }
    let (u1, u2) = (uv[1] - uv[0], uv[2] - uv[0]);
    let inv = [Vec2::new(b.y, -a.y) / det, Vec2::new(-b.x, a.x) / det];
    let j0 = u1 * inv[0].x + u2 * inv[0].y;
    let j1 = u1 * inv[1].x + u2 * inv[1].y;
    let (aa, ab, bb) = (j0.dot(j0), j0.dot(j1), j1.dot(j1));
    let trace = aa + bb;
    let d = aa * bb - ab * ab;
    let spread = (trace * trace / 4.0 - d).max(0.0).sqrt();
    let most = (trace / 2.0 + spread).max(0.0).sqrt();
    let least = (trace / 2.0 - spread).max(0.0).sqrt();
    (most > 0.0 && most.is_finite()).then_some((most, least))
}

#[cfg(test)]
mod tests;
