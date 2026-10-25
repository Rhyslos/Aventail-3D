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
use crate::text::tr;
use crate::tool_menu::Ready;
use crate::workbench::WorkbenchAction;
use egui::{Modifiers, Ui};

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
            // Quadify now and Weld by distance now are in the Mesh menu's
            // Clean up group (decisions 0150 and 0152), though they work in
            // the Model mode too; the game bevel in its Modifiers (0156).
            let place = if crate::booleans::kind_of(command).is_some()
                || crate::modifiers::kind_of(command).is_some()
                || command == crate::commands::CommandId::GameBevel
                || command == crate::commands::CommandId::QuadifyNow
                || command == crate::commands::CommandId::WeldByDistanceNow
            {
                tr("Mesh")
            } else if command == crate::commands::CommandId::SizeToKitGrid {
                tr("Create")
            } else if command.modes().contains(&Mode::Uv) {
                tr("UV")
            } else if command.modes() == [Mode::Object] {
                // The pivot commands are in the Tools menu.
                tr("Tools")
            } else if command.modes().contains(&Mode::Model) {
                tr("Object")
            } else {
                tr("Mesh")
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
                name: crate::trf!("{} mode", mode.name()),
                place: tr("Modes").into(),
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
                not_now: self.menu_not_now(action),
                action: SearchAction::Menu(action),
            });
        }
        for shape in crate::shape_picker::Shape::all() {
            out.push(SearchEntry {
                name: shape.name().to_string(),
                place: tr("Create").into(),
                help: shape.description().to_string(),
                keys: None,
                not_now: None,
                action: SearchAction::Create(shape),
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
            SearchAction::Create(shape) => self.create_shape(shape, requests),
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
            MenuAction::Glossary => self.open_glossary(ctx),
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
            MenuAction::ShowCollision => {
                let options = self.viewport.options_mut();
                options.show_collision = !options.show_collision;
            }
            MenuAction::ShowTriangles => {
                let options = self.viewport.options_mut();
                options.show_triangles = !options.show_triangles;
            }
            MenuAction::HighContrast => {
                let options = self.viewport.options_mut();
                options.high_contrast = !options.high_contrast;
            }
            MenuAction::EditPivot => self.toggle_edit_pivot(),
            MenuAction::ApplyMirror => self.apply_mirror(),
            MenuAction::MakeLods => self.make_lods(requests),
            MenuAction::Group => requests.push(DocRequest::Run(scene::Command::Create(
                scene::ObjectKind::Group,
            ))),
            MenuAction::KitSizedShapes => {
                let grid = self.document.scene().kit_grid();
                requests.push(DocRequest::Run(scene::Command::SetKitGrid(
                    scene::KitGrid {
                        sized_shapes: !grid.sized_shapes,
                        ..grid
                    },
                )));
            }
            MenuAction::SnapToKitGrid => {
                let snap = &mut self.viewport.options_mut().snap;
                snap.kit_grid = !snap.kit_grid;
            }
            MenuAction::MakesSweep => {
                self.curves_make(scene::CurveSolid::Sweep(scene::Sweep::default()), requests)
            }
            MenuAction::MakesLathe => self.curves_make(
                scene::CurveSolid::Lathe(scene::LatheShape::default()),
                requests,
            ),
        }
    }

    /// Why a menu entry found by the search can't be used now, if it
    /// can't.
    fn menu_not_now(&self, action: MenuAction) -> Option<String> {
        match action {
            MenuAction::MakesSweep | MenuAction::MakesLathe
                if self.selected_curves().is_empty() =>
            {
                Some(tr("Select a curve first").to_string())
            }
            _ => None,
        }
    }

    /// Sets what the selected curves make to `solid`'s kind (Attributes →
    /// Makes): those already making that kind keep their settings.
    fn curves_make(&mut self, solid: scene::CurveSolid, requests: &mut Vec<DocRequest>) {
        let scene = self.document.scene();
        let same = |id: &scene::ObjectId| {
            scene
                .object(*id)
                .and_then(|o| o.kind.curve())
                .is_some_and(|c| std::mem::discriminant(&c.solid) == std::mem::discriminant(&solid))
        };
        let curves = self.selected_curves();
        if curves.is_empty() {
            self.note = Some(crate::trf!("{}: select a curve first.", solid.name()));
            return;
        }
        let ids: Vec<scene::ObjectId> = curves.into_iter().filter(|id| !same(id)).collect();
        if ids.is_empty() {
            self.note = Some(crate::trf!(
                "The selected curves already make a {}.",
                solid.name().to_lowercase()
            ));
            return;
        }
        requests.push(DocRequest::Run(scene::Command::SetCurveSolid {
            ids,
            solid,
        }));
    }

    /// Make levels of detail (decision 0155): switches levels of detail on
    /// for the selected shapes, with the first one's settings.
    pub(super) fn make_lods(&mut self, requests: &mut Vec<DocRequest>) {
        let scene = self.document.scene();
        let ids: Vec<scene::ObjectId> = self
            .document
            .selection()
            .into_iter()
            .filter(|id| scene.object(*id).is_some_and(|o| o.kind.has_shape()))
            .collect();
        let Some(first) = ids.first().and_then(|id| scene.object(*id)) else {
            self.note = Some(tr("Make levels of detail: select a shape first.").into());
            return;
        };
        let lods = scene::Lods {
            on: true,
            ..first.lods.clone()
        };
        requests.push(DocRequest::Run(scene::Command::SetLods { ids, lods }));
        // Each level is set and made in the Levels panel (decision 0165).
        crate::layout::show_levels(&mut self.dock);
    }

    /// Help → User manual: opens the manual in the web browser.
    pub(crate) fn open_manual(&mut self, ctx: &egui::Context) {
        ctx.open_url(egui::OpenUrl::new_tab(&self.info.manual_url));
        self.note = Some(tr("The user manual opens in your web browser.").into());
    }

    /// Help → Words used in the app: the manual's glossary page, beside
    /// its table of contents.
    pub(crate) fn open_glossary(&mut self, ctx: &egui::Context) {
        let url = match self.info.manual_url.rsplit_once('/') {
            Some((folder, _)) => format!("{folder}/concepts.md"),
            None => self.info.manual_url.clone(),
        };
        ctx.open_url(egui::OpenUrl::new_tab(url));
        self.note = Some(tr("The glossary opens in your web browser.").into());
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
