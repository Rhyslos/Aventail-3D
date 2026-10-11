//! The workbench's part in selecting: the select keys and the Select
//! menu, and carrying out what the viewport and keys asked for once the
//! frame's scene drawing is known ([decision 0029](../../../../docs/decisions/0029-selection.md)).

use super::Workbench;
use crate::components::kind_for;
use crate::drawing::SceneDrawing;
use crate::selecting::{
    INVERT_SHORTCUT, SELECT_ALL_SHORTCUT, SELECT_TOOL_KEY, SelectTool, Selector, count,
};
use egui::{Modifiers, Ui};

/// A selection command from a key or the Select menu, carried out once
/// the frame's scene drawing is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SelectRequest {
    /// Select everything.
    All,
    /// Invert the selection.
    Invert,
    /// Unselect the edited objects' elements (Esc in an element mode).
    Clear,
    /// Grow (positive) or shrink (negative) by this many steps.
    Grow(i32),
}

/// The Select menu's tooltip for the visible-only switch, and the mode
/// palette's.
pub(crate) const VISIBLE_ONLY_HELP: &str = "When on, the box, lasso and brush only select what you can see. \
     When off (the standard), they select through the shape, catching its far side too.";

impl Workbench {
    /// `Q` steps through the select tools; Ctrl + A selects all and
    /// Ctrl + I inverts. Not while typing.
    pub(super) fn select_keys(&mut self, ui: &Ui) {
        if ui.ctx().memory(|m| m.focused().is_some()) || self.objects.is_renaming() {
            return;
        }
        let (next, all, invert) = ui.input_mut(|i| {
            (
                i.consume_key(Modifiers::NONE, SELECT_TOOL_KEY),
                i.consume_shortcut(&SELECT_ALL_SHORTCUT),
                i.consume_shortcut(&INVERT_SHORTCUT),
            )
        });
        if next {
            self.next_select_tool();
        }
        if all {
            self.select_requests.push(SelectRequest::All);
        }
        if invert {
            self.select_requests.push(SelectRequest::Invert);
        }
    }

    /// Switches to the next select tool and says so in the status bar.
    pub(super) fn next_select_tool(&mut self) {
        let tool = self.viewport.select_tool().next();
        self.set_select_tool(tool);
    }

    fn set_select_tool(&mut self, tool: SelectTool) {
        self.viewport.set_select_tool(tool);
        self.note = Some(format!("{}: {}.", tool.name(), tool.description()));
    }

    /// The Select menu.
    pub(super) fn select_menu(&mut self, ui: &mut Ui) {
        let keys = |ui: &Ui, shortcut| ui.ctx().format_shortcut(shortcut);
        let entry = |ui: &mut Ui, label: &str, keys: String, help: &str| {
            ui.add(egui::Button::new(label).shortcut_text(keys))
                .on_hover_text(help)
                .clicked()
        };
        let all_keys = keys(ui, &SELECT_ALL_SHORTCUT);
        if entry(
            ui,
            "Select all",
            all_keys,
            "Select every object, or in the Points, Edges and Polygons modes every element of the selected objects",
        ) {
            self.select_requests.push(SelectRequest::All);
            ui.close();
        }
        if entry(
            ui,
            "Deselect",
            "Esc".into(),
            "Unselect the points, edges or polygons; in the Model mode, the objects",
        ) {
            self.select_requests.push(SelectRequest::Clear);
            ui.close();
        }
        let invert_keys = keys(ui, &INVERT_SHORTCUT);
        if entry(
            ui,
            "Invert",
            invert_keys,
            "Select what isn't selected, and unselect what is",
        ) {
            self.select_requests.push(SelectRequest::Invert);
            ui.close();
        }
        ui.separator();
        let modifier = wheel_keys(ui);
        let wheel = |direction: &str| format!("{modifier}{direction}");
        if entry(
            ui,
            "Grow selection",
            wheel("Wheel up"),
            "Add every element touching the selection",
        ) {
            self.select_requests.push(SelectRequest::Grow(1));
            ui.close();
        }
        if entry(
            ui,
            "Shrink selection",
            wheel("Wheel down"),
            "Take away the selection's outer rim",
        ) {
            self.select_requests.push(SelectRequest::Grow(-1));
            ui.close();
        }
        ui.add_enabled(false, egui::Button::new("Loop: double-click"))
            .on_disabled_hover_text(
                "Double-click an edge for its loop; Shift + double-click for its ring",
            );
        ui.separator();
        let current = self.viewport.select_tool();
        for tool in SelectTool::ALL {
            let mut button = egui::Button::selectable(tool == current, tool.name());
            if tool == current.next() {
                button = button.shortcut_text(SELECT_TOOL_KEY.name());
            }
            if ui.add(button).on_hover_text(tool.description()).clicked() {
                self.set_select_tool(tool);
                ui.close();
            }
        }
        ui.separator();
        let options = self.viewport.options_mut();
        ui.checkbox(&mut options.select_visible_only, "Select only visible")
            .on_hover_text(VISIBLE_ONLY_HELP);
    }

    /// Carries out the selection gestures and requests of this frame,
    /// given what is drawn.
    pub(super) fn carry_out_selection(&mut self, drawing: &SceneDrawing) {
        let gestures = self.viewport.take_gestures();
        let requests = std::mem::take(&mut self.select_requests);
        if gestures.is_empty() && requests.is_empty() {
            return;
        }
        let mut selector = Selector {
            document: &mut self.document,
            drawing,
            scene: self.components.pick_scene(drawing),
            mode: self.mode,
            visible_only: self.viewport.options().select_visible_only,
        };
        let mut notes = Vec::new();
        for gesture in gestures {
            notes.push(selector.gesture(gesture));
        }
        for request in requests {
            notes.push(match request {
                SelectRequest::All => selector.select_all(),
                SelectRequest::Invert => selector.invert(),
                SelectRequest::Grow(steps) => {
                    selector.gesture(crate::selecting::Gesture::Grow(steps))
                }
                SelectRequest::Clear => {
                    if !selector.clear_elements() {
                        selector.document.clear_selection();
                    }
                    None
                }
            });
        }
        if let Some(note) = notes.into_iter().flatten().last() {
            self.note = Some(note);
        }
    }

    /// The selected elements in words for the status bar, in an element
    /// mode: " 4 points selected." (nothing when none are).
    pub(super) fn selected_elements_text(&self) -> String {
        let Some(kind) = kind_for(self.mode) else {
            return String::new();
        };
        let total: usize = self
            .document
            .selection()
            .iter()
            .filter_map(|id| self.document.elements(*id))
            .map(|s| s.count(kind))
            .sum();
        if total == 0 {
            String::new()
        } else {
            format!(" {} selected.", count(total, kind))
        }
    }
}

/// "Ctrl+" (or the Mac's Command sign and a plus), for the wheel entries.
fn wheel_keys(ui: &Ui) -> String {
    let shortcut = egui::KeyboardShortcut::new(Modifiers::COMMAND, egui::Key::A);
    let text = ui.ctx().format_shortcut(&shortcut);
    let modifier = text.strip_suffix('A').unwrap_or(&text).trim_end();
    if modifier.ends_with('+') {
        modifier.to_string()
    } else {
        format!("{modifier}+")
    }
}
