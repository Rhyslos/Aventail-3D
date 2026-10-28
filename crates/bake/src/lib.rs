//! Maps baked from a shape itself ([decision 0192](../../../docs/decisions/0192-the-baker.md)):
//! what the texture painter's guides and smart masks read, worked out
//! for each texel of a texture set from the shape's own polygons (no
//! high-detail shape needed).
//!
//! - **Position:** where the texel is on the shape, across its box, as
//!   red, green and blue (X, Y, Z; Z is up).
//! - **Normals:** which way the surface faces there, in the shape's own
//!   space (X red, Y green, Z blue; 0.5 is zero).
//! - **Occlusion** (ambient occlusion): how open the surface is to the
//!   sky around it; white open, dark in corners and creases.
//! - **Curvature:** light along outer (convex) edges, dark along inner
//!   (concave) ones, mid-grey elsewhere.
//! - **ID:** a flat colour per material slot.
//! - **Thickness:** light where the shape is thin, dark where thick.
//!
//! Everything here is plain Rust with no window or graphics card, shared
//! out over the processor's cores.

mod curvature;
mod lighting;
mod rays;
mod smart;
mod texels;

pub use lighting::lighting;
pub use rays::Rays;
pub use smart::smart_mask;

use glam::{Vec2, Vec3};
use paint::{Image, PaintSurface};

/// A kind of baked map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BakeKind {
    /// Where on the shape, across its box.
    Position,
    /// Which way the surface faces, in the shape's own space.
    Normal,
    /// How open the surface is (ambient occlusion).
    Occlusion,
    /// Outer edges light, inner ones dark.
    Curvature,
    /// A colour per material slot.
    Id,
    /// Thin light, thick dark.
    Thickness,
}

impl BakeKind {
    /// Every kind, in the order shown.
    pub const ALL: [BakeKind; 6] = [
        BakeKind::Position,
        BakeKind::Normal,
        BakeKind::Occlusion,
        BakeKind::Curvature,
        BakeKind::Id,
        BakeKind::Thickness,
    ];
}

/// How the maps are baked.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BakeSettings {
    /// Rays a texel for occlusion and thickness: more is smoother and
    /// slower.
    pub rays: u32,
    /// How far occlusion looks for something blocking the sky, and the
    /// greatest thickness told apart, as a share of the shape's size
    /// (the length of its box's diagonal).
    pub reach: f32,
    /// How wide the curvature's edges are, as a share of the shape's
    /// size.
    pub edge_width: f32,
}

impl Default for BakeSettings {
    fn default() -> Self {
        Self {
            rays: 32,
            reach: 0.2,
            edge_width: 0.015,
        }
    }
}

/// What to bake from: a shape's surface, its slots on the texture set,
/// and the texture's size.
#[derive(Debug, Clone, Copy)]
pub struct Source<'a> {
    /// The shape.
    pub surface: &'a PaintSurface,
    /// Its material slots on the texture set.
    pub slots: &'a [u32],
    /// The texture's size in pixels.
    pub size: u32,
}

/// Bakes map `kind` of `sources` (the objects sharing the texture set)
/// with `settings`, `size` pixels across. Texels no polygon covers are
/// left empty (see-through). `None` for a size out of range.
pub fn bake(sources: &[Source<'_>], kind: BakeKind, settings: &BakeSettings) -> Option<Image> {
    let size = sources.first().map_or(64, |s| s.size);
    let mut image = Image::new(size)?;
    for source in sources {
        bake_into(&mut image, source, kind, settings);
    }
    Some(image)
}

fn bake_into(image: &mut Image, source: &Source<'_>, kind: BakeKind, settings: &BakeSettings) {
    let surface = source.surface;
    let (lo, hi) = bounds(surface, source.slots);
    let diagonal = (hi - lo).length().max(1e-6);
    let rays = matches!(kind, BakeKind::Occlusion | BakeKind::Thickness).then(|| Rays::of(surface));
    let edges = (kind == BakeKind::Curvature)
        .then(|| curvature::Edges::of(surface, source.slots, diagonal * settings.edge_width));
    let texels = texels::covered(surface, source.slots, image.size());
    let colours = mesh::parallel::in_parallel(texels.len(), |range| {
        texels[range]
            .iter()
            .map(|t| {
                let value = match kind {
                    BakeKind::Position => {
                        let v = (t.at - lo) / (hi - lo).max(Vec3::splat(1e-6));
                        colour(v)
                    }
                    BakeKind::Normal => colour(t.normal * 0.5 + Vec3::splat(0.5)),
                    BakeKind::Occlusion => {
                        let open = rays.as_ref().map_or(1.0, |r| {
                            r.openness(
                                t.at,
                                t.normal,
                                diagonal * settings.reach,
                                settings.rays,
                                t.seed,
                            )
                        });
                        grey(open)
                    }
                    BakeKind::Thickness => {
                        let thin = rays.as_ref().map_or(0.0, |r| {
                            r.thinness(
                                t.at,
                                t.normal,
                                diagonal * settings.reach,
                                settings.rays,
                                t.seed,
                            )
                        });
                        grey(thin)
                    }
                    BakeKind::Curvature => grey(edges.as_ref().map_or(0.5, |e| e.at(t.at))),
                    BakeKind::Id => id_colour(t.slot),
                };
                (t.x, t.y, value)
            })
            .collect::<Vec<_>>()
    });
    for part in colours {
        for (x, y, value) in part {
            image.set_pixel(x, y, value);
        }
    }
}

/// A gradient fill's pixels (decision 0194): `gradient` laid along each
/// of `sources`' own boxes, on the texels its polygons cover; the rest
/// see-through. `None` for a size out of range.
pub fn gradient(sources: &[Source<'_>], gradient: &paint::Gradient) -> Option<Image> {
    let size = sources.first().map_or(64, |s| s.size);
    let mut image = Image::new(size)?;
    let axis = gradient.axis.index();
    for source in sources {
        let (lo, hi) = bounds(source.surface, source.slots);
        let span = (hi[axis] - lo[axis]).max(1e-6);
        for t in texels::covered(source.surface, source.slots, size) {
            image.set_pixel(t.x, t.y, gradient.at((t.at[axis] - lo[axis]) / span));
        }
    }
    Some(image)
}

/// The box around `surface`'s triangles in `slots`.
fn bounds(surface: &PaintSurface, slots: &[u32]) -> (Vec3, Vec3) {
    let mut lo = Vec3::splat(f32::MAX);
    let mut hi = Vec3::splat(f32::MIN);
    for t in surface
        .triangles()
        .iter()
        .filter(|t| slots.contains(&t.slot))
    {
        for p in t.points {
            lo = lo.min(p);
            hi = hi.max(p);
        }
    }
    if lo.x > hi.x {
        return (Vec3::ZERO, Vec3::ONE);
    }
    (lo, hi)
}

fn colour(v: Vec3) -> [u8; 4] {
    let c = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
    [c(v.x), c(v.y), c(v.z), 255]
}

fn grey(v: f32) -> [u8; 4] {
    let g = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    [g, g, g, 255]
}

/// A clear colour for material slot `slot`, each far from the others:
/// hues a golden-ratio turn apart.
pub fn id_colour(slot: u32) -> [u8; 4] {
    let hue = (slot as f32 * 0.618_034).fract();
    let [r, g, b] = paint::palette::from_hsv([hue, 0.75, 0.95]);
    [r, g, b, 255]
}

/// A place on the texture, for tests and callers that want one texel's
/// place on the shape.
pub fn texel_middle(x: u32, y: u32, size: u32) -> Vec2 {
    Vec2::new(
        (x as f32 + 0.5) / size as f32,
        1.0 - (y as f32 + 0.5) / size as f32,
    )
}

#[cfg(test)]
mod tests;
