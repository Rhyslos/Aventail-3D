//! The Attributes panel's Selected edges section
//! ([decision 0143](../../../../docs/decisions/0143-subdivide-and-creases.md)):
//! in the Edges mode, the selected edges' crease, 0 to 1, for the
//! Subdivide modifier. Changing it asks the workbench to record the
//! crease as an edit (one drag is one edit and one undo step).

use super::{AttributesPanel, LABEL_WIDTH, label, section, stepper};
use crate::text::tr;
use crate::theme;
use egui::{RichText, Ui, vec2};

/// The selected edges' creases, as the workbench sees them each frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SelectedCreases {
    /// How many edges are selected.
    pub(crate) edges: usize,
    /// Their crease (the first edge's, when they differ).
    pub(crate) crease: f32,
    /// Whether the edges have different creases.
    pub(crate) mixed: bool,
}

/// The Crease field's accessible name.
pub(crate) const CREASE_LABEL: &str = crate::text!("Crease");

impl AttributesPanel {
    /// The Selected edges section, in the Edges mode with edges selected.
    pub(super) fn crease_section(&mut self, ui: &mut Ui) {
        let Some(selected) = self.creases else {
            return;
        };
        if selected.edges == 0 {
            return;
        }
        section(ui, &crate::trf!("Selected edges: {}", selected.edges));
        let mut crease = selected.crease;
        let mut touched = stepper::Touched::default();
        egui::Grid::new("attributes_creases")
            .num_columns(2)
            .min_col_width(LABEL_WIDTH)
            .spacing(vec2(12.0, 6.0))
            .show(ui, |ui| {
                label(
                    ui,
                    tr(CREASE_LABEL),
                    tr(
                        "How sharply the Subdivide modifier keeps these edges: 0 smooths them \
                         like any other, 1 keeps them fully sharp, 0.5 sharp for half the levels",
                    ),
                );
                touched = stepper::number(ui, &mut crease, (0.0, 1.0), (0.05, 2), tr(CREASE_LABEL));
                ui.end_row();
            });
        if selected.mixed {
            ui.label(
                RichText::new(tr(
                    "The edges have different creases: a new value gives them all the same.",
                ))
                .small()
                .color(theme::TEXT_WEAK),
            );
        }
        if touched.began {
            self.gesture += 1;
        }
        if touched.changed {
            self.crease_asked = Some((crease.clamp(0.0, 1.0), self.gesture));
        }
    }
}
