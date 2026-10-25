//! The menu bar along the top of the window (File, Edit, Create, Modes,
//! Select, Tools, Window, Help; the Mesh and UV menus have files of their
//! own) and the status bar along the bottom.

use super::layouts::layout_tabs;
use super::{MIDDLE_MENUS, Workbench, WorkbenchAction, showing};
use crate::commands::CommandId;
use crate::components;
use crate::objects::{self, DocRequest};
use crate::palette::Mode;
use crate::panel::Panel;
use crate::project::{
    NEW_SHORTCUT, OPEN_SHORTCUT, Pending, SAVE_AS_SHORTCUT, SAVE_SHORTCUT, recent_label,
};
use crate::settings::SETTINGS_SHORTCUT;
use crate::shape_picker::Shape;
use crate::text::tr;
use crate::theme;
use egui::{KeyboardShortcut, Ui};
use scene::{Command, ObjectKind, PrimitiveKind};

impl Workbench {
    pub(super) fn status_bar(&self, ui: &mut Ui) {
        ui.horizontal_centered(|ui| {
            let selected = match self.document.selection().len() {
                0 => String::new(),
                1 => tr(" 1 object selected.").into(),
                n => crate::trf!(" {} objects selected.", n),
            };
            let under = match (components::kind_for(self.mode), self.components.hovered()) {
                (Some(_), Some(element)) => crate::trf!(
                    " Under the mouse: {}.",
                    components::describe(&self.document, element)
                ),
                (Some(_), None) if !self.editing_anything() => crate::trf!(
                    " Select an object to see its {}.",
                    self.mode.name().to_lowercase()
                ),
                _ => String::new(),
            };
            let elements = self.selected_elements_text();
            let hidden = self.hidden_text();
            let left = crate::trf!(
                "{} mode: {}.{}{}{}{}",
                self.mode.name(),
                lower_first(self.mode.description()),
                selected,
                elements,
                hidden,
                under
            );
            let note = match self.note.as_deref() {
                Some(note) => note.to_string(),
                None => navigation_hint(self.viewport.options().navigation.scheme),
            };
            super::status_line::two_sides(ui, left, note, self.file_work().is_some());
        });
    }

    pub(super) fn menu_bar(
        &mut self,
        ui: &mut Ui,
        actions: &mut Vec<WorkbenchAction>,
        requests: &mut Vec<DocRequest>,
    ) {
        crate::dialog::menu(ui, tr("File"), |ui| {
            let entry = |ui: &mut Ui, label: &str, keys: &KeyboardShortcut, help: &str| {
                let keys = ui.ctx().format_shortcut(keys);
                ui.add(egui::Button::new(label).shortcut_text(keys))
                    .on_hover_text(help)
                    .clicked()
            };
            if entry(
                ui,
                tr("New"),
                &NEW_SHORTCUT,
                tr("Start a new, empty project"),
            ) {
                self.ask_then(Pending::New, actions);
                ui.close();
            }
            if entry(ui, tr("Open…"), &OPEN_SHORTCUT, tr("Open a saved project")) {
                self.ask_then(Pending::Open(None), actions);
                ui.close();
            }
            let recent = self.file.recent.clone();
            ui.add_enabled_ui(!recent.is_empty(), |ui| {
                crate::dialog::menu(ui, tr("Open recent"), |ui| {
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
                    if ui.button(tr("Clear the list")).clicked() {
                        self.file.recent.clear();
                        ui.close();
                    }
                })
            })
            .response
            .on_disabled_hover_text(tr("Projects you save or open are listed here"));
            ui.separator();
            if entry(ui, tr("Save"), &SAVE_SHORTCUT, tr("Save the project")) {
                actions.push(WorkbenchAction::Save {
                    path: self.file.path.clone(),
                });
                ui.close();
            }
            if entry(
                ui,
                tr("Save As…"),
                &SAVE_AS_SHORTCUT,
                tr("Save the project under a new name or in another folder"),
            ) {
                actions.push(WorkbenchAction::Save { path: None });
                ui.close();
            }
            ui.separator();
            self.export_entries(ui, actions);
            ui.separator();
            // No key for Quit, on purpose: Ctrl and Q are both used all the
            // time, and a slip shouldn't close the app.
            if ui
                .button(tr("Quit"))
                .on_hover_text(tr("Close the app"))
                .clicked()
            {
                self.ask_then(Pending::Quit, actions);
                ui.close();
            }
        });
        crate::dialog::menu(ui, tr("Edit"), |ui| {
            objects::undo_entries(ui, &self.document, requests);
            ui.separator();
            objects::rename_delete_entries(ui, &mut self.document, &mut self.objects, requests);
            ui.separator();
            self.copy_entries(ui, requests);
            ui.separator();
            let keys = ui.ctx().format_shortcut(&SETTINGS_SHORTCUT);
            if ui
                .add(egui::Button::new(tr("Settings…")).shortcut_text(keys))
                .on_hover_text(tr(
                    "Every preference in one place: look, units, navigation, undo",
                ))
                .clicked()
            {
                self.settings.open = true;
                ui.close();
            }
        });
        // Menu order as in DESIGN.md: File, Edit, Create, Modes, Select,
        // Tools, Mesh, UV, View, Window, Help.
        crate::dialog::menu(ui, tr("Create"), |ui| {
            for shape in Shape::all() {
                // The basic shapes (B), then the rest (Alt + B), the curves
                // last (decision 0138).
                if matches!(
                    shape,
                    Shape::Primitive(PrimitiveKind::Icosphere) | Shape::DrawCurve
                ) {
                    ui.separator();
                }
                if ui
                    .button(shape.name())
                    .on_hover_text(shape.description())
                    .clicked()
                {
                    self.create_shape(shape, requests);
                    ui.close();
                }
            }
            ui.separator();
            if ui
                .button(tr("Group"))
                .on_hover_text(tr("An empty object to hold and organise other objects"))
                .clicked()
            {
                requests.push(DocRequest::Run(Command::Create(ObjectKind::Group)));
                ui.close();
            }
            // The kit helpers in a list of their own, so the Create menu
            // stays short (decision 0160). It stays open while its
            // settings are changed.
            egui::containers::menu::SubMenuButton::new(tr(crate::kit::KIT_MENU))
                .config(
                    crate::dialog::menu_config()
                        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside),
                )
                .ui(ui, |ui| {
                    ui.set_min_width(crate::dialog::MENU_WIDTH);
                    self.kit_menu(ui, requests);
                });
            ui.separator();
            ui.label(
                egui::RichText::new(tr(
                    "Tip: B for the basic shapes, Alt + B for the rest and the curves",
                ))
                .small()
                .color(theme::TEXT_WEAK),
            );
        });
        crate::dialog::menu(ui, tr("Modes"), |ui| {
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
            crate::dialog::menu(ui, tr(title), |ui| {
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
        crate::dialog::menu(ui, tr("View"), |ui| {
            if ui
                .button(tr("Reset view"))
                .on_hover_text(tr("Go back to the opening camera view"))
                .clicked()
            {
                self.viewport.reset_view();
                ui.close();
            }
            crate::viewport::chrome::frame_entries(ui, &mut self.viewport);
            ui.separator();
            crate::dialog::menu(ui, tr(SHOW_HIDE_MENU), |ui| {
                let entry = |ui: &mut Ui, label: &str, keys: &KeyboardShortcut, help: &str| {
                    let keys = ui.ctx().format_shortcut(keys);
                    ui.add(egui::Button::new(label).shortcut_text(keys))
                        .on_hover_text(help)
                        .clicked()
                };
                if entry(
                    ui,
                    tr("Hide the selection"),
                    &showing::HIDE_KEYS,
                    tr("Hide the selected objects (or the selected polygons while editing)"),
                ) {
                    self.hide(showing::Hiding::Selection);
                    ui.close();
                }
                if entry(
                    ui,
                    tr("Isolate the selection"),
                    &showing::ISOLATE_KEYS,
                    tr("Hide everything except the selection"),
                ) {
                    self.hide(showing::Hiding::Others);
                    ui.close();
                }
                if entry(
                    ui,
                    tr("Show everything"),
                    &showing::SHOW_ALL_KEYS,
                    tr("Show all that's hidden"),
                ) {
                    self.show_everything();
                    ui.close();
                }
            });
            ui.separator();
            crate::viewport::chrome::grid_entries(ui, &mut self.viewport.options_mut().grid_detail);
            ui.separator();
            if ui.button(tr("Reset panel layout")).clicked() {
                self.reset_layout();
                ui.close();
            }
        });
        crate::dialog::menu(ui, tr("Window"), |ui| {
            let shown = self.dock.find_tab(&Panel::UvEditor).is_some();
            if ui
                .add(egui::Button::selectable(shown, tr(UV_EDITOR_ENTRY)))
                .on_hover_text(tr(
                    "Show the UV Editor beside the viewport, or put it away. The UV Edit layout \
                     (top right) always has it.",
                ))
                .clicked()
            {
                self.toggle_uv_editor();
                ui.close();
            }
            let layers_shown = self.dock.find_tab(&Panel::EditLayers).is_some();
            if ui
                .add(egui::Button::selectable(
                    layers_shown,
                    tr(EDIT_LAYERS_ENTRY),
                ))
                .on_hover_text(tr(
                    "Show the Edit layers panel beside Objects, or put it away. It shows by \
                     itself once an object has more than one layer.",
                ))
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
            let levels_shown = self.dock.find_tab(&Panel::Levels).is_some();
            if ui
                .add(egui::Button::selectable(levels_shown, tr(LEVELS_ENTRY)))
                .on_hover_text(tr(
                    "Show the Levels panel beside Attributes, or put it away: the selected                      shape's levels of detail and how each is made. It shows by itself when a                      shape with levels is selected.",
                ))
                .clicked()
            {
                match self.dock.find_tab(&Panel::Levels) {
                    Some(path) => {
                        self.dock.remove_tab(path);
                    }
                    None => crate::layout::show_levels(&mut self.dock),
                }
                ui.close();
            }
        });
        crate::dialog::menu(ui, tr("Help"), |ui| {
            if ui
                .button(tr(MANUAL_ENTRY))
                .on_hover_text(tr("Open the user manual in your web browser"))
                .clicked()
            {
                self.open_manual(ui.ctx());
                ui.close();
            }
            if ui
                .button(tr(GLOSSARY_ENTRY))
                .on_hover_text(tr(
                    "What the app's words mean: points, pieces, seams, pivots, smoothing and the \
                     rest, each explained in plain words, in your web browser",
                ))
                .clicked()
            {
                self.open_glossary(ui.ctx());
                ui.close();
            }
            if ui
                .add(
                    egui::Button::new(tr(SEARCH_ENTRY))
                        .shortcut_text(crate::command_search::SEARCH_KEY.name()),
                )
                .on_hover_text(tr("Find any command by typing part of its name"))
                .clicked()
            {
                self.search.open();
                ui.close();
            }
            if ui
                .button(tr("Keys"))
                .on_hover_text(tr(
                    "Every key on a drawn keyboard (also the Keys button on the menu bar)",
                ))
                .clicked()
            {
                self.shortcuts.open = true;
                ui.close();
            }
            ui.separator();
            crate::dialog::menu(ui, tr(DEVELOPER_MENU), |ui| {
                if ui
                    .button(tr("Add a heavy test shape"))
                    .on_hover_text(tr(
                        "Adds a bumpy ball of 3 million polygons, like a detailed model you \
                         might import, to see how your computer copes with heavy shapes. Undo \
                         takes it away again.",
                    ))
                    .clicked()
                {
                    self.add_heavy_test_shape();
                    ui.close();
                }
            });
            ui.separator();
            if ui.button(crate::trf!("About {}", self.info.name)).clicked() {
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
pub(crate) const MANUAL_ENTRY: &str = crate::text!("User manual");

/// Help → Words used in the app: the manual's glossary (decision 0126).
pub(crate) const GLOSSARY_ENTRY: &str = crate::text!("Words used in the app");

/// Help → Search for a command.
pub(crate) const SEARCH_ENTRY: &str = crate::text!("Search for a command");

/// Help → Developer: tools for checking the app itself (decision 0126).
pub(crate) const DEVELOPER_MENU: &str = crate::text!("Developer");

/// View → Show and hide (decision 0126).
pub(crate) const SHOW_HIDE_MENU: &str = crate::text!("Show and hide");

/// The Window menu's entry for the Edit layers panel.
pub(crate) const EDIT_LAYERS_ENTRY: &str = crate::text!("Edit layers");

/// The Window menu's entry for the Levels panel (decision 0165).
pub(crate) const LEVELS_ENTRY: &str = crate::text!("Levels");

/// The Window menu's entry for the UV Editor.
pub(crate) const UV_EDITOR_ENTRY: &str = crate::text!("UV Editor");

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
