//! The rows about the object itself: its heading, name, shape settings,
//! collision shape and smoothing.

use super::fields::{Value, fields};
use super::{AttributesPanel, FIELD_WIDTH, LABEL_WIDTH, label, stepper};
use crate::objects::{DocRequest, kind_icon};
use crate::text::tr;
use crate::theme;
use crate::units::Units;
use egui::{Ui, vec2};
use mesh::primitives::MAX_SIZE;
use scene::{Command, Document, ObjectId, ObjectKind, Primitive};

impl AttributesPanel {
    /// The name, typed in place; kept when Enter is pressed or the box is
    /// left.
    pub(super) fn name_row(
        &mut self,
        ui: &mut Ui,
        id: ObjectId,
        name: &str,
        out: &mut Vec<DocRequest>,
    ) {
        label(ui, tr("Name"), tr("The name shown in the Objects panel"));
        if self
            .name_draft
            .as_ref()
            .is_none_or(|(draft_id, _)| *draft_id != id)
        {
            self.name_draft = Some((id, name.to_string()));
        }
        let Some((_, draft)) = self.name_draft.as_mut() else {
            return;
        };
        let response = ui.add(egui::TextEdit::singleline(draft).desired_width(FIELD_WIDTH));
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, tr("Object name"))
        });
        if response.lost_focus() {
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) || draft.trim().is_empty() {
                *draft = name.to_string();
            } else if draft.as_str() != name {
                out.push(DocRequest::Run(Command::Rename {
                    id,
                    name: draft.clone(),
                }));
            }
        } else if !response.has_focus() && draft.as_str() != name {
            // Renamed elsewhere (or undone): show the current name.
            *draft = name.to_string();
        }
        ui.end_row();
    }

    /// A primitive's settings. A change anywhere gives the shape new
    /// settings; every frame of one drag is the same undo step.
    pub(super) fn shape_rows(
        &mut self,
        ui: &mut Ui,
        id: ObjectId,
        primitive: Primitive,
        units: Units,
        out: &mut Vec<DocRequest>,
    ) {
        let mut edited = primitive;
        let mut changed = false;
        egui::Grid::new("attributes_shape")
            .num_columns(2)
            .min_col_width(LABEL_WIDTH)
            .spacing(vec2(12.0, 6.0))
            .show(ui, |ui| {
                for field in fields(&mut edited) {
                    label(ui, field.label, field.help);
                    let touched = match field.value {
                        Value::Length(value, min) => {
                            stepper::length(ui, value, (min, MAX_SIZE), units, field.label)
                        }
                        Value::Count(value, low, high) => {
                            stepper::count(ui, value, (low, high), field.label)
                        }
                        Value::Angle(value, low, high) => {
                            stepper::degrees(ui, value, (low, high), field.label)
                        }
                        Value::Toggle(value) => {
                            let response = ui.checkbox(value, "");
                            response.widget_info(|| {
                                egui::WidgetInfo::selected(
                                    egui::WidgetType::Checkbox,
                                    true,
                                    *value,
                                    field.label,
                                )
                            });
                            stepper::Touched {
                                changed: response.changed(),
                                began: response.clicked(),
                            }
                        }
                    };
                    if touched.began {
                        // A new drag, a new value typed, or a click: a new
                        // undo step.
                        self.gesture += 1;
                    }
                    changed |= touched.changed;
                    ui.end_row();
                }
            });
        if changed && edited != primitive {
            out.push(DocRequest::RunMerging(
                Command::SetPrimitive {
                    ids: vec![id],
                    primitive: edited,
                },
                self.gesture,
            ));
        }
    }
}

/// The selected object's settings, if it is a primitive.
pub(super) fn primitive_of(doc: &Document, id: ObjectId) -> Option<Primitive> {
    match doc.scene().object(id)?.kind {
        ObjectKind::Primitive(p) => Some(p),
        _ => None,
    }
}

/// The object's icon, name and kind, at the top of the panel.
pub(super) fn heading(ui: &mut Ui, kind: &ObjectKind, name: &str) {
    ui.horizontal(|ui| {
        let (icon, color) = kind_icon(kind);
        let (rect, _) = ui.allocate_exact_size(vec2(18.0, 18.0), egui::Sense::hover());
        crate::icons::paint(ui.painter(), rect, icon, color);
        // One label, so it reads as one line ("Cube (cube)").
        let mut job = egui::text::LayoutJob::default();
        let font = egui::TextStyle::Body.resolve(ui.style());
        job.append(
            name,
            0.0,
            egui::TextFormat::simple(font.clone(), theme::TEXT_STRONG),
        );
        job.append(
            &format!(" {}", kind_label(kind)),
            2.0,
            egui::TextFormat::simple(font, theme::TEXT_WEAK),
        );
        ui.label(job);
    });
    ui.add_space(4.0);
}

/// "Cube", "Group" and so on, for the heading.
fn kind_label(kind: &ObjectKind) -> String {
    match kind {
        ObjectKind::Primitive(p) => format!("({})", p.name().to_lowercase()),
        ObjectKind::Mesh(_) => "(mesh)".to_string(),
        ObjectKind::Instance(_) => "(instance)".to_string(),
        ObjectKind::Curve(c) => match &c.solid {
            scene::CurveSolid::Line => format!("({})", tr("curve")),
            scene::CurveSolid::Sweep(_) => format!("({})", tr("curve, swept")),
            scene::CurveSolid::Lathe(_) => format!("({})", tr("curve, lathed")),
        },
        ObjectKind::Lattice(_) => format!("({})", tr("lattice cage")),
        _ => "(group)".to_string(),
    }
}

/// The Collision row (decision 0078): the collision shape a game engine
/// makes for the object, as Godot reads it from an export made for it.
pub(super) fn collision_row(
    ui: &mut Ui,
    id: ObjectId,
    current: scene::Collision,
    out: &mut Vec<DocRequest>,
) {
    label(
        ui,
        tr("Collision"),
        tr(
            "The collision shape Godot makes for it when you export for Godot: what the player \
         bumps into",
        ),
    );
    let mut chosen = current;
    let response = egui::ComboBox::from_id_salt(("collision", id))
        .selected_text(current.name())
        .width(FIELD_WIDTH)
        .show_ui(ui, |ui| {
            for kind in scene::Collision::ALL {
                ui.selectable_value(&mut chosen, kind, kind.name())
                    .on_hover_text(kind.description());
            }
        })
        .response
        .on_hover_text(current.description());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, tr("Collision shape"))
    });
    if chosen != current {
        out.push(DocRequest::Run(Command::SetCollision {
            ids: vec![id],
            collision: chosen,
        }));
    }
    ui.end_row();
}

/// The Smoothing row (decision 0126): whether the object is smoothed by
/// angle, and from what angle edges shade hard; unticked, only the edges
/// marked hard do.
pub(super) fn smoothing_row(
    ui: &mut Ui,
    id: ObjectId,
    current: mesh::Hardness,
    out: &mut Vec<DocRequest>,
) {
    label(
        ui,
        tr("Smoothing"),
        tr(
            "How the shading breaks into crisp edges. By angle: wherever polygons meet more \
         sharply than the angle, after every change. Unticked: only the edges you mark hard \
         (Mesh → Mark hard edges). Marking edges switches it to that.",
        ),
    );
    ui.horizontal(|ui| {
        let mut by_angle = matches!(current, mesh::Hardness::ByAngle(_));
        let switch = ui.checkbox(&mut by_angle, tr("By angle"));
        switch.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::Checkbox,
                true,
                by_angle,
                tr(SMOOTH_BY_ANGLE),
            )
        });
        if switch.changed() {
            out.push(DocRequest::Run(Command::SetHardness {
                ids: vec![id],
                hardness: if by_angle {
                    mesh::Hardness::ByAngle(scene::STANDARD_SMOOTHING_ANGLE)
                } else {
                    mesh::Hardness::Marked
                },
            }));
        }
        if let mesh::Hardness::ByAngle(angle) = current {
            let mut value = angle;
            let response = ui
                .add(
                    egui::DragValue::new(&mut value)
                        .range(0.0..=180.0)
                        .speed(0.5)
                        .suffix("°"),
                )
                .on_hover_text(tr(
                    "Edges where polygons meet more sharply than this are crisp",
                ));
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::DragValue, true, tr(SMOOTHING_ANGLE))
            });
            if value != angle && value.is_finite() {
                out.push(DocRequest::RunMerging(
                    Command::SetHardness {
                        ids: vec![id],
                        hardness: mesh::Hardness::ByAngle(value.clamp(0.0, 180.0)),
                    },
                    SMOOTHING_KEY ^ id.get(),
                ));
            }
        }
    });
    ui.end_row();
}

/// The Smoothing row's switch and angle, for screen readers and tests.
pub(crate) const SMOOTH_BY_ANGLE: &str = crate::text!("Object smoothed by angle");

pub(crate) const SMOOTHING_ANGLE: &str = crate::text!("Smoothing angle");

/// Merge key for dragging an object's smoothing angle (one drag, one step).
const SMOOTHING_KEY: u64 = 0x5300_0000_0000_0000;
