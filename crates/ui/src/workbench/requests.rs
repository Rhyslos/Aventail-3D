//! The keys that change the document or the mode (undo, delete, the
//! mode keys, Tab to step back), and carrying out what the keys, menus
//! and panels asked for each frame.

use super::selection::SelectRequest;
use super::{BACK_KEY, MODE_KEYS, Workbench};
use crate::commands::CommandId;
use crate::components;
use crate::objects::{self, DocRequest};
use crate::palette::Mode;
use crate::panel::Panel;
use crate::text::tr;
use crate::transforming::Tool;
use egui::{Key, Modifiers, Ui};
use scene::Command;

impl Workbench {
    /// Keys that change the document: undo, redo, delete, deselect, group
    /// and ungroup. Ignored while typing in a text box, which has its own.
    pub(super) fn document_keys(&mut self, ui: &Ui, out: &mut Vec<DocRequest>) {
        // (While a name is typed, Esc means "keep the old name", even on
        // the frame egui has already taken the keyboard away.)
        if ui.ctx().memory(|m| m.focused().is_some()) || self.objects.is_renaming() {
            return;
        }
        // While the gizmo is dragged, Esc cancels the drag, and nothing
        // else happens.
        if self.uv_editor.dragging_gizmo() {
            if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
                self.uv_editor.cancel_gizmo(&mut self.document);
            }
            return;
        }
        if self.viewport.dragging_gizmo() {
            if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
                self.viewport.cancel_gizmo_drag();
            }
            return;
        }
        // The same for a modelling tool's drag.
        if self.viewport.dragging_tool() {
            if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
                self.viewport.cancel_tool_drag();
            }
            return;
        }
        // While measuring in the UV Editor, Esc stops (decision 0115).
        if self.uv_editor.measuring()
            && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.uv_editor.set_measuring(false);
            return;
        }
        // While a target click is awaited, Esc cancels it.
        if self.target_pick.is_some()
            && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.cancel_target_pick();
            return;
        }
        // Draw curve (decision 0138): Enter finishes the curve, Esc cancels
        // it, Backspace takes its last point back.
        if self.tool == Tool::DrawCurve {
            if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) {
                self.finish_curve_and_put_away();
                return;
            }
            if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
                self.cancel_curve();
                return;
            }
            if self.curve_session.is_some()
                && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Backspace))
            {
                self.take_back_curve_point();
                return;
            }
        }
        // While a slit's line is drawn, Enter finishes it and Esc cancels
        // it, taking its cut back, as Esc cancels everywhere (decision
        // 0126). Tab (back to the Model mode) finishes it too.
        if self.slit_session.is_some() {
            if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) {
                self.finish_slit();
                return;
            }
            if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
                self.cancel_slit();
                return;
            }
        }
        let popup_open = egui::Popup::is_any_open(ui.ctx());
        let anything = !self.document.selection().is_empty();
        let mode = self.mode;
        // A modifier clicked in a list: Delete and Backspace in the Model
        // mode remove it, not its object (decision 0168).
        let picked =
            crate::modifier_list::picked_place(&self.document, self.attributes.picked_modifier)
                .filter(|_| components::kind_for(mode).is_none());
        let mut commands = Vec::new();
        ui.input_mut(|i| {
            // Ctrl+Shift+Z first: Ctrl+Z would match it too.
            if i.consume_shortcut(&objects::REDO_SHORTCUT_2)
                || i.consume_shortcut(&objects::REDO_SHORTCUT)
            {
                out.push(DocRequest::Redo);
            } else if i.consume_shortcut(&objects::UNDO_SHORTCUT) {
                out.push(DocRequest::Undo);
            }
            // The object commands with keys, from the command list. In
            // the Points, Edges and Polygons modes, Delete deletes the
            // selected elements and Backspace dissolves them instead.
            let editing = components::kind_for(mode).is_some();
            let (delete, dissolve) = if editing {
                (CommandId::DeleteElements, CommandId::Dissolve)
            } else {
                (CommandId::Delete, CommandId::Delete)
            };
            for command in [
                CommandId::Ungroup,
                CommandId::MakeEditable,
                CommandId::Group,
            ] {
                let pressed = command
                    .info()
                    .shortcut
                    .is_some_and(|keys| i.consume_shortcut(&keys));
                if pressed && anything {
                    commands.push(command);
                }
            }
            if let Some((id, index)) = picked {
                if i.consume_key(Modifiers::NONE, Key::Delete)
                    | i.consume_key(Modifiers::NONE, Key::Backspace)
                {
                    out.push(DocRequest::Run(Command::RemoveModifier { id, index }));
                }
            } else {
                if i.consume_key(Modifiers::NONE, Key::Delete) && anything {
                    commands.push(delete);
                }
                if i.consume_key(Modifiers::NONE, Key::Backspace) && anything {
                    commands.push(dissolve);
                }
            }
        });
        for command in commands {
            self.run_command(command, ui.ctx(), out);
        }
        // Esc closes an open menu first; otherwise it deselects: the
        // points, edges or polygons in those modes, else the objects.
        // With a window open, Esc closes it instead (decision 0087).
        if !popup_open
            && !self.window_open()
            && ui.input_mut(|i| i.consume_shortcut(&objects::DESELECT_SHORTCUT))
        {
            if components::kind_for(self.mode).is_some() {
                self.select_requests.push(SelectRequest::Clear);
            } else {
                self.document.clear_selection();
            }
        }
    }

    /// Carries out what the menus, keys and panels asked for this frame,
    /// saying in the status bar what happened when it isn't plain to see.
    pub(super) fn carry_out(&mut self, requests: Vec<DocRequest>) {
        for request in requests {
            self.note = match request {
                DocRequest::Run(command) => {
                    let what = command.describe(self.document.scene());
                    // A modifier added to one shape shows its settings at
                    // once (decision 0168): the one whose id is new.
                    let added_to = match &command {
                        Command::AddModifier { ids, .. } if ids.len() == 1 => Some(ids[0]),
                        _ => None,
                    };
                    let ids_before = |doc: &scene::Document, id| -> Vec<scene::ModifierId> {
                        doc.scene().modifiers(id).iter().map(|m| m.id).collect()
                    };
                    let before = added_to.map(|id| ids_before(&self.document, id));
                    match self.document.execute(command) {
                        Ok(()) => {
                            if let (Some(id), Some(before)) = (added_to, before) {
                                let new = ids_before(&self.document, id)
                                    .into_iter()
                                    .find(|m| !before.contains(m));
                                if let Some(new) = new {
                                    self.attributes.picked_modifier = Some((id, new));
                                }
                            }
                            None
                        }
                        Err(why) => Some(crate::trf!("Couldn't {}: {}.", what.to_lowercase(), why)),
                    }
                }
                DocRequest::RunMerging(command, key) => {
                    let what = command.describe(self.document.scene());
                    match self.document.execute_merging(command, key) {
                        Ok(()) => None,
                        Err(why) => Some(crate::trf!("Couldn't {}: {}.", what.to_lowercase(), why)),
                    }
                }
                DocRequest::Undo => Some(match self.document.undo() {
                    Some(what) => crate::trf!("Undone: {}.", what),
                    None => tr("Nothing to undo.").into(),
                }),
                DocRequest::Redo => Some(match self.document.redo() {
                    Some(what) => crate::trf!("Redone: {}.", what),
                    None => tr("Nothing to redo.").into(),
                }),
                DocRequest::Select(ids) => {
                    self.document.set_selection(ids);
                    None
                }
            };
        }
        if self.document.forgotten_steps() > 0 && !self.forgotten_told && self.note.is_none() {
            self.forgotten_told = true;
            self.note = Some(crate::trf!(
                "Undo memory is full ({}): the oldest steps are now forgotten as you work. \
                 Settings → Undo sets the limit.",
                self.undo_memory.short()
            ));
        }
    }

    /// Adds an edit layer to the selected objects that can take one, and
    /// brings the Edit layers panel to the front (decision 0091).
    pub(super) fn add_edit_layer(&mut self, requests: &mut Vec<DocRequest>) {
        let targets = crate::layers::layer_targets(&self.document);
        if targets.is_empty() {
            self.note = Some(
                tr("Select an object with a shape first: groups and instances have no edit layers.")
                    .into(),
            );
            return;
        }
        requests.push(DocRequest::Run(Command::AddLayer(targets)));
        crate::layout::show_tab(&mut self.dock, Panel::EditLayers);
    }

    /// Whether a selected object has a shape to edit (for the Points,
    /// Edges and Polygons modes).
    pub(super) fn editing_anything(&self) -> bool {
        self.document.selection().iter().any(|id| {
            self.document
                .scene()
                .object(*id)
                .is_some_and(|o| o.kind.has_shape())
        })
    }

    /// The mode keys: 1 Points, 2 Edges, 3 Polygons; the same key again
    /// goes back to the Model mode. Tab steps straight back ([`BACK_KEY`]).
    /// Not while typing (there Tab moves to the next field).
    pub(super) fn mode_keys(&mut self, ui: &Ui) {
        if ui.ctx().memory(|m| m.focused().is_some()) || self.objects.is_renaming() {
            return;
        }
        for (key, mode) in MODE_KEYS {
            if ui.input_mut(|i| i.consume_key(Modifiers::NONE, key)) {
                self.press_mode_key(mode);
            }
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, BACK_KEY)) {
            // egui would also move the keyboard focus to the next button,
            // which would then take every key: not here.
            ui.ctx()
                .memory_mut(|m| m.move_focus(egui::FocusDirection::None));
            self.step_back();
        }
    }

    /// 1, 2 or 3. With a modelling tool in use (Extrude, Slit, Weld...),
    /// the key puts it away and selects in that mode, staying in it if
    /// already there ([decision 0060](../../../../docs/decisions/0060-mode-keys-put-tools-away.md)):
    /// pressing the mode key is how the project lead goes back to
    /// selecting. Otherwise the key switches to its mode, or back to the
    /// Model mode when already in it.
    pub(crate) fn press_mode_key(&mut self, mode: Mode) {
        if self.tool.has_settings() {
            let tool = self.tool.name();
            // Finishes a Slit line too, keeping what it cut.
            self.set_tool(Tool::Select);
            self.mode = mode;
            self.note = Some(crate::trf!(
                "{} mode: selecting ({} put away; its key brings it back).",
                mode.name(),
                tool
            ));
            return;
        }
        self.mode = if self.mode == mode { Mode::Model } else { mode };
    }

    /// Tab: finishes whatever is going on and lands in the Model mode with
    /// the Select tool (or the Move, Rotate or Scale tool in use, which
    /// work on objects too), so a click selects objects again. A drag
    /// under way is left alone: let go (or Esc) first.
    pub(crate) fn step_back(&mut self) {
        let dragging = self.gizmo_session.is_some()
            || self.coordinates_session.is_some()
            || self.modelling_session.is_some()
            || self.loop_cut_session.is_some()
            || self.pair_session.is_some();
        if dragging {
            self.note = Some(tr("Let go of the drag (or press Esc) first, then Tab.").into());
            return;
        }
        let menu = self.tool_menu.is_open();
        self.tool_menu.close();
        self.target_pick = None;
        let tool = self.tool.has_settings();
        if tool {
            // Finishes a Slit line too, keeping what it cut.
            self.set_tool(Tool::Select);
        }
        let mode = self.mode != Mode::Model;
        self.mode = Mode::Model;
        self.note = Some(if mode || tool || menu {
            tr("Back in the Model mode: click an object to select it.").into()
        } else {
            tr(
                "Already in the Model mode: Tab steps back here from editing points, edges or \
             polygons, or from a modelling tool.",
            )
            .into()
        });
    }
}

impl Workbench {
    /// Edit pivot on or off (decision 0126): the Object mode is the Model
    /// mode with Edit pivot on. From an element mode, it goes to the Model
    /// mode with Edit pivot on.
    pub(crate) fn toggle_edit_pivot(&mut self) {
        if self.mode == Mode::Object {
            self.mode = Mode::Model;
            self.note = Some(tr("Edit pivot off: the gizmo moves the objects again.").into());
        } else {
            self.mode = Mode::Object;
            self.note = Some(
                tr(
                    "Edit pivot on: the gizmo moves and turns the pivots, not the shapes. \
                 Click Edit pivot again (or press Tab) to stop.",
                )
                .into(),
            );
        }
    }
}
