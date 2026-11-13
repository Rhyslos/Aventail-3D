//! The running app's frame: the tabs' window drawn, what the person asked
//! for carried out (each action for the tab it came from, decision
//! 0266), background work reported back, and the settings saved.

use super::{
    ACCENT_KEY, AUTOSAVE_KEY, App, BRUSHES_KEY, EXPORT_KEY, LAYOUT_KEY, LIBRARY_GRAPHS_KEY,
    MATERIAL_LAYOUT_KEY, OWN_BOXES_KEY, PAINT_LAYOUT_KEY, RECENT_KEY, RECIPES_KEY, TOOL_MENU_KEY,
    UNDO_MEMORY_KEY, UNITS_KEY, USED_COLOURS_KEY, UV_LAYOUT_KEY, UV_SETTINGS_KEY, VIEW_KEY,
};
use crate::file_work::AfterWork;
use crate::project;
use crate::updates::{self, PendingInstall};
use eframe::egui;
use std::time::{Duration, Instant};
use ui::{FilePurpose, TabId, Workbench, WorkbenchAction};

impl App {
    /// One frame of the running app.
    pub(super) fn frame(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        crate::platform::keep_keyboard_focus(frame);
        let ctx = ui.ctx().clone();
        // Tabs opened or closed since last frame get or lose their parts.
        self.parts.follow(&mut self.tabs);
        // The window's close button: wait for a save or export under way,
        // and ask about unsaved changes (in every tab) first.
        if ctx.input(|i| i.viewport().close_requested()) && !self.closing {
            if self.files.busy() {
                let workbench = self.tabs.get_or_active_mut(self.files_tab);
                self.files.close_when_done(workbench, false);
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            } else if !self.tabs.request_close() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
        }
        let pen = self.tablet.poll(frame);
        self.tabs.active_mut().set_tablet_pen(pen);
        let active = self.tabs.active_id();
        let actions = match self.parts.get_mut(active) {
            Some(parts) => self.tabs.show(ui, parts.renderer.as_mut()),
            None => Vec::new(),
        };
        // What the app did (opened, restored, ...) shows on the next frame.
        if !actions.is_empty() {
            ctx.request_repaint();
        }
        for (tab, action) in actions {
            self.carry_out(tab, action, &ctx);
        }
        // A tab opened by what was carried out gets its parts now, so its
        // first frame has its renderer.
        self.parts.follow(&mut self.tabs);
        self.sleep_tabs(&ctx);
        self.background_work(&ctx);
        self.remember_recent(frame);
        let title = self.tabs.title();
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }

    /// Carries out `action`, asked for in the tab `tab`.
    fn carry_out(&mut self, tab: TabId, action: WorkbenchAction, ctx: &egui::Context) {
        if let WorkbenchAction::OpenInNewTab { path } = action {
            let Some(from) = self.tabs.get_mut(tab) else {
                return;
            };
            if let Some(path) = project::which_to_open(from, path, FilePurpose::OpenProjectInNewTab)
            {
                project::open_in_new_tab(&mut self.tabs, tab, path);
            }
            return;
        }
        if matches!(
            action,
            WorkbenchAction::Save { .. }
                | WorkbenchAction::Export(_)
                | WorkbenchAction::ExportKit(_)
                | WorkbenchAction::ExportToTarget(_)
        ) && !self.files.busy()
        {
            self.files_tab = Some(tab);
        }
        if matches!(action, WorkbenchAction::Quit) {
            if self.files.busy() {
                let workbench = self.tabs.get_or_active_mut(self.files_tab);
                self.files.close_when_done(workbench, true);
            } else {
                self.closing = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            return;
        }
        let Some(workbench) = self.tabs.get_mut(tab) else {
            return;
        };
        match action {
            WorkbenchAction::Save { path } => {
                if let Some(path) = project::where_to_save(workbench, path) {
                    self.files.save(workbench, path);
                }
            }
            WorkbenchAction::Open { path } => project::open(workbench, path),
            WorkbenchAction::Autosave { document, keep } => {
                if let Some(parts) = self.parts.get_mut(tab) {
                    parts.backups.write(*document, keep, workbench);
                }
            }
            WorkbenchAction::Restore {
                folder,
                copy,
                project,
            } => {
                let keep = workbench.autosave_settings().clamped().copies as usize;
                if let Some(parts) = self.parts.get_mut(tab) {
                    parts
                        .backups
                        .restore(&folder, &copy, project, keep, workbench);
                }
            }
            WorkbenchAction::DiscardBackups { folder } => {
                if let Some(parts) = self.parts.get_mut(tab) {
                    parts.backups.discard(&folder);
                }
            }
            WorkbenchAction::Export(request) => {
                if let Some(path) = project::export_path(workbench, &request) {
                    self.files.export(workbench, request, path);
                }
            }
            WorkbenchAction::ExportKit(request) => {
                if let Some(folder) = project::kit_folder(workbench, &request) {
                    self.files.export_kit(workbench, request, folder);
                }
            }
            WorkbenchAction::ExportToTarget(request) => {
                self.files.export_to_game(workbench, request);
            }
            WorkbenchAction::SkipUpdate => self.updating.skipped(),
            WorkbenchAction::InstallUpdate { now } => {
                if let Some(installer) = self.updating.ready.clone() {
                    updates::set_pending(PendingInstall {
                        installer,
                        reopen: now,
                    });
                    if now {
                        // Closing asks about unsaved changes as usual.
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            }
            other => file_action(workbench, other),
        }
    }

    /// Puts tabs left long enough ago to sleep (decision 0267), or every
    /// tab left at once while memory is low, and looks again when one
    /// will be due (and every little while for memory meanwhile).
    fn sleep_tabs(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        let low = self.memory.low(now, crate::platform::memory);
        let parts = &mut self.parts;
        if let Some(wait) = self
            .tabs
            .sleep_tabs(now, low, |id| parts.sleep_renderer(id))
        {
            ctx.request_repaint_after(wait.min(crate::memory::LOOK_EVERY));
        }
    }

    /// Reports background work back: updates, every tab's backups and
    /// emergency snapshot, and the save or export under way.
    fn background_work(&mut self, ctx: &egui::Context) {
        if self.updating.update(self.tabs.active_mut()) {
            // Hand on what the looking finds without waiting for input.
            ctx.request_repaint_after(Duration::from_secs(5));
        }
        // The emergency snapshots are taken between frames, when each
        // document is whole.
        if let Some(wait) = self.parts.update(&mut self.tabs) {
            ctx.request_repaint_after(wait);
        }
        let workbench = self.tabs.get_or_active_mut(self.files_tab);
        match self.files.update(workbench) {
            AfterWork::Stay => {}
            // The close button was pressed meanwhile: try again, so
            // changes made during the save are asked about.
            AfterWork::AskToClose => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            AfterWork::Close => {
                self.closing = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        if self.files.busy() {
            // Look again shortly for the work to be done.
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }

    /// Writes the recent projects at once when they change, so Open recent
    /// keeps a project even if the app is then killed.
    fn remember_recent(&mut self, frame: &mut eframe::Frame) {
        let recent = self.tabs.active().recent_projects();
        if recent != self.recent_written.as_slice()
            && let Some(storage) = frame.storage_mut()
        {
            self.recent_written = recent.to_vec();
            eframe::set_value(storage, RECENT_KEY, &self.recent_written);
            storage.flush();
        }
    }

    /// Saves the settings (shared by every tab) and the panel layouts, as
    /// the tab in use has them.
    pub(super) fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let workbench = self.tabs.active();
        eframe::set_value(storage, LAYOUT_KEY, workbench.layout());
        eframe::set_value(storage, UV_LAYOUT_KEY, workbench.uv_layout());
        eframe::set_value(storage, PAINT_LAYOUT_KEY, workbench.paint_layout());
        eframe::set_value(storage, MATERIAL_LAYOUT_KEY, workbench.material_layout());
        eframe::set_value(storage, BRUSHES_KEY, &workbench.saved_brushes());
        eframe::set_value(storage, OWN_BOXES_KEY, &workbench.own_library());
        eframe::set_value(storage, LIBRARY_GRAPHS_KEY, &workbench.graph_library());
        eframe::set_value(storage, RECIPES_KEY, &workbench.saved_recipes());
        eframe::set_value(storage, USED_COLOURS_KEY, &workbench.used_colours());
        eframe::set_value(storage, VIEW_KEY, &workbench.view_options());
        eframe::set_value(storage, ACCENT_KEY, &workbench.accent());
        eframe::set_value(storage, UNITS_KEY, &workbench.new_project_units());
        eframe::set_value(storage, UNDO_MEMORY_KEY, &workbench.undo_memory());
        eframe::set_value(storage, RECENT_KEY, &workbench.recent_projects());
        eframe::set_value(storage, AUTOSAVE_KEY, &workbench.autosave_settings());
        eframe::set_value(storage, TOOL_MENU_KEY, &workbench.tool_menu_settings());
        eframe::set_value(storage, UV_SETTINGS_KEY, &workbench.uv_settings());
        eframe::set_value(storage, EXPORT_KEY, &workbench.export_settings());
    }

    /// A normal close: no tab's backup copies are needed any more.
    pub(super) fn on_exit(&mut self) {
        // A save or export under way is finished first.
        let workbench = self.tabs.get_or_active_mut(self.files_tab);
        self.files.finish(workbench);
        self.parts.end();
    }
}

/// Carries out an action that only asks for a file and reads or writes
/// it, for `workbench`.
fn file_action(workbench: &mut Workbench, action: WorkbenchAction) {
    match action {
        WorkbenchAction::Import { path } => project::import(workbench, path),
        WorkbenchAction::ImportPaintImage { material, path } => {
            project::import_paint_image(workbench, &material, path);
        }
        WorkbenchAction::ChooseStencil { path } => project::choose_stencil(workbench, path),
        WorkbenchAction::ChooseReference { path } => project::choose_reference(workbench, path),
        WorkbenchAction::ChoosePhoto { path } => project::choose_photo(workbench, path),
        WorkbenchAction::ChooseStamp { path } => project::choose_stamp(workbench, path),
        WorkbenchAction::ChooseUvTexture { path } => project::choose_uv_texture(workbench, path),
        WorkbenchAction::ExportUvLayout { svg, path } => {
            project::export_uv_layout(workbench, svg, path);
        }
        WorkbenchAction::ImportPalette { path } => project::import_palette(workbench, path),
        WorkbenchAction::ExportPalette { index, path } => {
            project::export_palette(workbench, index, path);
        }
        WorkbenchAction::SavePreset { kind, path } => project::save_preset(workbench, kind, path),
        WorkbenchAction::LoadPreset { path } => project::load_preset(workbench, path),
        WorkbenchAction::SaveBoxFile { path } => project::save_box_file(workbench, path),
        WorkbenchAction::LoadBoxFile { path } => project::load_box_file(workbench, path),
        WorkbenchAction::SavePack { path } => project::save_pack(workbench, path),
        WorkbenchAction::BringIntoLibrary { path } => project::bring_into_library(workbench, path),
        WorkbenchAction::ChooseBakeFolder => project::choose_bake_folder(workbench),
        WorkbenchAction::ChooseGraphTexturesFolder => {
            project::choose_graph_textures_folder(workbench);
        }
        WorkbenchAction::ChooseExportFolder { start } => {
            project::choose_export_folder(workbench, start);
        }
        // Carried out by the tabs or the frame itself.
        _ => {}
    }
}
