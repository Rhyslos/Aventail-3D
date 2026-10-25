//! The workbench's part in moving, rotating and scaling: the tool, lock
//! and axes keys, placing the gizmo each frame, and carrying out its
//! drags as one undo step each ([decision 0030](../../../../docs/decisions/0030-transform-gizmos.md)).

use super::Workbench;
use crate::commands::CommandId;
use crate::coordinates::{CoordinatesView, Field, Interaction};
use crate::drawing::SceneDrawing;
use crate::text::tr;
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

/// The undo key changes from the Symmetry menu share, so they merge into
/// one step.
const MIRROR_SETTINGS_KEY: u64 = (1 << 59) | 0x6d69_7272;

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
        if tool != Tool::DrawCurve {
            // Keeps the curve drawn so far (decision 0138).
            self.finish_curve();
        }
        if tool != self.tool {
            // Extrude's bend angle is for the extrude about to be made:
            // leaving the tool sets it straight again.
            self.attributes.bend.angle = 0.0;
        }
        self.tool = tool;
        self.note = Some(match tool {
            Tool::Select => crate::trf!(
                "Select: {} (press Q again for the next way of selecting).",
                self.viewport.select_tool().name().to_lowercase()
            ),
            Tool::Extrude => tr(
                "Extrude: drag anywhere in the viewport, the way the selected \
                 polygons (or open border edges) should go. Shift for fine steps; Esc or a \
                 right click cancels.",
            )
            .into(),
            Tool::Bevel => tr(
                "Bevel: drag away from the selected edges (or points) to cut them \
                 away; back towards them to narrow it. Segments and Profile are in the \
                 Attributes panel. Shift for fine steps; Esc or a right click cancels.",
            )
            .into(),
            Tool::Groove => tr(
                "Groove: drag into the surface to cut a groove along the selected edges, \
                 or out of it for a ridge. Esc or a right click cancels.",
            )
            .into(),
            Tool::LoopCut => tr(
                "Loop cut: point at an edge to see the new loop, click to cut \
                 it, or press and drag along the edge to slide it into place. Cuts is in the \
                 Attributes panel. Esc or a right click cancels a drag.",
            )
            .into(),
            Tool::Slit => tr(
                "Slit: click points over the shape to cut along them. Drag a point to \
                 move it, Ctrl + click to remove it; Esc or Enter finishes, and clicking the \
                 first point closes the line into a loop.",
            )
            .into(),
            Tool::Bridge => tr(
                "Bridge: drag from one open edge to another to join them. With two \
                 open borders (or two polygons) selected, C bridges them at once.",
            )
            .into(),
            Tool::Weld => tr(
                "Weld: drag a point (or, in the Polygons mode, a polygon) onto a point \
                 to melt it there (the whole selection, if you drag a selected one). M welds \
                 the selection at its middle.",
            )
            .into(),
            Tool::DrawCurve => tr(
                "Draw curve: click to place a sharp point, press and drag to pull out handles \
                 for a smooth bend. Enter or a right click finishes, Esc cancels, Backspace \
                 takes the last point back; clicking the first point closes the curve.",
            )
            .into(),
            Tool::Inset => tr(
                "Inset: drag left or right anywhere in the viewport to shrink the \
                 selected polygons inward. Shift for fine steps; Esc or a right click cancels.",
            )
            .into(),
            Tool::Move | Tool::Rotate | Tool::Scale | Tool::All
                if self.mode == crate::palette::Mode::Uv =>
            {
                crate::trf!(
                    "{}: drag in the UV Editor, or on the model to {} the texture there. Shift \
                     for fine steps; Esc cancels.",
                    tool.name(),
                    match tool {
                        Tool::Rotate => tr("turn"),
                        Tool::Scale => tr("scale"),
                        _ => tr("slide"),
                    }
                )
            }
            _ => crate::trf!(
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
            crate::trf!(
                "{} is locked: moving, rotating and scaling leave it alone.",
                name
            )
        } else {
            crate::trf!("{} is free again.", name)
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
            Axes::World => tr("The gizmo follows the world's axes.").into(),
            Axes::Object => tr("The gizmo follows the object's own axes.").into(),
        });
    }

    /// Turns snapping on or off (the palette button).
    pub(crate) fn toggle_snapping(&mut self) {
        let snap = &mut self.viewport.options_mut().snap;
        snap.on = !snap.on;
        self.note = Some(if snap.on {
            tr("Snapping on.").into()
        } else {
            tr("Snapping off.").into()
        });
    }

    /// The selected objects, and the objects inside them, that a live
    /// mirror can go on (those with a shape).
    fn mirrorable(&self) -> Vec<scene::ObjectId> {
        let scene = self.document.scene();
        let mut out = Vec::new();
        for id in self.document.selection() {
            for inner in scene.subtree(id) {
                if !out.contains(&inner) && scene.object(inner).is_some_and(|o| o.kind.has_shape())
                {
                    out.push(inner);
                }
            }
        }
        out
    }

    /// The live mirror of the first selected object that has one
    /// (decision 0088).
    pub(crate) fn selected_mirror(&self) -> Option<scene::LiveMirror> {
        let scene = self.document.scene();
        self.mirrorable()
            .into_iter()
            .find_map(|id| scene.object(id).and_then(|o| o.mirror))
    }

    /// The Symmetry button (decision 0088): gives the selected objects a
    /// live mirror with the Symmetry menu's settings, or takes theirs
    /// away if any has one.
    pub(crate) fn toggle_symmetry(&mut self) {
        let ids = self.mirrorable();
        if ids.is_empty() {
            self.note = Some(tr("Symmetry: select the objects to mirror first.").into());
            return;
        }
        let settings = self.viewport.options().symmetry;
        let mirror = if self.selected_mirror().is_some() {
            None
        } else {
            match settings.mirror() {
                Some(mirror) => Some(mirror),
                None => {
                    self.note = Some(
                        tr(
                            "Symmetry: pick X, Y or Z in its menu first (the arrow beside the \
                         Symmetry button).",
                        )
                        .into(),
                    );
                    return;
                }
            }
        };
        let on = mirror.is_some();
        if let Err(why) = self.document.execute(Command::SetMirror { ids, mirror }) {
            self.note = Some(crate::trf!("Couldn't change the mirror: {}.", why));
            return;
        }
        self.note = Some(if on {
            let names: Vec<&str> = ["X", "Y", "Z"]
                .into_iter()
                .zip(settings.axes)
                .filter(|(_, on)| *on)
                .map(|(name, _)| name)
                .collect();
            crate::trf!(
                "Symmetry on, across {}: the other side follows every edit and every move.",
                names.join(tr(" and "))
            )
        } else {
            tr("Symmetry off: the mirrored side is gone (Ctrl + Z brings it back).").into()
        });
    }

    /// Apply mirror (decision 0126, as Blender's Apply): the selected
    /// objects' mirrored halves become part of their shapes, and the
    /// mirrors are gone, as one undo step.
    pub(crate) fn apply_mirror(&mut self) {
        let scene = self.document.scene();
        let ids: Vec<scene::ObjectId> = self
            .mirrorable()
            .into_iter()
            .filter(|id| scene.object(*id).is_some_and(|o| o.mirror.is_some()))
            .collect();
        if ids.is_empty() {
            self.note = Some(
                tr("Apply mirror: select an object with symmetry on first (the Symmetry button).")
                    .into(),
            );
            return;
        }
        let n = ids.len();
        match self.document.execute(Command::ApplyMirror(ids)) {
            Ok(_) => {
                self.note = Some(crate::trf!(
                    "Mirror applied: both halves are part of {} now, and symmetry is off. \
                     Ctrl + Z takes it back.",
                    if n == 1 {
                        tr("the shape").to_string()
                    } else {
                        crate::trf!("{} shapes", n)
                    }
                ));
            }
            Err(why) => self.note = Some(crate::trf!("Couldn't apply the mirror: {}.", why)),
        }
    }

    /// The Symmetry menu changed its settings: they are kept for the next
    /// mirror, and the selected objects' mirrors change to match, as one
    /// undo step while the menu is used.
    pub(crate) fn change_symmetry(&mut self, settings: crate::Symmetry) {
        self.viewport.options_mut().symmetry = settings;
        let Some(mirror) = settings.mirror() else {
            return;
        };
        let scene = self.document.scene();
        let ids: Vec<scene::ObjectId> = self
            .mirrorable()
            .into_iter()
            .filter(|id| scene.object(*id).is_some_and(|o| o.mirror.is_some()))
            .collect();
        if ids.is_empty() {
            return;
        }
        if let Err(why) = self.document.execute_labelled(
            Command::SetMirror {
                ids,
                mirror: Some(mirror),
            },
            tr("Change the mirror"),
            Some(MIRROR_SETTINGS_KEY),
        ) {
            self.note = Some(crate::trf!("Couldn't change the mirror: {}.", why));
        }
    }

    /// `W`, `R`, `S`, `T` pick a tool; Shift+`X`, `Y`, `Z` lock axes; `L`
    /// switches world and object axes. Not while typing.
    pub(super) fn transform_keys(&mut self, ui: &Ui) {
        if ui.ctx().memory(|m| m.focused().is_some()) || self.renaming() {
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
        // Edit pivot: the Object mode as a switch (decision 0126).
        if ui
            .add(egui::Button::selectable(
                self.mode == crate::palette::Mode::Object,
                tr(crate::palette::EDIT_PIVOT_LABEL),
            ))
            .on_hover_text(tr(
                "Move and turn the selected objects' pivots (the point they turn and scale \
                 around), not their shapes. Also on the top palette, by the axis switches.",
            ))
            .clicked()
        {
            self.toggle_edit_pivot();
            ui.close();
        }
        // Placing pivots (decision 0068); the places on the shape's box
        // in a list of their own (decision 0160).
        let presets = CommandId::PivotPresets;
        crate::dialog::menu(ui, presets.name(), |ui| {
            for command in presets.members() {
                self.pivot_entry(ui, *command);
            }
        })
        .response
        .on_hover_text(presets.info().help);
        for command in [
            CommandId::PivotToOrigin,
            CommandId::PivotToObject,
            CommandId::ObjectToOrigin,
        ] {
            self.pivot_entry(ui, command);
        }
        ui.separator();
        // Snap to and Align (decision 0093).
        let ready = !self.document.selection().is_empty();
        for (label, help, what) in [
            (
                tr(super::aligning::SNAP_TO_LABEL),
                tr(super::aligning::SNAP_TO_HELP),
                super::aligning::TargetUse::SnapTo,
            ),
            (
                tr(super::aligning::ALIGN_LABEL),
                tr(super::aligning::ALIGN_HELP),
                super::aligning::TargetUse::Align,
            ),
        ] {
            if ui
                .add_enabled(ready, egui::Button::new(label))
                .on_hover_text(help)
                .on_disabled_hover_text(tr("Select one or more objects first"))
                .clicked()
            {
                self.start_target_pick(what);
                ui.close();
            }
        }
        ui.separator();
        let locked = self.viewport.options().locked;
        for (keys, axis) in LOCK_KEYS {
            let label = crate::trf!("Lock {}", ["X", "Y", "Z"][axis]);
            let keys = ui.ctx().format_shortcut(&keys);
            if ui
                .add(egui::Button::selectable(locked[axis], label).shortcut_text(keys))
                .on_hover_text(tr(
                    "A locked axis doesn't change while moving, rotating or scaling",
                ))
                .clicked()
            {
                self.toggle_lock(axis);
                ui.close();
            }
        }
        let object = self.viewport.options().axes == Axes::Object;
        if ui
            .add(egui::Button::selectable(object, tr("Object axes")).shortcut_text(AXES_KEY.name()))
            .on_hover_text(tr(
                "On: the gizmo follows the object's own axes. Off: the world's.",
            ))
            .clicked()
        {
            self.toggle_axes();
            ui.close();
        }
        ui.separator();
        let snapping = self.viewport.options().snap.on;
        if ui
            .add(egui::Button::selectable(snapping, tr("Snapping")))
            .on_hover_text(tr("Snap moves to points, edges, polygons and the grid. Settings → Viewport has the details."))
            .clicked()
        {
            self.toggle_snapping();
            ui.close();
        }
        let symmetry = self.selected_mirror().is_some();
        if ui
            .add(egui::Button::selectable(symmetry, tr("Symmetry")))
            .on_hover_text(tr(crate::palette::SYMMETRY_HELP))
            .clicked()
        {
            self.toggle_symmetry();
            ui.close();
        }
        ui.separator();
        ui.label(
            egui::RichText::new(tr("Tip: press E for the tool menu"))
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
        // The UV mode moves UVs, not the shape: its gizmo is the UV
        // Editor's (the next milestone, decision 0096).
        if self.mode == crate::palette::Mode::Uv {
            self.viewport.set_gizmo(None);
            return;
        }
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
                GizmoEvent::Begin { slide, .. } => {
                    if let Some(target) = self.target(drawing) {
                        // Shift + Alt + drag on the Move gizmo slides points along
                        // their edges (not with symmetry, which mirrors moves).
                        let slide = slide
                            && matches!(self.tool, Tool::Move | Tool::All)
                            && crate::components::kind_for(self.mode).is_some();
                        let target =
                            if slide {
                                self.note = Some(
                                tr("Sliding: the points move along their edges, staying on the \
                                 surface.")
                                    .into(),
                            );
                                target.sliding(drawing)
                            } else {
                                target
                            };
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
                        Err(why) => crate::trf!("Couldn't do that: {}.", why),
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
                        self.note = Some(tr("Cancelled: everything is back where it was.").into());
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
                self.note = Some(crate::trf!("Couldn't do that: {}.", why));
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
            if target.pivots {
                // A pivot has no size.
                return Ok(());
            }
            match single {
                Some(object) => {
                    let mut t = object.transform;
                    t.scale[i] *= factor;
                    session.show_commands(
                        document,
                        vec![Command::SetTransforms(vec![(object.id, t)])],
                        &crate::trf!("Scale {}", object.name),
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
            let label = if target.pivots {
                crate::trf!("Rotate {}'s pivot", object.name)
            } else {
                crate::trf!("Rotate {}", object.name)
            };
            session.show_commands(
                document,
                vec![target.objects_command(vec![(object.id, t)])],
                &label,
            )
        }
    }
}
