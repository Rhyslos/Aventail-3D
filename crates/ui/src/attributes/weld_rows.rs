//! Weld by distance's rows in the Attributes panel ([decision 0152](../../../../docs/decisions/0152-weld-by-distance.md)):
//! the distance and its two switches, shown for the Weld by distance
//! modifier and, under Last edit, for a one-off Weld by distance now.

use super::curve_rows::Rows;
use super::{label, stepper};
use crate::text::tr;
use egui::Ui;
use scene::{EditSettings, WeldOptions};

/// Weld by distance's settings (in a two-column grid).
pub(super) fn weld_rows(ui: &mut Ui, options: &mut WeldOptions, rows: &mut Rows) {
    label(
        ui,
        tr("Distance"),
        tr(
            "Points closer together than this melt into one. Keep it tiny (0.1 mm) to close only \
             cracks; raise it to join pieces placed roughly edge to edge",
        ),
    );
    rows.take(stepper::millimetres(
        ui,
        &mut options.distance,
        (0.0, WeldOptions::MAX_DISTANCE),
        tr("Distance"),
    ));
    ui.end_row();
    for (value, name, help) in [
        (
            &mut options.only_borders,
            tr("Only at open borders"),
            tr(
                "Weld only points on open borders (edges with a polygon on one side only), where \
                 cracks are, so small details elsewhere never melt",
            ),
        ),
        (
            &mut options.keep_seams,
            tr("Keep UV seams"),
            tr(
                "Where joined pieces' UVs don't meet, mark the join as a UV seam, so a later \
                 Unwrap keeps the pieces as they are laid out",
            ),
        ),
    ] {
        label(ui, name, help);
        rows.toggle(ui, value, name);
        ui.end_row();
    }
}

/// The rows of a one-off Weld by distance's settings under Last edit, if
/// `edited` has them: a change goes into `edited` and is remembered in
/// `remembered` for the next one.
pub(super) fn last_edit_rows(
    ui: &mut Ui,
    edited: &mut EditSettings,
    remembered: &mut WeldOptions,
) -> Rows {
    let mut rows = Rows::default();
    if let Some(mut options) = edited.weld {
        weld_rows(ui, &mut options, &mut rows);
        if rows.changed {
            edited.weld = Some(options);
            *remembered = options;
        }
    }
    rows
}
