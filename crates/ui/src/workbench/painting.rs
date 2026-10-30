//! Painting on the workbench ([decision 0177](../../../../docs/decisions/0177-texture-painter-plan.md)):
//! images imported as paint layers on a material's texture set, and
//! palette files read and written (decision 0183).

use super::Workbench;
use crate::text::tr;
use paint::{Image, Layer, TextureSet};
use std::path::Path;

impl Workbench {
    /// Adds `image`, read from `path`, as a paint layer on top of the
    /// texture set of the material `material` (made at the project's
    /// texture size if it has none), stretched to the set's size; one undo
    /// step. Shows the Painted surface, so the result is seen at once.
    pub fn import_paint_image(
        &mut self,
        path: &Path,
        material: &str,
        image: &fileio::image_file::ReadImage,
    ) {
        let material = &self.own_for_change(material);
        let scene = self.document.scene();
        let mut set = match scene.paint().set(material) {
            Some(set) => (**set).clone(),
            None => match TextureSet::new(scene.texture().size) {
                Some(set) => set,
                None => return self.import_failed(path, tr("the texture size is out of range")),
            },
        };
        let Some(pixels) = Image::from_rgba(set.size, image.width, image.height, &image.rgba)
        else {
            return self.import_failed(path, tr("its pixels don't match its size"));
        };
        if set.layers.len() >= paint::set::MAX_LAYERS {
            return self.import_failed(path, tr("the texture set has as many layers as it can"));
        }
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| tr("Image").to_string());
        let stem: String = stem.chars().take(paint::set::MAX_NAME - 8).collect();
        let name = set.free_name(&stem);
        set.layers.push(Layer::paint(name.clone(), pixels));
        let label = crate::trf!("Import {} onto {}", super::files::file_name(path), material);
        let command = scene::Command::SetTextureSet {
            material: material.to_string(),
            set: Some(set),
        };
        match self.document.execute_labelled(command, label, None) {
            Ok(()) => {
                self.viewport.options_mut().surface = crate::viewport::Surface::Painted;
                self.note = Some(crate::trf!(
                    "Imported {} as the paint layer {} on {}. Display → Surface → Painted shows \
                     it.",
                    super::files::file_name(path),
                    name,
                    material
                ));
            }
            Err(why) => self.import_failed(path, &why.to_string()),
        }
    }

    /// Adds the palette in `text` (a GIMP palette file read from `path`)
    /// to the project's palettes, one undo step.
    pub fn import_palette(&mut self, path: &Path, text: &str) {
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| tr("Palette").to_string());
        let Some(palette) = paint::palette::Palette::from_gpl(text, &stem) else {
            return self.import_failed(path, tr("it isn't a GIMP palette file"));
        };
        let mut palettes = self.document.scene().paint().palettes().to_vec();
        if palettes.len() >= paint::palette::MAX_PALETTES {
            return self.import_failed(path, tr("the project has as many palettes as it can"));
        }
        let name = palette.name.clone();
        palettes.push(palette);
        let shown = palettes.len() - 1;
        let label = crate::trf!("Import {}", super::files::file_name(path));
        match self
            .document
            .execute_labelled(scene::Command::SetPalettes(palettes), label, None)
        {
            Ok(()) => {
                self.attributes.colour.palette = shown;
                self.note = Some(crate::trf!("Imported the palette {}.", name));
            }
            Err(why) => self.import_failed(path, &why.to_string()),
        }
    }

    /// The project's palette number `index` as a GIMP palette file's
    /// text, with its name for the file, if there is one.
    pub fn palette_file(&self, index: usize) -> Option<(String, String)> {
        let palette = self.document.scene().paint().palettes().get(index)?;
        Some((palette.name.clone(), palette.to_gpl()))
    }

    /// Says the palette was written to `path`, or why it wasn't.
    pub fn palette_exported(&mut self, path: &Path, result: Result<(), String>) {
        self.note = Some(match result {
            Ok(()) => crate::trf!("Exported the palette to {}.", super::files::file_name(path)),
            Err(why) => crate::trf!(
                "Couldn't export the palette to {}: {}.",
                super::files::file_name(path),
                why
            ),
        });
    }
}
