//! The Attributes panel: the settings of the selected object. Its name
//! and whether it is shown, and for a primitive its shape settings (size,
//! segments and so on), which change the shape as you drag or type.
//!
//! Like the Objects panel, the panel never changes the scene itself: it
//! hands back commands. Dragging a value changes it every frame, so those
//! commands are merged into one undo step per drag (or per value typed).

mod crease_rows;
mod curve_points;
mod curve_rows;
mod decimate_rows;
mod edits;
mod fields;
mod last_edit;
mod lattice_rows;
mod materials;
mod mesh_check;
pub(crate) mod modifier_rows;
mod object_rows;
mod scatter_rows;
pub(crate) mod stepper;
mod tool_rows;
mod tool_section;

use crate::objects::DocRequest;
use crate::panels::placeholder;
use crate::text::tr;
use crate::theme;
use crate::transforming::Tool;
use crate::units::Units;
#[cfg(test)]
pub(crate) use crease_rows::CREASE_LABEL;
pub(crate) use crease_rows::SelectedCreases;
pub(crate) use curve_points::CurveLook;
use edits::edits_section;
pub(crate) use edits::{DELETE_EDIT_LABEL, FREEZE_LABEL, OpenedEdit, edit_warning};
use egui::{RichText, Ui, vec2};
pub(crate) use last_edit::{last_edit, made_by};
use mesh_check::{MeshChecker, mesh_section};
use object_rows::{collision_row, heading, primitive_of, smoothing_row};
use scene::{Command, Document, EditId, Grouping, ObjectId, ObjectKind, Primitive};
pub(crate) use tool_section::{BevelLook, GrooveLook, HardAngle, JoinLook, LoopLook};
// The tests reach these through `crate::attributes::`, as when they were
// defined here.
#[cfg(test)]
pub(crate) use edits::CLOSE_EDIT_LABEL;
#[cfg(test)]
pub(crate) use last_edit::CHANGE_LAST_EDIT_LABEL;

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
    /// The curve commands' radius and count, and what the Curve points
    /// section asked for (decision 0139).
    pub(crate) curve: CurveLook,
    /// The Materials section's names being typed and its message.
    materials: materials::MaterialsState,
    /// How many polygons of the shown object are selected, in the
    /// Polygons mode (set by the workbench each frame); `None` in other
    /// modes. Assign needs some.
    pub(crate) polygons_selected: Option<usize>,
    /// The material slot whose Assign button was clicked, for the
    /// workbench to give the selected polygons (decision 0074).
    pub(crate) assign: Option<u32>,
    /// The selected edges' creases, in the Edges mode (set by the
    /// workbench each frame); `None` in other modes (decision 0143).
    pub(crate) creases: Option<SelectedCreases>,
    /// A crease set in the Crease field, with the gesture it was set by,
    /// for the workbench to record on the selected edges.
    pub(crate) crease_asked: Option<(f32, u64)>,
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

impl AttributesPanel {
    /// Whether the next extrude bends (then its way, end size, steps and
    /// flat end don't apply).
    pub(crate) fn extrude_bends(&self) -> bool {
        self.bend.bends()
    }
}

/// The menu entries for the units lengths are shown in.
pub(crate) fn unit_entries(ui: &mut Ui, units: &mut Units) {
    for choice in Units::ALL {
        if ui
            .radio(*units == choice, choice.name())
            .on_hover_text(crate::trf!(
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
        .add_enabled(single.is_some(), egui::Button::new(tr("Copy settings")))
        .on_hover_text(tr("Copy the selected shape's settings"))
        .on_disabled_hover_text(tr("Select one shape to copy its settings"))
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
        Some(copied) => crate::trf!("Paste {} settings", copied.name().to_lowercase()),
        None => tr("Paste settings").to_string(),
    };
    if ui
        .add_enabled(!targets.is_empty(), egui::Button::new(paste))
        .on_hover_text(tr(
            "Give the selected shapes of the same kind the copied settings",
        ))
        .on_disabled_hover_text(tr(
            "Copy a shape's settings, then select shapes of the same kind",
        ))
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
        crate::dialog::menu(ui, tr("Mode"), |ui| {
            let _ = ui.radio(true, tr("Object settings"));
            ui.add_enabled(false, egui::RadioButton::new(false, tr("Tool settings")))
                .on_disabled_hover_text(tr("Arrives with the tools in Phase 3"));
        });
        crate::dialog::menu(ui, tr("Edit"), |ui| {
            settings_entries(ui, doc, shared.clipboard, out)
        });
        crate::dialog::menu(ui, tr("Units"), |ui| unit_entries(ui, shared.units));
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
                    tr("Nothing selected"),
                    tr("Select an object or pick a tool, and its settings show up here."),
                );
            }
            [one] => *one,
            many => {
                return placeholder(
                    ui,
                    &crate::trf!("{} objects selected", many.len()),
                    tr("Select one object to see its settings."),
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
                // A curve that is only a line has no surface (decision
                // 0138): no collision, smoothing, booleans or materials.
                let solid = object.kind.has_shape()
                    && !matches!(&object.kind, ObjectKind::Curve(c) if c.solid == scene::CurveSolid::Line);
                section(ui, tr("Basic"));
                egui::Grid::new("attributes_basic")
                    .num_columns(2)
                    .min_col_width(LABEL_WIDTH)
                    .spacing(vec2(12.0, 6.0))
                    .show(ui, |ui| {
                        self.name_row(ui, id, &object.name, out);
                        label(ui, tr("Visible"), tr("Whether it is shown in the viewport"));
                        let mut visible = object.visible;
                        let switch = ui.checkbox(&mut visible, "");
                        switch.widget_info(|| {
                            egui::WidgetInfo::selected(
                                egui::WidgetType::Checkbox,
                                true,
                                visible,
                                tr("Visible"),
                            )
                        });
                        if switch.changed() {
                            out.push(DocRequest::Run(Command::SetVisible {
                                ids: vec![id],
                                visible,
                            }));
                        }
                        ui.end_row();
                        if solid {
                            collision_row(ui, id, object.collision, out);
                        }
                        if solid && !matches!(object.kind, ObjectKind::Instance(_)) {
                            smoothing_row(ui, id, object.hardness, out);
                        }
                    });
                if solid && !matches!(object.kind, ObjectKind::Instance(_)) {
                    self.crease_section(ui);
                }
                if let ObjectKind::Instance(source) = &object.kind {
                    section(ui, tr("Instance"));
                    let original = doc
                        .scene()
                        .object(*source)
                        .map_or_else(|| "(gone)".to_string(), |o| o.name.clone());
                    ui.label(crate::trf!("Shows the shape of: {}", original));
                    ui.label(
                        RichText::new(tr(
                            "Change the original and every instance follows. Bake into shape \
                             (Ctrl + E) gives this one its own shape.",
                        ))
                        .small()
                        .color(theme::TEXT_WEAK),
                    );
                }
                if let ObjectKind::Primitive(primitive) = &object.kind {
                    section(ui, tr("Shape"));
                    self.shape_rows(ui, id, *primitive, units, out);
                }
                if matches!(object.kind, ObjectKind::Curve(_)) {
                    self.curve_section(ui, doc, id, units, out);
                }
                if matches!(object.kind, ObjectKind::Lattice(_)) {
                    self.lattice_section(ui, doc, id, out);
                }
                if solid {
                    crate::booleans::attributes_section(ui, doc, id, out);
                    self.modifiers_section(ui, doc, id, units, out);
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
}

/// What the Attributes panel shares with the rest of the window: the
/// copied settings and the units lengths are shown in.
pub(crate) struct Shared<'a> {
    /// Settings copied with Copy settings.
    pub(crate) clipboard: &'a mut Option<Primitive>,
    /// Metres or centimetres.
    pub(crate) units: &'a mut Units,
}

/// A section title with a thin line under it.
pub(crate) fn section(ui: &mut Ui, title: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(title).strong());
    ui.separator();
}

/// A row's label, explained on hover.
fn label(ui: &mut Ui, text: &str, help: &str) {
    ui.label(RichText::new(text).color(theme::TEXT_WEAK))
        .on_hover_text(help);
}
