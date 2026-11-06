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
}

impl Budget {
    /// The budget of a material `real_size_cm` across at `size` pixels,
    /// for the export as `export` sets it up, in a project aiming for
    /// `aimed` pixels per metre.
    pub(crate) fn of(size: u32, real_size_cm: f32, export: ExportSettings, aimed: f32) -> Self {
        let choice = export.choice();
        let glb = choice.format == ExportFormat::Glb;
        // A graph's material always has roughness, metal and a height (its
        // normal map); nothing in it glows yet.
        let used = MapsUsed {
            realism: true,
            height: true,
            glow: false,
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
        }
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
        if self.aimed.is_finite() && self.aimed > 0.0 && matching != self.size {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::ExportChoice;
    use fileio::export::ChannelPacking;

    fn godot() -> ExportSettings {
        ExportSettings::default()
    }

    #[test]
    fn a_material_for_godot_goes_out_as_colour_orm_and_normal() {
        let b = Budget::of(2048, 200.0, godot(), 512.0);
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
        let b = Budget::of(1024, 100.0, ExportSettings::of(choice), 512.0);
        // Inside the glTF: colour, ORM, normal; beside it: each on its own.
        assert_eq!(
            b.maps_line(),
            "colour, ORM, colour, roughness, metal, height, normal map, normal map"
        );
        choice.format = ExportFormat::Obj;
        choice.packing = ChannelPacking::Standard;
        let b = Budget::of(1024, 100.0, ExportSettings::of(choice), 512.0);
        assert_eq!(b.maps_line(), "colour, ORM, normal map");
        choice.textures = TextureFormat::None;
        let b = Budget::of(1024, 100.0, ExportSettings::of(choice), 512.0);
        assert!(b.warnings()[0].contains("leave textures out"));
    }

    #[test]
    fn warnings_say_when_it_is_heavy_blurry_or_sharper_than_needed() {
        // Just right: 1024 pixels over 2 m is 512 a metre.
        let right = Budget::of(1024, 200.0, godot(), 512.0);
        assert!(right.warnings().is_empty(), "{:?}", right.warnings());
        assert_eq!(right.matching_size(), 1024);
        // 4096 over 2 m: heavy and four times sharper than aimed for.
        let heavy = Budget::of(4096, 200.0, godot(), 512.0);
        let words = heavy.warnings();
        assert!(words.iter().any(|w| w.contains("a lot for one material")));
        assert!(
            words
                .iter()
                .any(|w| w.starts_with("Sharper") && w.contains("1024 pixels"))
        );
        // 256 over 4 m: blurry.
        let blurry = Budget::of(256, 400.0, godot(), 512.0);
        assert!(
            blurry
                .warnings()
                .iter()
                .any(|w| w.starts_with("Blurrier") && w.contains("2048"))
        );
        // A damaged density or size never panics or warns about it.
        let odd = Budget::of(1024, f32::NAN, godot(), f32::NAN);
        let _ = odd.warnings();
        let zero = Budget::of(1024, 0.0, godot(), 0.0);
        assert!(zero.pixels_per_metre.is_finite());
    }
}
