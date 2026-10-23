//! The menu bar along the top of the window (File, Edit, Create, Modes,
//! Select, Tools, Window, Help; the Mesh and UV menus have files of their
//! own) and the status bar along the bottom.

use super::layouts::layout_tabs;
use super::{MIDDLE_MENUS, Workbench, WorkbenchAction, create, showing};
use crate::commands::CommandId;
use crate::components;
use crate::objects::{self, DocRequest};
use crate::palette::Mode;
use crate::panel::Panel;
use crate::project::{
    NEW_SHORTCUT, OPEN_SHORTCUT, Pending, SAVE_AS_SHORTCUT, SAVE_SHORTCUT, recent_label,
};
use crate::settings::SETTINGS_SHORTCUT;
use crate::theme;
use egui::{KeyboardShortcut, Ui};
use scene::{Command, ObjectKind, PrimitiveKind};

impl Workbench {
    pub(super) fn status_bar(&self, ui: &mut Ui) {
        ui.horizontal_centered(|ui| {
            let selected = match self.document.selection().len() {
                0 => String::new(),
                1 => " 1 object selected.".into(),
                n => format!(" {n} objects selected."),
            };
            let under = match (components::kind_for(self.mode), self.components.hovered()) {
                (Some(_), Some(element)) => format!(
                    " Under the mouse: {}.",
                    components::describe(&self.document, element)
                ),
                (Some(_), None) if !self.editing_anything() => format!(
                    " Select an object to see its {}.",
                    self.mode.name().to_lowercase()
                ),
                _ => String::new(),
            };
            let elements = self.selected_elements_text();
            let hidden = self.hidden_text();
            ui.label(format!(
                "{} mode: {}.{selected}{elements}{hidden}{under}",
                self.mode.name(),
                lower_first(self.mode.description())
            ));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let note = match self.note.as_deref() {
                    Some(note) => note.to_string(),
                    None => navigation_hint(self.viewport.options().navigation.scheme),
                };
                ui.label(egui::RichText::new(note).weak());
            });
        });
    }

    pub(super) fn menu_bar(
        &mut self,
        ui: &mut Ui,
        actions: &mut Vec<WorkbenchAction>,
        requests: &mut Vec<DocRequest>,
    ) {
        crate::dialog::menu(ui, "File", |ui| {
            let entry = |ui: &mut Ui, label: &str, keys: &KeyboardShortcut, help: &str| {
                let keys = ui.ctx().format_shortcut(keys);
                ui.add(egui::Button::new(label).shortcut_text(keys))
                    .on_hover_text(help)
                    .clicked()
            };
            if entry(ui, "New", &NEW_SHORTCUT, "Start a new, empty project") {
                self.ask_then(Pending::New, actions);
                ui.close();
            }
            if entry(ui, "Open…", &OPEN_SHORTCUT, "Open a saved project") {
                self.ask_then(Pending::Open(None), actions);
                ui.close();
            }
            let recent = self.file.recent.clone();
            ui.add_enabled_ui(!recent.is_empty(), |ui| {
                crate::dialog::menu(ui, "Open recent", |ui| {
                    for path in &recent {
                        if ui
                            .button(recent_label(path))
                            .on_hover_text(path.display().to_string())
                            .clicked()
                        {
                            self.ask_then(Pending::Open(Some(path.clone())), actions);
                            ui.close();
                        }
                    }
                    ui.separator();
                    if ui.button("Clear the list").clicked() {
                        self.file.recent.clear();
                        ui.close();
                    }
                })
            })
            .response
            .on_disabled_hover_text("Projects you save or open are listed here");
            ui.separator();
            if entry(ui, "Save", &SAVE_SHORTCUT, "Save the project") {
                actions.push(WorkbenchAction::Save {
                    path: self.file.path.clone(),
                });
                ui.close();
            }
            if entry(
                ui,
                "Save As…",
                &SAVE_AS_SHORTCUT,
                "Save the project under a new name or in another folder",
            ) {
                actions.push(WorkbenchAction::Save { path: None });
                ui.close();
            }
            ui.separator();
            self.export_entries(ui, actions);
            ui.separator();
            // No key for Quit, on purpose: Ctrl and Q are both used all the
            // time, and a slip shouldn't close the app.
            if ui.button("Quit").on_hover_text("Close the app").clicked() {
                self.ask_then(Pending::Quit, actions);
                ui.close();
            }
        });
        crate::dialog::menu(ui, "Edit", |ui| {
            objects::undo_entries(ui, &self.document, requests);
            ui.separator();
            objects::rename_delete_entries(ui, &mut self.document, &mut self.objects, requests);
            ui.separator();
            self.copy_entries(ui, requests);
            ui.separator();
            let keys = ui.ctx().format_shortcut(&SETTINGS_SHORTCUT);
            if ui
                .add(egui::Button::new("Settings…").shortcut_text(keys))
                .on_hover_text("Every preference in one place: look, units, navigation, undo")
                .clicked()
            {
                self.settings.open = true;
                ui.close();
            }
        });
        // Menu order as in DESIGN.md: File, Edit, Create, Modes, Select,
        // Tools, Mesh, UV, View, Window, Help.
        crate::dialog::menu(ui, "Create", |ui| {
            for kind in PrimitiveKind::ALL {
                // The basic shapes (B), then the rest (Alt + B).
                if kind == PrimitiveKind::Icosphere {
                    ui.separator();
                }
                if ui
                    .button(kind.name())
                    .on_hover_text(kind.description())
                    .clicked()
                {
                    requests.push(create(kind));
                    ui.close();
                }
            }
            ui.separator();
            if ui
                .button("Group")
                .on_hover_text("An empty object to hold and organise other objects")
                .clicked()
            {
                requests.push(DocRequest::Run(Command::Create(ObjectKind::Group)));
                ui.close();
            }
            ui.separator();
            ui.label(
                egui::RichText::new("Tip: B for the basic shapes, Alt + B for the rest")
                    .small()
                    .color(theme::TEXT_WEAK),
            );
        });
        crate::dialog::menu(ui, "Modes", |ui| {
            for mode in Mode::SHOWN {
                let current =
                    self.mode == mode || (mode == Mode::Model && self.mode == Mode::Object);
                let mut entry = egui::Button::selectable(current, mode.name());
                if let Some(key) = mode.key() {
                    entry = entry.shortcut_text(key.name());
                }
                if ui.add(entry).on_hover_text(mode.description()).clicked() {
                    self.mode = mode;
                    ui.close();
                }
            }
        });
        for title in MIDDLE_MENUS {
            crate::dialog::menu(ui, title, |ui| {
                if title == "Select" {
                    self.select_menu(ui);
                }
                if title == "Tools" {
                    self.tools_menu(ui);
                }
                if title == "UV" {
                    self.uv_menu_entries(ui, requests);
                }
                if title == "Mesh" {
                    self.mesh_menu(ui, requests);
                }
            });
        }
        crate::dialog::menu(ui, "View", |ui| {
            if ui
                .button("Reset view")
                .on_hover_text("Go back to the opening camera view")
                .clicked()
            {
                self.viewport.reset_view();
                ui.close();
            }
            crate::viewport::chrome::frame_entries(ui, &mut self.viewport);
            ui.separator();
            crate::dialog::menu(ui, SHOW_HIDE_MENU, |ui| {
                let entry = |ui: &mut Ui, label: &str, keys: &KeyboardShortcut, help: &str| {
                    let keys = ui.ctx().format_shortcut(keys);
                    ui.add(egui::Button::new(label).shortcut_text(keys))
                        .on_hover_text(help)
                        .clicked()
                };
                if entry(
                    ui,
                    "Hide the selection",
                    &showing::HIDE_KEYS,
                    "Hide the selected objects (or the selected polygons while editing)",
                ) {
                    self.hide(showing::Hiding::Selection);
                    ui.close();
                }
                if entry(
                    ui,
                    "Isolate the selection",
                    &showing::ISOLATE_KEYS,
                    "Hide everything except the selection",
                ) {
                    self.hide(showing::Hiding::Others);
                    ui.close();
                }
                if entry(
                    ui,
                    "Show everything",
                    &showing::SHOW_ALL_KEYS,
                    "Show all that's hidden",
                ) {
                    self.show_everything();
                    ui.close();
                }
            });
            ui.separator();
            crate::viewport::chrome::grid_entries(ui, &mut self.viewport.options_mut().grid_detail);
            ui.separator();
            if ui.button("Reset panel layout").clicked() {
                self.reset_layout();
                ui.close();
            }
        });
        crate::dialog::menu(ui, "Window", |ui| {
            let shown = self.dock.find_tab(&Panel::UvEditor).is_some();
            if ui
                .add(egui::Button::selectable(shown, UV_EDITOR_ENTRY))
                .on_hover_text(
                    "Show the UV Editor beside the viewport, or put it away. The UV Edit layout \
                     (top right) always has it.",
                )
                .clicked()
            {
                self.toggle_uv_editor();
                ui.close();
            }
            let layers_shown = self.dock.find_tab(&Panel::EditLayers).is_some();
            if ui
                .add(egui::Button::selectable(layers_shown, EDIT_LAYERS_ENTRY))
                .on_hover_text(
                    "Show the Edit layers panel beside Objects, or put it away. It shows by \
                     itself once an object has more than one layer.",
                )
                .clicked()
            {
                match self.dock.find_tab(&Panel::EditLayers) {
                    Some(path) => {
                        self.dock.remove_tab(path);
                    }
                    None => crate::layout::show_tab(&mut self.dock, Panel::EditLayers),
                }
                ui.close();
            }
        });
        crate::dialog::menu(ui, "Help", |ui| {
            if ui
                .button(MANUAL_ENTRY)
                .on_hover_text("Open the user manual in your web browser")
                .clicked()
            {
                self.open_manual(ui.ctx());
                ui.close();
            }
            if ui
                .button(GLOSSARY_ENTRY)
                .on_hover_text(
                    "What the app's words mean: points, pieces, seams, pivots, smoothing and the \
                     rest, each explained in plain words, in your web browser",
                )
                .clicked()
            {
                self.open_glossary(ui.ctx());
                ui.close();
            }
            if ui
                .add(
                    egui::Button::new(SEARCH_ENTRY)
                        .shortcut_text(crate::command_search::SEARCH_KEY.name()),
                )
                .on_hover_text("Find any command by typing part of its name")
                .clicked()
            {
                self.search.open();
                ui.close();
            }
            if ui
                .button("Keys")
                .on_hover_text(
                    "Every key on a drawn keyboard (also the Keys button on the menu bar)",
                )
                .clicked()
            {
                self.shortcuts.open = true;
                ui.close();
            }
            ui.separator();
            crate::dialog::menu(ui, DEVELOPER_MENU, |ui| {
                if ui
                    .button("Add a heavy test shape")
                    .on_hover_text(
                        "Adds a bumpy ball of 3 million polygons, like a detailed model you \
                         might import, to see how your computer copes with heavy shapes. Undo \
                         takes it away again.",
                    )
                    .clicked()
                {
                    self.add_heavy_test_shape();
                    ui.close();
                }
            });
            ui.separator();
            if ui.button(format!("About {}", self.info.name)).clicked() {
                self.about_open = true;
                ui.close();
            }
        });
        self.keys_button(ui);
        let unsaved = self.has_unsaved_changes();
        if let Some(choice) = layout_tabs(ui, &self.file.name(), unsaved, self.layout_choice) {
            self.switch_layout(choice);
        }
    }
}

/// `text` with its first letter small, for the middle of a sentence
/// (names such as UV keep their capitals).
fn lower_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Help → User manual.
pub(crate) const MANUAL_ENTRY: &str = "User manual";

/// Help → Words used in the app: the manual's glossary (decision 0126).
pub(crate) const GLOSSARY_ENTRY: &str = "Words used in the app";

/// Help → Search for a command.
pub(crate) const SEARCH_ENTRY: &str = "Search for a command";

/// Help → Developer: tools for checking the app itself (decision 0126).
pub(crate) const DEVELOPER_MENU: &str = "Developer";

/// View → Show and hide (decision 0126).
pub(crate) const SHOW_HIDE_MENU: &str = "Show and hide";

/// The Window menu's entry for the Edit layers panel.
pub(crate) const EDIT_LAYERS_ENTRY: &str = "Edit layers";

/// The Window menu's entry for the UV Editor.
pub(crate) const UV_EDITOR_ENTRY: &str = "UV Editor";

/// The status bar's standing hint: the navigation controls in use.
pub fn navigation_hint(scheme: tools::Scheme) -> String {
    format!("{}: {}", scheme.name(), scheme.summary())
}

/// Menu entries for commands from the command list, each with its key;
/// one not built yet is greyed out, saying when it arrives. Returns the
/// command clicked, if any.
pub(super) fn commands_entries(ui: &mut Ui, list: &[CommandId]) -> Option<CommandId> {
    let mut chosen = None;
    for command in list {
        let info = command.info();
        let mut button = egui::Button::new(info.name);
        if let Some(keys) = info.shortcut {
            button = button.shortcut_text(ui.ctx().format_shortcut(&keys));
        }
        let built = info.arrives.is_none();
        let response = ui.add_enabled(built, button);
        match info.arrives {
            Some(when) => {
                response.on_disabled_hover_text(format!("{}\n{when}", info.help));
            }
            None => {
                if response.on_hover_text(info.help).clicked() {
                    chosen = Some(*command);
                }
            }
        }
    }
    chosen
}
