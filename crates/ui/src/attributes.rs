//! The Attributes panel: the settings of the selected object. Its name
//! and whether it is shown, and for a primitive its shape settings (size,
//! segments and so on), which change the shape as you drag or type.
//!
//! Like the Objects panel, the panel never changes the scene itself: it
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

/// The Attributes panel's own state.
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
    /// Bevel's rows and profile, for the next drag (decision 0040).
    pub(crate) bevel: BevelLook,
    /// Loop cut's number of cuts, for the next cut (decision 0047).
    pub(crate) loop_cut: LoopLook,
    /// Whether Slit cuts through to the far side (decision 0049).
    pub(crate) slit_through: bool,
    /// Extrude's bend, for the next drag (decision 0050).
    pub(crate) bend: scene::Bend,
    /// Connect's cuts and Bridge's rows and twist (decision 0051).
    pub(crate) join: JoinLook,
    /// Hard edges by angle's angle, for its next use (decision 0053).
    pub(crate) hard_angle: HardAngle,
    /// Symmetry (set by the workbench each frame, and read back): its
    /// axes are chosen here while it is on (decision 0054).
    pub(crate) symmetry: crate::Symmetry,
}

/// Hard edges by angle's angle in degrees, for its next use: edges whose
/// polygons meet more sharply than this become hard.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct HardAngle(pub(crate) f32);

impl Default for HardAngle {
    fn default() -> Self {
        Self(HARD_ANGLE)
    }
}

/// Hard edges by angle's standard angle, in degrees: the corners of boxy
/// shapes come out hard and curves made of 12 or more sides soft.
pub(crate) const HARD_ANGLE: f32 = 30.0;

/// Connect's and Bridge's settings, for their next use.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct JoinLook {
    /// How many cuts Connect makes along each edge.
    pub(crate) connect_cuts: u32,
    /// Bridge's rows and twist.
    pub(crate) bridge: (u32, i32),
}

impl Default for JoinLook {
    fn default() -> Self {
        Self {
            connect_cuts: 1,
            bridge: (1, 0),
        }
    }
}

/// Loop cut's settings, set in the Attributes panel before a cut.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LoopLook {
    /// How many evenly spaced cuts.
    pub(crate) cuts: u32,
}

impl Default for LoopLook {
    fn default() -> Self {
        Self { cuts: 1 }
    }
}

/// Bevel's rows and profile, set in the Attributes panel before a drag.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BevelLook {
    pub(crate) segments: u32,
    pub(crate) profile: f32,
}

impl Default for BevelLook {
    fn default() -> Self {
        Self {
            segments: 1,
            profile: 1.0,
        }
    }
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
/// and the Objects panel's right-click menu. The copied settings live in
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
                .and_then(|o| last_edit(&o.stack).map(|(_, _, op)| made_by(op, self.tool)))
                .unwrap_or(false),
            _ => false,
        };
        if self.symmetry.on {
            self.symmetry_section(ui);
        }
        if self.tool.has_settings() && !last_shown {
            self.tool_section(ui, units);
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

    /// Symmetry's axes, while it is on: X, Y and Z switches in a row.
    fn symmetry_section(&mut self, ui: &mut Ui) {
        section(ui, "Symmetry");
        ui.horizontal(|ui| {
            label(
                ui,
                "Mirror across",
                "The shape's own axes to mirror across: X mirrors left and right, Y front and \
                 back, Z top and bottom. The mirror goes through the shape's own middle.",
            );
            for (axis, name) in ["X", "Y", "Z"].into_iter().enumerate() {
                let response = ui.checkbox(&mut self.symmetry.axes[axis], name);
                response.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::Checkbox,
                        true,
                        self.symmetry.axes[axis],
                        format!("Symmetry {name}"),
                    )
                });
            }
        });
        if !self.symmetry.axes.contains(&true) {
            ui.label(
                RichText::new("Pick an axis: with none, nothing is mirrored.")
                    .small()
                    .color(theme::TEXT_WEAK),
            );
        }
    }

    /// Extrude's, Inset's, Bevel's or Loop cut's own settings, while it is
    /// the tool in use: they apply to the next drag or cut.
    fn tool_section(&mut self, ui: &mut Ui, units: Units) {
        section(ui, self.tool.name());
        ui.push_id("tool", |ui| {
            egui::Grid::new("attributes_tool")
                .num_columns(2)
                .min_col_width(LABEL_WIDTH)
                .spacing(vec2(12.0, 6.0))
                .show(ui, |ui| {
                    if self.tool == Tool::Slit {
                        through_row(ui, &mut self.slit_through);
                        ui.end_row();
                    } else if self.tool == Tool::Bridge {
                        bridge_rows(ui, &mut self.join.bridge);
                    } else if self.tool == Tool::Weld {
                        // Nothing to set: the hint says how.
                    } else if self.tool == Tool::LoopCut {
                        cuts_row(ui, &mut self.loop_cut.cuts);
                        ui.end_row();
                    } else if self.tool == Tool::Bevel {
                        let mut look = self.bevel;
                        segments_row(ui, &mut look.segments);
                        ui.end_row();
                        profile_row(ui, &mut look.profile);
                        ui.end_row();
                        self.bevel = look;
                    } else {
                        let mut together = self.grouping == Grouping::Together;
                        if keep_together(ui, &mut together) {
                            self.grouping = grouping_of(together);
                        }
                        ui.end_row();
                        if self.tool == Tool::Extrude {
                            let mut bend = self.bend;
                            bend_rows(ui, &mut bend, units);
                            self.bend = bend;
                        }
                    }
                });
        });
        let hint = if self.tool == Tool::Bridge {
            "Drag from one open edge to another. With two borders or two polygons selected, C \
             bridges them at once."
        } else if self.tool == Tool::Weld {
            "Drag a point onto another; a selected point brings the whole selection."
        } else if self.tool == Tool::Slit {
            "Click or drag points over the shape. Esc or Enter finishes the line."
        } else if self.tool == Tool::LoopCut {
            "Point at an edge to see the loop; click to cut, or drag along the edge to slide it."
        } else if self.tool == Tool::Bevel {
            "Drag away from the selected edges or points to widen. Hold Shift for fine steps."
        } else {
            "Drag anywhere in the viewport. Hold Shift for fine steps."
        };
        ui.label(RichText::new(hint).small().color(theme::TEXT_WEAK));
    }

    /// The last edit's settings, if it is an extrude, inset, bevel or loop
    /// cut: they
    /// can still be changed, and the shape follows (decisions 0038 and
    /// 0040). Once another edit is made on top, it is fixed. Changes here
    /// also become the tool's settings for the next drag.
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
        let Some(settings) = operation.settings() else {
            return;
        };
        section(ui, &format!("Last edit: {}", operation.describe()));
        let mut edited = settings;
        let mut changed = false;
        let bevel = matches!(
            operation,
            Operation::BevelEdges { .. } | Operation::BevelPoints { .. }
        );
        let low = match operation {
            Operation::ExtrudePolygons { .. } | Operation::ExtrudeEdges { .. } => -MAX_SIZE,
            _ => 0.0,
        };
        // Its own id scope: its Width field mustn't share ids with the
        // shape's Width below.
        ui.push_id("last_edit", |ui| {
            egui::Grid::new("attributes_last_edit")
                .num_columns(2)
                .min_col_width(LABEL_WIDTH)
                .spacing(vec2(12.0, 6.0))
                .show(ui, |ui| {
                    if let Some(mut cuts) = edited.cuts {
                        let touched = cuts_row(ui, &mut cuts);
                        if touched.began {
                            self.gesture += 1;
                        }
                        if touched.changed {
                            edited.cuts = Some(cuts);
                            if matches!(operation, Operation::Connect { .. }) {
                                self.join.connect_cuts = cuts;
                            } else {
                                self.loop_cut.cuts = cuts;
                            }
                            changed = true;
                        }
                        ui.end_row();
                    }
                    if let Some(mut angle) = edited.angle {
                        let touched = angle_row(ui, &mut angle);
                        if touched.began {
                            self.gesture += 1;
                        }
                        if touched.changed {
                            edited.angle = Some(angle);
                            self.hard_angle = HardAngle(angle);
                            changed = true;
                        }
                        ui.end_row();
                    }
                    if let Some(mut bridge) = edited.bridge {
                        let touched = bridge_rows(ui, &mut bridge);
                        if touched.began {
                            self.gesture += 1;
                        }
                        if touched.changed {
                            edited.bridge = Some(bridge);
                            self.join.bridge = bridge;
                            changed = true;
                        }
                    }
                    if let Some(mut through) = edited.through {
                        if through_row(ui, &mut through) {
                            self.gesture += 1;
                            edited.through = Some(through);
                            self.slit_through = through;
                            changed = true;
                        }
                        ui.end_row();
                    }
                    if let Some(mut position) = edited.position {
                        let touched = position_row(ui, &mut position);
                        if touched.began {
                            self.gesture += 1;
                        }
                        if touched.changed {
                            edited.position = Some(position);
                            changed = true;
                        }
                        ui.end_row();
                    }
                    let Some(mut distance) = edited.distance else {
                        return;
                    };
                    let (name, help) = if bevel {
                        (
                            "Width",
                            "How far the bevel reaches into each polygon (in the object's own \
                         size). Change it and the shape follows.",
                        )
                    } else {
                        (
                            "Distance",
                            "How far it was pushed (in the object's own size). Change it and the \
                         shape follows.",
                        )
                    };
                    label(ui, name, help);
                    let touched = stepper::length(ui, &mut distance, (low, MAX_SIZE), units, name);
                    if touched.began {
                        self.gesture += 1;
                    }
                    if touched.changed {
                        edited.distance = Some(distance);
                        changed = true;
                    }
                    ui.end_row();
                    if let Some(grouping) = edited.grouping {
                        let mut together = grouping == Grouping::Together;
                        if keep_together(ui, &mut together) {
                            self.gesture += 1;
                            self.grouping = grouping_of(together);
                            edited.grouping = Some(self.grouping);
                            changed = true;
                        }
                        ui.end_row();
                    }
                    if let Some(bend) = edited.bend {
                        // A straight extrude shows the tool's bend, turned
                        // to no angle: give it an angle and it bends so.
                        let mut bend = if matches!(operation, Operation::ExtrudeBent { .. }) {
                            bend
                        } else {
                            scene::Bend {
                                angle: 0.0,
                                ..self.bend
                            }
                        };
                        let touched = bend_rows(ui, &mut bend, units);
                        if touched.began {
                            self.gesture += 1;
                        }
                        if touched.changed {
                            edited.bend = Some(bend);
                            self.bend = bend;
                            changed = true;
                        }
                    }
                    if let Some(mut segments) = edited.segments {
                        let touched = segments_row(ui, &mut segments);
                        if touched.began {
                            self.gesture += 1;
                        }
                        if touched.changed {
                            edited.segments = Some(segments);
                            self.bevel.segments = segments;
                            changed = true;
                        }
                        ui.end_row();
                    }
                    if let Some(mut profile) = edited.profile {
                        let touched = profile_row(ui, &mut profile);
                        if touched.began {
                            self.gesture += 1;
                        }
                        if touched.changed {
                            edited.profile = Some(profile);
                            self.bevel.profile = profile;
                            changed = true;
                        }
                        ui.end_row();
                    }
                });
        });
        let after = operation.with(edited);
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
             shade and bake wrongly. Mesh → Fix facing turns them the right way.",
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
pub(crate) fn last_edit(stack: &Stack) -> Option<(usize, usize, &Operation)> {
    let (layer, top) = stack
        .layers()
        .iter()
        .enumerate()
        .rev()
        .find(|(_, l)| !l.entries.is_empty())?;
    let index = top.entries.len() - 1;
    Some((layer, index, &top.entries[index].operation))
}

fn grouping_of(together: bool) -> Grouping {
    if together {
        Grouping::Together
    } else {
        Grouping::Separate
    }
}

/// Whether `operation` is the kind of edit `tool` makes.
pub(crate) fn made_by(operation: &Operation, tool: Tool) -> bool {
    matches!(
        (operation, tool),
        (
            Operation::ExtrudePolygons { .. }
                | Operation::ExtrudeBent { .. }
                | Operation::ExtrudeEdges { .. },
            Tool::Extrude
        ) | (Operation::InsetPolygons { .. }, Tool::Inset)
            | (
                Operation::BevelEdges { .. } | Operation::BevelPoints { .. },
                Tool::Bevel
            )
            | (Operation::LoopCut { .. }, Tool::LoopCut)
            | (Operation::Slit { .. }, Tool::Slit)
            | (Operation::Bridge { .. }, Tool::Bridge)
            | (Operation::Weld { .. }, Tool::Weld)
    )
}

/// A bevel's rows, with its label.
fn segments_row(ui: &mut Ui, segments: &mut u32) -> stepper::Touched {
    label(
        ui,
        "Segments",
        "How many rows the bevel has: 1 is a flat cut (a chamfer), more make it round",
    );
    stepper::count(ui, segments, (1, mesh::bevel::MAX_SEGMENTS), "Segments")
}

/// Extrude's bend: Angle, Direction, Bend radius and Segments, a row each
/// (decision 0050).
fn bend_rows(ui: &mut Ui, bend: &mut scene::Bend, units: Units) -> stepper::Touched {
    let mut out = stepper::Touched::default();
    let mut add = |t: stepper::Touched| {
        out.changed |= t.changed;
        out.began |= t.began;
    };
    let degrees = |ui: &mut Ui, value: &mut f32, most: f32, name: &str, help: &str| {
        label(ui, name, help);
        let response = ui.add(
            egui::Slider::new(value, 0.0..=most)
                .suffix("°")
                .custom_formatter(|v, _| format!("{v:.0}")),
        );
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Slider, true, name));
        ui.end_row();
        stepper::Touched {
            changed: response.changed(),
            began: response.drag_started() || (response.changed() && !response.dragged()),
        }
    };
    let most = if bend.radius > 0.0 {
        180.0
    } else {
        mesh::extrude::MAX_MITRE_ANGLE
    };
    add(degrees(
        ui,
        &mut bend.angle,
        most,
        "Angle",
        "How far the extrude turns before going on straight: 0° is straight, 90° a corner. \
         The pipe keeps its full width through the turn.",
    ));
    add(degrees(
        ui,
        &mut bend.direction,
        360.0,
        "Direction",
        "Which way round it turns: 0° turns up (an upright pipe turns towards +X), and it \
         goes round from there.",
    ));
    label(
        ui,
        "Bend radius",
        "How wide the turn is, from its middle to the middle of the pipe: 0 is a sharp \
         corner (a mitre joint, up to 150°), more is a smooth elbow.",
    );
    add(stepper::length(
        ui,
        &mut bend.radius,
        (0.0, MAX_SIZE),
        units,
        "Bend radius",
    ));
    ui.end_row();
    label(
        ui,
        "Segments",
        "How many rows a smooth turn has (with a bend radius). Ctrl + wheel changes it.",
    );
    add(stepper::count(
        ui,
        &mut bend.segments,
        (1, mesh::extrude::MAX_BEND_SEGMENTS),
        "Bend segments",
    ));
    ui.end_row();
    *bend = bend.clamped();
    out
}

/// Bridge's Segments and Twist, a row each.
fn bridge_rows(ui: &mut Ui, (segments, twist): &mut (u32, i32)) -> stepper::Touched {
    label(
        ui,
        "Segments",
        "How many rows the bridge has: more make a longer bridge bend smoothly later. Ctrl + \
         wheel changes it.",
    );
    let mut out = stepper::count(
        ui,
        segments,
        (1, mesh::join::MAX_BRIDGE_SEGMENTS),
        "Bridge segments",
    );
    ui.end_row();
    label(
        ui,
        "Twist",
        "Turns a bridge between two closed loops round by whole steps, if the points pair up \
         wrongly and the band twists.",
    );
    let mut steps = (*twist + 1000) as u32;
    let t = stepper::count(ui, &mut steps, (0, 2000), "Twist");
    if t.changed {
        *twist = steps as i32 - 1000;
    }
    out.changed |= t.changed;
    out.began |= t.began;
    ui.end_row();
    out
}

/// Slit's Cut through switch, with its label. Returns whether it changed.
fn through_row(ui: &mut Ui, through: &mut bool) -> bool {
    label(
        ui,
        "Cut through",
        "On: everything under the line is cut, the far side too. Off: only what you \
         can see.",
    );
    let response = ui.checkbox(through, "");
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, *through, "Cut through")
    });
    response.changed()
}

/// Hard edges by angle's angle, with its label.
fn angle_row(ui: &mut Ui, angle: &mut f32) -> stepper::Touched {
    label(
        ui,
        "Angle",
        "Edges whose polygons meet more sharply than this become hard (a crisp edge); the \
         rest become soft (smooth). 0° is flat.",
    );
    let response = ui.add(
        egui::Slider::new(angle, 0.0..=mesh::hardness::MAX_ANGLE)
            .suffix("°")
            .custom_formatter(|v, _| format!("{v:.0}")),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Slider, true, "Angle"));
    stepper::Touched {
        changed: response.changed(),
        began: response.drag_started() || (response.changed() && !response.dragged()),
    }
}

/// Loop cut's number of cuts, with its label.
fn cuts_row(ui: &mut Ui, cuts: &mut u32) -> stepper::Touched {
    label(
        ui,
        "Cuts",
        "How many new loops are cut, spread evenly across the polygons",
    );
    stepper::count(ui, cuts, (1, mesh::loopcut::MAX_CUTS), "Cuts")
}

/// Where a loop cut sits across its edge, as a percentage, with its label.
fn position_row(ui: &mut Ui, position: &mut f32) -> stepper::Touched {
    label(
        ui,
        "Position",
        "Where the cut sits across the polygons: 50% is the middle. With several \
         cuts, it slides them all together.",
    );
    let mut percent = *position * 100.0;
    let response = ui.add(
        egui::Slider::new(&mut percent, 0.0..=100.0)
            .suffix("%")
            // Shown as a whole percent, without rounding the value itself
            // (that would change the cut as soon as it is shown).
            .custom_formatter(|v, _| format!("{v:.0}")),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Slider, true, "Position"));
    if response.changed() {
        *position = (percent / 100.0).clamp(0.0, 1.0);
    }
    stepper::Touched {
        changed: response.changed(),
        began: response.drag_started() || (response.changed() && !response.dragged()),
    }
}

/// A bevel's profile as a percentage, with its label.
fn profile_row(ui: &mut Ui, profile: &mut f32) -> stepper::Touched {
    label(
        ui,
        "Profile",
        "The shape across the bevel: 100% is round, 0% is flat, -100% curves inward \
         (hollow, like a cove). Needs 2 or more segments to show.",
    );
    let mut percent = *profile * 100.0;
    let response = ui.add(egui::Slider::new(&mut percent, -100.0..=100.0).suffix("%"));
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Slider, true, "Profile"));
    if response.changed() {
        *profile = (percent / 100.0).clamp(-1.0, 1.0);
    }
    stepper::Touched {
        changed: response.changed(),
        began: response.drag_started() || (response.changed() && !response.dragged()),
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
    let too_big = warnings.iter().filter(|w| w.too_big).count();
    if too_big > 0 {
        let text = match too_big {
            1 => "1 edit was skipped: the shape would be too large".to_string(),
            n => format!("{n} edits were skipped: the shape would be too large"),
        };
        ui.label(RichText::new(text).color(theme::WARNING))
            .on_hover_text(format!(
                "These edits would have made the shape larger than {} million polygons, \
                 more than the app can work with safely, so they were left out. Undo \
                 them, or use fewer rows or a smaller selection.",
                scene::MAX_SHAPE_POLYGONS / 1_000_000
            ));
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
