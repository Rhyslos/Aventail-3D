//! The Array modifier's rows in the Attributes panel ([decision 0153](../../../../docs/decisions/0153-array.md)):
//! the layout (a line, a grid or a ring) and its counts and offsets, a
//! ring's angle, axis, centre and radius, merging touching copies, and
//! laying their UVs side by side.

use super::curve_rows::Rows;
use super::{FIELD_WIDTH, label, stepper};
use crate::text::tr;
use crate::units::Units;
use egui::Ui;
use glam::Vec3;
use scene::{ArrayAxis, ArrayLayout, ArrayOptions, ArraySettings, ObjectId};

/// The name of the choice of layout.
const LAYOUT: &str = crate::text!("Layout");
/// The name of the choice of axis.
const AXIS: &str = crate::text!("Axis");
/// The name of the choice of centre.
const CENTRE: &str = crate::text!("Centre");

/// A layout's name.
fn layout_name(layout: ArrayLayout) -> &'static str {
    match layout {
        ArrayLayout::Line => tr("Line"),
        ArrayLayout::Grid => tr("Grid"),
        ArrayLayout::Radial => tr("Radial"),
    }
}

/// An axis's name.
fn axis_name(axis: ArrayAxis) -> &'static str {
    match axis {
        ArrayAxis::X => tr("X"),
        ArrayAxis::Y => tr("Y"),
        ArrayAxis::Z => tr("Z"),
    }
}

/// A drop-down list of `choices` (each with its name) under `name`,
/// changing `value` when another is picked.
fn choice<T: Copy + PartialEq>(
    ui: &mut Ui,
    rows: &mut Rows,
    name: &str,
    value: &mut T,
    choices: &[(T, String)],
) {
    let shown = choices
        .iter()
        .find(|(c, _)| c == value)
        .map_or_else(String::new, |(_, n)| n.clone());
    let response = egui::ComboBox::from_id_salt(ui.id().with(("array", name)))
        .selected_text(shown)
        .width(FIELD_WIDTH)
        .show_ui(ui, |ui| {
            for (option, text) in choices {
                if ui.selectable_label(option == value, text).clicked() && option != value {
                    *value = *option;
                    rows.changed = true;
                    rows.began = true;
                }
            }
        })
        .response;
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, name));
}

/// The objects a ring could turn round, by name (for the Centre list):
/// given by the caller, as only the panel knows the scene.
pub(super) type Centres = [(ObjectId, String)];

/// The Array modifier's settings (in a two-column grid). `centres` lists
/// the objects its ring could turn round.
pub(super) fn array_rows(
    ui: &mut Ui,
    settings: &mut ArraySettings,
    units: Units,
    rows: &mut Rows,
    centres: &Centres,
) {
    let options = &mut settings.options;
    label(
        ui,
        tr(LAYOUT),
        tr(
            "How the copies are laid out: one after another in a line, along X, Y and Z in a \
             grid, or turned round a centre in a ring",
        ),
    );
    let layouts: Vec<(ArrayLayout, String)> = ArrayLayout::ALL
        .into_iter()
        .map(|l| (l, layout_name(l).to_string()))
        .collect();
    choice(ui, rows, tr(LAYOUT), &mut options.layout, &layouts);
    ui.end_row();
    match options.layout {
        ArrayLayout::Line => line_rows(ui, options, units, rows),
        ArrayLayout::Grid => grid_rows(ui, options, units, rows),
        ArrayLayout::Radial => radial_rows(ui, settings, units, rows, centres),
    }
    merge_rows(ui, &mut settings.options, rows);
}

/// A line's Count, and its step.
fn line_rows(ui: &mut Ui, options: &mut ArrayOptions, units: Units, rows: &mut Rows) {
    label(
        ui,
        tr("Count"),
        tr("How many copies, the shape itself included"),
    );
    rows.take(stepper::count(
        ui,
        &mut options.count,
        (1, ArrayOptions::MAX_COUNT),
        tr("Count"),
    ));
    ui.end_row();
    relative_row(ui, options, rows);
    let help = if options.relative {
        tr(
            "How far each copy is from the one before, as a share of the shape's size: 1 touches, \
            more leaves a gap. Up and along at once makes stairs",
        )
    } else {
        tr("How far each copy is from the one before. Up and along at once makes stairs")
    };
    let names = [tr("Offset X"), tr("Offset Y"), tr("Offset Z")];
    vector_rows(
        ui,
        &mut options.offset,
        options.relative,
        units,
        rows,
        names,
        help,
    );
}

/// A grid's counts along X, Y and Z, and its spacing.
fn grid_rows(ui: &mut Ui, options: &mut ArrayOptions, units: Units, rows: &mut Rows) {
    let names = [tr("Count X"), tr("Count Y"), tr("Count Z")];
    for (count, name) in options.counts.iter_mut().zip(names) {
        label(
            ui,
            name,
            tr("How many copies along this axis, the shape included"),
        );
        rows.take(stepper::count(
            ui,
            count,
            (1, ArrayOptions::MAX_COUNT),
            name,
        ));
        ui.end_row();
    }
    relative_row(ui, options, rows);
    let help = if options.relative {
        tr(
            "How far apart the copies are along this axis, as a share of the shape's size: 1 \
            touches, more leaves a gap",
        )
    } else {
        tr("How far apart the copies are along this axis")
    };
    let names = [tr("Spacing X"), tr("Spacing Y"), tr("Spacing Z")];
    vector_rows(
        ui,
        &mut options.spacing,
        options.relative,
        units,
        rows,
        names,
        help,
    );
}

/// The *Relative to size* switch.
fn relative_row(ui: &mut Ui, options: &mut ArrayOptions, rows: &mut Rows) {
    label(
        ui,
        tr("Relative to size"),
        tr(
            "On, offsets are shares of the shape's size, so 1 places copies touching whatever \
             its size; off, they are distances",
        ),
    );
    rows.toggle(ui, &mut options.relative, tr("Relative to size"));
    ui.end_row();
}

/// Three fields for X, Y and Z: shares of the size, or lengths.
fn vector_rows(
    ui: &mut Ui,
    value: &mut Vec3,
    relative: bool,
    units: Units,
    rows: &mut Rows,
    names: [&str; 3],
    help: &str,
) {
    let most = ArrayOptions::MAX_DISTANCE;
    for (axis, name) in names.into_iter().enumerate() {
        label(ui, name, help);
        let field = &mut value[axis];
        rows.take(if relative {
            stepper::number(ui, field, (-most, most), (0.1, 3), name)
        } else {
            stepper::length(ui, field, (-most, most), units, name)
        });
        ui.end_row();
    }
}

/// A ring's Count, Angle, Axis, Centre and Radius.
fn radial_rows(
    ui: &mut Ui,
    settings: &mut ArraySettings,
    units: Units,
    rows: &mut Rows,
    centres: &Centres,
) {
    let options = &mut settings.options;
    label(
        ui,
        tr("Count"),
        tr("How many copies round the ring, the shape itself included"),
    );
    rows.take(stepper::count(
        ui,
        &mut options.count,
        (1, ArrayOptions::MAX_COUNT),
        tr("Count"),
    ));
    ui.end_row();
    label(
        ui,
        tr("Angle"),
        tr(
            "How far round the ring goes: 360° is a whole turn, shared out evenly; less makes a \
             fan with copies at both ends. Below 0 it turns the other way",
        ),
    );
    rows.take(stepper::degrees(
        ui,
        &mut options.angle,
        (-360.0, 360.0),
        tr("Angle"),
    ));
    ui.end_row();
    label(
        ui,
        tr(AXIS),
        tr("Which of the shape's own axes the ring turns round. Z, up, lays it on the ground"),
    );
    let axes: Vec<(ArrayAxis, String)> = ArrayAxis::ALL
        .into_iter()
        .map(|a| (a, axis_name(a).to_string()))
        .collect();
    choice(ui, rows, tr(AXIS), &mut options.axis, &axes);
    ui.end_row();
    label(
        ui,
        tr(CENTRE),
        tr(
            "What the ring turns round: the shape's own pivot, or another object's (it stays \
             live: move that object and the ring follows)",
        ),
    );
    let mut list: Vec<(Option<ObjectId>, String)> = vec![(None, tr("Its pivot").to_string())];
    list.extend(centres.iter().map(|(id, name)| (Some(*id), name.clone())));
    choice(ui, rows, tr(CENTRE), &mut settings.centre, &list);
    ui.end_row();
    let options = &mut settings.options;
    label(
        ui,
        tr("Radius"),
        tr(
            "How far each copy is moved out from the centre before it is turned. With 0, the \
             copies turn where the shape is: move the shape off its pivot, or raise this",
        ),
    );
    let most = ArrayOptions::MAX_DISTANCE;
    rows.take(stepper::length(
        ui,
        &mut options.radius,
        (-most, most),
        units,
        tr("Radius"),
    ));
    ui.end_row();
}

/// Merging touching copies, and their UVs side by side.
fn merge_rows(ui: &mut Ui, options: &mut ArrayOptions, rows: &mut Rows) {
    label(
        ui,
        tr("Merge touching copies"),
        tr(
            "Weld the points where copies touch into one, so open copies (a fence, a pipe) join \
             into one surface. Closed copies stay closed shapes side by side",
        ),
    );
    rows.toggle(ui, &mut options.merge, tr("Merge touching copies"));
    ui.end_row();
    if options.merge {
        label(
            ui,
            tr("Merge distance"),
            tr("Points of different copies closer together than this melt into one"),
        );
        rows.take(stepper::millimetres(
            ui,
            &mut options.merge_distance,
            (0.0, scene::WeldOptions::MAX_DISTANCE),
            tr("Merge distance"),
        ));
        ui.end_row();
    }
    label(
        ui,
        tr("UVs side by side"),
        tr(
            "Give each copy its own square of the texture, so the copies don't overlap there and \
             can be painted or baked one by one. Off, every copy shares the shape's UVs",
        ),
    );
    rows.toggle(ui, &mut options.uvs_side_by_side, tr("UVs side by side"));
    ui.end_row();
}
