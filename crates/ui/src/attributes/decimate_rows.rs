//! The Decimate modifier's rows in the Attributes panel
//! ([decision 0145](../../../../docs/decisions/0145-decimate.md)): how
//! the target is given (a ratio or a number of triangles), what is kept,
//! and whether flat pairs of triangles join into quads.

use super::curve_rows::Rows;
use super::{FIELD_WIDTH, label, stepper};
use crate::text::tr;
use egui::Ui;
use scene::{DecimateOptions, ReduceBy};

/// The name of the choice between a ratio and a number of triangles.
const REDUCE_BY: &str = crate::text!("Reduce by");

/// A way of giving the target's name.
fn reduce_by_name(by: ReduceBy) -> &'static str {
    match by {
        ReduceBy::Ratio => tr("Ratio"),
        ReduceBy::Triangles => tr("Target triangles"),
    }
}

/// Decimate's settings (in a two-column grid).
pub(super) fn decimate_rows(ui: &mut Ui, options: &mut DecimateOptions, rows: &mut Rows) {
    label(
        ui,
        tr(REDUCE_BY),
        tr(
            "How the new polygon count is given: a share of the triangles the shape has, or \
             a number of triangles (a game's budget)",
        ),
    );
    let current = options.reduce_by;
    let response = egui::ComboBox::from_id_salt(ui.id().with("decimate_by"))
        .selected_text(reduce_by_name(current))
        .width(FIELD_WIDTH)
        .show_ui(ui, |ui| {
            for by in [ReduceBy::Ratio, ReduceBy::Triangles] {
                if ui
                    .selectable_label(by == current, reduce_by_name(by))
                    .clicked()
                    && by != current
                {
                    options.reduce_by = by;
                    rows.changed = true;
                    rows.began = true;
                }
            }
        })
        .response;
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, tr(REDUCE_BY)));
    ui.end_row();
    match options.reduce_by {
        ReduceBy::Ratio => {
            label(
                ui,
                tr("Ratio"),
                tr(
                    "The share of the shape's triangles to keep: 50% halves them. Counted in \
                     triangles, as a game engine counts them (a square polygon is two).",
                ),
            );
            rows.take(stepper::percent(
                ui,
                &mut options.ratio,
                (DecimateOptions::MIN_RATIO, 1.0),
                tr("Ratio"),
            ));
        }
        ReduceBy::Triangles => {
            label(
                ui,
                tr("Target triangles"),
                tr(
                    "How many triangles to end with. Fewer can't always be reached: kept lines' \
                     corners never move.",
                ),
            );
            rows.take(stepper::big_count(
                ui,
                &mut options.triangles,
                (
                    DecimateOptions::MIN_TRIANGLES,
                    DecimateOptions::MAX_TRIANGLES,
                ),
                100,
                tr("Target triangles"),
            ));
        }
    }
    ui.end_row();
    for (value, name, help) in [
        (
            &mut options.keep.seams,
            tr("Keep UV seams"),
            tr(
                "Keep the UV seams (marked, or where the UVs split) as lines, so the texture \
                 stays where it was along them",
            ),
        ),
        (
            &mut options.keep.hard_edges,
            tr("Keep hard edges"),
            tr(
                "Keep the edges the shading shows hard (marked, or sharper than the Smoothing angle)",
            ),
        ),
        (
            &mut options.keep.materials,
            tr("Keep material borders"),
            tr("Keep the lines where one material meets another exactly where they are"),
        ),
        (
            &mut options.keep.borders,
            tr("Keep open borders"),
            tr(
                "Keep the edges of holes and open sheets where they are, so modular pieces \
                 still line up",
            ),
        ),
        (
            &mut options.quads_where_flat,
            tr("Quads where flat"),
            tr(
                "Join pairs of triangles lying flat together into four-sided polygons. Off, \
                 the result is all triangles.",
            ),
        ),
    ] {
        label(ui, name, help);
        rows.toggle(ui, value, name);
        ui.end_row();
    }
}

/// How many triangles the shape shown has, for the Decimate rows.
pub(super) fn shown_triangles_row(ui: &mut Ui, triangles: usize) {
    label(
        ui,
        tr("Triangles shown"),
        tr("How many triangles the shape in the viewport has, after all its modifiers"),
    );
    ui.label(crate::workbench::exporting::thousands(triangles));
    ui.end_row();
}
