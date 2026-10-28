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

/// How the realism channels go out (decision 0208): inside a glTF they
/// always follow glTF's own layout (occlusion, roughness and metal in one
/// image's red, green and blue: the "ORM" Godot, Unreal, Blender and
/// Solis read); this chooses what goes beside the file too, for engines
/// and tools that want the maps as files of their own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ChannelPacking {
    /// Only what the file holds itself: glTF's ORM inside a glTF; an ORM
    /// beside an OBJ (named in its `.mtl`).
    #[default]
    Standard,
    /// An ORM file beside the model too: occlusion, roughness and metal
    /// in red, green and blue (Godot, Unreal Engine).
    Orm,
    /// Unity's mask map beside the model: metal, occlusion, detail mask
    /// and smoothness in red, green, blue and alpha (HDRP).
    UnityMask,
    /// Roughness and metal as two grey files beside the model.
    Separate,
}

impl ChannelPacking {
    /// Every choice, in the order offered.
    pub const ALL: [ChannelPacking; 4] =
        [Self::Standard, Self::Orm, Self::UnityMask, Self::Separate];
}

/// Which of a material's maps a texture is (decision 0208).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextureMap {
    /// Its colour (base colour).
    #[default]
    Colour,
    /// Occlusion, roughness and metal in red, green and blue.
    Orm,
    /// The light it gives off.
    Emissive,
    /// Unity's mask map: metal, occlusion, detail mask, smoothness.
    MaskMap,
    /// Roughness as grey.
    Roughness,
    /// Metal as grey.
    Metallic,
    /// The height channel's bumps as a tangent-space normal map
    /// (decision 0211).
    Normal,
}

impl TextureMap {
    /// What its file's name ends with, before the extension.
    pub fn suffix(self) -> &'static str {
        match self {
            Self::Colour => "",
            Self::Orm => "_ORM",
            Self::Emissive => "_Emissive",
            Self::MaskMap => "_MaskMap",
            Self::Roughness => "_Roughness",
            Self::Metallic => "_Metallic",
            Self::Normal => "_Normal",
        }
    }
}

/// Whether a texture set's realism channels go out: it uses them, or a
/// layer changes one (decision 0207).
fn uses_realism(set: &paint::TextureSet) -> bool {
    set.realism || paint::channel::any_channels(&set.layers)
}

/// Whether a layer of `layers` (or their groups) changes the height
/// (decision 0211).
fn has_height(layers: &[paint::Layer]) -> bool {
    layers.iter().any(|layer| {
        layer.channels.height.is_some() || layer.children().is_some_and(|c| has_height(c))
    })
}

/// Whether a layer of `layers` (or their groups) glows.
fn glows(layers: &[paint::Layer]) -> bool {
    layers
        .iter()
        .any(|layer| layer.channels.glow.is_some() || layer.children().is_some_and(|c| glows(c)))
}

/// The maps a painted set goes out as, and whether each goes inside the
/// file (a glTF's own) rather than beside it.
fn maps_of(set: &paint::TextureSet, packing: ChannelPacking, glb: bool) -> Vec<(TextureMap, bool)> {
    let mut maps = vec![(TextureMap::Colour, glb)];
    if !uses_realism(set) {
        return maps;
    }
    if glb {
        maps.push((TextureMap::Orm, true));
    }
    match packing {
        ChannelPacking::Standard if !glb => maps.push((TextureMap::Orm, false)),
        ChannelPacking::Standard => {}
        ChannelPacking::Orm => maps.push((TextureMap::Orm, false)),
        ChannelPacking::UnityMask => maps.push((TextureMap::MaskMap, false)),
        ChannelPacking::Separate => {
            maps.push((TextureMap::Roughness, false));
            maps.push((TextureMap::Metallic, false));
        }
    }
    if glows(&set.layers) {
        maps.push((TextureMap::Emissive, glb));
    }
    // The height's bumps as a normal map (decision 0211): inside a glTF
    // (and beside it too when maps are asked for beside), beside an OBJ.
    if has_height(&set.layers) {
        maps.push((TextureMap::Normal, glb));
        if glb && packing != ChannelPacking::Standard {
            maps.push((TextureMap::Normal, false));
        }
    }
    maps
}

/// How a normal map is made (decision 0211): how big a texel is on the
/// surface, and whether green points down (DirectX, as Unreal Engine
/// wants) rather than up (OpenGL: glTF, Godot, Unity, Blender).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NormalStyle {
    /// Centimetres of surface a texel covers (from the texel density).
    pub texel_cm: f32,
    /// Whether green points down.
    pub green_down: bool,
}

/// One texture going out.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportTexture {
    /// The material it belongs to.
    pub material: String,
    /// Which of its maps it is (decision 0208).
    pub map: TextureMap,
    /// Whether it goes inside the file (a glTF's own maps) rather than
    /// beside it.
    pub inside: bool,
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
    /// Which of its maps it is (decision 0208).
    pub map: TextureMap,
    /// Whether it goes inside the file rather than beside it.
    pub inside: bool,
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
///
/// With the realism channels (decision 0208), each material's other maps
/// too: inside a glTF (`glb`) its ORM and glow, and beside the file what
/// `packing` asks for.
pub fn plan(
    document: &Document,
    objects: &[ExportedObject],
    format: TextureFormat,
    stem: &str,
    packing: ChannelPacking,
    glb: bool,
) -> Vec<TextureReport> {
    let Some(ext) = format.extension() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for m in file_materials(objects) {
        let Some(set) = document.scene().paint().set(&m.name) else {
            continue;
        };
        for (map, inside) in maps_of(set, packing, glb) {
            // A glTF's own maps are PNG inside it; beside it, the format
            // asked for.
            let format = if inside { TextureFormat::Png } else { format };
            let ext = format.extension().unwrap_or(ext);
            out.push(TextureReport {
                file_name: format!("{stem}_{}{}.{ext}", file_safe(&m.name), map.suffix()),
                material: m.name.clone(),
                map,
                inside,
                size: set.size,
                format,
                channels: 4,
                file_bytes: 0,
                gpu_bytes: gpu_bytes(set.size),
            });
        }
    }
    out
}

/// The textures of `objects`' painted materials in `document`, in
/// `format` (for glTF, PNG whatever is asked), padded by `padding`
/// texels round their UV pieces.
#[allow(clippy::too_many_arguments)]
pub fn make(
    document: &Document,
    objects: &[ExportedObject],
    format: TextureFormat,
    padding: u32,
    stem: &str,
    packing: ChannelPacking,
    glb: bool,
    green_down: bool,
) -> Vec<ExportTexture> {
    let materials = file_materials(objects);
    plan(document, objects, format, stem, packing, glb)
        .into_iter()
        .filter_map(|mut report| {
            let set = document.scene().paint().set(&report.material)?;
            let colour = materials
                .iter()
                .find(|m| m.name == report.material)
                .map_or([0.8; 3], |m| m.color);
            let size = set.size;
            // A glTF's own normal map is always green up, as glTF says.
            let style = NormalStyle {
                texel_cm: 100.0 / document.scene().texture().density.max(1.0),
                green_down: green_down && !report.inside,
            };
            let mut rgba = map_pixels(set, report.map, colour, style)?;
            let mut covered = coverage(objects, &report.material, size);
            pad(&mut rgba, &mut covered, size, padding.min(MAX_PADDING));
            let bytes = encode(&rgba, size, report.format)?;
            report.file_bytes = bytes.len();
            Some(ExportTexture {
                material: report.material.clone(),
                map: report.map,
                inside: report.inside,
                file_name: report.file_name.clone(),
                bytes,
                report,
            })
        })
        .collect()
}

/// `set`'s map `map` as opaque pixels, ready to pad and write: its colour
/// over the material's `colour`, or its realism channels mixed over
/// their standard values and packed as the map lays them out.
pub(crate) fn map_pixels(
    set: &paint::TextureSet,
    map: TextureMap,
    colour: [f32; 3],
    style: NormalStyle,
) -> Option<Vec<u8>> {
    use paint::Channel;
    if map == TextureMap::Colour {
        let composite = paint::Composite::of(set)?;
        return Some(flatten(composite.image(), colour));
    }
    let mixed = |channel: Channel| -> Option<paint::Image> {
        let view = paint::channel::channel_view(set, channel);
        Some(paint::Composite::of(&view)?.image().clone())
    };
    let size = set.size;
    if map == TextureMap::Normal {
        return Some(normal_map(&mixed(Channel::Height)?, style));
    }
    let rough = mixed(Channel::Roughness)?;
    let metal = mixed(Channel::Metal)?;
    let glow = if map == TextureMap::Emissive {
        Some(mixed(Channel::Glow)?)
    } else {
        None
    };
    let mut out = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let r = paint::channel::over_standard(Channel::Roughness, rough.pixel(x, y))[0];
            let m = paint::channel::over_standard(Channel::Metal, metal.pixel(x, y))[0];
            let pixel = match map {
                // No occlusion is baked into it yet: white, so it darkens
                // nothing (decision 0208).
                TextureMap::Orm => [255, r, m, 255],
                TextureMap::MaskMap => [m, 255, 0, 255 - r],
                TextureMap::Roughness => [r, r, r, 255],
                TextureMap::Metallic => [m, m, m, 255],
                TextureMap::Emissive => {
                    let g = glow.as_ref().map_or([0; 4], |g| g.pixel(x, y));
                    let a = u32::from(g[3]);
                    let lit = |c: u8| ((u32::from(c) * a + 127) / 255) as u8;
                    [lit(g[0]), lit(g[1]), lit(g[2]), 255]
                }
                TextureMap::Colour | TextureMap::Normal => [0; 4],
            };
            out.extend_from_slice(&pixel);
        }
    }
    Some(out)
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

/// The mixed height `height` (over its standard, level) as a
/// tangent-space normal map: each texel faces away from where the height
/// rises, by how fast it rises across the surface, red right (+U), green
/// up (+V, or down with `style.green_down`), blue out.
pub(crate) fn normal_map(height: &paint::Image, style: NormalStyle) -> Vec<u8> {
    use paint::Channel;
    let size = height.size();
    let h: Vec<f32> = (0..size * size)
        .map(|i| {
            let p = height.pixel(i % size, i / size);
            f32::from(paint::channel::over_standard(Channel::Height, p)[0]) / 255.0
        })
        .collect();
    let at = |x: i64, y: i64| {
        let c = |v: i64| v.clamp(0, size as i64 - 1) as u32;
        h[(c(y) * size + c(x)) as usize]
    };
    // Height 0 to 1 is HEIGHT_DEPTH centimetres; a step of two texels is
    // twice their width.
    let k = paint::channel::HEIGHT_DEPTH / (2.0 * style.texel_cm.max(1e-4));
    let mut out = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size as i64 {
        for x in 0..size as i64 {
            let du = (at(x + 1, y) - at(x - 1, y)) * k;
            // Rows run down the texture: V rises upward.
            let dv = (at(x, y - 1) - at(x, y + 1)) * k;
            let n = glam::Vec3::new(-du, -dv, 1.0).normalize();
            let green = if style.green_down { -n.y } else { n.y };
            let byte = |v: f32| ((v * 0.5 + 0.5) * 255.0).round().clamp(0.0, 255.0) as u8;
            out.extend_from_slice(&[byte(n.x), byte(green), byte(n.z), 255]);
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
