//! A script shown but not edited (decision 0307): a script kept with the
//! project, or one being installed. Coloured and its lines numbered as in
//! the editor; nothing in it runs, and it can't be typed into.

use super::code_view::{Coloured, gutter_width, paint_line_numbers};
use egui::Ui;
use script::{Language, ScriptFile};

/// A script being looked at.
pub(crate) struct Viewer {
    pub(crate) file: ScriptFile,
    language: Language,
    coloured: Coloured,
}

impl Viewer {
    /// A viewer for `file` (Lua if its ending says nothing).
    pub(crate) fn new(file: ScriptFile) -> Self {
        Self {
            language: file.language().unwrap_or(Language::Lua),
            file,
            coloured: Coloured::default(),
        }
    }

    /// Draws the script, filling the room given.
    pub(crate) fn show(&mut self, ui: &mut Ui) {
        let id = ui.id().with(r"script viewer").with(&self.file.name);
        let font = egui::TextStyle::Monospace.resolve(ui.style());
        let lines = self.file.text.lines().count().max(1);
        let gutter = gutter_width(ui, lines, &font);
        let Self {
            file,
            language,
            coloured,
        } = self;
        let language = *language;
        let mut layouter = |ui: &Ui, buffer: &dyn egui::TextBuffer, _wrap: f32| {
            coloured.galley(ui.ctx(), buffer.as_str(), language, &file.name, &[], &font)
        };
        let mut text = file.text.as_str();
        egui::ScrollArea::both()
            .id_salt(id.with(r"scroll"))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    let (gutter_rect, _) =
                        ui.allocate_exact_size(egui::vec2(gutter, 0.0), egui::Sense::hover());
                    let size = ui.available_size();
                    let output = egui::TextEdit::multiline(&mut text)
                        .id(id)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .min_size(size)
                        .frame(egui::Frame::NONE)
                        .layouter(&mut layouter)
                        .show(ui);
                    paint_line_numbers(
                        ui,
                        &output.galley,
                        output.galley_pos,
                        gutter_rect.right() - 6.0,
                        &font,
                    );
                });
            });
    }
}
