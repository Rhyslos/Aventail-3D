//! The Attribute Manager: the settings of the selected object. Its name
//! and whether it is shown, and for a primitive its shape settings (size,
//! segments and so on), which change the shape as you drag or type.
//!
//! Like the Object Manager, the panel never changes the scene itself: it
//! hands back commands. Dragging a value changes it every frame, so those
//! commands are merged into one undo step per drag (or per value typed).

mod fields;
mod stepper;

use crate::objects::{DocRequest, kind_icon};
use crate::panels::placeholder;
use crate::theme;
use crate::units::Units;
use egui::{RichText, Ui, vec2};
use fields::{Value, fields};
use mesh::MeshReport;
use mesh::primitives::MAX_SIZE;
use scene::{Command, Document, Mesh, ObjectId, ObjectKind, Primitive, Stack};
use std::sync::Arc;

/// Width of the column of setting names, so both sections line up.
const LABEL_WIDTH: f32 = 110.0;
/// Width of a text field (the name), matching a value field with its
/// arrows.
const FIELD_WIDTH: f32 = stepper::NUMBER_WIDTH + 16.0;

/// The Attribute Manager's own state.
#[derive(Debug, Clone, Default)]
pub(crate) struct AttributesPanel {
    /// Counts drags, typed values and clicks, so each one is its own undo
    /// step (and every frame of one drag is the same step).
    gesture: u64,
    /// A name being typed, for which object.
    name_draft: Option<(ObjectId, String)>,
    /// The last mesh checked and its report, so the check runs only when
    /// the shape changes.
    checked: Option<(Arc<Mesh>, MeshReport)>,
}

/// The menu entries for the units lengths are shown in.
pub(crate) fn unit_entries(ui: &mut Ui, units: &mut Units) {
    for choice in Units::ALL {
        if ui
            .radio(*units == choice, choice.name())
            .on_hover_text(format!(
                "Show and type lengths in {}",
                choice.name().to_lowercase()
            ))
            .clicked()
        {
            *units = choice;
            ui.close();
        }
    }
}

/// Copy settings and Paste settings, for the Attributes panel's Edit menu
/// and the Object Manager's right-click menu. The copied settings live in
/// `clipboard`.
pub(crate) fn settings_entries(
    ui: &mut Ui,
    doc: &Document,
    clipboard: &mut Option<Primitive>,
    out: &mut Vec<DocRequest>,
) {
    let selection = doc.selection();
    let single = match selection.as_slice() {
        [one] => primitive_of(doc, *one),
        _ => None,
    };
    if ui
        .add_enabled(single.is_some(), egui::Button::new("Copy settings"))
        .on_hover_text("Copy the selected shape's settings")
        .on_disabled_hover_text("Select one shape to copy its settings")
        .clicked()
    {
        *clipboard = single;
        ui.close();
    }
    let targets: Vec<ObjectId> = match clipboard {
        Some(copied) => selection
            .iter()
            .copied()
            .filter(|id| primitive_of(doc, *id).is_some_and(|p| p.kind() == copied.kind()))
            .collect(),
        None => Vec::new(),
    };
    let paste = match clipboard {
        Some(copied) => format!("Paste {} settings", copied.name().to_lowercase()),
        None => "Paste settings".to_string(),
    };
    if ui
        .add_enabled(!targets.is_empty(), egui::Button::new(paste))
        .on_hover_text("Give the selected shapes of the same kind the copied settings")
        .on_disabled_hover_text("Copy a shape's settings, then select shapes of the same kind")
        .clicked()
        && let Some(primitive) = *clipboard
    {
        out.push(DocRequest::Run(Command::SetPrimitive {
            ids: targets,
            primitive,
        }));
        ui.close();
    }
}

impl AttributesPanel {
    /// The panel's menu line: Mode, Edit and Units.
    pub(crate) fn menus(
        &mut self,
        ui: &mut Ui,
        doc: &Document,
        shared: &mut Shared<'_>,
        out: &mut Vec<DocRequest>,
    ) {
        ui.menu_button("Mode", |ui| {
            let _ = ui.radio(true, "Object settings");
            ui.add_enabled(false, egui::RadioButton::new(false, "Tool settings"))
                .on_disabled_hover_text("Arrives with the tools in Phase 3");
        });
        ui.menu_button("Edit", |ui| {
            settings_entries(ui, doc, shared.clipboard, out)
        });
        ui.menu_button("Units", |ui| unit_entries(ui, shared.units));
    }

    /// Draws the settings of the selection.
    pub(crate) fn show(
        &mut self,
        ui: &mut Ui,
        doc: &Document,
        units: &mut Units,
        out: &mut Vec<DocRequest>,
    ) {
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
                if let ObjectKind::Primitive(primitive) = &object.kind {
                    shape_section(ui, units);
                    self.shape_rows(ui, id, *primitive, *units, out);
                }
                if let Some(report) = self.report(doc, id) {
                    mesh_section(ui, &report, matches!(object.kind, ObjectKind::Mesh(_)));
                }
                if !object.stack.is_empty() {
                    edits_section(ui, doc, id, &object.stack);
                }
            });
    }

    /// The check of `id`'s final mesh, worked out again only when the
    /// mesh changes. `None` for an object without a shape.
    fn report(&mut self, doc: &Document, id: ObjectId) -> Option<MeshReport> {
        let shape = doc.scene().evaluated(id)?;
        match &self.checked {
            Some((mesh, report)) if Arc::ptr_eq(mesh, &shape.mesh) => Some(*report),
            _ => {
                let report = MeshReport::new(&shape.mesh);
                self.checked = Some((shape.mesh, report));
                Some(report)
            }
        }
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

/// What the Attributes panel shares with the rest of the window: the
/// copied settings and the units lengths are shown in.
pub(crate) struct Shared<'a> {
    /// Settings copied with Copy settings.
    pub(crate) clipboard: &'a mut Option<Primitive>,
    /// Metres or centimetres.
    pub(crate) units: &'a mut Units,
}

/// The "Shape" section title, with a quick switch between metres and
/// centimetres on the right.
fn shape_section(ui: &mut Ui, units: &mut Units) {
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Shape").strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            for choice in Units::ALL.into_iter().rev() {
                let response = ui
                    .selectable_label(*units == choice, choice.symbol())
                    .on_hover_text(format!("Show lengths in {}", choice.name().to_lowercase()));
                response.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::SelectableLabel,
                        true,
                        *units == choice,
                        format!("Lengths in {}", choice.name().to_lowercase()),
                    )
                });
                if response.clicked() {
                    *units = choice;
                }
            }
        });
    });
    ui.separator();
}

/// The Mesh section, for every shape: what its final mesh is made of
/// (counted from the shape with its edits) and whether anything is wrong
/// with it. A plain mesh also says what it is for.
fn mesh_section(ui: &mut Ui, report: &MeshReport, plain_mesh: bool) {
    section(ui, "Mesh");
    egui::Grid::new("attributes_mesh")
        .num_columns(2)
        .min_col_width(LABEL_WIDTH)
        .spacing(vec2(12.0, 6.0))
        .show(ui, |ui| {
            let mut rows = vec![
                ("Points", report.points, "The corners the mesh is made of"),
                ("Edges", report.edges, "The lines between the points"),
                (
                    "Polygons",
                    report.faces,
                    "The faces, with any number of corners",
                ),
                (
                    "Triangles",
                    report.triangles,
                    "The triangles a game engine cuts the faces into: the usual measure of \
                     how heavy a model is",
                ),
            ];
            if report.pieces > 1 {
                rows.push(("Pieces", report.pieces, "Separate parts that don't touch"));
            }
            for (name, count, help) in rows {
                label(ui, name, help);
                ui.label(count.to_string()).widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Label,
                        true,
                        format!("{name}: {count}"),
                    )
                });
                ui.end_row();
            }
            label(
                ui,
                "Surface",
                "Closed: no open borders, like a solid. Open: some edges have a face on \
                 one side only, as on a plane or an open pipe",
            );
            let surface = if report.is_closed() {
                "Closed".to_string()
            } else {
                format!("Open ({})", count(report.border_edges, "border edge"))
            };
            ui.label(surface);
            ui.end_row();
        });
    let problems = [
        (
            report.non_manifold_edges,
            "edge shared by three or more faces",
            "edges shared by three or more faces",
            "No real solid has these, and most tools can't work on them. They usually come \
             from faces stacked on each other.",
        ),
        (
            report.flipped_edges,
            "edge between faces facing opposite ways",
            "edges between faces facing opposite ways",
            "One face on each of these edges points inwards (a flipped normal); it will \
             shade and bake wrongly.",
        ),
        (
            report.zero_length_edges,
            "edge with no length",
            "edges with no length",
            "Both ends sit on the same spot. Usually left over from moving points together.",
        ),
        (
            report.flat_faces,
            "face with no area",
            "faces with no area",
            "Faces squashed flat to a line or a point: invisible, but they still count and \
             can break tools.",
        ),
        (
            report.loose_points,
            "loose point",
            "loose points",
            "Points that no face uses.",
        ),
    ];
    let mut any = false;
    for (n, one, many, help) in problems {
        if n > 0 {
            any = true;
            let text = if n == 1 {
                format!("1 {one}")
            } else {
                format!("{n} {many}")
            };
            ui.label(RichText::new(text).color(theme::WARNING))
                .on_hover_text(help);
        }
    }
    if !any {
        ui.label(RichText::new("No problems found").color(theme::TEXT_WEAK))
            .on_hover_text(
                "Checked for edges shared by three or more faces, faces facing the wrong \
                 way, edges with no length, faces with no area and loose points",
            );
    }
    if plain_mesh {
        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "A plain mesh: it has no settings, so its shape changes only by editing it. \
                 Editing points, edges and polygons arrives later in Phase 3.",
            )
            .small()
            .color(theme::TEXT_WEAK),
        );
    }
}

/// "1 border edge", "4 border edges".
fn count(n: usize, thing: &str) -> String {
    if n == 1 {
        format!("1 {thing}")
    } else {
        format!("{n} {thing}s")
    }
}

/// What is recorded on top of the shape: how many edits, and any that no
/// longer fully match it.
fn edits_section(ui: &mut Ui, doc: &Document, id: ObjectId, stack: &Stack) {
    section(ui, "Edits");
    let count = stack.operation_count();
    let text = match count {
        1 => "1 edit recorded on top of the shape".to_string(),
        n => format!("{n} edits recorded on top of the shape"),
    };
    ui.label(text).on_hover_text(
        "Edits are kept apart from the shape's settings, so the settings stay \
         editable: change them and the edits are made again on the new shape.",
    );
    let warnings = doc
        .scene()
        .evaluated(id)
        .map_or(0, |shape| shape.warnings.len());
    if warnings > 0 {
        let text = match warnings {
            1 => "1 edit no longer fully fits the shape".to_string(),
            n => format!("{n} edits no longer fully fit the shape"),
        };
        ui.label(RichText::new(text).color(theme::WARNING))
            .on_hover_text(
                "Some of the points these edits move are gone (for example after \
                 giving the shape fewer segments). The edits still apply to the \
                 points that remain; nothing else is lost, and going back to the \
                 earlier settings brings the missing points back.",
            );
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
