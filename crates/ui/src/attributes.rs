//! The Attributes panel: the settings of the selected object. Its name
//! and whether it is shown, and for a primitive its shape settings (size,
//! segments and so on), which change the shape as you drag or type.
//!
//! Like the Objects panel, the panel never changes the scene itself: it
//! hands back commands. Dragging a value changes it every frame, so those
//! commands are merged into one undo step per drag (or per value typed).

mod fields;
mod materials;
pub(crate) mod stepper;

use crate::objects::{DocRequest, kind_icon};
use crate::panels::placeholder;
use crate::theme;
use crate::transforming::Tool;
use crate::units::Units;
use egui::{RichText, Ui, vec2};
use fields::{Value, fields};
use mesh::MeshReport;
use mesh::primitives::MAX_SIZE;
use scene::{
    Command, Document, EditId, Grouping, Mesh, ObjectId, ObjectKind, Operation, Primitive, Stack,
};
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
    /// Groove's width and sides, for the next drag (decision 0086).
    pub(crate) groove: GrooveLook,
    /// Loop cut's number of cuts, for the next cut (decision 0047).
    pub(crate) loop_cut: LoopLook,
    /// Whether Slit cuts through to the far side (decision 0049).
    pub(crate) slit_through: bool,
    /// Extrude's bend, for the next drag (decision 0050).
    pub(crate) bend: scene::Bend,
    /// Extrude's way, end size, steps and flat end, for the next drag
    /// (decision 0070).
    pub(crate) extrude: scene::ExtrudeShape,
    /// Connect's cuts and Bridge's rows and twist (decision 0051).
    pub(crate) join: JoinLook,
    /// Hard edges by angle's angle, for its next use (decision 0053).
    pub(crate) hard_angle: HardAngle,
    /// The Materials section's names being typed and its message.
    materials: materials::MaterialsState,
    /// How many polygons of the shown object are selected, in the
    /// Polygons mode (set by the workbench each frame); `None` in other
    /// modes. Assign needs some.
    pub(crate) polygons_selected: Option<usize>,
    /// The material slot whose Assign button was clicked, for the
    /// workbench to give the selected polygons (decision 0074).
    pub(crate) assign: Option<u32>,
    /// The edit opened from the edit list (decision 0092), shown in place
    /// of the Last edit while its object is the one selected.
    pub(crate) opened: Option<OpenedEdit>,
    /// For each object shown, its last edit and the selection it was made
    /// with (decision 0120): the id its next edit would get, and the
    /// document's element revision then. Once something else is selected,
    /// the Last edit is left alone and the panel shows the tool's settings
    /// for the next use.
    last_anchors: std::collections::HashMap<ObjectId, (EditId, u64)>,
}

/// An edit opened from the edit list (decision 0092).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OpenedEdit {
    /// Its object.
    pub(crate) object: ObjectId,
    /// The edit.
    pub(crate) edit: EditId,
    /// The id the object's next edit would get when it was opened: once a
    /// new edit is made, the panel goes back to showing the last edit.
    newest: EditId,
}

impl OpenedEdit {
    /// Whether this is edit `edit` of object `object`, still open (no new
    /// edit made since).
    pub(crate) fn is(&self, doc: &Document, object: ObjectId, edit: EditId) -> bool {
        self.object == object
            && self.edit == edit
            && doc
                .scene()
                .object(object)
                .is_some_and(|o| o.stack.next_edit_id() == self.newest)
    }

    /// Edit `edit` of object `object`, opened now.
    pub(crate) fn new(doc: &Document, object: ObjectId, edit: EditId) -> Self {
        let newest = doc
            .scene()
            .object(object)
            .map_or(edit, |o| o.stack.next_edit_id());
        Self {
            object,
            edit,
            newest,
        }
    }
}

impl AttributesPanel {
    /// Whether the next extrude bends (then its way, end size, steps and
    /// flat end don't apply).
    pub(crate) fn extrude_bends(&self) -> bool {
        self.bend.bends()
    }
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
pub(crate) const HARD_ANGLE: f32 = mesh::hardness::DEFAULT_ANGLE;

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

/// Groove's width (in world centimetres) and sides, set in the Attributes
/// panel before a drag (decision 0086); the drag sets the depth.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct GrooveLook {
    pub(crate) width: f32,
    pub(crate) sides: scene::GrooveSides,
}

impl Default for GrooveLook {
    /// A centimetre wide, straight sides.
    fn default() -> Self {
        Self {
            width: 1.0,
            sides: scene::GrooveSides::Straight,
        }
    }
}

/// Bevel's rows and profile, set in the Attributes panel before a drag.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BevelLook {
    pub(crate) segments: u32,
    pub(crate) profile: f32,
    /// Skew, even width, stopping at overlaps and welding (decision 0085).
    pub(crate) options: scene::BevelOptions,
}

impl Default for BevelLook {
    fn default() -> Self {
        Self {
            segments: 1,
            profile: 1.0,
            options: scene::BevelOptions::default(),
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
        crate::dialog::menu(ui, "Mode", |ui| {
            let _ = ui.radio(true, "Object settings");
            ui.add_enabled(false, egui::RadioButton::new(false, "Tool settings"))
                .on_disabled_hover_text("Arrives with the tools in Phase 3");
        });
        crate::dialog::menu(ui, "Edit", |ui| {
            settings_entries(ui, doc, shared.clipboard, out)
        });
        crate::dialog::menu(ui, "Units", |ui| unit_entries(ui, shared.units));
    }

    /// Keeps track of the last edit and the selection it was made with
    /// (decision 0120). Called by the workbench every frame, before
    /// anything that changes the last edit.
    pub(crate) fn track_last_edit(&mut self, doc: &Document) {
        let selection = doc.selection();
        let [id] = selection.as_slice() else {
            return;
        };
        let Some(next) = doc.scene().object(*id).map(|o| o.stack.next_edit_id()) else {
            return;
        };
        if self
            .last_anchors
            .get(id)
            .is_none_or(|(newest, _)| *newest != next)
        {
            self.last_anchors
                .insert(*id, (next, doc.element_revision()));
        }
    }

    /// Takes the selection as it is now as the one the last edit was made
    /// with: for a tool that moves the selection on after its edit (Extrude
    /// moves it to the new outer edges).
    pub(crate) fn keep_last_edit_live(&mut self, doc: &Document) {
        for id in doc.selection() {
            self.last_anchors.remove(&id);
        }
        self.track_last_edit(doc);
    }

    /// Whether object `id`'s last edit can still be changed from the
    /// panel and with Ctrl + wheel (decision 0120): nothing else has been
    /// selected since it was made.
    pub(crate) fn last_edit_is_live(&self, doc: &Document, id: ObjectId) -> bool {
        let Some(next) = doc.scene().object(id).map(|o| o.stack.next_edit_id()) else {
            return false;
        };
        self.last_anchors
            .get(&id)
            .is_some_and(|(newest, revision)| {
                *newest == next && *revision == doc.element_revision()
            })
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
        let opened = match selection.as_slice() {
            [one] => self.opened_edit(doc, *one),
            _ => None,
        };
        let last_shown = match selection.as_slice() {
            _ if opened.is_some() => true,
            [one] if self.last_edit_is_live(doc, *one) => doc
                .scene()
                .object(*one)
                .and_then(|o| last_edit(&o.stack).map(|(_, _, op)| made_by(op, self.tool)))
                .unwrap_or(false),
            _ => false,
        };
        // Another tool's last edit (an extrude while Inset is in use) is
        // left out, so the panel only shows the settings of the tool in
        // use: its rows would otherwise read as the tool's own.
        let own_tool_shown = self.tool.has_settings() && !last_shown;
        if own_tool_shown {
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
                if !own_tool_shown {
                    let stretch = scene::stretch_of(doc.scene().shape_matrix(id));
                    match opened {
                        Some(edit) => self.opened_edit_section(
                            ui,
                            doc,
                            id,
                            edit,
                            &object.stack,
                            units,
                            stretch,
                            out,
                        ),
                        None if self.last_edit_is_live(doc, id) => {
                            self.last_edit_section(ui, id, &object.stack, units, stretch, out);
                        }
                        None => self.left_edit_line(ui, doc, id, &object.stack),
                    }
                } else if opened.is_none() && !self.last_edit_is_live(doc, id) {
                    // The tool's settings are at the top, for its next
                    // use; the edit before them can still be opened.
                    self.left_edit_line(ui, doc, id, &object.stack);
                }
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
                        if object.kind.has_shape() {
                            collision_row(ui, id, object.collision, out);
                        }
                    });
                if let ObjectKind::Instance(source) = &object.kind {
                    section(ui, "Instance");
                    let original = doc
                        .scene()
                        .object(*source)
                        .map_or_else(|| "(gone)".to_string(), |o| o.name.clone());
                    ui.label(format!("Shows the shape of: {original}"));
                    ui.label(
                        RichText::new(
                            "Change the original and every instance follows. Make editable \
                             (Ctrl + E) gives this one its own shape.",
                        )
                        .small()
                        .color(theme::TEXT_WEAK),
                    );
                }
                if let ObjectKind::Primitive(primitive) = &object.kind {
                    section(ui, "Shape");
                    self.shape_rows(ui, id, *primitive, units, out);
                }
                if object.kind.has_shape() {
                    self.materials_section(ui, doc, id, out);
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
                        bevel_option_rows(ui, &mut look.options, units);
                        self.bevel = look;
                    } else if self.tool == Tool::Groove {
                        let mut look = self.groove;
                        groove_rows(ui, &mut look.width, &mut look.sides, units);
                        self.groove = look;
                    } else {
                        let mut together = self.grouping == Grouping::Together;
                        if keep_together(ui, &mut together) {
                            self.grouping = grouping_of(together);
                        }
                        ui.end_row();
                        if self.tool == Tool::Extrude {
                            let mut shape = self.extrude;
                            extrude_rows(ui, &mut shape, !self.bend.bends());
                            self.extrude = shape;
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
            "Click or drag points over the shape. Esc or Enter finishes the line; click the first point to close it into a loop."
        } else if self.tool == Tool::LoopCut {
            "Point at an edge to see the loop; click to cut, or drag along the edge to slide it."
        } else if self.tool == Tool::Bevel {
            "Drag away from the selected edges or points to widen. Hold Shift for fine steps."
        } else if self.tool == Tool::Groove {
            "Drag into the surface for a groove, out of it for a ridge. Hold Shift for fine steps."
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
        stretch: glam::Vec3,
        out: &mut Vec<DocRequest>,
    ) {
        let Some((layer, index, operation)) = last_edit(stack) else {
            return;
        };
        if operation.settings().is_none() {
            return;
        }
        section(ui, &format!("Last edit: {}", operation.describe()));
        self.edit_settings(ui, id, (layer, index), operation, units, stretch, out);
    }

    /// Once something else is selected after an edit with settings, a
    /// line instead of its settings (decision 0120): changing them now
    /// would change that edit, not the next one. A button opens it, to
    /// change it on purpose.
    fn left_edit_line(&mut self, ui: &mut Ui, doc: &Document, id: ObjectId, stack: &Stack) {
        let Some((layer, index, operation)) = last_edit(stack) else {
            return;
        };
        if operation.settings().is_none() {
            return;
        }
        let edit = stack.layers()[layer].entries[index].id;
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("Last edit: {}", operation.describe()))
                    .small()
                    .color(theme::TEXT_WEAK),
            );
            if ui
                .small_button(CHANGE_LAST_EDIT_LABEL)
                .on_hover_text(
                    "You've selected something else since, so the settings above are for your \
                     next use of the tool. Open this edit to change it.",
                )
                .clicked()
            {
                self.opened = Some(OpenedEdit::new(doc, id, edit));
            }
        });
    }

    /// The edit opened from the edit list on object `id`, if it is still
    /// there.
    pub(crate) fn opened_edit(&self, doc: &Document, id: ObjectId) -> Option<EditId> {
        let opened = self.opened?;
        let stack = &doc.scene().object(id)?.stack;
        (opened.object == id
            && stack.entry(opened.edit).is_some()
            && stack.next_edit_id() == opened.newest)
            .then_some(opened.edit)
    }

    /// An edit opened from the edit list (decision 0092): its switches,
    /// its settings (if it has any), and Freeze up to here and Delete edit.
    #[allow(clippy::too_many_arguments)]
    fn opened_edit_section(
        &mut self,
        ui: &mut Ui,
        doc: &Document,
        id: ObjectId,
        edit: EditId,
        stack: &Stack,
        units: Units,
        stretch: glam::Vec3,
        out: &mut Vec<DocRequest>,
    ) {
        let Some((layer, index, entry)) = stack.entry(edit) else {
            return;
        };
        let entry = entry.clone();
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("Edit: {}", entry.operation.describe())).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button(CLOSE_EDIT_LABEL)
                    .on_hover_text("Go back to showing the last edit")
                    .clicked()
                {
                    self.opened = None;
                }
            });
        });
        ui.separator();
        let layer_name = stack.layers()[layer].name.clone();
        let total = stack.layers()[layer].entries.len();
        ui.label(
            RichText::new(format!(
                "Edit {} of {total} in {layer_name}. Change it and the edits after it are made \
                 again on the new shape.",
                index + 1
            ))
            .small()
            .color(theme::TEXT_WEAK),
        );
        if let Some(problem) = doc
            .scene()
            .evaluated(id)
            .and_then(|e| edit_warning(&e.warnings, layer, index))
        {
            ui.label(RichText::new(problem).color(theme::WARNING));
        }
        ui.push_id("opened_edit_switches", |ui| {
            egui::Grid::new("attributes_opened_edit")
                .num_columns(2)
                .min_col_width(LABEL_WIDTH)
                .spacing(vec2(12.0, 6.0))
                .show(ui, |ui| {
                    label(
                        ui,
                        "On",
                        "Off: the shape is made as if this edit wasn't there. Nothing is lost.",
                    );
                    let mut on = entry.enabled;
                    let switch = ui.checkbox(&mut on, "");
                    switch.widget_info(|| {
                        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, on, "Edit on")
                    });
                    if switch.changed() {
                        out.push(DocRequest::Run(Command::SetEditEnabled {
                            id,
                            edit,
                            enabled: on,
                        }));
                    }
                    ui.end_row();
                    label(
                        ui,
                        "Keep editable",
                        "Keep this edit changeable: Freeze never bakes it, so its settings stay \
                         open for good",
                    );
                    let mut kept = entry.pinned;
                    let switch = ui.checkbox(&mut kept, "");
                    switch.widget_info(|| {
                        egui::WidgetInfo::selected(
                            egui::WidgetType::Checkbox,
                            true,
                            kept,
                            KEEP_EDITABLE_SWITCH,
                        )
                    });
                    if switch.changed() {
                        out.push(DocRequest::Run(Command::SetEditPinned {
                            id,
                            edit,
                            pinned: kept,
                        }));
                    }
                    ui.end_row();
                });
        });
        if entry.operation.settings().is_some() {
            self.edit_settings(
                ui,
                id,
                (layer, index),
                &entry.operation,
                units,
                stretch,
                out,
            );
        } else {
            ui.label(
                RichText::new("This kind of edit has no settings to change.")
                    .small()
                    .color(theme::TEXT_WEAK),
            );
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let problem = stack.freeze_problem(edit);
            let count = scene::frozen_count(stack, edit);
            let help = match problem {
                Some(why) => format!("Can't freeze: {why}."),
                None => format!(
                    "Bake the shape's settings and the {} up to and including this one into \
                     the shape, so they no longer have to be made again: big models stay quick. \
                     The shape doesn't change, but those edits can't be changed any more.",
                    if count == 1 {
                        "edit".to_string()
                    } else {
                        format!("{count} edits")
                    }
                ),
            };
            if ui
                .add_enabled(problem.is_none(), egui::Button::new(FREEZE_LABEL))
                .on_hover_text(&help)
                .on_disabled_hover_text(&help)
                .clicked()
            {
                out.push(DocRequest::Run(Command::Freeze { id, edit }));
                self.opened = None;
            }
            if ui
                .button(DELETE_EDIT_LABEL)
                .on_hover_text(
                    "Take this edit out. The edits after it are made again without it \
                     (Ctrl + Z brings it back).",
                )
                .clicked()
            {
                out.push(DocRequest::Run(Command::DeleteEdit { id, edit }));
                self.opened = None;
            }
        });
    }

    /// The settings rows of the edit at `at` (layer, place), changed in
    /// place with [`Command::ReplaceOperation`]: the Last edit section's,
    /// and an edit opened from the edit list's (decision 0092).
    #[allow(clippy::too_many_arguments)]
    fn edit_settings(
        &mut self,
        ui: &mut Ui,
        id: ObjectId,
        (layer, index): (usize, usize),
        operation: &Operation,
        units: Units,
        stretch: glam::Vec3,
        out: &mut Vec<DocRequest>,
    ) {
        let Some(settings) = operation.settings() else {
            return;
        };
        let mut edited = settings;
        let mut changed = false;
        // A mirrored edit shows the edit's own settings.
        let kind = operation.inner();
        let bevel = matches!(
            kind,
            Operation::BevelEdges { .. } | Operation::BevelPoints { .. }
        );
        let low = match kind {
            Operation::ExtrudePolygons { .. }
            | Operation::ExtrudeEdges { .. }
            | Operation::Groove { .. } => -MAX_SIZE,
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
                            if matches!(kind, Operation::Connect { .. }) {
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
                    let groove = matches!(kind, Operation::Groove { .. });
                    let (name, help) = if bevel {
                        (
                            "Width",
                            "How far the bevel reaches into each polygon (in the object's own \
                         size). Change it and the shape follows.",
                        )
                    } else if groove {
                        (
                            "Depth",
                            "How far the groove goes in (below 0) or the ridge comes out (above \
                             0), in the object's own size. Change it and the shape follows.",
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
                    if let Some(mut shape) = edited.extrude {
                        let touched = extrude_rows(ui, &mut shape, true);
                        if touched.began {
                            self.gesture += 1;
                        }
                        if touched.changed {
                            edited.extrude = Some(shape);
                            self.extrude = shape;
                            changed = true;
                        }
                    } else if matches!(kind, Operation::ExtrudeBent { .. }) {
                        // The shape rows stay, greyed out, so the bend rows
                        // below don't jump up the moment the Angle leaves 0.
                        // They show the shape it takes if the bend is taken
                        // back to nothing: then it becomes a straight
                        // extrude shaped as the tool is.
                        let mut shape = self.extrude;
                        extrude_rows(ui, &mut shape, false);
                        edited.extrude = Some(self.extrude);
                    }
                    if let Some(bend) = edited.bend {
                        // A straight extrude shows the tool's bend, turned
                        // to no angle: give it an angle and it bends so.
                        let mut bend = if matches!(kind, Operation::ExtrudeBent { .. }) {
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
                            // The rest carries on to the next extrude, but
                            // not the turn: that starts straight.
                            self.bend = scene::Bend { angle: 0.0, ..bend };
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
                    if let Some(mut shape) = edited.groove {
                        let touched = groove_rows(ui, &mut shape.width, &mut shape.sides, units);
                        if touched.began {
                            self.gesture += 1;
                        }
                        if touched.changed {
                            edited.groove = Some(shape);
                            self.groove.sides = shape.sides;
                            changed = true;
                        }
                    }
                    if let Some(mut options) = edited.bevel_options {
                        let touched = bevel_option_rows(ui, &mut options, units);
                        if touched.began {
                            self.gesture += 1;
                        }
                        if touched.changed {
                            edited.bevel_options = Some(options);
                            self.bevel.options = options;
                            changed = true;
                        }
                    }
                });
        });
        // Any change is worked out as the object is stretched now, so the
        // edit keeps its shape as seen (decisions 0081 and 0082).
        edited.stretch = Some(stretch);
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
             shade and bake wrongly. Mesh → Fix normals turns them the right way.",
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

/// The button that closes an opened edit.
pub(crate) const CLOSE_EDIT_LABEL: &str = "Close";
/// The opened edit's Keep editable switch, for screen readers and tests.
pub(crate) const KEEP_EDITABLE_SWITCH: &str = "Keep this edit editable";
/// Freezes the edits up to the opened one (decision 0092).
pub(crate) const FREEZE_LABEL: &str = "Freeze up to here";
/// Deletes the opened edit.
pub(crate) const DELETE_EDIT_LABEL: &str = "Delete edit";

/// What is wrong with edit `index` of layer `layer`, in plain words, if
/// anything truly is: points it can't find, or skipped as too big. (An
/// edit that found its points again by place is fine.)
pub(crate) fn edit_warning(
    warnings: &[scene::StackWarning],
    layer: usize,
    index: usize,
) -> Option<String> {
    let warning = warnings
        .iter()
        .find(|w| w.layer == layer && w.operation == index)?;
    if warning.too_big {
        Some("Skipped: it would make the shape too large.".into())
    } else if warning.missing > 0 {
        Some(match warning.missing {
            1 => "1 of its parts can't be found on the shape any more; it applies to the rest."
                .into(),
            n => format!(
                "{n} of its parts can't be found on the shape any more; it applies to the rest."
            ),
        })
    } else {
        None
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

/// The button that opens the last edit once something else is selected
/// (decision 0120).
pub(crate) const CHANGE_LAST_EDIT_LABEL: &str = "Change it";

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
        (operation.inner(), tool),
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
            | (Operation::Groove { .. }, Tool::Groove)
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

/// The name of an extrude's way, as listed.
pub(crate) fn direction_name(direction: scene::ExtrudeDirection) -> &'static str {
    use scene::ExtrudeDirection as D;
    match direction {
        D::Straight => "Straight out",
        D::AlongNormals => "Along normals",
        D::AlongX => "Along X",
        D::AlongY => "Along Y",
        D::AlongZ => "Along Z",
    }
}

/// What an extrude's way does, in plain words.
fn direction_help(direction: scene::ExtrudeDirection) -> &'static str {
    use scene::ExtrudeDirection as D;
    match direction {
        D::Straight => {
            "Every point goes the same way: straight out along the polygons' average facing. \
             The end keeps the polygons' shape."
        }
        D::AlongNormals => {
            "Each point goes along its own facing, so the end follows the surface's curve at \
             an even thickness, like puffing it out."
        }
        D::AlongX | D::AlongY | D::AlongZ => {
            "Straight along this axis of the object (its own axes), out the way the polygons \
             face: pull a sloped patch out level, say."
        }
    }
}

/// Extrude's shape: Direction, End size, Steps and Flat end, a row each
/// (decision 0070). Greyed out (`enabled` false) while a bend is set: a
/// bent extrude goes out its own way.
fn extrude_rows(ui: &mut Ui, shape: &mut scene::ExtrudeShape, enabled: bool) -> stepper::Touched {
    let mut out = stepper::Touched::default();
    let mut add = |t: stepper::Touched| {
        out.changed |= t.changed;
        out.began |= t.began;
    };
    let why = "A bent extrude goes out its own way: set the bend's Angle to 0 for these.";
    label(
        ui,
        "Direction",
        "Which way the polygons go. Straight out is the standard.",
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
            egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, "Extrude direction")
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
        "End size",
        "How big the new end is, compared with where it started: 100% the same, less tapers \
         it in (0% to a point), more flares it out.",
    );
    ui.add_enabled_ui(enabled, |ui| {
        add(stepper::percent(
            ui,
            &mut shape.end_size,
            (0.0, mesh::edit::MAX_END_SIZE),
            "End size",
        ));
    });
    ui.end_row();
    label(
        ui,
        "Steps",
        "How many rows the new walls have: more give loops to bend or shape later. Ctrl + \
         wheel changes it.",
    );
    ui.add_enabled_ui(enabled, |ui| {
        add(stepper::count(
            ui,
            &mut shape.steps,
            (1, mesh::edit::MAX_EXTRUDE_STEPS),
            "Extrude steps",
        ));
    });
    ui.end_row();
    label(
        ui,
        "Flat end",
        "Lay the new end flat, square to the way it went: a curved patch ends in a flat face.",
    );
    ui.add_enabled_ui(enabled, |ui| {
        let response = ui.checkbox(&mut shape.flat_end, "");
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, shape.flat_end, "Flat end")
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
fn bend_rows(ui: &mut Ui, bend: &mut scene::Bend, units: Units) -> stepper::Touched {
    // A small heading, so the bend reads as its own group under the
    // extrude's Distance.
    ui.label(RichText::new("Bend").small().color(theme::TEXT_WEAK));
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
        "Angle",
        "How far the extrude turns before going on straight: 0° is straight, 90° a corner. \
         The pipe keeps its full width through the turn. With a bend radius it can go past \
         360° into a spiral (up to 3600°, ten turns): each loop climbs just clear of the one \
         before.",
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
        "How many rows a smooth turn has (with a bend radius); for a spiral, how many each \
         full turn has. Ctrl + wheel changes it.",
    );
    add(stepper::count(
        ui,
        &mut bend.segments,
        (1, mesh::extrude::MAX_BEND_SEGMENTS),
        "Bend segments",
    ));
    ui.end_row();
    // The rise only does something with a bend radius, so it's greyed out
    // without one.
    label(
        ui,
        "Rise per turn",
        "Extra room between a spiral's loops: how much further each full turn climbs \
         along the spiral's middle. Past one turn the loops already clear each other, so 0 \
         leaves them just apart; under one turn it tilts the bend into a ramp. Needs a bend \
         radius.",
    );
    let rounded = bend.radius > 0.0;
    ui.add_enabled_ui(rounded, |ui| {
        add(stepper::length(
            ui,
            &mut bend.rise,
            (0.0, MAX_SIZE),
            units,
            "Rise per turn",
        ));
    });
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

/// Groove's Width and Sides, a row each (decision 0086).
fn groove_rows(
    ui: &mut Ui,
    width: &mut f32,
    sides: &mut scene::GrooveSides,
    units: Units,
) -> stepper::Touched {
    label(
        ui,
        "Groove width",
        "How wide the groove (or ridge) is, across the line of edges",
    );
    let mut out = stepper::length(
        ui,
        width,
        (0.0, mesh::groove::MAX_GROOVE_WIDTH),
        units,
        "Groove width",
    );
    ui.end_row();
    label(
        ui,
        "Sides",
        "Straight: walls square to the surface and a flat bottom, a crisp panel line. Sloped: \
         the sides slope down to a fold along the middle, a V.",
    );
    ui.horizontal(|ui| {
        for (choice, name) in [
            (scene::GrooveSides::Straight, "Straight"),
            (scene::GrooveSides::Sloped, "Sloped"),
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
/// Flip side, Even width, Stop at overlaps, and Weld overlapping points
/// with its distance.
fn bevel_option_rows(
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
        "Skew",
        "Puts more of the bevel on one side of the edges than the other: 0% is even. Along a \
         loop it stays on the same side all the way round. Flip side swaps the side.",
    );
    ui.horizontal(|ui| {
        let mut percent = options.skew * 100.0;
        let response = ui.add(
            egui::Slider::new(&mut percent, -100.0..=100.0)
                .suffix("%")
                .show_value(true),
        );
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Slider, true, "Skew"));
        if response.changed() {
            options.skew = (percent / 100.0).clamp(-1.0, 1.0);
        }
        add(stepper::Touched {
            changed: response.changed(),
            began: response.drag_started() || (response.changed() && !response.dragged()),
        });
        let mut flip = options.flip_side;
        let flip_button = ui
            .toggle_value(&mut flip, "Flip side")
            .on_hover_text("Swap which side of the edges the skew favours");
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
        "Even width",
        "Make every bevelled edge's new polygon the same width, however sharp or flat the \
         edge is (the width you set is what a right-angled edge gets)",
    ) {
        add(clicked);
    }
    if check(
        ui,
        &mut options.clamp,
        "Stop at overlaps",
        "Keep the bevel from crossing over itself: it gets only as wide as fits, so corners \
         meet instead of passing each other",
    ) {
        add(clicked);
    }
    if check(
        ui,
        &mut options.weld,
        "Weld points",
        "Weld overlapping points: new points that land on top of each other (where bevels \
         meet) become one, so there are no doubled points or tiny slivers",
    ) {
        add(clicked);
    }
    label(
        ui,
        "Weld distance",
        "How close points must be to weld: those nearer than this become one",
    );
    ui.add_enabled_ui(options.weld, |ui| {
        add(stepper::length(
            ui,
            &mut options.weld_distance,
            (0.0, mesh::bevel::MAX_WELD_DISTANCE),
            units,
            "Weld distance",
        ));
    });
    ui.end_row();
    *options = options.clamped();
    out
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
    let mut text = match count {
        1 => "1 edit recorded on top of the shape".to_string(),
        n => format!("{n} edits recorded on top of the shape"),
    };
    // Several layers: the Edit layers panel lists them (decision 0091).
    let layers = stack.layers().len();
    if layers > 1 {
        text.push_str(&format!(", in {layers} layers"));
    }
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
        ObjectKind::Instance(_) => "(instance)".to_string(),
        _ => "(group)".to_string(),
    }
}

/// The Collision row (decision 0078): the collision shape a game engine
/// makes for the object, as Godot reads it from an export made for it.
fn collision_row(ui: &mut Ui, id: ObjectId, current: scene::Collision, out: &mut Vec<DocRequest>) {
    label(
        ui,
        "Collision",
        "The collision shape Godot makes for it when you export for Godot: what the player \
         bumps into",
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
        egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, "Collision shape")
    });
    if chosen != current {
        out.push(DocRequest::Run(Command::SetCollision {
            ids: vec![id],
            collision: chosen,
        }));
    }
    ui.end_row();
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
