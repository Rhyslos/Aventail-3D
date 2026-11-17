//! The Scripts panel's output (decision 0301): what scripts printed, their
//! warnings and mistakes, how each run ended, and the console's lines. A
//! mistake in the script being edited is clicked to go to its line.

use super::{Row, ScriptsPanel};
use crate::text::tr;
use egui::{RichText, Ui};
use script::Line;

impl ScriptsPanel {
    /// Draws the output.
    pub(crate) fn show_output(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr("Output")).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(!self.rows.is_empty(), egui::Button::new(tr("Clear")))
                    .on_hover_text(tr("Clear the output"))
                    .clicked()
                {
                    self.rows.clear();
                }
            });
        });
        if self.rows.is_empty() {
            ui.label(
                RichText::new(tr(
                    "What scripts print, and their mistakes, show here. Run the script you're editing with Ctrl + Enter, or one from a file with Scripts → Run a script file…",
                ))
                .weak(),
            );
            return;
        }
        let warning = ui.visuals().warn_fg_color;
        let mistake = ui.visuals().error_fg_color;
        let editing = self.editor.as_ref().map(|e| e.file_name.clone());
        let mut go_to = None;
        egui::ScrollArea::vertical()
            .id_salt(ui.id().with(r"script output"))
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for row in &self.rows {
                    let text = match row {
                        Row::Started(name) => {
                            RichText::new(crate::trf!("Running {}", name)).strong()
                        }
                        Row::Typed(line) => RichText::new(format!("> {line}")).monospace().weak(),
                        Row::Line(Line::Said(text)) => RichText::new(text).monospace(),
                        Row::Line(Line::Warning(text)) => {
                            RichText::new(crate::trf!("Warning: {}", text)).color(warning)
                        }
                        Row::Line(Line::Note(text)) => RichText::new(text).weak(),
                        Row::Line(Line::Mistake(problem)) => {
                            let here = problem.line > 0
                                && editing.as_deref() == Some(problem.file.as_str());
                            let text = RichText::new(problem.to_string()).color(mistake);
                            if here {
                                let link = ui
                                    .add(
                                        egui::Label::new(text.underline())
                                            .wrap()
                                            .sense(egui::Sense::click()),
                                    )
                                    .on_hover_text(tr("Click to go to this line in the editor"));
                                if link.clicked() {
                                    go_to = Some(problem.line);
                                }
                                continue;
                            }
                            text
                        }
                        Row::Ended(words, true) => RichText::new(words).weak(),
                        Row::Ended(words, false) => RichText::new(words).color(mistake),
                    };
                    ui.add(egui::Label::new(text).wrap());
                }
            });
        if let (Some(line), Some(editor)) = (go_to, self.editor.as_mut()) {
            editor.go_to(line);
        }
    }
}
