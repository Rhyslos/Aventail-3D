//! Quadify's rows in the Attributes panel ([decision 0150](../../../../docs/decisions/0150-quadify.md)):
//! the two limits and what is kept, shown for the Quadify modifier and,
//! under Last edit, for a one-off Quadify now.

use super::curve_rows::Rows;
use super::{label, stepper};
use crate::text::tr;
use egui::Ui;
use mesh::quadify::{MAX_ANGLE, MAX_DISTORTION};
use scene::{EditSettings, QuadifyOptions};

/// Quadify's settings (in a two-column grid).
pub(super) fn quadify_rows(ui: &mut Ui, options: &mut QuadifyOptions, rows: &mut Rows) {
    label(
        ui,
        tr("Max angle"),
        tr(
            "How far two triangles may lean from each other and still be joined: low keeps the \
             quads flat, high joins round a curve",
        ),
    );
    rows.take(stepper::degrees(
        ui,
        &mut options.max_angle,
        (0.0, MAX_ANGLE),
        tr("Max angle"),
    ));
    ui.end_row();
    label(
        ui,
        tr("Max shape distortion"),
        tr(
            "How far any corner of a new quad may be from a right angle: low makes only \
             near-square quads, high takes slanted ones too",
        ),
    );
    rows.take(stepper::degrees(
        ui,
        &mut options.max_distortion,
        (0.0, MAX_DISTORTION),
        tr("Max shape distortion"),
    ));
    ui.end_row();
    for (value, name, help) in [
        (
            &mut options.keep_seams,
            tr("Keep UV seams"),
            tr(
                "Never join across a UV seam (marked, or where the UVs split), so the texture \
                 stays as it was",
            ),
        ),
        (
            &mut options.keep_hard_edges,
            tr("Keep hard edges"),
            tr(
                "Never join across an edge the shading shows hard (marked, or sharper than the \
                 Smoothing angle)",
            ),
        ),
        (
            &mut options.keep_materials,
            tr("Keep material borders"),
            tr("Never join two triangles with different materials"),
        ),
    ] {
        label(ui, name, help);
        rows.toggle(ui, value, name);
        ui.end_row();
    }
}

/// The rows of a one-off Quadify's settings under Last edit, if `edited`
/// has them: a change goes into `edited` and is remembered in
/// `remembered` for the next one.
pub(super) fn last_edit_rows(
    ui: &mut Ui,
    edited: &mut EditSettings,
    remembered: &mut QuadifyOptions,
) -> Rows {
    let mut rows = Rows::default();
    if let Some(mut options) = edited.quadify {
        quadify_rows(ui, &mut options, &mut rows);
        if rows.changed {
            edited.quadify = Some(options);
            *remembered = options;
        }
    }
    rows
}
