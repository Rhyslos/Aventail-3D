//! Material layers' pixels ([decision 0210](../../../docs/decisions/0210-material-layers.md)):
//! a tiling pattern (brushed metal, wood, stone, paint) laid on each
//! texel from three sides of the shape (along X, Y and Z), blended by
//! which way the surface faces, so the pattern needs no particular UVs
//! and has no seams where UV pieces meet.

use crate::smart::noise;
use crate::{Source, texels};
use glam::{Vec2, Vec3};
use paint::{Image, MaterialFill, MaterialKind};

/// A material layer as made: its colour, and its roughness and metal as
/// greys (for the realism channels).
#[derive(Debug, Clone, PartialEq)]
pub struct MadeMaterial {
    /// The colour, opaque where the shape's polygons are.
    pub colour: Image,
    /// Roughness, grey.
    pub roughness: Image,
    /// Metal, grey.
    pub metal: Image,
}

/// `fill` made on `sources` (the objects sharing the texture set), on the
/// texels their polygons cover; the rest see-through. `None` for a size
/// out of range.
pub fn material(sources: &[Source<'_>], fill: &MaterialFill) -> Option<MadeMaterial> {
    let size = sources.first().map_or(64, |s| s.size);
    let mut out = MadeMaterial {
        colour: Image::new(size)?,
        roughness: Image::new(size)?,
        metal: Image::new(size)?,
    };
    let metal = grey(fill.metal);
    for source in sources {
        let texels = texels::covered(source.surface, source.slots, size);
        let values = mesh::parallel::in_parallel(texels.len(), |range| {
            texels[range]
                .iter()
                .map(|t| (t.x, t.y, at(fill, t.at, t.normal)))
                .collect::<Vec<_>>()
        });
        for part in values {
            for (x, y, (v, rough)) in part {
                out.colour.set_pixel(x, y, mix(fill.colour, fill.second, v));
                out.roughness.set_pixel(x, y, grey(rough));
                out.metal.set_pixel(x, y, metal);
            }
        }
    }
    Some(out)
}

/// The pattern at `p` (shape space, centimetres) on a surface facing
/// `normal`: how far toward the second colour (0 to 1), and the
/// roughness there.
fn at(fill: &MaterialFill, p: Vec3, normal: Vec3) -> (f32, f32) {
    let q = p / fill.scale.max(0.001);
    // Each side's weight: sharper blends raise the facing to a higher
    // power, so one side wins sooner.
    let power = 1.0 + fill.sharpness.clamp(0.0, 1.0) * 15.0;
    let w = normal.abs().powf(power);
    let total = (w.x + w.y + w.z).max(1e-6);
    let sides = [
        (Vec2::new(q.y, q.z), w.x / total, 0),
        (Vec2::new(q.x, q.z), w.y / total, 1),
        (Vec2::new(q.x, q.y), w.z / total, 2),
    ];
    let mut v = 0.0;
    for (uv, weight, side) in sides {
        if weight > 1e-4 {
            v += weight * pattern(fill.kind, uv, fill.seed.wrapping_add(side * 7919));
        }
    }
    let v = v.clamp(0.0, 1.0);
    // The pattern roughens or smooths a little around the setting.
    let rough = (fill.roughness + (v - 0.5) * 0.15).clamp(0.0, 1.0);
    (v, rough)
}

/// One side's pattern of `kind` at `q` (repeats), 0 to 1.
fn pattern(kind: MaterialKind, q: Vec2, seed: u32) -> f32 {
    let n = |x: f32, y: f32, s: u32| noise(Vec3::new(x, y, 0.5), seed.wrapping_add(s));
    match kind {
        // Fine streaks along one way, a faint blotch over them.
        MaterialKind::Metal => {
            let streaks = n(q.x * 0.5, q.y * 40.0, 1);
            let blotch = n(q.x * 2.0, q.y * 2.0, 2);
            (streaks * 0.75 + blotch * 0.25 - 0.2).clamp(0.0, 1.0)
        }
        // Rings across the grain, wobbling, and fine grain along it.
        MaterialKind::Wood => {
            let wobble = n(q.x * 0.6, q.y * 0.6, 3) * 2.5;
            let ring = (q.y * 6.0 + wobble).fract();
            let rings = smoothstep(0.55, 0.85, ring) * (1.0 - smoothstep(0.85, 1.0, ring));
            let grain = n(q.x * 0.4, q.y * 30.0, 4);
            (rings * 0.8 + (grain - 0.5) * 0.4).clamp(0.0, 1.0)
        }
        // Layered blotches with dark specks.
        MaterialKind::Stone => {
            let big = n(q.x * 2.0, q.y * 2.0, 5);
            let mid = n(q.x * 6.0, q.y * 6.0, 6);
            let specks = smoothstep(0.78, 0.85, n(q.x * 40.0, q.y * 40.0, 7));
            (big * 0.55 + mid * 0.35 + specks * 0.6 - 0.15).clamp(0.0, 1.0)
        }
        // A faint, even texture.
        MaterialKind::Paint => n(q.x * 20.0, q.y * 20.0, 8) * 0.35,
    }
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a).max(1e-6)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mix(a: [u8; 4], b: [u8; 4], t: f32) -> [u8; 4] {
    let mut out = [255u8; 4];
    for k in 0..3 {
        let v = f32::from(a[k]) * (1.0 - t) + f32::from(b[k]) * t;
        out[k] = v.round().clamp(0.0, 255.0) as u8;
    }
    out
}

fn grey(v: f32) -> [u8; 4] {
    let g = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    [g, g, g, 255]
}
