//! The Shell modifier's rows in the Attributes panel
//! ([decision 0154](../../../../docs/decisions/0154-shell.md)): the
//! thickness, which way it grows, even thickness and the rims.

use super::curve_rows::Rows;
use super::{FIELD_WIDTH, label, stepper};
use crate::text::tr;
use crate::units::Units;
use egui::Ui;
use scene::{ShellDirection, ShellOptions};

/// The name of the choice of direction.
const DIRECTION: &str = crate::text!("Direction");

/// A direction's name.
fn direction_name(direction: ShellDirection) -> &'static str {
    match direction {
        ShellDirection::Inward => tr("Inward"),
        ShellDirection::Outward => tr("Outward"),
        ShellDirection::Both => tr("Both sides"),
    }
}

/// Shell's settings (in a two-column grid).
pub(super) fn shell_rows(ui: &mut Ui, options: &mut ShellOptions, units: Units, rows: &mut Rows) {
    label(
        ui,
        tr("Thickness"),
        tr("How thick the surface becomes: the distance between its two sides"),
    );
    rows.take(stepper::length(
        ui,
        &mut options.thickness,
        (0.0, ShellOptions::MAX_THICKNESS),
        units,
        tr("Thickness"),
    ));
    ui.end_row();
    direction_row(ui, options, rows);
    for (value, name, help) in [
        (
            &mut options.even,
            tr("Even thickness"),
            tr(
                "Move points at corners further, by the angle there, so the walls stay the same \
                 thickness right into sharp corners. Off, corners come out thinner",
            ),
        ),
        (
            &mut options.fill_rims,
            tr("Fill the rims"),
            tr(
                "Close the gap between the two sides along open borders (a plane's edges, a \
                 cup's lip) with a strip of polygons, so the result is a closed solid",
            ),
        ),
    ] {
        label(ui, name, help);
        rows.toggle(ui, value, name);
        ui.end_row();
    }
}

/// The choice of direction.
fn direction_row(ui: &mut Ui, options: &mut ShellOptions, rows: &mut Rows) {
    label(
        ui,
        tr(DIRECTION),
        tr(
            "Which way the thickness grows: inward keeps the surface as the outside (its size \
             stays), outward keeps it as the inside, both sides puts it in the middle",
        ),
    );
    let current = options.direction;
    let response = egui::ComboBox::from_id_salt(ui.id().with("shell_direction"))
        .selected_text(direction_name(current))
        .width(FIELD_WIDTH)
        .show_ui(ui, |ui| {
            for direction in ShellDirection::ALL {
                if ui
                    .selectable_label(direction == current, direction_name(direction))
                    .clicked()
                    && direction != current
                {
                    options.direction = direction;
                    rows.changed = true;
                    rows.began = true;
                }
            }
        })
        .response;
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, tr(DIRECTION)));
    ui.end_row();
}
