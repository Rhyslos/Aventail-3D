//! The workbench's part in command search (decision 0126): what can be
//! found, and carrying out what was chosen.

use super::Workbench;
use super::selection::SelectRequest;
use super::showing::Hiding;
use crate::command_search::{MenuAction, SEARCH_KEY, SearchAction, SearchEntry};
use crate::commands::ALL;
use crate::objects::DocRequest;
use crate::palette::Mode;
use crate::project::Pending;
use crate::tool_menu::Ready;
use crate::workbench::WorkbenchAction;
use egui::{Modifiers, Ui};
use scene::PrimitiveKind;

impl Workbench {
    /// Every entry the search can find, with whether it can be used now.
    pub(crate) fn search_entries(&self, ctx: &egui::Context) -> Vec<SearchEntry> {
        let keys = |k: egui::KeyboardShortcut| ctx.format_shortcut(&k);
        let mut out = Vec::new();
        for command in ALL {
            if command.is_group() {
                continue;
            }
            let info = command.info();
            let place = if command.modes().contains(&Mode::Uv) {
                "UV"
            } else if command.modes().contains(&Mode::Model) {
                "Object"
            } else {
                "Mesh"
            };
            out.push(SearchEntry {
                name: info.name.to_string(),
                place: place.to_string(),
                help: info.help.to_string(),
                keys: info.shortcut.map(keys),
                not_now: match self.readiness(command) {
                    Ready::Yes => None,
                    Ready::NotNow(why) => Some(why),
                    Ready::Later(when) => Some(when.to_string()),
                },
                action: SearchAction::Command(command),
            });
        }
        for mode in Mode::SHOWN {
            out.push(SearchEntry {
                name: format!("{} mode", mode.name()),
                place: "Modes".into(),
                help: mode.description().to_string(),
                keys: mode.key().map(|k| k.name().to_string()),
                not_now: None,
                action: SearchAction::Mode(mode),
            });
        }
        for action in MenuAction::ALL {
            let (name, place, help) = action.info();
            out.push(SearchEntry {
                name: name.into(),
                place: place.into(),
                help: help.into(),
                keys: action.shortcut().map(keys),
                not_now: None,
                action: SearchAction::Menu(action),
            });
        }
        for kind in PrimitiveKind::ALL {
            out.push(SearchEntry {
                name: kind.name().to_string(),
                place: "Create".into(),
                help: kind.description().to_string(),
                keys: None,
                not_now: None,
                action: SearchAction::Create(kind),
            });
        }
        out
    }

    /// Space opens the search (the project lead's choice), when nothing
    /// is being typed and no window or menu is open.
    pub(super) fn search_keys(&mut self, ui: &Ui, typing: bool) {
        if typing || self.search.open || self.tool_menu.is_open() || self.shape_picker.is_open() {
            return;
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, SEARCH_KEY)) {
            self.search.open();
        }
    }

    /// Draws the search while open and carries out what is chosen.
    pub(super) fn search_window(
        &mut self,
        ctx: &egui::Context,
        actions: &mut Vec<WorkbenchAction>,
        requests: &mut Vec<DocRequest>,
    ) {
        if !self.search.open {
            return;
        }
        let entries = self.search_entries(ctx);
        if let Some(action) = self.search.show(ctx, &entries) {
            self.run_search(action, ctx, actions, requests);
        }
    }

    /// Carries out a found entry, as its menu entry or key would.
    pub(crate) fn run_search(
        &mut self,
        action: SearchAction,
        ctx: &egui::Context,
        actions: &mut Vec<WorkbenchAction>,
        requests: &mut Vec<DocRequest>,
    ) {
        match action {
            SearchAction::Command(command) => self.run_command(command, ctx, requests),
            SearchAction::Mode(mode) => self.mode = mode,
            SearchAction::Create(kind) => requests.push(super::create(kind)),
            SearchAction::Menu(menu) => self.run_menu_action(menu, ctx, actions, requests),
        }
    }

    fn run_menu_action(
        &mut self,
        action: MenuAction,
        ctx: &egui::Context,
        actions: &mut Vec<WorkbenchAction>,
        requests: &mut Vec<DocRequest>,
    ) {
        match action {
            MenuAction::New => self.ask_then(Pending::New, actions),
            MenuAction::Open => self.ask_then(Pending::Open(None), actions),
            MenuAction::Save => actions.push(WorkbenchAction::Save {
                path: self.file.path.clone(),
            }),
            MenuAction::SaveAs => actions.push(WorkbenchAction::Save { path: None }),
            MenuAction::Import => actions.push(WorkbenchAction::Import { path: None }),
            MenuAction::Export => self.exporting.open = true,
            MenuAction::CheckForBaking => self.baking.open = true,
            MenuAction::Settings => self.settings.open = true,
            MenuAction::Keys => self.shortcuts.open = true,
            MenuAction::Manual => self.open_manual(ctx),
            MenuAction::Undo => requests.push(DocRequest::Undo),
            MenuAction::Redo => requests.push(DocRequest::Redo),
            MenuAction::SelectAll => self.select_requests.push(SelectRequest::All),
            MenuAction::SelectNothing => self.select_requests.push(SelectRequest::Clear),
            MenuAction::InvertSelection => self.select_requests.push(SelectRequest::Invert),
            MenuAction::SelectConnected => self.select_requests.push(SelectRequest::Connected),
            MenuAction::SelectSeams => {
                self.mode = Mode::Edges;
                self.select_requests.push(SelectRequest::Seams);
            }
            MenuAction::SelectHardEdges => {
                self.mode = Mode::Edges;
                self.select_requests.push(SelectRequest::HardEdges);
            }
            MenuAction::SelectFlipped => {
                self.mode = Mode::Polygons;
                self.select_requests.push(SelectRequest::Flipped);
            }
            MenuAction::Hide => self.hide(Hiding::Selection),
            MenuAction::Isolate => self.hide(Hiding::Others),
            MenuAction::ShowAll => self.show_everything(),
            MenuAction::UvEditor => self.toggle_uv_editor(),
            MenuAction::ShowHardEdges => {
                let options = self.viewport.options_mut();
                options.hide_hard_edges = !options.hide_hard_edges;
            }
            MenuAction::ShowSeams => {
                let options = self.viewport.options_mut();
                options.show_seams = !options.show_seams;
            }
            MenuAction::ShowNormals => {
                let options = self.viewport.options_mut();
                options.show_normals = !options.show_normals;
            }
            MenuAction::HighContrast => {
                let options = self.viewport.options_mut();
                options.high_contrast = !options.high_contrast;
            }
            MenuAction::EditPivot => self.toggle_edit_pivot(),
            MenuAction::Group => requests.push(DocRequest::Run(scene::Command::Create(
                scene::ObjectKind::Group,
            ))),
        }
    }

    /// Help → User manual: opens the manual in the web browser.
    pub(crate) fn open_manual(&mut self, ctx: &egui::Context) {
        ctx.open_url(egui::OpenUrl::new_tab(&self.info.manual_url));
        self.note = Some("The user manual opens in your web browser.".into());
    }

    /// Whether the search is open.
    #[cfg(test)]
    pub(crate) fn is_search_open(&self) -> bool {
        self.search.open
    }
}

#[cfg(test)]
#[path = "searching_tests.rs"]
mod tests;
