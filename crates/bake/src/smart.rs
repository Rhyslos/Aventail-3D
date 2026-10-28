//! Smart masks' pixels ([decision 0195](../../../docs/decisions/0195-smart-masks.md)):
//! worked out from the shape's own polygons, as the baked maps are
//! (curvature for edges and cavities, facing for facing up, place for
//! height), with noise to break the border up.

use crate::curvature::Edges;
use crate::{Source, bounds, texels};
use glam::Vec3;
use paint::{Image, SmartKind, SmartMask};

/// The widest band edges and cavities take in, as a share of the
/// shape's size, at `amount` 1.
const MOST_WIDTH: f32 = 0.1;

/// A smart mask's pixels on `sources` (the objects sharing the texture
/// set): white where it shows, black where it hides, see-through off the
/// polygons. `None` for a size out of range.
pub fn smart_mask(sources: &[Source<'_>], mask: &SmartMask) -> Option<Image> {
    let size = sources.first().map_or(64, |s| s.size);
    let mut image = Image::new(size)?;
    for source in sources {
        let (lo, hi) = bounds(source.surface, source.slots);
        let diagonal = (hi - lo).length().max(1e-6);
        let edges = matches!(mask.kind, SmartKind::Edges | SmartKind::Cavities).then(|| {
            Edges::of(
                source.surface,
                source.slots,
                diagonal * MOST_WIDTH * mask.amount.max(0.01),
            )
        });
        let texels = texels::covered(source.surface, source.slots, size);
        let values = mesh::parallel::in_parallel(texels.len(), |range| {
            texels[range]
                .iter()
                .map(|t| {
                    let raw = match mask.kind {
                        SmartKind::Edges => edges
                            .as_ref()
                            .map_or(0.0, |e| ((e.at(t.at) - 0.5) * 2.0).max(0.0)),
                        SmartKind::Cavities => edges
                            .as_ref()
                            .map_or(0.0, |e| ((0.5 - e.at(t.at)) * 2.0).max(0.0)),
                        // Straight up 1, sideways 0: shown above 1 − amount.
                        SmartKind::FacingUp => ramp(t.normal.z, 1.0 - mask.amount),
                        SmartKind::Height => {
                            let h = (t.at.z - lo.z) / (hi.z - lo.z).max(1e-6);
                            ramp(h, mask.amount)
                        }
                    };
                    let n = noise(t.at / (diagonal * mask.noise_size.max(0.005)), mask.seed);
                    let v = raw + (n - 0.5) * mask.noise;
                    let w = 0.5 * mask.softness.max(0.02);
                    let shown = smoothstep(0.5 - w, 0.5 + w, v);
                    (t.x, t.y, shown)
                })
                .collect::<Vec<_>>()
        });
        for part in values {
            for (x, y, v) in part {
                let g = (v * 255.0).round() as u8;
                image.set_pixel(x, y, [g, g, g, 255]);
            }
        }
    }
    Some(image)
}

/// 0.5 at `level`, rising across a short distance either side, so the
/// softness can shape the border from there.
fn ramp(value: f32, level: f32) -> f32 {
    (0.5 + (value - level) * 4.0).clamp(0.0, 1.0)
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a).max(1e-6)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Smooth noise in 3D, 0 to 1: two layers of value noise, the second
/// half the size and half as strong.
fn noise(p: Vec3, seed: u32) -> f32 {
    (value_noise(p, seed) * 2.0 + value_noise(p * 2.0, seed.wrapping_add(17))) / 3.0
}

fn value_noise(p: Vec3, seed: u32) -> f32 {
    let f = p.floor();
    let t = p - f;
    let s = t * t * (Vec3::splat(3.0) - 2.0 * t);
    let (x, y, z) = (f.x as i32, f.y as i32, f.z as i32);
    let at = |dx: i32, dy: i32, dz: i32| lattice(x + dx, y + dy, z + dz, seed);
    let lerp = |a: f32, b: f32, k: f32| a + (b - a) * k;
    let x00 = lerp(at(0, 0, 0), at(1, 0, 0), s.x);
    let x10 = lerp(at(0, 1, 0), at(1, 1, 0), s.x);
    let x01 = lerp(at(0, 0, 1), at(1, 0, 1), s.x);
    let x11 = lerp(at(0, 1, 1), at(1, 1, 1), s.x);
    lerp(lerp(x00, x10, s.y), lerp(x01, x11, s.y), s.z)
}

/// A number 0 to 1 for a lattice point.
fn lattice(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343)
        ^ (y as u32).wrapping_mul(0xd816_3841)
        ^ (z as u32).wrapping_mul(0xcb1a_b31f)
        ^ seed.wrapping_mul(0x9e37_79b9);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^= h >> 15;
    h as f32 / u32::MAX as f32
}
