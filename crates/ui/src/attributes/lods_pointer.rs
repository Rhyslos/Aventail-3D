//! The Attributes panel's Levels of detail line (decision 0165): the
//! levels moved to the Levels panel, a tab beside Attributes; this says
//! how many the object has and opens that panel.

use super::{AttributesPanel, section};
use crate::text::tr;
use crate::theme;
use egui::{RichText, Ui};
use scene::{Document, ObjectId};

/// The button that brings the Levels panel to the front.
pub(crate) const OPEN_LEVELS: &str = crate::text!("Open the Levels panel");

impl AttributesPanel {
    /// The Levels of detail section: one line and a button.
    pub(super) fn lods_section(&mut self, ui: &mut Ui, doc: &Document, id: ObjectId) {
        let Some(object) = doc.scene().object(id) else {
            return;
        };
        section(ui, tr("Levels of detail"));
        ui.horizontal_wrapped(|ui| {
            let line = if object.lods.on {
                crate::trf!(
                    "{} levels, set in the Levels panel.",
                    object.lods.levels.len()
                )
            } else {
                tr("None yet: make them in the Levels panel.").to_string()
            };
            ui.label(RichText::new(line).color(theme::TEXT_WEAK));
            if ui
                .small_button(tr(OPEN_LEVELS))
                .on_hover_text(tr(
                    "The Levels panel, beside Attributes (Window → Levels), shows each level of \
                     detail and how it is made",
                ))
                .clicked()
            {
                self.show_levels = true;
            }
        });
    }
}
