//! The Struts modifier's rows in the Attributes panel
//! ([decision 0159](../../../../docs/decisions/0159-struts.md)): the
//! struts' thickness and sides, even thickness, and keeping the faces.

use super::curve_rows::Rows;
use super::{label, stepper};
use crate::text::tr;
use crate::units::Units;
use egui::Ui;
use scene::StrutsOptions;

/// Struts' settings (in a two-column grid).
pub(super) fn struts_rows(ui: &mut Ui, options: &mut StrutsOptions, units: Units, rows: &mut Rows) {
    label(
        ui,
        tr("Thickness"),
        tr("How wide each strut is. The joints are a little wider, to cover the struts' ends"),
    );
    rows.take(stepper::length(
        ui,
        &mut options.thickness,
        (0.0, StrutsOptions::MAX_THICKNESS),
        units,
        tr("Thickness"),
    ));
    ui.end_row();
    label(
        ui,
        tr("Sides"),
        tr(
            "How many sides each strut has round it: 4 is a square bar, 3 a triangle, 8 or more \
             a round rod. Struts with 6 sides or fewer have sharp corners",
        ),
    );
    rows.take(stepper::count(
        ui,
        &mut options.sides,
        (StrutsOptions::MIN_SIDES, StrutsOptions::MAX_SIDES),
        tr("Sides"),
    ));
    ui.end_row();
    for (value, name, help) in [
        (
            &mut options.even,
            tr("Even thickness"),
            tr(
                "Measure the thickness across the struts' flat sides, so every strut is exactly \
                 that wide. Off, it is measured across their corners, so a strut fits inside a \
                 round rod that thick",
            ),
        ),
        (
            &mut options.keep_faces,
            tr("Keep the faces"),
            tr(
                "Keep the shape's own polygons as well, with the struts round their edges: a \
                 panel in a frame. Off, only the struts and joints are left",
            ),
        ),
    ] {
        label(ui, name, help);
        rows.toggle(ui, value, name);
        ui.end_row();
    }
}
