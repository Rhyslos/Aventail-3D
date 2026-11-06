//! UV → Texture picture ([decision 0244](../../../../docs/decisions/0244-seeing-textures.md)):
//! a picture of your own shown on the model by its UVs and behind the
//! UVs in the UV Editor, a mip level to see it as a game does from far
//! away, and the UV layout exported as a PNG or SVG to paint over in
//! another program.

use super::{Workbench, WorkbenchAction};
use crate::text::tr;
use crate::viewport::{MAX_MIP, Picture, Surface};
use egui::Ui;
use std::path::Path;

/// The submenu's name and what it is for.
pub(crate) const TEXTURE_PICTURE_LABEL: &str = crate::text!("Texture picture");
pub(crate) const TEXTURE_PICTURE_HELP: &str = crate::text!(
    "Show a picture of your own on the model and behind the UVs, see it as a game does from far \
     away, and export the UV layout to paint over in another program"
);
/// The menu's entries.
pub(crate) const CHOOSE_PICTURE_LABEL: &str = crate::text!("Choose picture…");
pub(crate) const CLEAR_PICTURE_LABEL: &str = crate::text!("Clear picture");
pub(crate) const EXPORT_PNG_LABEL: &str = crate::text!("Export UV layout as PNG…");
pub(crate) const EXPORT_SVG_LABEL: &str = crate::text!("Export UV layout as SVG…");

impl Workbench {
    /// The submenu.
    pub(super) fn uv_texture_entries(&mut self, ui: &mut Ui) {
        let shown = self.viewport.picture().map(|p| p.name.clone());
        match &shown {
            Some(name) => ui.label(crate::trf!("Showing {}", name)),
            None => ui.weak(tr("No picture chosen")),
        };
        if ui
            .button(tr(CHOOSE_PICTURE_LABEL))
            .on_hover_text(tr(
                "Pick an image (PNG, JPEG…) to lay on the model by its UVs: a texture you made \
                 elsewhere, to check that the UVs suit it",
            ))
            .clicked()
        {
            self.file_asks.push(WorkbenchAction::ChooseUvTexture { path: None });
            ui.close();
        }
        if ui
            .add_enabled(shown.is_some(), egui::Button::new(tr(CLEAR_PICTURE_LABEL)))
            .clicked()
        {
            self.clear_uv_texture();
            ui.close();
        }
        let options = self.viewport.options_mut();
        let mut mip = options.mip_level.min(MAX_MIP);
        ui.horizontal(|ui| {
            ui.label(tr("Mip level"));
            if ui
                .add(egui::Slider::new(&mut mip, 0..=MAX_MIP))
                .on_hover_text(tr(
                    "0 is the picture itself. Each level is half as sharp: how a game shows the \
                     texture from further and further away, so you can see whether small \
                     details, thin lines and pieces close together still read",
                ))
                .changed()
            {
                options.mip_level = mip;
            }
        });
        ui.separator();
        let size = self.document.scene().texture().size;
        for (label, svg) in [(EXPORT_PNG_LABEL, false), (EXPORT_SVG_LABEL, true)] {
            if ui
                .button(tr(label))
                .on_hover_text(crate::trf!(
                    "The selected objects' UV edges as a {} × {} picture, to paint over in \
                     another program",
                    size,
                    size
                ))
                .clicked()
            {
                self.file_asks.push(WorkbenchAction::ExportUvLayout { svg, path: None });
                ui.close();
            }
        }
    }

    /// Shows `image` (read from the file `name`) as the texture picture,
    /// and the viewport's Texture picture surface.
    pub fn set_uv_texture(&mut self, name: &str, image: &fileio::image_file::ReadImage) {
        let Some(picture) = Picture::square(name, image.width, image.height, &image.rgba) else {
            self.note = Some(tr("That picture couldn't be used.").to_string());
            return;
        };
        let size = picture.size;
        self.viewport.set_picture(Some(picture));
        self.viewport.options_mut().surface = Surface::Texture;
        self.note = Some(crate::trf!(
            "{} is laid on the model by its UVs ({} × {} px). Display → Surface changes back.",
            name,
            size,
            size
        ));
    }

    /// Stops showing the texture picture.
    pub(crate) fn clear_uv_texture(&mut self) {
        self.viewport.set_picture(None);
        self.note = Some(tr("The texture picture is cleared.").to_string());
    }

    /// The selected objects' UV edges, in the texture's units, for the
    /// layout export.
    pub fn uv_layout_lines(&mut self) -> Vec<[glam::Vec2; 2]> {
        self.uv_editor.layout_lines(&self.document)
    }

    /// The layout export's file contents: SVG text or PNG bytes, at the
    /// project's texture size; `None` (and a note) when nothing selected
    /// has UVs.
    pub fn uv_layout_file(&mut self, svg: bool) -> Option<Vec<u8>> {
        let lines = self.uv_layout_lines();
        if lines.is_empty() {
            self.note = Some(tr("Export UV layout: select objects with UVs first.").to_string());
            return None;
        }
        let size = self.document.scene().texture().size;
        let width = (size as f32 / 1024.0).max(1.0);
        if svg {
            Some(fileio::uv_layout::layout_svg(&lines, size, width).into_bytes())
        } else {
            match fileio::uv_layout::layout_png(&lines, size, width) {
                Ok(bytes) => Some(bytes),
                Err(why) => {
                    self.note = Some(crate::trf!("Couldn't draw the UV layout: {}.", why));
                    None
                }
            }
        }
    }

    /// Says how the layout export went.
    pub fn uv_layout_exported(&mut self, path: &Path, result: Result<(), String>) {
        self.note = Some(match result {
            Ok(()) => crate::trf!(
                "Exported the UV layout to {}.",
                super::files::file_name(path)
            ),
            Err(why) => crate::trf!(
                "Couldn't export the UV layout to {}: {}.",
                super::files::file_name(path),
                why
            ),
        });
    }
}
