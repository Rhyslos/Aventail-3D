//! The Blend seams tool's section in the Paint tab ([decision 0293](../../../../docs/decisions/0293-blend-seams.md)):
//! what the tool does, the Seam blend layer it marks seams on, its width
//! and the buttons that mark every seam that shows or unmark them all.

use super::{AttributesPanel, section};
use crate::dialog::help;
use crate::paint_layers::seam_rows;
use crate::text::tr;
use egui::{RichText, Ui};

/// The Blend seams tool's keys and clicks, as the Keys section lists them.
const SEAM_KEYS: &[(&str, &str)] = &[
    ("9", crate::text!("Blend seams")),
    (
        crate::text!("Click"),
        crate::text!("Mark the seam under the mouse (in place of those marked)"),
    ),
    (
        crate::text!("Shift + click"),
        crate::text!("Mark one more seam, or unmark a marked one"),
    ),
];

impl AttributesPanel {
    /// The Blend seams section, in place of the brush's while the tool
    /// is in use.
    pub(crate) fn seam_section(&mut self, ui: &mut Ui) {
        section(ui, tr("Blend seams"));
        help(
            ui,
            tr(
                "A seam is where the model is cut open on the texture: the two sides lie apart, \
                 so paint rarely matches across them. Click a seam on the model, or either of its \
                 two lines in the UV Editor, and the texture on both sides is blended so they \
                 meet smoothly. The blend goes in a Seam blend layer on top, which remakes \
                 itself when the paint under it changes.",
            ),
        );
        let (material, view) = &self.seam_tool;
        if !material.is_empty() {
            ui.label(
                RichText::new(crate::trf!("Material: {}", material))
                    .small()
                    .color(crate::theme::TEXT_WEAK),
            );
        }
        let view = *view;
        let mut dragging = false;
        if let Some(asked) = seam_rows::show(ui, &view, &mut dragging) {
            self.seam_asked = Some(asked);
        }
        self.seam_dragging = dragging;
        egui::Grid::new("seam_keys")
            .num_columns(2)
            .spacing(egui::vec2(12.0, 3.0))
            .show(ui, |ui| {
                for (key, what) in SEAM_KEYS {
                    ui.label(RichText::new(tr(key)).small().strong());
                    ui.label(
                        RichText::new(tr(what))
                            .small()
                            .color(crate::theme::TEXT_WEAK),
                    );
                    ui.end_row();
                }
            });
    }
}
