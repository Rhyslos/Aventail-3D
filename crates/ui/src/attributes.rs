//! The Attribute Manager: the settings of the selected object. Its name
//! and whether it is shown, and for a primitive its shape settings (size,
//! segments and so on), which change the shape as you drag or type.
//!
//! Like the Object Manager, the panel never changes the scene itself: it
//! hands back commands. Dragging a value changes it every frame, so those
//! commands are merged into one undo step per drag (or per value typed).

use crate::objects::{DocRequest, shape_icon};
use crate::panels::placeholder;
use crate::theme::{self, tint};
use egui::{DragValue, RichText, Ui, vec2};
use mesh::primitives::{MAX_SEGMENTS, MAX_SIDES, MAX_SIZE, MIN_SIDES, MIN_SIZE};
use scene::{Command, Document, ObjectId, ObjectKind, Primitive};

/// Width of the column of setting names, so both sections line up.
const LABEL_WIDTH: f32 = 110.0;
/// Width of a value field.
const FIELD_WIDTH: f32 = 130.0;

/// The Attribute Manager's own state.
#[derive(Debug, Clone, Default)]
pub(crate) struct AttributesPanel {
    /// Counts drags, typed values and clicks, so each one is its own undo
    /// step (and every frame of one drag is the same step).
    gesture: u64,
    /// A name being typed, for which object.
    name_draft: Option<(ObjectId, String)>,
    /// Settings copied with Edit → Copy settings.
    clipboard: Option<Primitive>,
}

/// What a setting holds, with its limits.
enum Value<'a> {
    /// A length in centimetres, from the minimum given.
    Length(&'a mut f32, f32),
    /// A count, between the limits given.
    Count(&'a mut u32, u32, u32),
    /// On or off.
    Toggle(&'a mut bool),
}

/// One row of shape settings.
struct Field<'a> {
    label: &'static str,
    help: &'static str,
    value: Value<'a>,
}

const fn field<'a>(label: &'static str, help: &'static str, value: Value<'a>) -> Field<'a> {
    Field { label, help, value }
}

/// The settings of a primitive, in the order the panel lists them, each
/// borrowing the value it edits.
fn fields(primitive: &mut Primitive) -> Vec<Field<'_>> {
    use Value::{Count, Length, Toggle};
    let segments = |v| Count(v, 1, MAX_SEGMENTS);
    let sides = |v| Count(v, MIN_SIDES, MAX_SIDES);
    match primitive {
        Primitive::Cube(p) => {
            let [sx, sy, sz] = &mut p.segments;
            vec![
                field("Width", "Size along X", Length(&mut p.size.x, MIN_SIZE)),
                field("Depth", "Size along Y", Length(&mut p.size.y, MIN_SIZE)),
                field(
                    "Height",
                    "Size along Z (up)",
                    Length(&mut p.size.z, MIN_SIZE),
                ),
                field(
                    "Segments X",
                    "How many pieces each side is cut into along X",
                    segments(sx),
                ),
                field(
                    "Segments Y",
                    "How many pieces each side is cut into along Y",
                    segments(sy),
                ),
                field(
                    "Segments Z",
                    "How many pieces each side is cut into along Z",
                    segments(sz),
                ),
            ]
        }
        Primitive::Plane(p) => {
            let [sx, sy] = &mut p.segments;
            vec![
                field("Width", "Size along X", Length(&mut p.width, MIN_SIZE)),
                field("Depth", "Size along Y", Length(&mut p.depth, MIN_SIZE)),
                field(
                    "Segments X",
                    "How many pieces it is cut into along X",
                    segments(sx),
                ),
                field(
                    "Segments Y",
                    "How many pieces it is cut into along Y",
                    segments(sy),
                ),
            ]
        }
        Primitive::Cylinder(p) => vec![
            field("Radius", "Half the width", Length(&mut p.radius, MIN_SIZE)),
            field(
                "Height",
                "Size along Z (up)",
                Length(&mut p.height, MIN_SIZE),
            ),
            field(
                "Sides",
                "How many flat sides make up the round side",
                sides(&mut p.sides),
            ),
            field(
                "Height segments",
                "How many pieces the side is cut into from bottom to top",
                segments(&mut p.height_segments),
            ),
            field(
                "Closed ends",
                "Whether the ends are closed",
                Toggle(&mut p.caps),
            ),
        ],
        Primitive::Cone(p) => vec![
            field(
                "Bottom radius",
                "Half the width at the bottom",
                Length(&mut p.bottom_radius, 0.0),
            ),
            field(
                "Top radius",
                "Half the width at the top: zero for a point, more to cut it off flat",
                Length(&mut p.top_radius, 0.0),
            ),
            field(
                "Height",
                "Size along Z (up)",
                Length(&mut p.height, MIN_SIZE),
            ),
            field(
                "Sides",
                "How many flat sides make up the round side",
                sides(&mut p.sides),
            ),
            field(
                "Height segments",
                "How many pieces the side is cut into from bottom to top",
                segments(&mut p.height_segments),
            ),
            field(
                "Closed ends",
                "Whether the ends are closed",
                Toggle(&mut p.caps),
            ),
        ],
        Primitive::Sphere(p) => vec![
            field("Radius", "Half the width", Length(&mut p.radius, MIN_SIZE)),
            field(
                "Sides",
                "How many pieces around (like lines of longitude)",
                sides(&mut p.sides),
            ),
            field(
                "Rings",
                "How many pieces from top to bottom (like lines of latitude)",
                Count(&mut p.rings, 2, MAX_SEGMENTS),
            ),
        ],
        Primitive::Disc(p) => vec![
            field("Radius", "Half the width", Length(&mut p.radius, MIN_SIZE)),
            field(
                "Hole radius",
                "Half the width of a hole in the middle: zero for none",
                Length(&mut p.inner_radius, 0.0),
            ),
            field(
                "Sides",
                "How many straight pieces make up the circle",
                sides(&mut p.sides),
            ),
        ],
        Primitive::Tube(p) => vec![
            field(
                "Outer radius",
                "Half the width of the outside",
                Length(&mut p.outer_radius, MIN_SIZE),
            ),
            field(
                "Inner radius",
                "Half the width of the hole",
                Length(&mut p.inner_radius, MIN_SIZE),
            ),
            field(
                "Height",
                "Size along Z (up)",
                Length(&mut p.height, MIN_SIZE),
            ),
            field(
                "Sides",
                "How many flat sides make up each round side",
                sides(&mut p.sides),
            ),
            field(
                "Height segments",
                "How many pieces the sides are cut into from bottom to top",
                segments(&mut p.height_segments),
            ),
        ],
    }
}

impl AttributesPanel {
    /// The panel's menu line: Mode and Edit.
    pub(crate) fn menus(&mut self, ui: &mut Ui, doc: &Document, out: &mut Vec<DocRequest>) {
        ui.menu_button("Mode", |ui| {
            let _ = ui.radio(true, "Object settings");
            ui.add_enabled(false, egui::RadioButton::new(false, "Tool settings"))
                .on_disabled_hover_text("Arrives with the tools in Phase 3");
        });
        ui.menu_button("Edit", |ui| {
            let selection = doc.selection();
            let single = match selection.as_slice() {
                [one] => primitive_of(doc, *one),
                _ => None,
            };
            if ui
                .add_enabled(single.is_some(), egui::Button::new("Copy settings"))
                .on_hover_text("Copy the selected shape's settings")
                .clicked()
            {
                self.clipboard = single;
                ui.close();
            }
            let targets: Vec<ObjectId> = match self.clipboard {
                Some(copied) => selection
                    .iter()
                    .copied()
                    .filter(|id| primitive_of(doc, *id).is_some_and(|p| p.kind() == copied.kind()))
                    .collect(),
                None => Vec::new(),
            };
            let paste = match self.clipboard {
                Some(copied) => format!("Paste {} settings", copied.name().to_lowercase()),
                None => "Paste settings".to_string(),
            };
            if ui
                .add_enabled(!targets.is_empty(), egui::Button::new(paste))
                .on_hover_text("Give the selected shapes of the same kind the copied settings")
                .on_disabled_hover_text(
                    "Copy a shape's settings, then select shapes of the same kind",
                )
                .clicked()
                && let Some(primitive) = self.clipboard
            {
                out.push(DocRequest::Run(Command::SetPrimitive {
                    ids: targets,
                    primitive,
                }));
                ui.close();
            }
        });
    }

    /// Draws the settings of the selection.
    pub(crate) fn show(&mut self, ui: &mut Ui, doc: &Document, out: &mut Vec<DocRequest>) {
        let selection = doc.selection();
        let id = match selection.as_slice() {
            [] => {
                return placeholder(
                    ui,
                    "Nothing selected",
                    "Select an object or pick a tool, and its settings show up here.",
                );
            }
            [one] => *one,
            many => {
                return placeholder(
                    ui,
                    &format!("{} objects selected", many.len()),
                    "Select one object to see its settings.",
                );
            }
        };
        let Some(object) = doc.scene().object(id).cloned() else {
            return;
        };
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                heading(ui, &object.kind, &object.name);
                section(ui, "Basic");
                egui::Grid::new("attributes_basic")
                    .num_columns(2)
                    .min_col_width(LABEL_WIDTH)
                    .spacing(vec2(12.0, 6.0))
                    .show(ui, |ui| {
                        self.name_row(ui, id, &object.name, out);
                        label(ui, "Visible", "Whether it is shown in the viewport");
                        let mut visible = object.visible;
                        let switch = ui.checkbox(&mut visible, "");
                        switch.widget_info(|| {
                            egui::WidgetInfo::selected(
                                egui::WidgetType::Checkbox,
                                true,
                                visible,
                                "Visible",
                            )
                        });
                        if switch.changed() {
                            out.push(DocRequest::Run(Command::SetVisible {
                                ids: vec![id],
                                visible,
                            }));
                        }
                        ui.end_row();
                    });
                if let ObjectKind::Primitive(primitive) = object.kind {
                    section(ui, "Shape");
                    self.shape_rows(ui, id, primitive, out);
                }
            });
    }

    /// The name, typed in place; kept when Enter is pressed or the box is
    /// left.
    fn name_row(&mut self, ui: &mut Ui, id: ObjectId, name: &str, out: &mut Vec<DocRequest>) {
        label(ui, "Name", "The name shown in the Objects panel");
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
            egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Object name")
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
    fn shape_rows(
        &mut self,
        ui: &mut Ui,
        id: ObjectId,
        primitive: Primitive,
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
                    let is_switch = matches!(field.value, Value::Toggle(_));
                    let response = match field.value {
                        Value::Length(value, min) => ui.add_sized(
                            vec2(FIELD_WIDTH, 20.0),
                            DragValue::new(value)
                                .range(min..=MAX_SIZE)
                                .speed(0.5)
                                .max_decimals(1)
                                .suffix(" cm"),
                        ),
                        Value::Count(value, low, high) => ui.add_sized(
                            vec2(FIELD_WIDTH, 20.0),
                            DragValue::new(value).range(low..=high).speed(0.05),
                        ),
                        Value::Toggle(value) => ui.checkbox(value, ""),
                    };
                    let response = response.on_hover_text(field.help);
                    let kind = if is_switch {
                        egui::WidgetType::Checkbox
                    } else {
                        egui::WidgetType::DragValue
                    };
                    response.widget_info(|| egui::WidgetInfo::labeled(kind, true, field.label));
                    if response.drag_started() || response.gained_focus() || response.clicked() {
                        // A new drag, a new value typed, or a click: a new
                        // undo step.
                        self.gesture += 1;
                    }
                    changed |= response.changed();
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
fn primitive_of(doc: &Document, id: ObjectId) -> Option<Primitive> {
    match doc.scene().object(id)?.kind {
        ObjectKind::Primitive(p) => Some(p),
        _ => None,
    }
}

/// The object's icon, name and kind, at the top of the panel.
fn heading(ui: &mut Ui, kind: &ObjectKind, name: &str) {
    ui.horizontal(|ui| {
        let (icon, color) = match kind {
            ObjectKind::Primitive(p) => (shape_icon(p.kind()), tint::SHAPE),
            _ => (crate::icons::Icon::Null, tint::ICON),
        };
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
        _ => "(group)".to_string(),
    }
}

/// A section title with a thin line under it.
fn section(ui: &mut Ui, title: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(title).strong());
    ui.separator();
}

/// A row's label, explained on hover.
fn label(ui: &mut Ui, text: &str, help: &str) {
    ui.label(RichText::new(text).color(theme::TEXT_WEAK))
        .on_hover_text(help);
}
