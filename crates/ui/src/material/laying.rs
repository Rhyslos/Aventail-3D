//! Laying a graph's material on a shape for the preview ([decision 0260](../../../../docs/decisions/0260-preview-and-checks.md)):
//! how many times it repeats across a shape's UVs at the material's
//! real-world size, and the material repeated that many times into a
//! texture the views already know how to draw (the painter's painted
//! surface, decision 0178, with its realism maps, decision 0207).
//!
//! The material is a seamless square that covers `real_size_cm`
//! centimetres. A shape's UVs say how much of the texture square each
//! part of its surface covers; with the shape's size that gives how many
//! centimetres one texture width covers on it, and so how many repeats
//! fit across. The shape then shows the material at its real size: a
//! 2 m brick wall material shows bricks of the right size on a 4 m wall.

use crate::paint_view::{PaintedTexture, RealismMaps, texture_key};
use glam::{Mat4, Vec3};
use matgraph::{Image, Value};
use paint::{TILE, TILE_BYTES};
use std::sync::{Arc, OnceLock};

/// The most repeats laid across a texture: past this the material's
/// pattern is smaller than the texture's pixels can show.
pub(crate) const MAX_REPEATS: f32 = 256.0;
/// The fewest: a material much larger than the shape still shows a part.
pub(crate) const MIN_REPEATS: f32 = 1.0 / 64.0;

/// The preview ball: the Sphere shape (1 m across, standing on the
/// ground), with its own UVs, made once.
pub(crate) fn ball() -> Arc<mesh::Mesh> {
    static BALL: OnceLock<Arc<mesh::Mesh>> = OnceLock::new();
    BALL.get_or_init(|| Arc::new(mesh::PrimitiveKind::Sphere.standard().mesh()))
        .clone()
}

/// The preview cube: the Cube shape (1 m on each side), made once.
pub(crate) fn cube() -> Arc<mesh::Mesh> {
    static CUBE: OnceLock<Arc<mesh::Mesh>> = OnceLock::new();
    CUBE.get_or_init(|| Arc::new(mesh::PrimitiveKind::Cube.standard().mesh()))
        .clone()
}

/// How many times a material `real_size_cm` across repeats across the
/// whole texture square on `mesh` placed by `transform`, kept within
/// [`MIN_REPEATS`] and [`MAX_REPEATS`]. `None` when the shape has no UVs
/// (or no area), so there is nothing to lay the material by.
pub(crate) fn repeats(mesh: &mesh::Mesh, transform: Mat4, real_size_cm: f32) -> Option<f32> {
    // Texture widths per centimetre of the shape, in its own space.
    let density = mesh::uvs::average_density(mesh)?;
    // The placement's size: the average of its axes' lengths.
    let scale = (transform.x_axis.truncate().length()
        + transform.y_axis.truncate().length()
        + transform.z_axis.truncate().length())
        / 3.0;
    let cm_per_texture = scale / density;
    let repeats = cm_per_texture / real_size_cm.max(matgraph::MIN_REAL_SIZE_CM);
    repeats
        .is_finite()
        .then(|| repeats.clamp(MIN_REPEATS, MAX_REPEATS))
}

/// `repeats` laid `tiling` times more often (a material's Tiling,
/// decision 0270), kept within [`MIN_REPEATS`] and [`MAX_REPEATS`]. The
/// painter lays a graph fill the same way (`matgraph::OnModel::repeats`).
pub(crate) fn tiled(repeats: f32, tiling: f32) -> f32 {
    (repeats * paint::GraphFill::usable_tiling(tiling)).clamp(MIN_REPEATS, MAX_REPEATS)
}

/// The box round `mesh` placed by `transform`: its middle and the radius
/// of a ball holding it, or `None` for an empty shape.
pub(crate) fn around(mesh: &mesh::Mesh, transform: Mat4) -> Option<(Vec3, f32)> {
    let bounds = mesh.bounds()?;
    let corners = (0..8).map(|i| {
        transform.transform_point3(Vec3::new(
            if i & 1 == 0 {
                bounds.min.x
            } else {
                bounds.max.x
            },
            if i & 2 == 0 {
                bounds.min.y
            } else {
                bounds.max.y
            },
            if i & 4 == 0 {
                bounds.min.z
            } else {
                bounds.max.z
            },
        ))
    });
    let (min, max) = corners.fold(
        (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
        |(lo, hi), p| (lo.min(p), hi.max(p)),
    );
    let middle = (min + max) * 0.5;
    let radius = (max - min).length() * 0.5;
    (middle.is_finite() && radius.is_finite()).then_some((middle, radius.max(1.0)))
}

/// The key the preview's texture goes by on the graphics card: never a
/// material's own (they are named; this name can't be typed).
pub(crate) fn preview_key() -> u64 {
    texture_key("\u{0}material preview")
}

/// `value` repeated `repeats` times across a texture `size` pixels
/// across (one of `paint`'s sizes), as the views' painted texture: its
/// colour, and for a whole material its roughness, metal and height as
/// the realism maps (roughness red, metal green, height blue, decision
/// 0211). Other values show as their colour, or greys. An alpha card's
/// opacity (decision 0276) is the colour's alpha. `crisp` (pixel art,
/// decision 0287) takes each texel from the nearest pixel, so big pixels
/// stay sharp squares; otherwise the four nearest are blended.
pub(crate) fn laid(value: &Value, repeats: f32, size: u32, crisp: bool) -> Option<PaintedTexture> {
    let mut colour = paint::Image::new(size)?;
    let mut surface = paint::Image::new(size)?;
    let material = value.material();
    let step = repeats / size as f32;
    for index in 0..colour.tile_count() {
        let (x0, y0, w, h) = colour.tile_area(index);
        let mut c = vec![0u8; TILE_BYTES];
        let mut s = vec![0u8; TILE_BYTES];
        for row in 0..h {
            for column in 0..w {
                // The texel's middle on the material, in whole textures.
                let (u, v) = (
                    ((x0 + column) as f32 + 0.5) * step,
                    ((y0 + row) as f32 + 0.5) * step,
                );
                let (u, v) = if crisp {
                    nearest_middle(value.size(), u, v)
                } else {
                    (u, v)
                };
                let at = ((row * TILE + column) * 4) as usize;
                let rgb = colour_of(value, u, v);
                // An alpha card's opacity (decision 0276): the views cut
                // it out where the item asks (`DrawItem::cut_out`).
                let alpha = material
                    .and_then(|m| m.opacity.as_ref())
                    .map_or(255, |o| byte(o.sample(u, v)));
                c[at..at + 4].copy_from_slice(&[byte(rgb[0]), byte(rgb[1]), byte(rgb[2]), alpha]);
                if let Some(m) = material {
                    s[at..at + 4].copy_from_slice(&[
                        byte(m.roughness.sample(u, v)),
                        byte(m.metal.sample(u, v)),
                        byte(m.height.sample(u, v)),
                        255,
                    ]);
                }
            }
        }
        colour.set_tile(index, Some(Arc::new(c)));
        if material.is_some() {
            surface.set_tile(index, Some(Arc::new(s)));
        }
    }
    let realism = material.map(|_| RealismMaps {
        surface,
        // Nothing glows: an empty (see-through) image.
        glow: paint::Image::new(size).unwrap_or_else(|| colour.clone()),
    });
    Some(PaintedTexture {
        key: preview_key(),
        material: String::new(),
        image: colour,
        realism,
    })
}

/// The middle of the pixel of a picture `size` across that `(u, v)` falls
/// in: sampled there, blending gives that pixel alone.
fn nearest_middle(size: u32, u: f32, v: f32) -> (f32, f32) {
    let n = size.max(1) as f32;
    let middle = |t: f32| ((t * n).floor() + 0.5) / n;
    (middle(u), middle(v))
}

/// The colour `value` shows at `(u, v)` (whole textures, wrapping),
/// blended between its four nearest pixels.
fn colour_of(value: &Value, u: f32, v: f32) -> [f32; 3] {
    match value {
        Value::Shape(g) | Value::Height(g) => {
            let grey = g.sample(u, v);
            [grey; 3]
        }
        Value::Colour(c) => sample_rgb(c, u, v),
        Value::Material(m) => sample_rgb(&m.colour, u, v),
    }
}

/// [`matgraph::Grey::sample`] for colours.
fn sample_rgb(image: &Image<[f32; 3]>, u: f32, v: f32) -> [f32; 3] {
    let n = image.size() as f32;
    let (x, y) = (u * n - 0.5, v * n - 0.5);
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let (x0, y0) = (x0 as i64, y0 as i64);
    let mix = |a: [f32; 3], b: [f32; 3], t: f32| {
        [
            a[0] + (b[0] - a[0]) * t,
            a[1] + (b[1] - a[1]) * t,
            a[2] + (b[2] - a[2]) * t,
        ]
    };
    let top = mix(image.at(x0, y0), image.at(x0 + 1, y0), fx);
    let bottom = mix(image.at(x0, y0 + 1), image.at(x0 + 1, y0 + 1), fx);
    mix(top, bottom, fy)
}

fn byte(v: f32) -> u8 {
    super::pictures::byte(v)
}

#[cfg(test)]
mod tests {

    #[test]
    fn tiling_lays_the_material_more_often_within_the_limits() {
        assert_eq!(super::tiled(2.0, 4.0), 8.0);
        assert_eq!(super::tiled(2.0, 0.5), 1.0);
        assert_eq!(super::tiled(200.0, 32.0), super::MAX_REPEATS);
        assert_eq!(super::tiled(1.0 / 64.0, 0.125), super::MIN_REPEATS);
        assert_eq!(super::tiled(3.0, f32::NAN), 3.0);
    }

    use super::*;
    use matgraph::{Size, ValueKind};

    #[test]
    fn the_ball_and_cube_repeat_a_material_at_its_real_size() {
        // The cube's six 1 m sides fit in the texture square side by side:
        // a texture width covers several metres, so a 1 m material
        // repeats several times; a 2 m one half as often.
        let cube = cube();
        let one = repeats(&cube, Mat4::IDENTITY, 100.0).unwrap();
        let two = repeats(&cube, Mat4::IDENTITY, 200.0).unwrap();
        assert!(one > 2.0, "{one}");
        assert!((one / two - 2.0).abs() < 1e-3, "{one} {two}");
        // Twice as big a cube: twice the repeats.
        let big = repeats(&cube, Mat4::from_scale(Vec3::splat(2.0)), 100.0).unwrap();
        assert!((big / one - 2.0).abs() < 1e-3);
        let ball = repeats(&ball(), Mat4::IDENTITY, 100.0).unwrap();
        assert!(ball > 1.0, "{ball}");
    }

    #[test]
    fn repeats_stay_in_range_and_need_uvs() {
        let cube = cube();
        let tiny = repeats(&cube, Mat4::IDENTITY, matgraph::MIN_REAL_SIZE_CM).unwrap();
        assert!(tiny <= MAX_REPEATS);
        let huge = repeats(&cube, Mat4::from_scale(Vec3::splat(1e-6)), 1e6).unwrap();
        assert_eq!(huge, MIN_REPEATS);
        assert!(repeats(&cube, Mat4::from_scale(Vec3::splat(f32::NAN)), 100.0).is_none());
        // No UVs: nothing to lay it by.
        let mut bare = (*cube).clone();
        bare.remove_attribute(mesh::attributes::UV);
        assert!(repeats(&bare, Mat4::IDENTITY, 100.0).is_none());
    }

    #[test]
    fn the_shape_is_framed_whole() {
        let (middle, radius) = around(&cube(), Mat4::IDENTITY).unwrap();
        assert!((middle.z - 50.0).abs() < 1e-3, "stands on the ground");
        assert!((radius - 3f32.sqrt() * 50.0).abs() < 1e-2);
        let moved = Mat4::from_translation(Vec3::new(300.0, 0.0, 0.0));
        assert!((around(&cube(), moved).unwrap().0.x - 300.0).abs() < 1e-3);
        assert!(around(&mesh::Mesh::default(), Mat4::IDENTITY).is_none());
    }

    #[test]
    fn a_material_is_laid_with_its_realism_maps_and_repeats() {
        let size = Size::new(16).unwrap();
        // Left half dark, right half light: laid twice across, the
        // texture goes dark, light, dark, light.
        let pixels = (0..16 * 16)
            .map(|i| if i % 16 < 8 { [0.0; 3] } else { [1.0; 3] })
            .collect();
        let colour = Arc::new(Image::from_pixels(size, pixels).unwrap());
        let grey = |v: f32| Arc::new(Image::filled(size, v));
        let m = matgraph::MaterialMaps {
            colour,
            height: grey(0.5),
            roughness: grey(0.8),
            metal: grey(0.0),
            normal: Arc::new(Image::filled(size, [0.0, 0.0, 1.0])),
            occlusion: grey(1.0),
            curvature: grey(0.5),
            opacity: None,
            translucency: None,
        };
        let texture = laid(&Value::Material(Arc::new(m.clone())), 2.0, 64, false).unwrap();
        assert_eq!(texture.key, preview_key());
        let px = |x| texture.image.pixel(x, 10)[0];
        assert!(px(4) < 30 && px(20) > 225 && px(36) < 30 && px(52) > 225);
        assert_eq!(texture.image.pixel(4, 10)[3], 255, "solid");
        let realism = texture.realism.expect("a material's maps");
        assert_eq!(realism.surface.pixel(5, 5), [204, 0, 128, 255]);
        assert!(realism.glow.is_empty(), "nothing glows");
        // An alpha card (decision 0276): its opacity is the alpha, so the
        // preview cuts it out; the CPU ball leaves it see-through too.
        let half = (0..16 * 16)
            .map(|i| if (i / 16) < 8 { 1.0 } else { 0.0 })
            .collect();
        let card = matgraph::MaterialMaps {
            opacity: Some(Arc::new(Image::from_pixels(size, half).unwrap())),
            ..m
        };
        let value = Value::Material(Arc::new(card));
        let texture = laid(&value, 1.0, 64, false).unwrap();
        assert_eq!(texture.image.pixel(10, 10)[3], 255);
        assert_eq!(texture.image.pixel(10, 50)[3], 0);
        let ball = super::super::preview_ball::ball(&value, 32);
        // The ball's top half shows the solid rows, its bottom half the
        // cut-away ones.
        assert!(ball.pixels[8 * 32 + 16].a() > 0);
        assert_eq!(ball.pixels[24 * 32 + 16].a(), 0, "a hole in the ball");
        let picture = super::super::pictures::colour_image(&value, 16);
        assert_eq!(picture.pixels[3 * 16 + 3].a(), 255);
        assert_eq!(picture.pixels[12 * 16 + 3].a(), 0);
    }

    #[test]
    fn a_shape_or_colour_is_laid_as_its_colour_alone() {
        let value = Value::flat_grey(ValueKind::Shape, Size::new(16).unwrap(), 1.0).unwrap();
        let texture = laid(&value, 1.0, 32, false).unwrap();
        assert_eq!(texture.image.pixel(3, 3), [255, 255, 255, 255]);
        assert!(texture.realism.is_none());
        // A size the views can't take: nothing.
        assert!(laid(&value, 1.0, 1, false).is_none());
    }

    #[test]
    fn pixel_art_is_laid_crisp_with_no_blend_between_its_pixels() {
        // Decision 0287: a 16-pixel picture of two colours in columns,
        // laid over 64 texels: crisp gives only the two colours, blended
        // gives the colours between them at the joins.
        let size = Size::new(16).unwrap();
        let pixels = (0..16 * 16)
            .map(|i| {
                if (i % 16) / 4 % 2 == 0 {
                    [1.0, 0.0, 0.0]
                } else {
                    [0.0, 0.0, 1.0]
                }
            })
            .collect();
        let value = Value::Colour(Arc::new(Image::from_pixels(size, pixels).unwrap()));
        let colours = |crisp: bool| {
            let texture = laid(&value, 1.0, 64, crisp).unwrap();
            let mut seen: Vec<[u8; 4]> = Vec::new();
            for x in 0..64 {
                let p = texture.image.pixel(x, 5);
                if !seen.contains(&p) {
                    seen.push(p);
                }
            }
            seen
        };
        assert_eq!(colours(true).len(), 2);
        assert!(colours(false).len() > 2);
        // Twice round: the same squares repeat.
        let twice = laid(&value, 2.0, 64, true).unwrap();
        assert_eq!(twice.image.pixel(0, 0), twice.image.pixel(32, 0));
    }
}
