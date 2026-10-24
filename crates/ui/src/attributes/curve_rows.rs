//! A curve object's rows in the Attributes panel
//! ([decision 0138](../../../../docs/decisions/0138-curves-sweep-lathe.md)):
//! its ready-made settings or its points, whether it is closed, and the
//! solid it makes (none, a sweep or a lathe) with that solid's settings.
//! Every change is a command; one drag of a value is one undo step.

use super::{AttributesPanel, FIELD_WIDTH, LABEL_WIDTH, label, section, stepper};
use crate::objects::DocRequest;
use crate::text::tr;
use crate::theme;
use crate::units::Units;
use egui::{RichText, Ui, vec2};
use mesh::curve::{lathe, ready, sweep};
use scene::{
    Command, CurveEdit, CurveObject, CurveShape, CurveSolid, Detail, Document, LatheAxis,
    LatheShape, ObjectId, ObjectKind, ReadyCurve, Sweep, SweepProfile,
};

/// The largest length a curve setting takes, in centimetres: 1 km.
const MAX_LENGTH: f32 = 100_000.0;

/// What a row did this frame.
#[derive(Default)]
pub(super) struct Rows {
    /// Something changed.
    pub(super) changed: bool,
    /// A new drag, typing or click began: a new undo step.
    pub(super) began: bool,
}

impl Rows {
    pub(super) fn take(&mut self, touched: stepper::Touched) {
        self.changed |= touched.changed;
        self.began |= touched.began;
    }

    pub(super) fn toggle(&mut self, ui: &mut Ui, value: &mut bool, name: &str) {
        let response = ui.checkbox(value, "");
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, *value, name)
        });
        self.changed |= response.changed();
        self.began |= response.clicked();
    }
}

/// An axis's name.
fn axis_name(axis: LatheAxis) -> &'static str {
    match axis {
        LatheAxis::X => "X",
        LatheAxis::Y => "Y",
        LatheAxis::Z => "Z",
    }
}

impl AttributesPanel {
    /// The Curve section and its solid's section.
    pub(super) fn curve_section(
        &mut self,
        ui: &mut Ui,
        doc: &Document,
        id: ObjectId,
        units: Units,
        out: &mut Vec<DocRequest>,
    ) {
        let Some(curve) = doc.scene().object(id).and_then(|o| o.kind.curve()).cloned() else {
            return;
        };
        section(ui, tr("Curve"));
        egui::Grid::new("attributes_curve")
            .num_columns(2)
            .min_col_width(LABEL_WIDTH)
            .spacing(vec2(12.0, 6.0))
            .show(ui, |ui| {
                self.curve_shape_rows(ui, id, &curve, units, out);
                self.solid_row(ui, id, &curve, out);
            });
        // What uses it as a cross-section.
        let users: Vec<String> = doc
            .scene()
            .walk()
            .into_iter()
            .filter_map(|(other, _)| {
                let object = doc.scene().object(other)?;
                (object.kind.curve()?.solid.profile_object() == Some(id))
                    .then(|| object.name.clone())
            })
            .collect();
        if !users.is_empty() {
            ui.label(
                RichText::new(crate::trf!("The cross-section of: {}", users.join(", ")))
                    .small()
                    .color(theme::TEXT_WEAK),
            );
        }
        match curve.solid {
            CurveSolid::Line => {}
            CurveSolid::Sweep(s) => self.sweep_section(ui, doc, id, s, units, out),
            CurveSolid::Lathe(l) => self.lathe_section(ui, id, l, out),
        }
        let profile = doc.scene().profile_curve(id);
        if let Some(problem) = curve.problem(profile.as_ref()) {
            ui.label(
                RichText::new(crate::trf!("Nothing to show: {}.", problem))
                    .small()
                    .color(theme::WARNING),
            );
        }
        if self.curve.points_mode {
            self.curve_points_section(ui, doc, id, units);
        }
    }

    /// A ready-made curve's settings, or a drawn one's points and Closed.
    fn curve_shape_rows(
        &mut self,
        ui: &mut Ui,
        id: ObjectId,
        curve: &CurveObject,
        units: Units,
        out: &mut Vec<DocRequest>,
    ) {
        match &curve.shape {
            CurveShape::Ready(ready) => {
                let mut edited = *ready;
                let mut rows = Rows::default();
                let length_row = |ui: &mut Ui, rows: &mut Rows, value: &mut f32, name, help| {
                    label(ui, name, help);
                    rows.take(stepper::length(ui, value, (0.01, MAX_LENGTH), units, name));
                    ui.end_row();
                };
                match &mut edited {
                    ReadyCurve::Line { length } => {
                        length_row(
                            ui,
                            &mut rows,
                            length,
                            tr("Length"),
                            tr("How long the line is"),
                        );
                    }
                    ReadyCurve::Circle { radius } => {
                        length_row(
                            ui,
                            &mut rows,
                            radius,
                            tr("Radius"),
                            tr("How far from the middle"),
                        );
                    }
                    ReadyCurve::Arc { radius, angle } => {
                        length_row(
                            ui,
                            &mut rows,
                            radius,
                            tr("Radius"),
                            tr("How far from the middle"),
                        );
                        label(ui, tr("Angle"), tr("How far round the arc goes"));
                        rows.take(stepper::degrees(ui, angle, (1.0, 360.0), tr("Angle")));
                        ui.end_row();
                    }
                    ReadyCurve::Spiral {
                        radius,
                        height,
                        turns,
                    } => {
                        length_row(
                            ui,
                            &mut rows,
                            radius,
                            tr("Radius"),
                            tr("How far from the middle"),
                        );
                        label(ui, tr("Climb"), tr("How high it climbs, bottom to top"));
                        rows.take(stepper::length(
                            ui,
                            height,
                            (0.0, MAX_LENGTH),
                            units,
                            tr("Climb"),
                        ));
                        ui.end_row();
                        label(ui, tr("Turns"), tr("How many times round it goes"));
                        rows.take(stepper::number(
                            ui,
                            turns,
                            (0.25, ready::MAX_TURNS),
                            (0.25, 2),
                            tr("Turns"),
                        ));
                        ui.end_row();
                    }
                }
                if rows.began {
                    self.gesture += 1;
                }
                if rows.changed && edited != *ready {
                    out.push(DocRequest::RunMerging(
                        Command::SetReadyCurve {
                            ids: vec![id],
                            ready: edited,
                        },
                        self.gesture,
                    ));
                }
            }
            CurveShape::Points(points) => {
                label(
                    ui,
                    tr("Points"),
                    tr("How many points it has: press 1 to move them"),
                );
                ui.label(points.points().len().to_string());
                ui.end_row();
                label(
                    ui,
                    tr("Closed"),
                    tr("Whether the last point joins the first, making a loop"),
                );
                let mut closed = points.closed();
                let can = points.points().len() >= 3;
                let response = ui.add_enabled(can, egui::Checkbox::new(&mut closed, ""));
                response.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::Checkbox,
                        can,
                        closed,
                        tr("Curve closed"),
                    )
                });
                if response.changed() {
                    out.push(DocRequest::Run(Command::EditCurve {
                        id,
                        edit: CurveEdit::SetClosed(closed),
                    }));
                }
                ui.end_row();
            }
        }
    }

    /// What the curve makes: a line only, a sweep or a lathe.
    fn solid_row(
        &mut self,
        ui: &mut Ui,
        id: ObjectId,
        curve: &CurveObject,
        out: &mut Vec<DocRequest>,
    ) {
        label(
            ui,
            tr("Makes"),
            tr(
                "What the curve makes: nothing (a line, to guide or be another sweep's \
                 cross-section), a sweep (a shape carried along it: pipes, cables, rails) or a \
                 lathe (the curve spun round an axis: vases, bottles, wheels)",
            ),
        );
        let current = curve.solid;
        let mut chosen = current;
        let choices = [
            CurveSolid::Line,
            CurveSolid::Sweep(Sweep::default()),
            CurveSolid::Lathe(LatheShape::default()),
        ];
        let same_kind =
            |a: &CurveSolid, b: &CurveSolid| std::mem::discriminant(a) == std::mem::discriminant(b);
        let response = egui::ComboBox::from_id_salt(("curve_solid", id))
            .selected_text(current.name())
            .width(FIELD_WIDTH)
            .show_ui(ui, |ui| {
                for choice in choices {
                    let on = same_kind(&current, &choice);
                    if ui.selectable_label(on, choice.name()).clicked() && !on {
                        chosen = choice;
                    }
                }
            })
            .response;
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, tr("What the curve makes"))
        });
        if chosen != current {
            out.push(DocRequest::Run(Command::SetCurveSolid {
                ids: vec![id],
                solid: chosen,
            }));
        }
        ui.end_row();
    }

    /// The Sweep section: its cross-section and how it follows the curve.
    fn sweep_section(
        &mut self,
        ui: &mut Ui,
        doc: &Document,
        id: ObjectId,
        current: Sweep,
        units: Units,
        out: &mut Vec<DocRequest>,
    ) {
        section(ui, tr("Sweep"));
        let mut edited = current;
        let mut rows = Rows::default();
        // The other curve objects, to take a cross-section from.
        let others: Vec<(ObjectId, String)> = doc
            .scene()
            .walk()
            .into_iter()
            .filter(|(other, _)| *other != id)
            .filter_map(|(other, _)| {
                let object = doc.scene().object(other)?;
                matches!(object.kind, ObjectKind::Curve(_)).then(|| (other, object.name.clone()))
            })
            .collect();
        let profile_name = |p: SweepProfile| match p {
            SweepProfile::Curve(other) => others
                .iter()
                .find(|(o, _)| *o == other)
                .map_or_else(|| tr("Another curve").to_string(), |(_, n)| n.clone()),
            built_in => built_in.name().to_string(),
        };
        egui::Grid::new("attributes_sweep")
            .num_columns(2)
            .min_col_width(LABEL_WIDTH)
            .spacing(vec2(12.0, 6.0))
            .show(ui, |ui| {
                label(
                    ui,
                    tr("Cross-section"),
                    tr(
                        "The shape carried along the curve: a circle for pipes and cables, a \
                         square or rectangle for bars, or another curve for rails and trims \
                         (laid flat on its plane, round its pivot)",
                    ),
                );
                let response = egui::ComboBox::from_id_salt(("sweep_profile", id))
                    .selected_text(profile_name(current.profile))
                    .width(FIELD_WIDTH)
                    .show_ui(ui, |ui| {
                        for built_in in SweepProfile::BUILT_IN {
                            ui.selectable_value(&mut edited.profile, built_in, built_in.name());
                        }
                        for (other, name) in &others {
                            ui.selectable_value(
                                &mut edited.profile,
                                SweepProfile::Curve(*other),
                                crate::trf!("Curve: {}", name),
                            );
                        }
                    })
                    .response;
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, tr("Cross-section"))
                });
                if edited.profile != current.profile {
                    rows.changed = true;
                    rows.began = true;
                }
                ui.end_row();
                let range = (sweep::MIN_SIZE, sweep::MAX_SIZE);
                match edited.profile {
                    SweepProfile::Circle => {
                        label(ui, tr("Width"), tr("How wide the circle is, across"));
                        rows.take(stepper::length(
                            ui,
                            &mut edited.width,
                            range,
                            units,
                            tr("Width"),
                        ));
                        ui.end_row();
                        label(
                            ui,
                            tr("Sides"),
                            tr("How many straight sides make the circle"),
                        );
                        rows.take(stepper::count(
                            ui,
                            &mut edited.sides,
                            (*sweep::SIDES.start(), *sweep::SIDES.end()),
                            tr("Sides"),
                        ));
                        ui.end_row();
                    }
                    SweepProfile::Square => {
                        label(ui, tr("Width"), tr("How wide the square is"));
                        rows.take(stepper::length(
                            ui,
                            &mut edited.width,
                            range,
                            units,
                            tr("Width"),
                        ));
                        ui.end_row();
                    }
                    SweepProfile::Rectangle => {
                        label(
                            ui,
                            tr("Width"),
                            tr("How wide the rectangle is, across the curve"),
                        );
                        rows.take(stepper::length(
                            ui,
                            &mut edited.width,
                            range,
                            units,
                            tr("Width"),
                        ));
                        ui.end_row();
                        label(
                            ui,
                            tr("Height"),
                            tr("How tall the rectangle is, up from the curve"),
                        );
                        rows.take(stepper::length(
                            ui,
                            &mut edited.height,
                            range,
                            units,
                            tr("Height"),
                        ));
                        ui.end_row();
                    }
                    SweepProfile::Curve(_) => {}
                }
                detail_rows(ui, &mut edited.detail, &mut rows);
                label(
                    ui,
                    tr("Twist"),
                    tr(
                        "How far the cross-section turns round the curve from start to end (whole \
                         turns on a closed curve)",
                    ),
                );
                rows.take(stepper::degrees(
                    ui,
                    &mut edited.twist,
                    (-sweep::MAX_TWIST, sweep::MAX_TWIST),
                    tr("Twist"),
                ));
                ui.end_row();
                label(
                    ui,
                    tr("Caps"),
                    tr("Close an open curve's ends with flat faces"),
                );
                rows.toggle(ui, &mut edited.caps, tr("Caps"));
                ui.end_row();
            });
        if rows.began {
            self.gesture += 1;
        }
        if rows.changed && edited != current {
            out.push(DocRequest::RunMerging(
                Command::SetCurveSolid {
                    ids: vec![id],
                    solid: CurveSolid::Sweep(edited),
                },
                self.gesture,
            ));
        }
    }

    /// The Lathe section: its axis, how far round, and how finely.
    fn lathe_section(
        &mut self,
        ui: &mut Ui,
        id: ObjectId,
        current: LatheShape,
        out: &mut Vec<DocRequest>,
    ) {
        section(ui, tr("Lathe"));
        let mut edited = current;
        let mut rows = Rows::default();
        egui::Grid::new("attributes_lathe")
            .num_columns(2)
            .min_col_width(LABEL_WIDTH)
            .spacing(vec2(12.0, 6.0))
            .show(ui, |ui| {
                label(
                    ui,
                    tr("Axis"),
                    tr("Which of the curve's own axes it spins round, through its pivot"),
                );
                let response = egui::ComboBox::from_id_salt(("lathe_axis", id))
                    .selected_text(axis_name(current.axis))
                    .width(FIELD_WIDTH)
                    .show_ui(ui, |ui| {
                        for axis in LatheAxis::ALL {
                            ui.selectable_value(&mut edited.axis, axis, axis_name(axis));
                        }
                    })
                    .response;
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, tr("Lathe axis"))
                });
                if edited.axis != current.axis {
                    rows.changed = true;
                    rows.began = true;
                }
                ui.end_row();
                label(
                    ui,
                    tr("Spin angle"),
                    tr("How far round it spins: 360° all the way"),
                );
                rows.take(stepper::degrees(
                    ui,
                    &mut edited.angle,
                    (1.0, 360.0),
                    tr("Spin angle"),
                ));
                ui.end_row();
                label(
                    ui,
                    tr("Segments round"),
                    tr("How many pieces round the axis"),
                );
                rows.take(stepper::count(
                    ui,
                    &mut edited.segments,
                    (*lathe::SEGMENTS.start(), *lathe::SEGMENTS.end()),
                    tr("Segments round"),
                ));
                ui.end_row();
                detail_rows(ui, &mut edited.detail, &mut rows);
                label(
                    ui,
                    tr("Caps"),
                    tr("Close open ends (and the sides of a part turn) with flat faces"),
                );
                rows.toggle(ui, &mut edited.caps, tr("Caps"));
                ui.end_row();
            });
        if rows.began {
            self.gesture += 1;
        }
        if rows.changed && edited != current {
            out.push(DocRequest::RunMerging(
                Command::SetCurveSolid {
                    ids: vec![id],
                    solid: CurveSolid::Lathe(edited),
                },
                self.gesture,
            ));
        }
    }
}

/// How finely a solid follows its curve: Segments and Adaptive.
fn detail_rows(ui: &mut Ui, detail: &mut Detail, rows: &mut Rows) {
    label(
        ui,
        tr("Segments"),
        tr(
            "Pieces along the curve: with Adaptive, for each quarter turn it bends; otherwise \
             for each stretch between two points",
        ),
    );
    rows.take(stepper::count(
        ui,
        &mut detail.segments,
        (1, Detail::MAX_SEGMENTS),
        tr("Segments"),
    ));
    ui.end_row();
    label(
        ui,
        tr("Adaptive"),
        tr("More pieces where the curve bends, one along a straight stretch"),
    );
    rows.toggle(ui, &mut detail.adaptive, tr("Adaptive"));
    ui.end_row();
}
