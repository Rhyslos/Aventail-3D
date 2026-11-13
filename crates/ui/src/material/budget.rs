//! The texture budget for a graph's material ([decision 0260](../../../../docs/decisions/0260-preview-and-checks.md)):
//! the maps it will go out as, what they take on the graphics card at the
//! chosen size, how sharp that is at its real-world size, and plain-words
//! warnings.
//!
//! It counts the way the Export window's texture budget does (decision
//! 0204): the same maps for the export as it is set up now
//! (`fileio::export::maps_going_out`, decision 0208), and the same bytes
//! per map (`fileio::export::textures::gpu_bytes`: four bytes a pixel and
//! a third more for the smaller copies games make).

use crate::text::tr;
use crate::workbench::ExportSettings;
use fileio::export::graph_textures::CardMaps;
use fileio::export::{ExportFormat, MapsUsed, TextureFormat, TextureMap, maps_going_out};

/// The sizes offered, in pixels across: a graph's sizes (`matgraph::Size`,
/// up to 4096) that games use.
pub(crate) const SIZES: [u32; 5] = [256, 512, 1024, 2048, 4096];

/// Above this much on the graphics card, one material is a lot.
pub(crate) const HEAVY_BYTES: usize = 128 * 1024 * 1024;

/// A material's texture budget.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Budget {
    /// Pixels across each map.
    pub(crate) size: u32,
    /// The maps going out, and whether each goes inside the file.
    pub(crate) maps: Vec<(TextureMap, bool)>,
    /// What one map takes on the graphics card.
    pub(crate) each: usize,
    /// What they all take.
    pub(crate) total: usize,
    /// Pixels of texture per metre of surface at the material's real
    /// size.
    pub(crate) pixels_per_metre: f32,
    /// The texel density the project aims for, in pixels per metre.
    pub(crate) aimed: f32,
    /// Whether the export is set to leave textures out.
    pub(crate) no_textures: bool,
    /// Whether it is pixel art (decision 0287): `size` is its big pixels
    /// across, and how sharp it is was chosen on purpose.
    pub(crate) pixel_art: bool,
}

impl Budget {
    /// The budget of a material `real_size_cm` across at `size` pixels,
    /// with an alpha card's `card` maps (decision 0276), for the export as
    /// `export` sets it up, in a project aiming for `aimed` pixels per
    /// metre.
    pub(crate) fn of(
        size: u32,
        real_size_cm: f32,
        card: CardMaps,
        export: ExportSettings,
        aimed: f32,
    ) -> Self {
        let choice = export.choice();
        let glb = choice.format == ExportFormat::Glb;
        // A graph's material always has roughness, metal and a height (its
        // normal map); nothing in it glows yet. An alpha card adds its maps.
        let used = MapsUsed {
            realism: true,
            height: true,
            glow: false,
            opacity: card.opacity,
            translucency: card.translucency,
        };
        let maps = maps_going_out(used, choice.packing, glb);
        let each = fileio::export::textures::gpu_bytes(size);
        let real = real_size_cm.max(matgraph::MIN_REAL_SIZE_CM);
        Self {
            size,
            total: each * maps.len(),
            maps,
            each,
            pixels_per_metre: size as f32 / real * 100.0,
            aimed,
            no_textures: choice.textures == TextureFormat::None,
            pixel_art: false,
        }
    }

    /// The same budget for pixel art (decision 0287): its `size` is the
    /// big pixels across, written one texel each. Its sharpness is the
    /// look chosen, so it isn't compared with the project's aim.
    pub(crate) fn as_pixel_art(self) -> Self {
        Self {
            pixel_art: true,
            ..self
        }
    }

    /// How many centimetres one pixel covers on the surface.
    pub(crate) fn cm_per_pixel(&self) -> f32 {
        100.0 / self.pixels_per_metre.max(1e-6)
    }

    /// The maps in a line: "colour, ORM, normal map".
    pub(crate) fn maps_line(&self) -> String {
        self.maps
            .iter()
            .map(|(map, _)| map_word(*map))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The size whose sharpness is nearest the project's texel density,
    /// from [`SIZES`].
    pub(crate) fn matching_size(&self) -> u32 {
        let wanted = self.size as f32 * self.aimed / self.pixels_per_metre.max(1e-6);
        SIZES
            .into_iter()
            .min_by(|a, b| {
                let off = |s: u32| (s as f32 / wanted).log2().abs();
                off(*a).total_cmp(&off(*b))
            })
            .unwrap_or(SIZES[2])
    }

    /// Plain-words warnings: a material that takes a lot of memory, or is
    /// much blurrier or sharper than the project aims for.
    pub(crate) fn warnings(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.no_textures {
            out.push(
                tr(
                    "The export is set to leave textures out (Export → Textures): these maps \
                    would only go out once it sends them.",
                )
                .to_string(),
            );
        }
        if self.total > HEAVY_BYTES {
            out.push(crate::trf!(
                "{} is a lot for one material on the graphics card: most game materials are \
                 1024 or 2048 pixels, and the largest sizes are kept for what the player sees \
                 up close.",
                crate::format_bytes(self.total)
            ));
        }
        let matching = self.matching_size();
        if !self.pixel_art && self.aimed.is_finite() && self.aimed > 0.0 && matching != self.size {
            let ratio = self.pixels_per_metre / self.aimed;
            if ratio < 0.5 {
                out.push(crate::trf!(
                    "Blurrier than the project aims for ({} pixels a metre against {}): {} \
                     pixels would match it, or a smaller real-world size.",
                    self.pixels_per_metre.round(),
                    self.aimed.round(),
                    matching
                ));
            } else if ratio > 2.0 {
                out.push(crate::trf!(
                    "Sharper than the project needs ({} pixels a metre against {}): {} pixels \
                     would match it and take less memory.",
                    self.pixels_per_metre.round(),
                    self.aimed.round(),
                    matching
                ));
            }
        }
        out
    }
}

/// How a pixel-art graph (decision 0287) `n` big pixels a repeat, laid
/// `repeats` times across a painted texture `size` pixels across, comes
/// out there, in plain words: one texel a pixel, or how many, and the
/// texture size that would give one.
pub(crate) fn pixel_note(n: u32, repeats: f32, size: u32) -> String {
    let exact = n as f32 * repeats;
    if !exact.is_finite() || exact <= 0.0 {
        return String::new();
    }
    let per = size as f32 / exact;
    let shown = |v: f32| (v * 10.0).round() / 10.0;
    if (per - 1.0).abs() < 0.01 {
        crate::trf!(
            "Pixel art: {} pixels a repeat, laid {} times across this {}-pixel texture: one \
             texel a pixel, exactly as made.",
            n,
            shown(repeats),
            size
        )
    } else {
        crate::trf!(
            "Pixel art: {} pixels a repeat, laid {} times across this {}-pixel texture: about \
             {} texels a pixel, laid crisp. A texture {} pixels across would give one texel a \
             pixel (or change the Tiling or the graph's real-world size). Turn on Export → \
             Pixel art → Crisp pixels so the game keeps them sharp.",
            n,
            shown(repeats),
            size,
            shown(per),
            exact.round()
        )
    }
}

/// A map's name, as the Export window's texture budget words it.
pub(crate) fn map_word(map: TextureMap) -> &'static str {
    match map {
        TextureMap::Colour => tr("colour"),
        TextureMap::Orm => tr("ORM"),
        TextureMap::Emissive => tr("glow"),
        TextureMap::MaskMap => tr("mask map"),
        TextureMap::Roughness => tr("roughness"),
        TextureMap::Metallic => tr("metal"),
        TextureMap::Normal => tr("normal map"),
        TextureMap::Height => tr("height"),
        TextureMap::Occlusion => tr("occlusion"),
        TextureMap::Opacity => tr("opacity"),
        TextureMap::Translucency => tr("translucency"),
        TextureMap::Thickness => tr("thickness"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pixel_art_fill_says_how_its_pixels_land_on_the_texture() {
        let exact = pixel_note(32, 2.0, 64);
        assert!(exact.contains("one texel a pixel, exactly"), "{exact}");
        let many = pixel_note(32, 2.0, 1024);
        assert!(many.contains("about 16 texels a pixel"), "{many}");
        assert!(many.contains("A texture 64 pixels across"), "{many}");
        assert!(pixel_note(32, f32::NAN, 64).is_empty());
        assert!(pixel_note(32, 0.0, 64).is_empty());
    }

    #[test]
    fn pixel_art_counts_its_big_pixels_and_isnt_called_blurry() {
        // 32 big pixels over 2 m: 16 a metre, each 6.25 cm across.
        let b = Budget::of(32, 200.0, CardMaps::default(), godot(), 512.0).as_pixel_art();
        assert_eq!(b.size, 32);
        assert!((b.pixels_per_metre - 16.0).abs() < 1e-4);
        assert!((b.cm_per_pixel() - 6.25).abs() < 1e-4);
        assert_eq!(b.each, fileio::export::textures::gpu_bytes(32));
        assert!(b.warnings().is_empty(), "{:?}", b.warnings());
        // The same size smooth is far blurrier than the project aims for.
        let smooth = Budget::of(32, 200.0, CardMaps::default(), godot(), 512.0);
        assert_eq!(smooth.warnings().len(), 1);
    }
    use crate::workbench::ExportChoice;
    use fileio::export::ChannelPacking;

    fn godot() -> ExportSettings {
        ExportSettings::default()
    }

    #[test]
    fn a_material_for_godot_goes_out_as_colour_orm_and_normal() {
        let b = Budget::of(2048, 200.0, CardMaps::default(), godot(), 512.0);
        assert_eq!(b.maps_line(), "colour, ORM, normal map");
        assert_eq!(b.each, fileio::export::textures::gpu_bytes(2048));
        assert_eq!(b.total, 3 * b.each);
        // 2048 pixels over 2 m.
        assert!((b.pixels_per_metre - 1024.0).abs() < 1e-3);
        assert!(!b.no_textures);
    }

    #[test]
    fn the_maps_follow_the_export_settings() {
        let mut choice = ExportChoice {
            preset: None,
            ..godot().choice()
        };
        choice.packing = ChannelPacking::Separate;
        let b = Budget::of(
            1024,
            100.0,
            CardMaps::default(),
            ExportSettings::of(choice),
            512.0,
        );
        // Inside the glTF: colour, ORM, normal; beside it: each on its own.
        assert_eq!(
            b.maps_line(),
            "colour, ORM, colour, roughness, metal, height, normal map, normal map"
        );
        choice.format = ExportFormat::Obj;
        choice.packing = ChannelPacking::Standard;
        let b = Budget::of(
            1024,
            100.0,
            CardMaps::default(),
            ExportSettings::of(choice),
            512.0,
        );
        assert_eq!(b.maps_line(), "colour, ORM, normal map");
        choice.textures = TextureFormat::None;
        let b = Budget::of(
            1024,
            100.0,
            CardMaps::default(),
            ExportSettings::of(choice),
            512.0,
        );
        assert!(b.warnings()[0].contains("leave textures out"));
    }

    #[test]
    fn warnings_say_when_it_is_heavy_blurry_or_sharper_than_needed() {
        // Just right: 1024 pixels over 2 m is 512 a metre.
        let right = Budget::of(1024, 200.0, CardMaps::default(), godot(), 512.0);
        assert!(right.warnings().is_empty(), "{:?}", right.warnings());
        assert_eq!(right.matching_size(), 1024);
        // 4096 over 2 m: heavy and four times sharper than aimed for.
        let heavy = Budget::of(4096, 200.0, CardMaps::default(), godot(), 512.0);
        let words = heavy.warnings();
        assert!(words.iter().any(|w| w.contains("a lot for one material")));
        assert!(
            words
                .iter()
                .any(|w| w.starts_with("Sharper") && w.contains("1024 pixels"))
        );
        // 256 over 4 m: blurry.
        let blurry = Budget::of(256, 400.0, CardMaps::default(), godot(), 512.0);
        assert!(
            blurry
                .warnings()
                .iter()
                .any(|w| w.starts_with("Blurrier") && w.contains("2048"))
        );
        // A damaged density or size never panics or warns about it.
        let odd = Budget::of(1024, f32::NAN, CardMaps::default(), godot(), f32::NAN);
        let _ = odd.warnings();
        let zero = Budget::of(1024, 0.0, CardMaps::default(), godot(), 0.0);
        assert!(zero.pixels_per_metre.is_finite());
    }

    #[test]
    fn an_alpha_card_counts_its_light_through_and_its_separate_opacity() {
        let card = CardMaps {
            opacity: true,
            translucency: true,
        };
        let b = Budget::of(1024, 100.0, card, godot(), 512.0);
        assert_eq!(b.maps_line(), "colour, ORM, normal map, translucency");
        assert_eq!(b.total, 4 * b.each);
        let mut choice = godot().choice();
        choice.preset = None;
        choice.format = ExportFormat::Obj;
        choice.packing = ChannelPacking::Separate;
        let b = Budget::of(1024, 100.0, card, ExportSettings::of(choice), 512.0);
        assert!(
            b.maps_line().ends_with("opacity, translucency"),
            "{}",
            b.maps_line()
        );
        choice.packing = ChannelPacking::UnityMask;
        let b = Budget::of(1024, 100.0, card, ExportSettings::of(choice), 512.0);
        assert!(b.maps_line().ends_with("thickness"), "{}", b.maps_line());
    }
}
