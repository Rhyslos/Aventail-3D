//! The workbench's part in moving, rotating and scaling: the tool, lock
//! and axes keys, placing the gizmo each frame, and carrying out its
//! drags as one undo step each ([decision 0030](../../../../docs/decisions/0030-transform-gizmos.md)).

use super::Workbench;
use crate::commands::CommandId;
use crate::coordinates::{CoordinatesView, Field, Interaction};
use crate::drawing::SceneDrawing;
use crate::transforming::{
    AXES_KEY, Axes, LOCK_KEYS, Session, Snapping, TOOL_KEYS, Target, Tool, describe,
};
use crate::viewport::GizmoEvent;
use glam::Vec3;
use scene::Command;
use tools::Change;

/// A change typed or dragged in the Coordinates panel: which field, the
/// change's session, and what the panel showed when it began.
#[derive(Debug, Clone)]
pub(super) struct CoordinatesSession {
    field: Field,
    session: Session,
    start: CoordinatesView,
}
use egui::{Modifiers, Ui};

impl Workbench {
    /// The tool in use.
    pub fn tool(&self) -> Tool {
        self.tool
    }

    /// Switches to `tool`, saying so in the status bar.
    pub(crate) fn set_tool(&mut self, tool: Tool) {
        if tool != Tool::Slit {
            self.finish_slit();
        }
        self.tool = tool;
        self.note = Some(match tool {
            Tool::Select => format!(
                "Select: {} (press Q again for the next way of selecting).",
                self.viewport.select_tool().name().to_lowercase()
            ),
            Tool::Extrude => "Extrude: drag anywhere in the viewport, the way the selected \
                 polygons (or open border edges) should go. Shift for fine steps; Esc or a \
                 right click cancels."
                .into(),
            Tool::Bevel => "Bevel: drag away from the selected edges (or points) to cut them \
                 away; back towards them to narrow it. Segments and Profile are in the \
                 Attributes panel. Shift for fine steps; Esc or a right click cancels."
                .into(),
            Tool::LoopCut => "Loop cut: point at an edge to see the new loop, click to cut \
                 it, or press and drag along the edge to slide it into place. Cuts is in the \
                 Attributes panel. Esc or a right click cancels a drag."
                .into(),
            Tool::Slit => "Slit: click points over the shape to cut along them. Drag a point to \
                 move it, Ctrl + click to remove it; Esc or Enter finishes."
                .into(),
            Tool::Inset => "Inset: drag left or right anywhere in the viewport to shrink the \
                 selected polygons inward. Shift for fine steps; Esc or a right click cancels."
                .into(),
            _ => format!(
                "{}: drag the gizmo's handles. Hold Shift for fine steps; Esc or a right click cancels.",
                tool.name()
            ),
        });
    }

    /// Flips the lock of an axis (0 X, 1 Y, 2 Z).
    pub(crate) fn toggle_lock(&mut self, axis: usize) {
        let locked = &mut self.viewport.options_mut().locked;
        locked[axis] = !locked[axis];
        let name = ["X", "Y", "Z"][axis];
        self.note = Some(if locked[axis] {
            format!("{name} is locked: moving, rotating and scaling leave it alone.")
        } else {
            format!("{name} is free again.")
        });
    }

    /// Switches the gizmo between the world's axes and the object's own.
    pub(crate) fn toggle_axes(&mut self) {
        let axes = &mut self.viewport.options_mut().axes;
        *axes = match axes {
            Axes::World => Axes::Object,
            Axes::Object => Axes::World,
        };
        self.note = Some(match axes {
            Axes::World => "The gizmo follows the world's axes.".into(),
            Axes::Object => "The gizmo follows the object's own axes.".into(),
        });
    }

    /// Turns snapping on or off (the palette button).
    pub(crate) fn toggle_snapping(&mut self) {
        let snap = &mut self.viewport.options_mut().snap;
        snap.on = !snap.on;
        self.note = Some(if snap.on {
            "Snapping on.".into()
        } else {
            "Snapping off.".into()
        });
    }

    /// `W`, `R`, `S`, `T` pick a tool; Shift+`X`, `Y`, `Z` lock axes; `L`
    /// switches world and object axes. Not while typing.
    pub(super) fn transform_keys(&mut self, ui: &Ui) {
        if ui.ctx().memory(|m| m.focused().is_some()) || self.objects.is_renaming() {
            return;
        }
        for (key, tool) in TOOL_KEYS {
            if ui.input_mut(|i| i.consume_key(Modifiers::NONE, key)) {
                self.set_tool(tool);
            }
        }
        for (keys, axis) in LOCK_KEYS {
            if ui.input_mut(|i| i.consume_shortcut(&keys)) {
                self.toggle_lock(axis);
            }
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, AXES_KEY)) {
            self.toggle_axes();
        }
    }

    /// The Tools menu: the tools, the axis locks, world or object axes,
    /// and snapping.
    pub(super) fn tools_menu(&mut self, ui: &mut Ui) {
        if ui
            .add(
                egui::Button::selectable(self.tool == Tool::Select, Tool::Select.name())
                    .shortcut_text(crate::selecting::SELECT_TOOL_KEY.name()),
            )
            .clicked()
        {
            self.set_tool(Tool::Select);
            ui.close();
        }
        // The rest from the command list.
        for (tool, command) in [
            (Tool::Move, CommandId::Move),
            (Tool::Rotate, CommandId::Rotate),
            (Tool::Scale, CommandId::Scale),
            (Tool::All, CommandId::AllGizmos),
        ] {
            let info = command.info();
            let mut button = egui::Button::selectable(self.tool == tool, info.name);
            if let Some(keys) = info.shortcut {
                button = button.shortcut_text(ui.ctx().format_shortcut(&keys));
            }
            if ui.add(button).on_hover_text(info.help).clicked() {
                self.set_tool(tool);
                ui.close();
            }
        }
        ui.separator();
        let locked = self.viewport.options().locked;
        for (keys, axis) in LOCK_KEYS {
            let label = format!("Lock {}", ["X", "Y", "Z"][axis]);
            let keys = ui.ctx().format_shortcut(&keys);
            if ui
                .add(egui::Button::selectable(locked[axis], label).shortcut_text(keys))
                .on_hover_text("A locked axis doesn't change while moving, rotating or scaling")
                .clicked()
            {
                self.toggle_lock(axis);
                ui.close();
            }
        }
        let object = self.viewport.options().axes == Axes::Object;
        if ui
            .add(egui::Button::selectable(object, "Object axes").shortcut_text(AXES_KEY.name()))
            .on_hover_text("On: the gizmo follows the object's own axes. Off: the world's.")
            .clicked()
        {
            self.toggle_axes();
            ui.close();
        }
        ui.separator();
        let snapping = self.viewport.options().snap.on;
        if ui
            .add(egui::Button::selectable(snapping, "Snapping"))
            .on_hover_text("Snap moves to points, edges, polygons and the grid. Settings → Viewport has the details.")
            .clicked()
        {
            self.toggle_snapping();
            ui.close();
        }
        ui.separator();
        ui.label(
            egui::RichText::new("Tip: press E for the tool menu")
                .small()
                .color(crate::theme::TEXT_WEAK),
        );
    }

    /// What the gizmo would change now, if a transform tool is in use.
    pub(super) fn target(&self, drawing: &SceneDrawing) -> Option<Target> {
        self.tool.gizmo()?;
        Target::of(
            &self.document,
            drawing,
            self.mode,
            self.viewport.options().axes,
        )
    }

    /// Places the gizmo for this frame.
    pub(super) fn place_gizmo(&mut self, drawing: &SceneDrawing) {
        let gizmo = self
            .tool
            .gizmo()
            .and_then(|tool| Some(self.target(drawing)?.gizmo(tool)));
        self.viewport.set_gizmo(gizmo);
    }

    /// Carries out this frame's gizmo events.
    pub(super) fn carry_out_gizmo(&mut self, ui: &Ui, drawing: &SceneDrawing) {
        let ctrl = ui.input(|i| i.modifiers.command);
        for event in self.viewport.take_gizmo_events() {
            match event {
                GizmoEvent::Begin { .. } => {
                    if let Some(target) = self.target(drawing) {
                        self.sessions += 1;
                        self.gizmo_session = Some(Session::new(target, self.sessions));
                        self.gizmo_shown = None;
                    }
                }
                GizmoEvent::Update {
                    change,
                    handle,
                    pointer,
                } => {
                    let Some(session) = &self.gizmo_session else {
                        continue;
                    };
                    let options = self.viewport.options();
                    let change = if options.snap.active(ctrl) {
                        let snapping = Snapping {
                            settings: options.snap,
                            grid: options.grid_detail,
                            scene: self.components.pick_scene(drawing),
                            drawing,
                            document: &self.document,
                            camera: pointer.camera,
                            size: pointer.size,
                            mouse: pointer.at,
                            locked: options.locked,
                        };
                        session.target.snapped(change, handle, &snapping)
                    } else {
                        change
                    };
                    if self.gizmo_shown == Some(change) {
                        // The mouse hasn't moved: nothing new to show.
                        continue;
                    }
                    self.gizmo_shown = Some(change);
                    let session = session.clone();
                    self.note = Some(match session.show(&mut self.document, &change) {
                        Ok(_) => describe(&change, &session.target, self.units),
                        Err(why) => format!("Couldn't do that: {why}."),
                    });
                }
                GizmoEvent::End => {
                    // The change stays as the last frame showed it; the
                    // status bar keeps saying how far it went.
                    self.gizmo_session = None;
                }
                GizmoEvent::Cancel => {
                    if let Some(session) = self.gizmo_session.take() {
                        session.cancel(&mut self.document);
                        self.note = Some("Cancelled: everything is back where it was.".into());
                    }
                }
            }
        }
    }

    /// What the Coordinates panel shows this frame.
    pub(super) fn coordinates_view(&self, drawing: &SceneDrawing) -> CoordinatesView {
        let target = Target::of(&self.document, drawing, self.mode, Axes::World);
        CoordinatesView::of(target.as_ref(), &self.document)
    }

    /// Carries out what was typed or dragged in the Coordinates panel.
    pub(super) fn carry_out_coordinates(&mut self, done: Interaction, drawing: &SceneDrawing) {
        if let Some(edit) = done.edit {
            let fresh = done.began
                || self
                    .coordinates_session
                    .as_ref()
                    .is_none_or(|s| s.field != edit.field);
            if fresh {
                self.coordinates_session =
                    Target::of(&self.document, drawing, self.mode, Axes::World).map(|target| {
                        self.sessions += 1;
                        let start = CoordinatesView::of(Some(&target), &self.document);
                        CoordinatesSession {
                            field: edit.field,
                            session: Session::new(target, self.sessions),
                            start,
                        }
                    });
            }
            if let Some(active) = self.coordinates_session.clone()
                && let Err(why) = coordinates_change(&active, edit.value, &mut self.document)
            {
                self.note = Some(format!("Couldn't do that: {why}."));
            }
        }
        if done.active.is_none() {
            self.coordinates_session = None;
        }
    }
}

/// Makes the scene show the field of `active` set to `value`, measured
/// from where everything was when the change began.
fn coordinates_change(
    active: &CoordinatesSession,
    value: f32,
    document: &mut scene::Document,
) -> Result<(), String> {
    let session = &active.session;
    let target = &session.target;
    let single = match &target.targets {
        crate::transforming::Targets::Objects(starts) if starts.len() == 1 => Some(&starts[0]),
        _ => None,
    };
    match active.field {
        Field::Position(i) => {
            let start = active.start.position.unwrap_or(target.pivot);
            let mut moved = Vec3::ZERO;
            moved[i] = value - start[i];
            session
                .show(document, &Change::Translate(moved))
                .map(|_| ())
        }
        Field::Size(i) => {
            let start = active.start.size.unwrap_or(Vec3::ONE)[i];
            if start.abs() < 1e-4 || value.abs() < 1e-3 {
                // Nothing to scale from (a flat side), or a size of 0,
                // which would squash the shape flat for good.
                return Ok(());
            }
            let factor = value / start;
            match single {
                Some(object) => {
                    let mut t = object.transform;
                    t.scale[i] *= factor;
                    session.show_commands(
                        document,
                        vec![Command::SetTransforms(vec![(object.id, t)])],
                        &format!("Scale {}", object.name),
                    )
                }
                None => {
                    let mut f = Vec3::ONE;
                    f[i] = factor;
                    session.show(document, &Change::Scale(f)).map(|_| ())
                }
            }
        }
        Field::Rotation(i) => {
            let Some(object) = single else {
                return Ok(());
            };
            let mut t = object.transform;
            t.rotation[i] = value.to_radians();
            session.show_commands(
                document,
                vec![Command::SetTransforms(vec![(object.id, t)])],
                &format!("Rotate {}", object.name),
            )
        }
    }
}
