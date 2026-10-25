//! The Attributes panel's Selected edges section
//! ([decision 0143](../../../../docs/decisions/0143-subdivide-and-creases.md),
//! [decision 0149](../../../../docs/decisions/0149-live-bevel.md)): in
//! the Edges mode, the selected edges' crease (for the Subdivide modifier)
//! and bevel weight (for the Live bevel modifier), each 0 to 1. Changing
//! one asks the workbench to record it as an edit (one drag is one edit
//! and one undo step).

use super::{AttributesPanel, LABEL_WIDTH, label, section, stepper};
use crate::edge_values::EdgeValue;
use crate::text::tr;
use crate::theme;
use egui::{RichText, Ui, vec2};

/// The selected edges' value of one kind, as the workbench sees it each
/// frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SelectedEdgeValue {
    /// How many edges are selected.
    pub(crate) edges: usize,
    /// Their value (the first edge's, when they differ).
    pub(crate) value: f32,
    /// Whether the edges have different values.
    pub(crate) mixed: bool,
}

/// The selected edges' crease and bevel weight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SelectedEdges {
    /// Their creases.
    pub(crate) crease: SelectedEdgeValue,
    /// Their bevel weights.
    pub(crate) bevel_weight: SelectedEdgeValue,
}

impl SelectedEdges {
    /// The value of one kind.
    fn get(&self, what: EdgeValue) -> SelectedEdgeValue {
        match what {
            EdgeValue::Crease => self.crease,
            EdgeValue::BevelWeight => self.bevel_weight,
        }
    }
}

/// A field's tooltip.
fn help(what: EdgeValue) -> &'static str {
    match what {
        EdgeValue::Crease => tr(
            "How sharply the Subdivide modifier keeps these edges: 0 smooths them like any \
             other, 1 keeps them fully sharp, 0.5 sharp for half the levels",
        ),
        EdgeValue::BevelWeight => tr(
            "Which edges a Live bevel set to Marked edges bevels, and how wide: 0 not at all, 1 \
             its full width, 0.5 half of it",
        ),
    }
}

impl AttributesPanel {
    /// The Selected edges section, in the Edges mode with edges selected.
    pub(super) fn crease_section(&mut self, ui: &mut Ui) {
        let Some(selected) = self.selected_edges else {
            return;
        };
        if selected.crease.edges == 0 {
            return;
        }
        section(
            ui,
            &crate::trf!("Selected edges: {}", selected.crease.edges),
        );
        let mut asked = None;
        egui::Grid::new("attributes_creases")
            .num_columns(2)
            .min_col_width(LABEL_WIDTH)
            .spacing(vec2(12.0, 6.0))
            .show(ui, |ui| {
                for what in EdgeValue::ALL {
                    let mut value = selected.get(what).value;
                    label(ui, what.name(), help(what));
                    let touched =
                        stepper::number(ui, &mut value, (0.0, 1.0), (0.05, 2), what.name());
                    ui.end_row();
                    if touched.began || touched.changed {
                        asked = Some((what, value, touched));
                    }
                }
            });
        let mixed: Vec<&str> = EdgeValue::ALL
            .into_iter()
            .filter(|w| selected.get(*w).mixed)
            .map(EdgeValue::name)
            .collect();
        if !mixed.is_empty() {
            ui.label(
                RichText::new(crate::trf!(
                    "The edges have different values ({}): a new value gives them all the same.",
                    mixed.join(", ")
                ))
                .small()
                .color(theme::TEXT_WEAK),
            );
        }
        if let Some((what, value, touched)) = asked {
            if touched.began {
                self.gesture += 1;
            }
            if touched.changed {
                self.edge_value_asked = Some((what, value.clamp(0.0, 1.0), self.gesture));
            }
        }
    }
}
