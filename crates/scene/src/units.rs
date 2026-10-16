//! Settings saved with a project: its units ([decision 0068](../../../docs/decisions/0068-pivot-editing-and-scene-units.md)),
//! the length every size in the project is shown and typed in (the scene
//! itself always stores centimetres; the units only change how lengths
//! read), its game project folder ([decision 0076](../../../docs/decisions/0076-engine-export-paths.md))
//! and its texture settings ([decision 0100](../../../docs/decisions/0100-texture-size-and-texel-density.md)).

/// The length a project is measured in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Units {
    /// Metres: 1 m, 2.5 m. The standard; what Godot and Unity use.
    #[default]
    Metres,
    /// Centimetres: 100 cm, 250 cm. What Unreal uses.
    Centimetres,
}

impl Units {
    /// The name, in lower case, for undo steps: "metres".
    pub fn name(self) -> &'static str {
        match self {
            Self::Metres => "metres",
            Self::Centimetres => "centimetres",
        }
    }
}

/// Where a project exports to in a game project (decision 0076): a
/// folder inside a Godot, Unity or Unreal Engine project, and whether
/// every save exports there too. The scene only keeps the folder's path
/// as text; reading and writing files is `fileio`'s and the app's work.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExportTarget {
    /// The folder, as the computer names it.
    pub folder: String,
    /// Whether saving the project exports into the folder too.
    pub on_save: bool,
}

/// The longest folder path a project keeps, in bytes.
pub const MAX_FOLDER_LEN: usize = 4096;

impl ExportTarget {
    /// Whether the folder's path is one a project can keep: not empty,
    /// not too long, on one line.
    pub fn is_valid(&self) -> bool {
        !self.folder.trim().is_empty()
            && self.folder.len() <= MAX_FOLDER_LEN
            && !self.folder.chars().any(char::is_control)
    }
}

/// The texture the project's UVs are made for (decision 0100): its size
/// in pixels (square), and the texel density aimed for, in pixels per
/// metre. UV tools measure sizes and padding in these pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextureSettings {
    /// The texture's width and height, in pixels.
    pub size: u32,
    /// The texel density aimed for: pixels of texture per metre of
    /// surface.
    pub density: f32,
}

/// The texture sizes offered, in pixels.
pub const TEXTURE_SIZES: [u32; 7] = [256, 512, 1024, 2048, 4096, 8192, 16384];

/// The highest texel density a project keeps, in pixels per metre.
pub const MAX_DENSITY: f32 = 100_000.0;

impl Default for TextureSettings {
    fn default() -> Self {
        Self {
            size: 2048,
            density: 512.0,
        }
    }
}

impl TextureSettings {
    /// Whether the settings are ones a project can keep: a size from 16
    /// to 16384 pixels and a real density above 0 and at most
    /// [`MAX_DENSITY`].
    pub fn is_valid(&self) -> bool {
        (16..=16_384).contains(&self.size)
            && self.density.is_finite()
            && self.density > 0.0
            && self.density <= MAX_DENSITY
    }

    /// The density aimed for in the texture's own units: texture widths
    /// per scene unit (the scene stores centimetres).
    pub fn uv_per_unit(&self) -> f32 {
        self.density / self.size as f32 / 100.0
    }

    /// A density in texture widths per scene unit, in pixels per metre.
    pub fn pixels_per_metre(&self, uv_per_unit: f32) -> f32 {
        uv_per_unit * self.size as f32 * 100.0
    }
}

#[cfg(test)]
mod texture_tests {
    use super::*;

    #[test]
    fn texture_settings_convert_and_are_checked() {
        let t = TextureSettings::default();
        assert!(t.is_valid());
        // 512 px per metre on a 2048 texture: a quarter of the texture
        // per metre, so 1/400 of it per centimetre.
        assert!((t.uv_per_unit() - 0.0025).abs() < 1e-9);
        assert!((t.pixels_per_metre(t.uv_per_unit()) - 512.0).abs() < 1e-3);
        for bad in [
            TextureSettings { size: 8, ..t },
            TextureSettings { size: 40_000, ..t },
            TextureSettings { density: 0.0, ..t },
            TextureSettings {
                density: f32::NAN,
                ..t
            },
            TextureSettings {
                density: MAX_DENSITY * 2.0,
                ..t
            },
        ] {
            assert!(!bad.is_valid(), "{bad:?}");
        }
        assert!(
            TEXTURE_SIZES
                .iter()
                .all(|s| TextureSettings { size: *s, ..t }.is_valid())
        );
    }
}

#[cfg(test)]
mod texture_command_tests {
    use crate::{Command, Document, TextureSettings};

    #[test]
    fn texture_settings_change_as_one_undoable_step_and_bad_ones_are_refused() {
        let mut doc = Document::new();
        assert_eq!(doc.scene().texture(), TextureSettings::default());
        let wanted = TextureSettings {
            size: 1024,
            density: 256.0,
        };
        doc.execute(Command::SetTexture(wanted)).unwrap();
        assert_eq!(doc.scene().texture(), wanted);
        doc.undo();
        assert_eq!(doc.scene().texture(), TextureSettings::default());
        doc.redo();
        assert_eq!(doc.scene().texture(), wanted);
        let steps = doc.undo_steps();
        assert!(
            doc.execute(Command::SetTexture(TextureSettings {
                size: 0,
                density: 256.0
            }))
            .is_err()
        );
        // The same again records nothing.
        doc.execute(Command::SetTexture(wanted)).unwrap();
        assert_eq!(doc.undo_steps(), steps);
    }
}
