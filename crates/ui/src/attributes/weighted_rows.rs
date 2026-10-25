//! The Weighted normals modifier's rows in the Attributes panel
//! ([decision 0148](../../../../docs/decisions/0148-weighted-normals.md)):
//! what makes a polygon count for more, how much the biggest wins, how
//! strong the change is, and whether hard edges stay; and a note when a
//! modifier after it drops its normals.

use super::curve_rows::Rows;
use super::{FIELD_WIDTH, label, stepper};
use crate::text::tr;
use crate::theme;
use egui::Ui;
use scene::{WeightBy, WeightedNormalsOptions};

/// The name of the choice of weighting.
const WEIGHT_BY: &str = crate::text!("Weight by");

/// A weighting's name.
fn weight_name(by: WeightBy) -> &'static str {
    match by {
        WeightBy::Area => tr("Polygon area"),
        WeightBy::Angle => tr("Corner angle"),
        WeightBy::AreaAndAngle => tr("Area and angle"),
    }
}

/// Weighted normals' settings (in a two-column grid).
pub(super) fn weighted_rows(ui: &mut Ui, options: &mut WeightedNormalsOptions, rows: &mut Rows) {
    label(
        ui,
        tr(WEIGHT_BY),
        tr(
            "What makes a polygon count for more at its corners: its area (big faces win, the \
             usual choice), the angle it fills there (as plain shading counts), or both",
        ),
    );
    let current = options.weight_by;
    let response = egui::ComboBox::from_id_salt(ui.id().with("weight_by"))
        .selected_text(weight_name(current))
        .width(FIELD_WIDTH)
        .show_ui(ui, |ui| {
            for by in [WeightBy::Area, WeightBy::Angle, WeightBy::AreaAndAngle] {
                if ui
                    .selectable_label(by == current, weight_name(by))
                    .clicked()
                    && by != current
                {
                    options.weight_by = by;
                    rows.changed = true;
                    rows.began = true;
                }
            }
        })
        .response;
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, tr(WEIGHT_BY)));
    ui.end_row();
    label(
        ui,
        tr("Face influence"),
        tr(
            "How much the biggest polygon at a point wins over the smaller ones. 0%: each counts \
             by its weight alone; 100%: only the biggest counts, so big faces shade perfectly flat",
        ),
    );
    rows.take(stepper::percent(
        ui,
        &mut options.influence,
        (0.0, 1.0),
        tr("Face influence"),
    ));
    ui.end_row();
    label(
        ui,
        tr("Strength"),
        tr(
            "How far the shading turns from the usual towards the weighted: 100% all the way, \
             0% not at all",
        ),
    );
    rows.take(stepper::percent(
        ui,
        &mut options.strength,
        (0.0, 1.0),
        tr("Strength"),
    ));
    ui.end_row();
    label(
        ui,
        tr("Keep hard edges"),
        tr(
            "Keep the shading crisp along the edges the object's shading shows hard (marked, or \
             sharper than its Smoothing angle). Off, the whole shape shades smooth.",
        ),
    );
    let name = tr("Keep hard edges");
    rows.toggle(ui, &mut options.keep_hard_edges, name);
    ui.end_row();
}

/// The note under Weighted normals (at `index` of `modifiers`) when a
/// modifier after it changes the shape: in the warning colour when that
/// modifier is on, so these normals are dropped; calmly when it is off,
/// saying what switching it on would do. The note keeps the same height
/// either way (the larger of the two), so switching the later modifier
/// on and off doesn't move the panel under the mouse. Wrapped to the
/// panel's width, never cut off.
pub(super) fn dropped_note(ui: &mut Ui, modifiers: &[scene::Modifier], index: usize) {
    let later = || {
        modifiers
            .iter()
            .skip(index.saturating_add(1))
            .filter(|m| !m.kind.keeps_shading())
    };
    let Some(first) = later().next() else {
        return;
    };
    let dropping = later().find(|m| m.on);
    let warning = |by: &str| {
        crate::trf!(
            "{} comes after this and changes the shape, so these normals are dropped: move \
             Weighted normals to the end with Down.",
            by
        )
    };
    let calm = crate::trf!(
        "{} comes after this and is off. Switched on, it changes the shape and these normals \
         are dropped: move Weighted normals to the end with Down.",
        first.kind.name()
    );
    let (shown, color) = match dropping {
        Some(m) => (warning(m.kind.name()), theme::WARNING),
        None => (calm.clone(), theme::TEXT_WEAK),
    };
    let other = match dropping {
        Some(_) => calm,
        None => warning(first.kind.name()),
    };
    super::steady_note(ui, &shown, &other, color);
}
