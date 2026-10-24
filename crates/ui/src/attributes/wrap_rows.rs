//! The Bend along curve and Shrinkwrap modifiers' rows in the Attributes
//! panel ([decision 0147](../../../../docs/decisions/0147-curve-deform-and-shrinkwrap.md)):
//! the curve or target each reads (with a button selecting it), how the
//! shape lies along the curve, how its points find the target's surface,
//! and which points move.

use super::curve_rows::Rows;
use super::{FIELD_WIDTH, label, stepper};
use crate::objects::DocRequest;
use crate::text::tr;
use crate::units::Units;
use egui::Ui;
use scene::{
    BendAxis, BendOptions, CurveBendSettings, Document, ObjectId, ShrinkwrapOptions,
    ShrinkwrapSettings, VertexId, WrapMode,
};

/// The name of the choice of axis.
const AXIS: &str = crate::text!("Axis");
/// The name of the choice of how points find the surface.
const MODE: &str = crate::text!("Mode");

/// An axis choice's name.
fn axis_name(axis: BendAxis) -> &'static str {
    match axis {
        BendAxis::Longest => tr("Longest side"),
        BendAxis::X => tr("X"),
        BendAxis::Y => tr("Y"),
        BendAxis::Z => tr("Z"),
    }
}

/// The object a modifier reads, by name, with a button selecting it.
pub(super) fn source_row(
    ui: &mut Ui,
    doc: &Document,
    source: ObjectId,
    bend: bool,
    out: &mut Vec<DocRequest>,
) {
    let (name, note, button, hint) = if bend {
        (
            tr("Curve"),
            tr(
                "The curve this shape is bent along. It stays live: select it and reshape it \
                 (press 1 for its points), or move it, and the shape follows.",
            ),
            tr("Select curve"),
            tr("Select the curve, to reshape or move it"),
        )
    } else {
        (
            tr("Target"),
            tr(
                "The shape this one is wrapped onto. It stays live: edit or move it and the \
                 wrapped shape follows. Hidden, it still counts.",
            ),
            tr("Select target"),
            tr("Select the target, to change or move it"),
        )
    };
    label(ui, name, note);
    ui.horizontal(|ui| {
        let shown = doc
            .scene()
            .object(source)
            .map_or_else(|| tr("(none)").to_string(), |o| o.name.clone());
        ui.label(shown);
        if ui.small_button(button).on_hover_text(hint).clicked() {
            out.push(DocRequest::Select(vec![source]));
        }
    });
    ui.end_row();
}

/// Bend along curve's settings (in a two-column grid).
pub(super) fn bend_rows(
    ui: &mut Ui,
    settings: &mut CurveBendSettings,
    units: Units,
    rows: &mut Rows,
) {
    let options = &mut settings.options;
    label(
        ui,
        tr(AXIS),
        tr(
            "Which of the shape's own axes runs along the curve: its longest side, or X, Y or Z. \
             The shape's start along it goes to the curve's start.",
        ),
    );
    let current = options.axis;
    let response = egui::ComboBox::from_id_salt(ui.id().with("bend_axis"))
        .selected_text(axis_name(current))
        .width(FIELD_WIDTH)
        .show_ui(ui, |ui| {
            for axis in BendAxis::ALL {
                if ui
                    .selectable_label(axis == current, axis_name(axis))
                    .clicked()
                    && axis != current
                {
                    options.axis = axis;
                    rows.changed = true;
                    rows.began = true;
                }
            }
        })
        .response;
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, tr(AXIS)));
    ui.end_row();
    label(
        ui,
        tr("Stretch to fit"),
        tr(
            "Stretch or squash the shape along the curve so it runs exactly from the curve's \
             start to its end. Off, it keeps its own length.",
        ),
    );
    rows.toggle(ui, &mut options.stretch, tr("Stretch to fit"));
    ui.end_row();
    label(
        ui,
        tr("Offset"),
        tr(
            "Slide the shape along the curve: how far from the curve's start it begins. Past an \
             open curve's ends it carries on straight; round a closed one it goes round again.",
        ),
    );
    rows.take(stepper::length(
        ui,
        &mut options.offset,
        (-BendOptions::MAX_OFFSET, BendOptions::MAX_OFFSET),
        units,
        tr("Offset"),
    ));
    ui.end_row();
    label(
        ui,
        tr("Twist"),
        tr(
            "Turn the shape round the curve, evenly from its start to its end, on top of the \
             way the curve turns it: 360° is one whole turn",
        ),
    );
    rows.take(stepper::degrees(
        ui,
        &mut options.twist,
        (-BendOptions::MAX_TWIST, BendOptions::MAX_TWIST),
        tr("Twist"),
    ));
    ui.end_row();
}

/// Shrinkwrap's settings (in a two-column grid). `chosen` is the points
/// selected on the shape in the Points mode (`None` in other modes), for
/// *Use selected*.
pub(super) fn shrinkwrap_rows(
    ui: &mut Ui,
    settings: &mut ShrinkwrapSettings,
    units: Units,
    rows: &mut Rows,
    chosen: Option<&[VertexId]>,
) {
    let options = &mut settings.options;
    label(
        ui,
        tr(MODE),
        tr(
            "How each point finds the target's surface: straight to the nearest place on it, \
             along the way the shape faces there, or straight along one of the shape's axes. \
             Along a line, a point that meets nothing stays where it is.",
        ),
    );
    let current = options.mode;
    let response = egui::ComboBox::from_id_salt(ui.id().with("wrap_mode"))
        .selected_text(current.name())
        .width(FIELD_WIDTH)
        .show_ui(ui, |ui| {
            for mode in WrapMode::ALL {
                if ui.selectable_label(mode == current, mode.name()).clicked() && mode != current {
                    options.mode = mode;
                    rows.changed = true;
                    rows.began = true;
                }
            }
        })
        .response;
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, tr(MODE)));
    ui.end_row();
    label(
        ui,
        tr("Offset"),
        tr(
            "Keep the points this far off the target's surface, along the way it faces: a \
             little above it stops the two flickering through each other. Below 0, inside it.",
        ),
    );
    rows.take(stepper::length(
        ui,
        &mut options.offset,
        (
            -ShrinkwrapOptions::MAX_OFFSET,
            ShrinkwrapOptions::MAX_OFFSET,
        ),
        units,
        tr("Offset"),
    ));
    ui.end_row();
    let texts = PointTexts {
        help: tr(
            "Every point is wrapped, or only those you choose: select them in the Points mode \
             (press 1) and click Use selected. The others stay where they are.",
        ),
        use_selected: tr("Wrap only the points selected now"),
        all: tr("Wrap every point again"),
    };
    points_row(ui, &mut settings.points, rows, chosen, &texts);
}

/// What the Points row says for one kind of modifier.
pub(super) struct PointTexts {
    /// The row's help.
    pub(super) help: &'static str,
    /// *Use selected*'s hint.
    pub(super) use_selected: &'static str,
    /// *All points*' hint.
    pub(super) all: &'static str,
}

/// Which points move: all, or those chosen, with buttons to choose the
/// selected ones or go back to all (Shrinkwrap's and Displace's).
pub(super) fn points_row(
    ui: &mut Ui,
    points: &mut Vec<VertexId>,
    rows: &mut Rows,
    chosen: Option<&[VertexId]>,
    texts: &PointTexts,
) {
    label(ui, tr("Points"), texts.help);
    ui.horizontal_wrapped(|ui| {
        if points.is_empty() {
            ui.label(tr("All points"));
        } else {
            ui.label(crate::trf!("{} chosen", points.len()));
        }
        let selected = chosen.filter(|c| !c.is_empty());
        let note = if selected.is_some() {
            texts.use_selected
        } else {
            tr("Select points of this shape in the Points mode (press 1) first")
        };
        if ui
            .add_enabled(
                selected.is_some(),
                egui::Button::new(tr("Use selected")).small(),
            )
            .on_hover_text(note)
            .on_disabled_hover_text(note)
            .clicked()
            && let Some(selected) = selected
        {
            *points = selected.to_vec();
            rows.changed = true;
            rows.began = true;
        }
        if ui
            .add_enabled(
                !points.is_empty(),
                egui::Button::new(tr("All points")).small(),
            )
            .on_hover_text(texts.all)
            .clicked()
        {
            points.clear();
            rows.changed = true;
            rows.began = true;
        }
    });
    ui.end_row();
}
