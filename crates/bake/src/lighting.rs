//! The light-and-shade layer's pixels ([decision 0196](../../../docs/decisions/0196-light-and-shade.md)):
//! each texel lit by how much it faces the sun, and darkened where the
//! shape shades itself, warm light to cool shade.

use crate::rays::Rays;
use crate::{Source, bounds, texels};
use glam::Vec3;
use paint::{Image, Lighting};

/// Rays toward the sun per texel when shadows are soft: spread over a
/// small disc, so shadow edges blur with the softness.
const SHADOW_RAYS: u32 = 4;

/// The light-and-shade pixels on `sources` (the objects sharing the
/// texture set); see-through off the polygons. `None` for a size out of
/// range.
pub fn lighting(sources: &[Source<'_>], lighting: &Lighting) -> Option<Image> {
    let size = sources.first().map_or(64, |s| s.size);
    let mut image = Image::new(size)?;
    let sun = Vec3::from(lighting.sun()).normalize_or(Vec3::Z);
    // A frame around the sun for the soft shadow rays.
    let side = sun.any_orthonormal_vector();
    let up = sun.cross(side);
    let spread = lighting.softness * 0.08;
    for source in sources {
        let (lo, hi) = bounds(source.surface, source.slots);
        let reach = (hi - lo).length().max(1e-3) * 2.0;
        let rays = lighting.shadows.then(|| Rays::of(source.surface));
        let texels = texels::covered(source.surface, source.slots, size);
        let values = mesh::parallel::in_parallel(texels.len(), |range| {
            texels[range]
                .iter()
                .map(|t| {
                    let facing = t.normal.dot(sun);
                    let mut lit = lighting.lit(facing);
                    if let Some(rays) = &rays
                        && lit > 0.0
                    {
                        let from = t.at + t.normal * (reach * 1e-4);
                        let count = if spread > 0.0 { SHADOW_RAYS } else { 1 };
                        let open = (0..count)
                            .filter(|&i| {
                                let a = i as f32 / count as f32 * std::f32::consts::TAU;
                                let d = (sun + (side * a.cos() + up * a.sin()) * spread)
                                    .normalize_or(sun);
                                rays.hit(from, d, reach).is_none()
                            })
                            .count();
                        lit *= open as f32 / count as f32;
                    }
                    (t.x, t.y, lighting.at(lit))
                })
                .collect::<Vec<_>>()
        });
        for part in values {
            for (x, y, c) in part {
                image.set_pixel(x, y, c);
            }
        }
    }
    Some(image)
}
