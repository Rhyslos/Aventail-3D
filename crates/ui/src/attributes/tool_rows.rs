//! The rows of the tools' settings, shared by the Tool section and an
//! opened edit: segments, direction, extrude and bend, bridge, angle,
//! cuts, position, groove, bevel options, profile and Keep together.

use super::{FIELD_WIDTH, label, stepper};
use crate::text::tr;
use crate::theme;
use crate::units::Units;
use egui::{RichText, Ui};
use mesh::primitives::MAX_SIZE;

/// A bevel's rows, with its label.
pub(super) fn segments_row(ui: &mut Ui, segments: &mut u32) -> stepper::Touched {
    label(
        ui,
        tr("Segments"),
        tr("How many rows the bevel has: 1 is a flat cut (a chamfer), more make it round"),
    );
    stepper::count(ui, segments, (1, mesh::bevel::MAX_SEGMENTS), tr("Segments"))
}

/// The name of an extrude's way, as listed.
pub(crate) fn direction_name(direction: scene::ExtrudeDirection) -> &'static str {
    use scene::ExtrudeDirection as D;
    match direction {
        D::Straight => tr("Straight out"),
        D::AlongNormals => tr("Along normals"),
        D::AlongX => tr("Along X"),
        D::AlongY => tr("Along Y"),
        D::AlongZ => tr("Along Z"),
    }
}

/// What an extrude's way does, in plain words.
fn direction_help(direction: scene::ExtrudeDirection) -> &'static str {
    use scene::ExtrudeDirection as D;
    match direction {
        D::Straight => tr(
            "Every point goes the same way: straight out along the polygons' average facing. \
             The end keeps the polygons' shape.",
        ),
        D::AlongNormals => tr(
            "Each point goes along its own facing, so the end follows the surface's curve at \
             an even thickness, like puffing it out.",
        ),
        D::AlongX | D::AlongY | D::AlongZ => tr(
            "Straight along this axis of the object (its own axes), out the way the polygons \
             face: pull a sloped patch out level, say.",
        ),
    }
}

/// Extrude's shape: Direction, End size, Steps and Flat end, a row each
/// (decision 0070). Greyed out (`enabled` false) while a bend is set: a
/// bent extrude goes out its own way.
pub(super) fn extrude_rows(
    ui: &mut Ui,
    shape: &mut scene::ExtrudeShape,
    enabled: bool,
) -> stepper::Touched {
    let mut out = stepper::Touched::default();
    let mut add = |t: stepper::Touched| {
        out.changed |= t.changed;
        out.began |= t.began;
    };
    let why = tr("A bent extrude goes out its own way: set the bend's Angle to 0 for these.");
    label(
        ui,
        tr("Direction"),
        tr("Which way the polygons go. Straight out is the standard."),
    );
    ui.add_enabled_ui(enabled, |ui| {
        let before = shape.direction;
        let response = egui::ComboBox::from_id_salt("extrude_direction")
            .width(FIELD_WIDTH)
            .selected_text(direction_name(shape.direction))
            .show_ui(ui, |ui| {
                for direction in scene::ExtrudeDirection::ALL {
                    ui.selectable_value(&mut shape.direction, direction, direction_name(direction))
                        .on_hover_text(direction_help(direction));
                }
            })
            .response
            .on_hover_text(direction_help(shape.direction))
            .on_disabled_hover_text(why);
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, tr("Extrude direction"))
        });
        if shape.direction != before {
            add(stepper::Touched {
                changed: true,
                began: true,
            });
        }
    });
    ui.end_row();
    label(
        ui,
        tr("End size"),
        tr(
            "How big the new end is, compared with where it started: 100% the same, less tapers \
         it in (0% to a point), more flares it out.",
        ),
    );
    ui.add_enabled_ui(enabled, |ui| {
        add(stepper::percent(
            ui,
            &mut shape.end_size,
            (0.0, mesh::edit::MAX_END_SIZE),
            tr("End size"),
        ));
    });
    ui.end_row();
    label(
        ui,
        tr("Steps"),
        tr(
            "How many rows the new walls have: more give loops to bend or shape later. Ctrl + \
         wheel changes it.",
        ),
    );
    ui.add_enabled_ui(enabled, |ui| {
        add(stepper::count(
            ui,
            &mut shape.steps,
            (1, mesh::edit::MAX_EXTRUDE_STEPS),
            tr("Extrude steps"),
        ));
    });
    ui.end_row();
    label(
        ui,
        tr("Flat end"),
        tr("Lay the new end flat, square to the way it went: a curved patch ends in a flat face."),
    );
    ui.add_enabled_ui(enabled, |ui| {
        let response = ui.checkbox(&mut shape.flat_end, "");
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::Checkbox,
                true,
                shape.flat_end,
                tr("Flat end"),
            )
        });
        if response.changed() {
            add(stepper::Touched {
                changed: true,
                began: true,
            });
        }
    });
    ui.end_row();
    *shape = shape.clamped();
    out
}

/// Extrude's bend: Angle, Direction, Bend radius, Segments and Rise per
/// turn, a row each (decisions 0050 and 0062).
pub(super) fn bend_rows(ui: &mut Ui, bend: &mut scene::Bend, units: Units) -> stepper::Touched {
    // A small heading, so the bend reads as its own group under the
    // extrude's Distance.
    ui.label(RichText::new(tr("Bend")).small().color(theme::TEXT_WEAK));
    ui.end_row();
    let mut out = stepper::Touched::default();
    let mut add = |t: stepper::Touched| {
        out.changed |= t.changed;
        out.began |= t.began;
    };
    let degrees = |ui: &mut Ui, value: &mut f32, most: f32, name: &str, help: &str| {
        label(ui, name, help);
        let touched = stepper::degrees(ui, value, (0.0, most), name);
        ui.end_row();
        touched
    };
    let most = if bend.radius > 0.0 {
        mesh::extrude::MAX_BEND_ANGLE
    } else {
        mesh::extrude::MAX_MITRE_ANGLE
    };
    add(degrees(
        ui,
        &mut bend.angle,
        most,
        tr("Angle"),
        tr(
            "How far the extrude turns before going on straight: 0° is straight, 90° a corner. \
         The pipe keeps its full width through the turn. With a bend radius it can go past \
         360° into a spiral (up to 3600°, ten turns): each loop climbs just clear of the one \
         before.",
        ),
    ));
    add(degrees(
        ui,
        &mut bend.direction,
        360.0,
        tr("Direction"),
        tr(
            "Which way round it turns: 0° turns up (an upright pipe turns towards +X), and it \
         goes round from there.",
        ),
    ));
    label(
        ui,
        tr("Bend radius"),
        tr(
            "How wide the turn is, from its middle to the middle of the pipe: 0 is a sharp \
         corner (a mitre joint, up to 150°), more is a smooth elbow.",
        ),
    );
    add(stepper::length(
        ui,
        &mut bend.radius,
        (0.0, MAX_SIZE),
        units,
        tr("Bend radius"),
    ));
    ui.end_row();
    label(
        ui,
        tr("Segments"),
        tr(
            "How many rows a smooth turn has (with a bend radius); for a spiral, how many each \
         full turn has. Ctrl + wheel changes it.",
        ),
    );
    add(stepper::count(
        ui,
        &mut bend.segments,
        (1, mesh::extrude::MAX_BEND_SEGMENTS),
        tr("Bend segments"),
    ));
    ui.end_row();
    // The rise only does something with a bend radius, so it's greyed out
    // without one.
    label(
        ui,
        tr("Rise per turn"),
        tr(
            "Extra room between a spiral's loops: how much further each full turn climbs \
         along the spiral's middle. Past one turn the loops already clear each other, so 0 \
         leaves them just apart; under one turn it tilts the bend into a ramp. Needs a bend \
         radius.",
        ),
    );
    let rounded = bend.radius > 0.0;
    ui.add_enabled_ui(rounded, |ui| {
        add(stepper::length(
            ui,
            &mut bend.rise,
            (0.0, MAX_SIZE),
            units,
            tr("Rise per turn"),
        ));
    });
    ui.end_row();
    *bend = bend.clamped();
    out
}

/// Bridge's Segments and Twist, a row each.
pub(super) fn bridge_rows(ui: &mut Ui, (segments, twist): &mut (u32, i32)) -> stepper::Touched {
    label(
        ui,
        tr("Segments"),
        tr(
            "How many rows the bridge has: more make a longer bridge bend smoothly later. Ctrl + \
         wheel changes it.",
        ),
    );
    let mut out = stepper::count(
        ui,
        segments,
        (1, mesh::join::MAX_BRIDGE_SEGMENTS),
        tr("Bridge segments"),
    );
    ui.end_row();
    label(
        ui,
        tr("Twist"),
        tr(
            "Turns a bridge between two closed loops round by whole steps, if the points pair up \
         wrongly and the band twists.",
        ),
    );
    let mut steps = (*twist + 1000) as u32;
    let t = stepper::count(ui, &mut steps, (0, 2000), tr("Twist"));
    if t.changed {
        *twist = steps as i32 - 1000;
    }
    out.changed |= t.changed;
    out.began |= t.began;
    ui.end_row();
    out
}

/// Slit's Cut through switch, with its label. Returns whether it changed.
pub(super) fn through_row(ui: &mut Ui, through: &mut bool) -> bool {
    label(
        ui,
        tr("Cut through"),
        tr(
            "On: everything under the line is cut, the far side too. Off: only what you \
         can see.",
        ),
    );
    let response = ui.checkbox(through, "");
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Checkbox,
            true,
            *through,
            tr("Cut through"),
        )
    });
    response.changed()
}

/// Hard edges by angle's angle, with its label.
pub(super) fn angle_row(ui: &mut Ui, angle: &mut f32) -> stepper::Touched {
    label(
        ui,
        tr("Angle"),
        tr(
            "Edges whose polygons meet more sharply than this become hard (a crisp edge); the \
         rest become soft (smooth). 0° is flat.",
        ),
    );
    let response = ui.add(
        egui::Slider::new(angle, 0.0..=mesh::hardness::MAX_ANGLE)
            .suffix("°")
            .custom_formatter(|v, _| format!("{v:.0}")),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Slider, true, tr("Angle")));
    stepper::Touched {
        changed: response.changed(),
        began: response.drag_started() || (response.changed() && !response.dragged()),
    }
}

/// Loop cut's number of cuts, with its label.
pub(super) fn cuts_row(ui: &mut Ui, cuts: &mut u32) -> stepper::Touched {
    label(
        ui,
        tr("Cuts"),
        tr("How many new loops are cut, spread evenly across the polygons"),
    );
    stepper::count(ui, cuts, (1, mesh::loopcut::MAX_CUTS), tr("Cuts"))
}

/// Where a loop cut sits across its edge, as a percentage, with its label.
pub(super) fn position_row(ui: &mut Ui, position: &mut f32) -> stepper::Touched {
    label(
        ui,
        tr("Position"),
        tr(
            "Where the cut sits across the polygons: 50% is the middle. With several \
         cuts, it slides them all together.",
        ),
    );
    let mut percent = *position * 100.0;
    let response = ui.add(
        egui::Slider::new(&mut percent, 0.0..=100.0)
            .suffix("%")
            // Shown as a whole percent, without rounding the value itself
            // (that would change the cut as soon as it is shown).
            .custom_formatter(|v, _| format!("{v:.0}")),
    );
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Slider, true, tr("Position")));
    if response.changed() {
        *position = (percent / 100.0).clamp(0.0, 1.0);
    }
    stepper::Touched {
        changed: response.changed(),
        began: response.drag_started() || (response.changed() && !response.dragged()),
    }
}

/// Groove's Width and Sides, a row each (decision 0086).
pub(super) fn groove_rows(
    ui: &mut Ui,
    width: &mut f32,
    sides: &mut scene::GrooveSides,
    units: Units,
) -> stepper::Touched {
    label(
        ui,
        tr("Groove width"),
        tr("How wide the groove (or ridge) is, across the line of edges"),
    );
    let mut out = stepper::length(
        ui,
        width,
        (0.0, mesh::groove::MAX_GROOVE_WIDTH),
        units,
        tr("Groove width"),
    );
    ui.end_row();
    label(
        ui,
        tr("Sides"),
        tr(
            "Straight: walls square to the surface and a flat bottom, a crisp panel line. Sloped: \
         the sides slope down to a fold along the middle, a V.",
        ),
    );
    ui.horizontal(|ui| {
        for (choice, name) in [
            (scene::GrooveSides::Straight, tr("Straight")),
            (scene::GrooveSides::Sloped, tr("Sloped")),
        ] {
            if ui.selectable_label(*sides == choice, name).clicked() && *sides != choice {
                *sides = choice;
                out.changed = true;
                out.began = true;
            }
        }
    });
    ui.end_row();
    out
}

/// A bevel's further settings (decision 0085), a row each: Skew with
/// Flip side, Even width, Stop at neighbours (Stop at overlaps before
/// decision 0141), and Weld overlapping points with its distance.
pub(super) fn bevel_option_rows(
    ui: &mut Ui,
    options: &mut scene::BevelOptions,
    units: Units,
) -> stepper::Touched {
    let mut out = stepper::Touched::default();
    let mut add = |t: stepper::Touched| {
        out.changed |= t.changed;
        out.began |= t.began;
    };
    let clicked = stepper::Touched {
        changed: true,
        began: true,
    };
    label(
        ui,
        tr("Skew"),
        tr(
            "Puts more of the bevel on one side of the edges than the other: 0% is even. Along a \
         loop it stays on the same side all the way round. Flip side swaps the side.",
        ),
    );
    ui.horizontal(|ui| {
        let mut percent = options.skew * 100.0;
        let response = ui.add(
            egui::Slider::new(&mut percent, -100.0..=100.0)
                .suffix("%")
                .show_value(true),
        );
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Slider, true, tr("Skew")));
        if response.changed() {
            options.skew = (percent / 100.0).clamp(-1.0, 1.0);
        }
        add(stepper::Touched {
            changed: response.changed(),
            began: response.drag_started() || (response.changed() && !response.dragged()),
        });
        let mut flip = options.flip_side;
        let flip_button = ui
            .toggle_value(&mut flip, tr("Flip side"))
            .on_hover_text(tr("Swap which side of the edges the skew favours"));
        if flip_button.changed() {
            options.flip_side = flip;
            add(clicked);
        }
    });
    ui.end_row();
    let check = |ui: &mut Ui, value: &mut bool, name: &str, help: &str| {
        label(ui, name, help);
        let response = ui.checkbox(value, "");
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, *value, name)
        });
        ui.end_row();
        response.changed()
    };
    if check(
        ui,
        &mut options.even,
        tr("Even width"),
        tr(
            "Make every bevelled edge's new polygon the same width, however sharp or flat the \
         edge is (the width you set is what a right-angled edge gets)",
        ),
    ) {
        add(clicked);
    }
    if check(
        ui,
        &mut options.clamp,
        tr("Stop at neighbours"),
        tr(
            "Keep the bevel from crossing over itself: where it would run past the end of an \
         edge or into the bevel coming the other way, it stops there, so corners meet instead \
         of passing each other. Only the points that need it are narrowed",
        ),
    ) {
        add(clicked);
    }
    if check(
        ui,
        &mut options.weld,
        tr("Weld points"),
        tr(
            "Weld overlapping points: new points that land on top of each other (where bevels \
         meet) become one, so there are no doubled points or tiny slivers",
        ),
    ) {
        add(clicked);
    }
    label(
        ui,
        tr("Weld distance"),
        tr("How close points must be to weld: those nearer than this become one"),
    );
    ui.add_enabled_ui(options.weld, |ui| {
        add(stepper::length(
            ui,
            &mut options.weld_distance,
            (0.0, mesh::bevel::MAX_WELD_DISTANCE),
            units,
            tr("Weld distance"),
        ));
    });
    ui.end_row();
    *options = options.clamped();
    out
}

/// A bevel's profile as a percentage, with its label.
pub(super) fn profile_row(ui: &mut Ui, profile: &mut f32) -> stepper::Touched {
    label(
        ui,
        tr("Profile"),
        tr(
            "The shape across the bevel: 100% is round, 0% is flat, -100% curves inward \
         (hollow, like a cove). Needs 2 or more segments to show.",
        ),
    );
    let mut percent = *profile * 100.0;
    let response = ui.add(egui::Slider::new(&mut percent, -100.0..=100.0).suffix("%"));
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Slider, true, tr("Profile")));
    if response.changed() {
        *profile = (percent / 100.0).clamp(-1.0, 1.0);
    }
    stepper::Touched {
        changed: response.changed(),
        began: response.drag_started() || (response.changed() && !response.dragged()),
    }
}

/// The Keep together switch and its label. Returns whether it changed.
pub(super) fn keep_together(ui: &mut Ui, together: &mut bool) -> bool {
    label(
        ui,
        tr("Keep together"),
        tr(
            "On: polygons that touch move as one piece, like a block pulled out of a \
         wall. Off: each polygon on its own, a separate bump for each.",
        ),
    );
    let response = ui.checkbox(together, "");
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Checkbox,
            true,
            *together,
            tr("Keep together"),
        )
    });
    response.changed()
}
