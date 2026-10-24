//! The Triangulate modifier's rows in the Attributes panel
//! ([decision 0161](../../../../docs/decisions/0161-triangulate.md)): the
//! method (Beauty or Fixed) and whether only quads are cut.

use super::curve_rows::Rows;
use super::{FIELD_WIDTH, label};
use crate::text::tr;
use egui::Ui;
use scene::{TriangulateMethod, TriangulateOptions};

/// The name of the choice of method.
const METHOD: &str = crate::text!("Method");
/// The name of the Quads only switch.
const QUADS_ONLY: &str = crate::text!("Quads only");

/// Triangulate's settings (in a two-column grid).
pub(super) fn triangulate_rows(ui: &mut Ui, options: &mut TriangulateOptions, rows: &mut Rows) {
    label(
        ui,
        tr(METHOD),
        tr(
            "How each polygon's triangles are chosen. Beauty evens them out, so there are fewer \
             long thin ones; Fixed cuts exactly as the export does",
        ),
    );
    let current = options.method;
    let response = egui::ComboBox::from_id_salt(ui.id().with("triangulate_method"))
        .selected_text(current.name())
        .width(FIELD_WIDTH)
        .show_ui(ui, |ui| {
            for method in TriangulateMethod::ALL {
                if ui
                    .selectable_label(method == current, method.name())
                    .on_hover_text(method.explanation())
                    .clicked()
                    && method != current
                {
                    options.method = method;
                    rows.changed = true;
                    rows.began = true;
                }
            }
        })
        .response;
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, tr(METHOD)));
    ui.end_row();
    label(
        ui,
        tr(QUADS_ONLY),
        tr(
            "Cut only four-sided polygons, and leave bigger ones (n-gons) as they are. Off, \
             every polygon with four or more corners is cut",
        ),
    );
    rows.toggle(ui, &mut options.quads_only, tr(QUADS_ONLY));
    ui.end_row();
}
