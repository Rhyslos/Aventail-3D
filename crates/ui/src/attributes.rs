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
use crate::transforming::Tool;
use crate::units::Units;
use egui::{RichText, Ui, vec2};
use fields::{Value, fields};
use mesh::MeshReport;
use mesh::primitives::MAX_SIZE;
use scene::{Command, Document, Grouping, Mesh, ObjectId, ObjectKind, Operation, Primitive, Stack};
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
    /// The mesh check, run only when the shape changes.
    checker: MeshChecker,
    /// The tool in use (set by the workbench each frame): Extrude and
    /// Inset show their settings at the top.
    pub(crate) tool: Tool,
    /// Whether Extrude and Inset keep neighbours together (the Keep
    /// together switch), for the next drag (decision 0038).
    pub(crate) grouping: Grouping,
}

/// Below this many polygons the mesh check is quick enough to run at
/// once; above, it runs in the background so the window never stalls
/// ([decision 0035](../../../docs/decisions/0035-heavy-meshes.md)).
const CHECK_IN_BACKGROUND_FROM: usize = 50_000;

/// Checks objects' meshes, keeping the last report.
#[derive(Default)]
struct MeshChecker {
    /// The last report, for which object and which mesh.
    shown: Option<(ObjectId, Arc<Mesh>, MeshReport)>,
    /// A check running in the background.
    running: Option<(ObjectId, Arc<Mesh>, std::thread::JoinHandle<MeshReport>)>,
}

impl Clone for MeshChecker {
    /// A copy starts afresh: reports are worked out again when needed.
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl std::fmt::Debug for MeshChecker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeshChecker")
            .field("running", &self.running.is_some())
            .finish()
    }
}

impl MeshChecker {
    /// The report on object `id`'s `mesh`, and whether it is up to date.
    /// For a heavy mesh the check runs in the background, and until it
    /// finishes the object's previous report (if any) is given, out of
    /// date; `ctx` is asked to draw again when it is done.
    fn report(
        &mut self,
        id: ObjectId,
        mesh: &Arc<Mesh>,
        ctx: &egui::Context,
    ) -> (Option<MeshReport>, bool) {
        if let Some((shown_id, shown, report)) = &self.shown
            && *shown_id == id
            && Arc::ptr_eq(shown, mesh)
        {
            return (Some(*report), true);
        }
        if self
            .running
            .as_ref()
            .is_some_and(|(.., handle)| handle.is_finished())
            && let Some((done_id, done, handle)) = self.running.take()
            && let Ok(report) = handle.join()
        {
            self.shown = Some((done_id, Arc::clone(&done), report));
            if done_id == id && Arc::ptr_eq(&done, mesh) {
                return (Some(report), true);
            }
        }
        if mesh.face_count() < CHECK_IN_BACKGROUND_FROM {
            let report = MeshReport::new(mesh);
            self.shown = Some((id, Arc::clone(mesh), report));
            return (Some(report), true);
        }
        if self.running.is_none() {
            let (work, ctx) = (Arc::clone(mesh), ctx.clone());
            let handle = std::thread::spawn(move || {
                let report = MeshReport::new(&work);
                ctx.request_repaint();
                report
            });
            self.running = Some((id, Arc::clone(mesh), handle));
        }
        let earlier = self
            .shown
            .as_ref()
            .filter(|(shown_id, ..)| *shown_id == id)
            .map(|(.., report)| *report);
        (earlier, false)
    }
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
        units: Units,
        out: &mut Vec<DocRequest>,
    ) {
        let selection = doc.selection();
        // Extrude's and Inset's switch, unless the last edit shows its own
        // (which sets the tool's too, so there is only one to think about).
        let last_shown = match selection.as_slice() {
            [one] => doc
                .scene()
                .object(*one)
                .and_then(|o| last_edit(&o.stack).map(|(_, _, op)| op.distance().is_some()))
                .unwrap_or(false),
            _ => false,
        };
        if self.tool.drags_anywhere() && !last_shown {
            self.tool_section(ui);
        }
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
                // The last extrude or inset first: it is what was just done.
                self.last_edit_section(ui, id, &object.stack, units, out);
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
                    section(ui, "Shape");
                    self.shape_rows(ui, id, *primitive, units, out);
                }
                if let Some(shape) = doc.scene().evaluated(id) {
                    let (report, current) = self.checker.report(id, &shape.mesh, ui.ctx());
                    mesh_section(
                        ui,
                        report.as_ref(),
                        current,
                        matches!(object.kind, ObjectKind::Mesh(_)),
                    );
                }
                if !object.stack.is_empty() {
                    edits_section(ui, doc, id, &object.stack);
                }
            });
    }

    /// Extrude's or Inset's own settings, while it is the tool in use.
    fn tool_section(&mut self, ui: &mut Ui) {
        section(ui, self.tool.name());
        egui::Grid::new("attributes_tool")
            .num_columns(2)
            .min_col_width(LABEL_WIDTH)
            .spacing(vec2(12.0, 6.0))
            .show(ui, |ui| {
                let mut together = self.grouping == Grouping::Together;
                if keep_together(ui, &mut together) {
                    self.grouping = grouping_of(together);
                }
                ui.end_row();
            });
        ui.label(
            RichText::new("Drag anywhere in the viewport. Hold Shift for fine steps.")
                .small()
                .color(theme::TEXT_WEAK),
        );
    }

    /// The last edit's settings, if it is an extrude or inset: they can
    /// still be changed, and the shape follows (decision 0038). Once
    /// another edit is made on top, it is fixed.
    fn last_edit_section(
        &mut self,
        ui: &mut Ui,
        id: ObjectId,
        stack: &Stack,
        units: Units,
        out: &mut Vec<DocRequest>,
    ) {
        let Some((layer, index, operation)) = last_edit(stack) else {
            return;
        };
        let (Some(distance), Some(grouping)) = (operation.distance(), operation.grouping()) else {
            return;
        };
        section(ui, &format!("Last edit: {}", operation.describe()));
        let mut edited = distance;
        let mut together = grouping == Grouping::Together;
        let mut changed = false;
        let low = match operation {
            Operation::InsetPolygons { .. } => 0.0,
            _ => -MAX_SIZE,
        };
        egui::Grid::new("attributes_last_edit")
            .num_columns(2)
            .min_col_width(LABEL_WIDTH)
            .spacing(vec2(12.0, 6.0))
            .show(ui, |ui| {
                label(
                    ui,
                    "Distance",
                    "How far it was pushed (in the object's own size). Change it and the \
                     shape follows.",
                );
                let touched = stepper::length(ui, &mut edited, (low, MAX_SIZE), units, "Distance");
                if touched.began {
                    self.gesture += 1;
                }
                changed |= touched.changed;
                ui.end_row();
                if keep_together(ui, &mut together) {
                    self.gesture += 1;
                    self.grouping = grouping_of(together);
                    changed = true;
                }
                ui.end_row();
            });
        let after = operation.with_settings(edited, grouping_of(together));
        if changed && after != *operation {
            out.push(DocRequest::RunMerging(
                Command::ReplaceOperation {
                    id,
                    layer,
                    index,
                    operation: after,
                },
                self.gesture,
            ));
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

/// The Mesh section, for every shape: what its final mesh is made of
/// (counted from the shape with its edits) and whether anything is wrong
/// with it. A plain mesh also says what it is for.
fn mesh_section(ui: &mut Ui, report: Option<&MeshReport>, current: bool, plain_mesh: bool) {
    section(ui, "Mesh");
    if !current {
        ui.label(RichText::new("Checking the mesh…").color(theme::TEXT_WEAK))
            .on_hover_text(
                "A heavy shape is checked in the background, so you can keep working. The \
                 numbers below are from before the last change until it finishes.",
            );
    }
    let Some(report) = report else {
        return;
    };
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
                "A plain mesh: it has no settings, so its shape changes only by editing \
                 its points, edges and polygons (press 1, 2 or 3).",
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

/// The top edit of `stack` (where it is, and what), if any.
fn last_edit(stack: &Stack) -> Option<(usize, usize, &Operation)> {
    let (layer, top) = stack
        .layers()
        .iter()
        .enumerate()
        .rev()
        .find(|(_, l)| !l.operations.is_empty())?;
    let index = top.operations.len() - 1;
    Some((layer, index, &top.operations[index]))
}

fn grouping_of(together: bool) -> Grouping {
    if together {
        Grouping::Together
    } else {
        Grouping::Separate
    }
}

/// The Keep together switch and its label. Returns whether it changed.
fn keep_together(ui: &mut Ui, together: &mut bool) -> bool {
    label(
        ui,
        "Keep together",
        "On: polygons that touch move as one piece, like a block pulled out of a \
         wall. Off: each polygon on its own, a separate bump for each.",
    );
    let response = ui.checkbox(together, "");
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, *together, "Keep together")
    });
    response.changed()
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
        .map(|shape| shape.warnings)
        .unwrap_or_default();
    let not_fitting = warnings.iter().filter(|w| w.missing > 0).count();
    let found_again = warnings
        .iter()
        .filter(|w| w.found_again && w.missing == 0)
        .count();
    if found_again > 0 {
        let text = match found_again {
            1 => "1 edit found its points again on the changed shape".to_string(),
            n => format!("{n} edits found their points again on the changed shape"),
        };
        ui.label(RichText::new(text).color(theme::TEXT_WEAK))
            .on_hover_text(
                "The shape's points were renumbered (more or fewer segments, say), \
                 so these edits looked for the points, edges and polygons they were \
                 made on by where they are. A polygon split into smaller ones moves \
                 as a whole. Nothing to fix.",
            );
    }
    if not_fitting > 0 {
        let text = match not_fitting {
            1 => "1 edit no longer fully fits the shape".to_string(),
            n => format!("{n} edits no longer fully fit the shape"),
        };
        ui.label(RichText::new(text).color(theme::WARNING))
            .on_hover_text(
                "Some of the points these edits move have no place on the shape now \
                 (for example after giving it fewer segments). The edits still apply \
                 to what remains; nothing else is lost, and going back to the earlier \
                 settings brings the rest back.",
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

#[cfg(test)]
mod checker_tests {
    use super::*;

    #[test]
    fn heavy_meshes_are_checked_in_the_background() {
        let ctx = egui::Context::default();
        let mut checker = MeshChecker::default();
        let id = ObjectId::from_raw(1);
        let light = Arc::new(mesh::PrimitiveKind::Cube.standard().mesh());
        let (report, current) = checker.report(id, &light, &ctx);
        assert!(current && report.unwrap().faces == 6, "light: at once");
        let heavy = Arc::new(mesh::samples::heavy_ball(60_000, 100.0));
        let (report, current) = checker.report(id, &heavy, &ctx);
        assert!(!current, "heavy: in the background");
        assert_eq!(
            report.unwrap().faces,
            6,
            "the object's last report meanwhile"
        );
        // Another object has no report to show meanwhile.
        let (other, _) = checker.report(ObjectId::from_raw(2), &heavy, &ctx);
        assert!(other.is_none());
        let start = std::time::Instant::now();
        loop {
            let (report, current) = checker.report(id, &heavy, &ctx);
            if current {
                assert_eq!(report.unwrap().faces, heavy.face_count());
                break;
            }
            assert!(start.elapsed().as_secs() < 60, "the check never finished");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}
