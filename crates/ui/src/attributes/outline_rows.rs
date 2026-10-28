//! The Outline rows in the Attributes panel ([decision 0201](../../../../docs/decisions/0201-outlines.md)):
//! whether the object has a cartoon outline, how thick and what colour.

use super::curve_rows::Rows;
use super::{AttributesPanel, label, stepper};
use crate::objects::DocRequest;
use crate::text::tr;
use crate::units::Units;
use egui::{Color32, Ui};
use scene::{Command, ObjectId, Outline};

/// The Outline switch's accessible name.
pub(crate) const OUTLINE_LABEL: &str = crate::text!("Draw an outline");

impl AttributesPanel {
    /// Outline, and its Width and Colour while on (in the Basic grid).
    pub(super) fn outline_rows(
        &mut self,
        ui: &mut Ui,
        id: ObjectId,
        object: &scene::Object,
        units: Units,
        out: &mut Vec<DocRequest>,
    ) {
        let mut rows = Rows::default();
        label(
            ui,
            tr("Outline"),
            tr(
                "A dark line round the object for a cartoon look: drawn in the viewport, and \
                 exported as an inside-out shell any engine shows (and a Godot shader)",
            ),
        );
        let mut on = object.outline.is_some();
        rows.toggle(ui, &mut on, tr(OUTLINE_LABEL));
        ui.end_row();
        let mut outline = if on {
            object.outline.unwrap_or_default()
        } else {
            Outline::default()
        };
        if on {
            label(
                ui,
                tr("Width"),
                tr("How thick the outline is. Thin details look best with a thin outline"),
            );
            rows.take(stepper::length(
                ui,
                &mut outline.width,
                (*Outline::WIDTHS.start(), *Outline::WIDTHS.end()),
                units,
                tr("Outline width"),
            ));
            ui.end_row();
            label(
                ui,
                tr("Colour"),
                tr("The outline's colour: near black reads as ink"),
            );
            let [r, g, b] = outline.colour;
            let mut c = Color32::from_rgb(r, g, b);
            let response = ui.color_edit_button_srgba(&mut c);
            if response.changed() {
                outline.colour = [c.r(), c.g(), c.b()];
                rows.changed = true;
                rows.began |= !self.dragging_colour;
            }
            self.dragging_colour = response.changed();
            ui.end_row();
        }
        if rows.began {
            self.gesture += 1;
        }
        let wanted = on.then_some(outline);
        if rows.changed && wanted != object.outline {
            out.push(DocRequest::RunMerging(
                Command::SetOutline {
                    ids: vec![id],
                    outline: wanted,
                },
                self.gesture,
            ));
        }
    }
}
