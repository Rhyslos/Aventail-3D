//! The Paint tab (decision 0213): the paint brush's settings in a panel of
//! their own, beside Attributes in the Paint layout, so Attributes is only
//! about the selected object while painting.

use super::AttributesPanel;
use crate::panels::placeholder;
use crate::text::tr;
use egui::Ui;
use scene::Document;

impl AttributesPanel {
    /// Draws the Paint tab: the brush's settings in Paint mode, otherwise
    /// a note saying where they come from.
    pub(crate) fn paint_panel(&mut self, ui: &mut Ui, doc: &Document) {
        if !self.painting {
            placeholder(
                ui,
                tr("Not painting"),
                tr(
                    "Switch to Paint mode (the Paint tab at the top right) and the brush's \
                    settings show up here.",
                ),
            );
            return;
        }
        self.trim_rows = !doc.scene().trim().rows.is_empty();
        egui::ScrollArea::vertical()
            .id_salt("paint_tab")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                self.brush_section(ui, doc.scene().paint().palettes());
            });
    }
}
