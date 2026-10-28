//! Texture export ([decision 0204](../../../../docs/decisions/0204-texture-export.md)):
//! each painted material's texture, its layers mixed over the material's
//! colour, padded past the edges of its UV pieces so the game's smaller
//! copies (mipmaps) don't pull in the colour around them, written as PNG
//! or TGA: inside a glTF (always PNG, as glTF wants), or beside an OBJ
//! and named in its `.mtl`.

use super::{ExportedObject, file_materials};
use glam::Vec2;
use scene::Document;
use serde::{Deserialize, Serialize};

/// How painted textures go out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum TextureFormat {
    /// PNG: smaller files, every engine reads it, and glTF takes it inside.
    #[default]
    Png,
    /// TGA: uncompressed, for tools that prefer it. glTF still gets PNG.
    Tga,
    /// Left out.
    None,
}

impl TextureFormat {
    /// Every choice, in the order offered.
    pub const ALL: [TextureFormat; 3] = [Self::Png, Self::Tga, Self::None];

    /// Its file extension, if it writes files.
    pub fn extension(self) -> Option<&'static str> {
        match self {
            Self::Png => Some("png"),
            Self::Tga => Some("tga"),
            Self::None => None,
        }
    }
}

/// The most texels of padding.
pub const MAX_PADDING: u32 = 64;

/// One texture going out.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportTexture {
    /// The material it belongs to.
    pub material: String,
    /// Its file's name (beside an OBJ), or its name inside a glTF.
    pub file_name: String,
    /// Its encoded bytes.
    pub bytes: Vec<u8>,
    /// What it is, for the budget report.
    pub report: TextureReport,
}

/// One texture's line in the budget report.
#[derive(Debug, Clone, PartialEq)]
pub struct TextureReport {
    /// The material.
    pub material: String,
    /// Its file's name.
    pub file_name: String,
    /// Pixels across and down.
    pub size: u32,
    /// How it's stored.
    pub format: TextureFormat,
    /// Colour channels (red, green, blue, and alpha).
    pub channels: u8,
    /// The file's size in bytes (0 before it's made).
    pub file_bytes: usize,
    /// What it takes on the graphics card, with its smaller copies: four
    /// bytes a pixel, a third more for the copies.
    pub gpu_bytes: usize,
}

/// What a texture of `size` pixels takes on the graphics card,
/// uncompressed, with its smaller copies.
pub fn gpu_bytes(size: u32) -> usize {
    let s = size as usize;
    s * s * 4 * 4 / 3
}

/// A material's name made safe for a file name: letters, digits, `-` and
/// `_`, the rest `_`.
fn file_safe(name: &str) -> String {
    let out: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if out.is_empty() {
        "material".into()
    } else {
        out
    }
}

/// The painted materials of `objects` in `document` and their file names
/// (`stem_material.ext`), for the report before exporting: no pixels
/// mixed or files made.
pub fn plan(
    document: &Document,
    objects: &[ExportedObject],
    format: TextureFormat,
    stem: &str,
) -> Vec<TextureReport> {
    let Some(ext) = format.extension() else {
        return Vec::new();
    };
    file_materials(objects)
        .into_iter()
        .filter_map(|m| {
            let set = document.scene().paint().set(&m.name)?;
            Some(TextureReport {
                file_name: format!("{stem}_{}.{ext}", file_safe(&m.name)),
                material: m.name,
                size: set.size,
                format,
                channels: 4,
                file_bytes: 0,
                gpu_bytes: gpu_bytes(set.size),
            })
        })
        .collect()
}

/// The textures of `objects`' painted materials in `document`, in
/// `format` (for glTF, PNG whatever is asked), padded by `padding`
/// texels round their UV pieces.
pub fn make(
    document: &Document,
    objects: &[ExportedObject],
    format: TextureFormat,
    padding: u32,
    stem: &str,
) -> Vec<ExportTexture> {
    let materials = file_materials(objects);
    plan(document, objects, format, stem)
        .into_iter()
        .filter_map(|mut report| {
            let set = document.scene().paint().set(&report.material)?;
            let composite = paint::Composite::of(set)?;
            let colour = materials
                .iter()
                .find(|m| m.name == report.material)
                .map_or([0.8; 3], |m| m.color);
            let size = set.size;
            let mut rgba = flatten(composite.image(), colour);
            let mut covered = coverage(objects, &report.material, size);
            pad(&mut rgba, &mut covered, size, padding.min(MAX_PADDING));
            let bytes = encode(&rgba, size, format)?;
            report.file_bytes = bytes.len();
            Some(ExportTexture {
                material: report.material.clone(),
                file_name: report.file_name.clone(),
                bytes,
                report,
            })
        })
        .collect()
}

/// `image`'s pixels laid over `colour` (sRGB 0 to 1): opaque, as the
/// viewport shows the material.
pub(crate) fn flatten(image: &paint::Image, colour: [f32; 3]) -> Vec<u8> {
    let size = image.size();
    let base = colour.map(|c| c.clamp(0.0, 1.0) * 255.0);
    let mut out = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let p = image.pixel(x, y);
            let a = f32::from(p[3]) / 255.0;
            for c in 0..3 {
                let v = base[c] * (1.0 - a) + f32::from(p[c]) * a;
                out.push(v.round().clamp(0.0, 255.0) as u8);
            }
            out.push(255);
        }
    }
    out
}

/// Which texels of a `size`-pixel texture `objects`' polygons of
/// `material` cover (where the texture repeats, too).
pub(crate) fn coverage(objects: &[ExportedObject], material: &str, size: u32) -> Vec<bool> {
    let side = size as f32;
    let mut covered = vec![false; (size * size) as usize];
    for object in objects {
        let shape = object.built();
        if shape.uvs.is_empty() {
            continue;
        }
        for polygon in &shape.polygons {
            let used = object
                .materials
                .get(polygon.material as usize)
                .is_some_and(|m| m.name == material);
            if !used || polygon.corners.len() < 3 {
                continue;
            }
            let uv = |k: usize| {
                let c = polygon.corners[k];
                c.uv.and_then(|i| shape.uvs.get(i as usize).copied())
                    .map(|uv| Vec2::new(uv.x * side, (1.0 - uv.y) * side))
            };
            for k in 1..polygon.corners.len() - 1 {
                let (Some(a), Some(b), Some(c)) = (uv(0), uv(k), uv(k + 1)) else {
                    continue;
                };
                let whole = [a, b, c];
                for shift in paint::surface::tile_shifts(&whole, side) {
                    fill_triangle(&mut covered, size, whole.map(|p| p + shift));
                }
            }
        }
    }
    covered
}

/// Marks the texels whose middles `t` (texel space) covers, and those
/// along its edges.
fn fill_triangle(covered: &mut [bool], size: u32, t: [Vec2; 3]) {
    let side = size as f32;
    let lo = (t[0].min(t[1]).min(t[2]) - Vec2::splat(0.5)).max(Vec2::ZERO);
    let hi = (t[0].max(t[1]).max(t[2]) + Vec2::splat(0.5)).min(Vec2::splat(side - 1.0));
    if lo.x > hi.x || lo.y > hi.y || !lo.is_finite() || !hi.is_finite() {
        return;
    }
    let area = (t[1] - t[0]).perp_dot(t[2] - t[0]);
    if area.abs() < 1e-9 {
        return;
    }
    let sign = area.signum();
    let edges = [(0, 1), (1, 2), (2, 0)].map(|(a, b)| {
        let e = t[b] - t[a];
        (t[a], Vec2::new(-e.y, e.x).normalize_or_zero() * sign)
    });
    for y in lo.y.floor() as u32..=hi.y.ceil() as u32 {
        for x in lo.x.floor() as u32..=hi.x.ceil() as u32 {
            if x >= size || y >= size {
                continue;
            }
            let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            if edges
                .iter()
                .all(|(on, inside)| (p - *on).dot(*inside) >= -0.71)
            {
                covered[(y * size + x) as usize] = true;
            }
        }
    }
}

/// Spreads covered texels' colours `padding` texels outward, ring by
/// ring: each uncovered texel next to covered ones takes their average.
pub(crate) fn pad(rgba: &mut [u8], covered: &mut [bool], size: u32, padding: u32) {
    let s = size as i64;
    for _ in 0..padding {
        let mut ring = Vec::new();
        for y in 0..s {
            for x in 0..s {
                if covered[(y * s + x) as usize] {
                    continue;
                }
                let mut sum = [0u32; 3];
                let mut n = 0;
                for (dx, dy) in [
                    (-1, 0),
                    (1, 0),
                    (0, -1),
                    (0, 1),
                    (-1, -1),
                    (1, 1),
                    (-1, 1),
                    (1, -1),
                ] {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= s || ny >= s {
                        continue;
                    }
                    let i = (ny * s + nx) as usize;
                    if covered[i] {
                        for c in 0..3 {
                            sum[c] += u32::from(rgba[i * 4 + c]);
                        }
                        n += 1;
                    }
                }
                if n > 0 {
                    ring.push(((y * s + x) as usize, sum.map(|v| (v / n) as u8)));
                }
            }
        }
        if ring.is_empty() {
            break;
        }
        for (i, c) in ring {
            rgba[i * 4..i * 4 + 3].copy_from_slice(&c);
            covered[i] = true;
        }
    }
}

/// `rgba` (`size` square) as a PNG or TGA file.
pub(crate) fn encode(rgba: &[u8], size: u32, format: TextureFormat) -> Option<Vec<u8>> {
    use image::ImageEncoder;
    let mut out = Vec::new();
    match format {
        TextureFormat::Png => image::codecs::png::PngEncoder::new(&mut out)
            .write_image(rgba, size, size, image::ExtendedColorType::Rgba8)
            .ok()?,
        TextureFormat::Tga => image::codecs::tga::TgaEncoder::new(&mut out)
            .write_image(rgba, size, size, image::ExtendedColorType::Rgba8)
            .ok()?,
        TextureFormat::None => return None,
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padding_spreads_colour_out_ring_by_ring() {
        let size = 8;
        let mut rgba = vec![0u8; 8 * 8 * 4];
        let mut covered = vec![false; 64];
        // One red texel in the middle.
        covered[3 * 8 + 3] = true;
        rgba[(3 * 8 + 3) * 4] = 255;
        pad(&mut rgba, &mut covered, size, 2);
        assert_eq!(rgba[(3 * 8 + 5) * 4], 255, "two texels out");
        assert!(!covered[3 * 8 + 6], "not three");
        assert_eq!(rgba[(3 * 8 + 6) * 4], 0);
    }

    #[test]
    fn paint_is_laid_over_the_material_colour() {
        let mut image = paint::Image::new(64).unwrap();
        image.set_pixel(0, 0, [0, 0, 255, 255]);
        image.set_pixel(1, 0, [0, 0, 255, 128]);
        let rgba = flatten(&image, [1.0, 0.0, 0.0]);
        assert_eq!(&rgba[0..4], &[0, 0, 255, 255]);
        assert_eq!(&rgba[4..8], &[127, 0, 128, 255]);
        assert_eq!(&rgba[8..12], &[255, 0, 0, 255], "unpainted: the colour");
    }

    #[test]
    fn png_and_tga_files_read_back() {
        let rgba: Vec<u8> = (0..16 * 16).flat_map(|i| [i as u8, 7, 9, 255]).collect();
        for format in [TextureFormat::Png, TextureFormat::Tga] {
            let bytes = encode(&rgba, 16, format).unwrap();
            let back = crate::image_file::read_image(&bytes).unwrap();
            assert_eq!((back.width, back.height), (16, 16));
            assert_eq!(back.rgba, rgba, "{format:?}");
        }
        assert!(encode(&rgba, 16, TextureFormat::None).is_none());
        assert_eq!(file_safe("Wood / old"), "Wood___old");
        assert_eq!(gpu_bytes(1024), 1024 * 1024 * 4 * 4 / 3);
    }
}
