//! The *Bend, twist or taper* modifier's rows in the Attributes panel
//! ([decision 0158](../../../../docs/decisions/0158-bend-twist-taper.md)):
//! its kind, the axis, a bend's or twist's angle (and a bend's
//! direction), a taper's amount and curve, the limits along the axis and
//! the centre.

use super::curve_rows::Rows;
use super::{FIELD_WIDTH, label, stepper};
use crate::text::tr;
use crate::units::Units;
use egui::Ui;
use scene::{AxisDeformKind, AxisDeformOptions, DeformAxis};

/// The name of the choice of kind.
const KIND: &str = crate::text!("Kind");
/// The name of the choice of axis.
const AXIS: &str = crate::text!("Axis");

/// A drop-down list of `choices` under `name`, changing `value` when
/// another is picked.
fn choice<T: Copy + PartialEq>(
    ui: &mut Ui,
    rows: &mut Rows,
    name: &str,
    value: &mut T,
    choices: &[(T, &str)],
) {
    let shown = choices
        .iter()
        .find(|(c, _)| c == value)
        .map_or("", |(_, n)| *n);
    let response = egui::ComboBox::from_id_salt(ui.id().with(("axis_deform", name)))
        .selected_text(shown)
        .width(FIELD_WIDTH)
        .show_ui(ui, |ui| {
            for (option, text) in choices {
                if ui.selectable_label(option == value, *text).clicked() && option != value {
                    *value = *option;
                    rows.changed = true;
                    rows.began = true;
                }
            }
        })
        .response;
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, name));
}

/// The modifier's settings (in a two-column grid).
pub(super) fn axis_deform_rows(
    ui: &mut Ui,
    options: &mut AxisDeformOptions,
    units: Units,
    rows: &mut Rows,
) {
    label(
        ui,
        tr(KIND),
        tr(
            "What it does along the axis. Bend: curls the shape into an arc. Twist: turns each \
             slice of it round the axis, like wringing a cloth. Taper: makes it narrower (or \
             wider) along the axis.",
        ),
    );
    let kinds: Vec<(AxisDeformKind, &str)> = AxisDeformKind::ALL
        .into_iter()
        .map(|k| (k, k.name()))
        .collect();
    choice(ui, rows, tr(KIND), &mut options.kind, &kinds);
    ui.end_row();
    label(
        ui,
        tr(AXIS),
        tr(
            "Which of the shape's own axes it works along. Z, up, bends a standing shape over, \
             and twists and tapers it from bottom to top. The helper line in the viewport shows \
             it.",
        ),
    );
    let axes: Vec<(DeformAxis, &str)> =
        DeformAxis::ALL.into_iter().map(|a| (a, a.name())).collect();
    choice(ui, rows, tr(AXIS), &mut options.axis, &axes);
    ui.end_row();
    match options.kind {
        AxisDeformKind::Bend => bend_rows(ui, options, rows),
        AxisDeformKind::Twist => twist_rows(ui, options, rows),
        AxisDeformKind::Taper => taper_rows(ui, options, rows),
    }
    limit_rows(ui, options, rows);
    centre_rows(ui, options, units, rows);
}

/// A bend's Angle and Direction.
fn bend_rows(ui: &mut Ui, options: &mut AxisDeformOptions, rows: &mut Rows) {
    label(
        ui,
        tr("Angle"),
        tr(
            "How far the shape bends over in all, between the limits: 90° lays its far end on \
             its side, 360° curls it into a ring. Below 0, it bends the other way.",
        ),
    );
    let most = AxisDeformOptions::MAX_ANGLE;
    rows.take(stepper::degrees(
        ui,
        &mut options.angle,
        (-most, most),
        tr("Angle"),
    ));
    ui.end_row();
    label(
        ui,
        tr("Direction"),
        tr(
            "Which way it bends, turned round the axis: 0° bends towards the next axis round \
             (Z towards X, X towards Y, Y towards Z), 90° a quarter turn on from there.",
        ),
    );
    rows.take(stepper::degrees(
        ui,
        &mut options.direction,
        (0.0, 360.0),
        tr("Direction"),
    ));
    ui.end_row();
}

/// A twist's Angle.
fn twist_rows(ui: &mut Ui, options: &mut AxisDeformOptions, rows: &mut Rows) {
    label(
        ui,
        tr("Angle"),
        tr(
            "How far the far end turns round the axis compared with the centre, between the \
             limits. Below 0, it twists the other way.",
        ),
    );
    let most = AxisDeformOptions::MAX_ANGLE;
    rows.take(stepper::degrees(
        ui,
        &mut options.angle,
        (-most, most),
        tr("Angle"),
    ));
    ui.end_row();
}

/// A taper's Amount and Curve.
fn taper_rows(ui: &mut Ui, options: &mut AxisDeformOptions, rows: &mut Rows) {
    let most = AxisDeformOptions::MAX_AMOUNT;
    label(
        ui,
        tr("Amount"),
        tr(
            "How much wider the shape gets towards the far end of the limits: -50% makes it \
             half as wide there, -100% a point, 100% twice as wide. The centre keeps its size.",
        ),
    );
    rows.take(stepper::percent(
        ui,
        &mut options.amount,
        (-most, most),
        tr("Amount"),
    ));
    ui.end_row();
    label(
        ui,
        tr("Curve"),
        tr(
            "Bulges the middle of the limits out (above 0) or pinches it in (below 0), the ends \
             as they were: a barrel or a waist.",
        ),
    );
    rows.take(stepper::percent(
        ui,
        &mut options.curve,
        (-most, most),
        tr("Curve"),
    ));
    ui.end_row();
}

/// The limits along the axis, as shares of the shape's length.
fn limit_rows(ui: &mut Ui, options: &mut AxisDeformOptions, rows: &mut Rows) {
    let help = tr(
        "Where along the axis it works, from 0% (the shape's lowest point along it) to 100% \
         (its highest). Past the limits the shape carries on unchanged, as the limit leaves it: \
         keep a bend to the top half with 50% to 100%.",
    );
    for (i, name) in [tr("Limit from"), tr("Limit to")].into_iter().enumerate() {
        label(ui, name, help);
        rows.take(stepper::percent(
            ui,
            &mut options.limits[i],
            (0.0, 1.0),
            name,
        ));
        ui.end_row();
    }
}

/// The centre, from the pivot.
fn centre_rows(ui: &mut Ui, options: &mut AxisDeformOptions, units: Units, rows: &mut Rows) {
    let help = tr(
        "Where the centre is, from the shape's pivot. The shape bends, twists and tapers away \
         from the centre, which stays where it is: move it (or the pivot) to bend from \
         somewhere else.",
    );
    let most = AxisDeformOptions::MAX_CENTRE;
    let names = [tr("Centre X"), tr("Centre Y"), tr("Centre Z")];
    for (axis, name) in names.into_iter().enumerate() {
        label(ui, name, help);
        rows.take(stepper::length(
            ui,
            &mut options.centre[axis],
            (-most, most),
            units,
            name,
        ));
        ui.end_row();
    }
}
