//! The Live bevel modifier's rows in the Attributes panel
//! ([decision 0149](../../../../docs/decisions/0149-live-bevel.md)):
//! which edges it bevels (by angle, the hard ones or the marked ones),
//! the angle, and the bevel's width, segments, profile and Stop at
//! neighbours.

use super::curve_rows::Rows;
use super::{FIELD_WIDTH, label, stepper};
use crate::text::tr;
use crate::units::Units;
use egui::Ui;
use mesh::bevel::MAX_SEGMENTS;
use mesh::live_bevel::MAX_WIDTH;
use scene::{BevelEdges, LiveBevelOptions};

/// The name of the choice of edges.
const EDGES: &str = crate::text!("Edges");

/// A choice of edges' name.
fn edges_name(edges: BevelEdges) -> &'static str {
    match edges {
        BevelEdges::ByAngle => tr("By angle"),
        BevelEdges::Hard => tr("Hard edges"),
        BevelEdges::Marked => tr("Marked edges"),
    }
}

/// Live bevel's settings (in a two-column grid).
pub(super) fn live_bevel_rows(
    ui: &mut Ui,
    options: &mut LiveBevelOptions,
    units: Units,
    rows: &mut Rows,
) {
    edges_row(ui, options, rows);
    if options.edges == BevelEdges::ByAngle {
        label(
            ui,
            tr("Angle"),
            tr(
                "Bevel the edges whose two polygons meet more sharply than this: 30° takes a \
                 box's edges but not the gentle steps round a cylinder",
            ),
        );
        rows.take(stepper::degrees(
            ui,
            &mut options.angle,
            (0.0, 180.0),
            tr("Angle"),
        ));
        ui.end_row();
    }
    label(
        ui,
        tr("Width"),
        tr(
            "How far the bevel reaches into the polygons on each side of an edge. Marked edges \
             get their Bevel weight's share of it.",
        ),
    );
    rows.take(stepper::length(
        ui,
        &mut options.width,
        (0.0, MAX_WIDTH),
        units,
        tr("Width"),
    ));
    ui.end_row();
    label(
        ui,
        tr("Segments"),
        tr("How many rows of polygons each bevel has: 1 is a flat cut, more is rounder"),
    );
    rows.take(stepper::count(
        ui,
        &mut options.segments,
        (1, MAX_SEGMENTS),
        tr("Segments"),
    ));
    ui.end_row();
    label(
        ui,
        tr("Profile"),
        tr("The bevel's curve: 1 round, 0 a straight slope, -1 hollow (curving inward)"),
    );
    rows.take(stepper::number(
        ui,
        &mut options.profile,
        (-1.0, 1.0),
        (0.1, 2),
        tr("Profile"),
    ));
    ui.end_row();
    label(
        ui,
        tr("Stop at neighbours"),
        tr(
            "Narrow the bevel only where it would run into another, so bevels never cross over \
             each other; the rest stays at full width",
        ),
    );
    let name = tr("Stop at neighbours");
    rows.toggle(ui, &mut options.clamp, name);
    ui.end_row();
}

/// The choice of edges.
fn edges_row(ui: &mut Ui, options: &mut LiveBevelOptions, rows: &mut Rows) {
    label(
        ui,
        tr(EDGES),
        tr(
            "Which edges are bevelled: those sharper than an angle, those the shading shows \
             hard, or those you give a Bevel weight (in the Edges mode, press 2)",
        ),
    );
    let current = options.edges;
    let response = egui::ComboBox::from_id_salt(ui.id().with("bevel_edges"))
        .selected_text(edges_name(current))
        .width(FIELD_WIDTH)
        .show_ui(ui, |ui| {
            for edges in [BevelEdges::ByAngle, BevelEdges::Hard, BevelEdges::Marked] {
                if ui
                    .selectable_label(edges == current, edges_name(edges))
                    .clicked()
                    && edges != current
                {
                    options.edges = edges;
                    rows.changed = true;
                    rows.began = true;
                }
            }
        })
        .response;
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, tr(EDGES)));
    ui.end_row();
}
