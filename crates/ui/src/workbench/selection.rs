//! The workbench's part in selecting: the select keys and the Select
//! menu, and carrying out what the viewport and keys asked for once the
//! frame's scene drawing is known ([decision 0029](../../../../docs/decisions/0029-selection.md)).

use super::Workbench;
use crate::components::kind_for;
use crate::drawing::SceneDrawing;
use crate::palette::Mode;
use crate::selecting::{
    INVERT_SHORTCUT, SELECT_ALL_SHORTCUT, SELECT_TOOL_KEY, SelectTool, Selector, count,
};
use egui::{Modifiers, Ui};
use scene::ElementKind;

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
    /// Select the polygons facing the wrong way.
    Flipped,
    /// Add the polygons facing like the selected ones.
    SimilarFacing,
    /// Select the hard edges.
    HardEdges,
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
            if self.tool == crate::transforming::Tool::Select {
                self.next_select_tool();
            } else {
                // Back to selecting, with the select tool used before.
                self.set_tool(crate::transforming::Tool::Select);
            }
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
        let polygons = |this: &mut Self, request| {
            // These select polygons, so they show the Polygons mode.
            this.mode = Mode::Polygons;
            this.select_requests.push(request);
        };
        if entry(
            ui,
            "Select flipped",
            String::new(),
            "Select the polygons facing the wrong way round: inside out compared with \
             their neighbours, or facing into a closed shape",
        ) {
            polygons(self, SelectRequest::Flipped);
            ui.close();
        }
        if entry(
            ui,
            "Select same facing",
            String::new(),
            "Add every polygon facing within 30° of a selected polygon: the top of a shape, \
             say, or every wall facing one way",
        ) {
            polygons(self, SelectRequest::SimilarFacing);
            ui.close();
        }
        if entry(
            ui,
            "Select hard edges",
            String::new(),
            "Select every hard edge: where the shading breaks into a crisp edge. Shows the \
             Edges mode.",
        ) {
            self.mode = Mode::Edges;
            self.select_requests.push(SelectRequest::HardEdges);
            ui.close();
        }
        ui.add_enabled(false, egui::Button::new("Loop and ring: double-click"))
            .on_disabled_hover_text(
                "Double-click an edge for its loop; Alt + double-click for its ring",
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
        let gestures = self.take_pivot_target_click(gestures, drawing);
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
            stroke: &mut self.stroke_memory,
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
                SelectRequest::Flipped => selector.select_flipped(),
                SelectRequest::HardEdges => selector.select_hard_edges(),
                SelectRequest::SimilarFacing => {
                    selector.select_similar_facing(super::cleaning::SIMILAR_FACING_ANGLE)
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
            .filter_map(|id| {
                let selected = self.document.elements(*id)?;
                let all = selected.count(kind);
                // Only what is still there: a deleted polygon stays in
                // the selection (so undo brings it back selected). Checked
                // only for selections small enough to check every frame.
                if all > STILL_THERE_CHECK {
                    return Some(all);
                }
                let mesh = self.document.scene().evaluated(*id)?.mesh;
                Some(match kind {
                    ElementKind::Points => selected
                        .points()
                        .filter(|v| mesh.position(*v).is_some())
                        .count(),
                    ElementKind::Edges => selected
                        .edges()
                        .filter(|e| mesh.topology().has_edge(*e))
                        .count(),
                    ElementKind::Polygons => selected
                        .polygons()
                        .filter(|f| mesh.face(*f).is_some())
                        .count(),
                })
            })
            .sum();
        if total == 0 {
            String::new()
        } else {
            format!(" {} selected.", count(total, kind))
        }
    }
}

/// Up to how many selected elements the status bar checks they are all
/// still there.
const STILL_THERE_CHECK: usize = 10_000;

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
