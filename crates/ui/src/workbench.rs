//! The app window's contents: menu bar, palettes, docked panels, status
//! bar and the About box.

use crate::attributes::{AttributesPanel, Shared};
use crate::autosave::{
    AutosaveSettings, Autosaver, RecoveryAnswer, RecoveryOffer, RecoveryQuestion, Tick,
};
use crate::commands::CommandId;
use crate::components::{self, ComponentTracker};
use crate::coordinates::{CoordinatesView, Interaction};
use crate::drawing::{DrawingCache, SceneSummary};
use crate::layout::{default_layout, fit_coordinates, hide_viewport_tab, restore_or_default};
use crate::objects::{self, DocRequest, ObjectManager};
use crate::palette::{Mode, PaletteCommand, PaletteState, command_palette, mode_palette};
use crate::panel::Panel;
use crate::panels::{about_contents, menu_line_with};
use crate::project::{
    NEW_SHORTCUT, OPEN_SHORTCUT, Pending, ProjectFile, SAVE_AS_SHORTCUT, SAVE_SHORTCUT,
    recent_label,
};
use crate::selecting::StrokeMemory;
use crate::settings::{SETTINGS_SHORTCUT, SettingsWindow, Values};
use crate::shape_picker::ShapePicker;
use crate::theme::{self, Accent};
use crate::tool_menu::ToolMenu;
use crate::transforming::Tool;
use crate::undo_memory::UndoMemory;
use crate::units::Units;
use crate::viewport::{ViewOptions, ViewportPanel, ViewportRenderer};
use egui::{Align2, Id, Key, KeyboardShortcut, Modifiers, Ui, WidgetText};
use egui_dock::{DockArea, DockState, TabViewer};
use scene::{Command, Document, ObjectKind, PrimitiveKind};
use selection::SelectRequest;

/// How quickly two clicks must follow each other to be a double-click, in
/// seconds: Windows' standard.
pub const DOUBLE_CLICK_SECONDS: f64 = 0.5;
pub(crate) use selection::VISIBLE_ONLY_HELP;
use std::path::{Path, PathBuf};

/// Facts about the program, shown in the window and the About box.
/// The app passes these in from its branding, so this crate never names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppInfo {
    /// Full product name, such as "Aventail 3D".
    pub name: String,
    /// Version, such as "0.0.1".
    pub version: String,
    /// The family of tools the app belongs to, such as "Rhylib".
    pub family: String,
}

/// Layouts shown on the right of the menu bar: the standard one, and
/// arrangements for particular kinds of work that arrive later.
const LAYOUTS: [(&str, Option<&str>); 3] = [
    ("Standard", None),
    ("Modelling", Some("A layout for modelling arrives later")),
    ("UV Edit", Some("A layout for UV work arrives in Phase 8")),
];

/// Something the user asked for that the program around the workbench
/// must carry out (the workbench never touches files or the window itself).
#[derive(Debug, Clone, PartialEq)]
pub enum WorkbenchAction {
    /// Close the window (unsaved changes have been dealt with).
    Quit,
    /// Save the document to `path`, or ask where first (Save As, or a
    /// project never saved) when `None`. Report back with
    /// [`Workbench::saved`], [`Workbench::save_failed`] or
    /// [`Workbench::save_cancelled`].
    Save {
        /// Where to save, if already known.
        path: Option<PathBuf>,
    },
    /// Open the project at `path`, or ask which first when `None`. Report
    /// back with [`Workbench::opened`] or [`Workbench::open_failed`].
    Open {
        /// Which project, if already known (File → Open recent).
        path: Option<PathBuf>,
    },
    /// Save a backup copy of `document` in the background, keeping the
    /// newest `keep` copies. Report back with [`Workbench::autosaved`] or
    /// [`Workbench::autosave_failed`].
    Autosave {
        /// A copy of the document as it is now.
        document: Box<Document>,
        /// How many copies to keep.
        keep: usize,
    },
    /// Bring back the backup copy `copy` from the backups `folder` left by
    /// a crash, for the project at `project` (if it had been saved). Report
    /// back with [`Workbench::recovered`] or [`Workbench::recovery_failed`];
    /// once restored, the folder can go.
    Restore {
        /// The leftover backups.
        folder: PathBuf,
        /// The copy chosen.
        copy: PathBuf,
        /// The project it belongs to.
        project: Option<PathBuf>,
    },
    /// Delete the backups `folder` left by a crash.
    DiscardBackups {
        /// The leftover backups.
        folder: PathBuf,
    },
}

/// The keys that switch to the Points, Edges and Polygons modes. Common in
/// 3D programs, and free here (Q is the Select tool, decision 0019); the
/// same key again goes back to the Model mode.
pub const MODE_KEYS: [(Key, Mode); 3] = [
    (Key::Num1, Mode::Points),
    (Key::Num2, Mode::Edges),
    (Key::Num3, Mode::Polygons),
];

/// `op`, made on both sides of `mirror` when there is one and the edit can
/// be mirrored ([decision 0059](../../../docs/decisions/0059-symmetry-for-every-edit.md)).
pub(crate) fn with_mirror(
    op: scene::Operation,
    mirror: Option<mesh::symmetry::Mirror>,
) -> scene::Operation {
    match mirror {
        Some(mirror) if scene::mirroring::mirrors(&op) => scene::Operation::Mirrored {
            op: Box::new(op),
            mirror,
        },
        _ => op,
    }
}

/// Steps straight back to the Model mode from wherever editing has got to
/// ([decision 0057](../../../docs/decisions/0057-after-phase-4-testing.md)):
/// a Slit line is finished, a modelling tool is put away and the Points,
/// Edges or Polygons mode is left, so a click selects objects again.
pub(crate) const BACK_KEY: Key = Key::Tab;

/// Menu entries that exist in the design but not yet in the app, with the
/// phase that brings them. (The Mesh menu's are listed below its working
/// entries.)
const COMING: [(&str, &[(&str, &str)]); 4] = [
    // The Select menu's entries all work (see `selection`).
    ("Select", &[]),
    (
        // The Tools menu's entries all work (see `transform`).
        "Tools",
        &[],
    ),
    // The Mesh menu lists the modelling tools from the command list.
    ("Mesh", &[]),
    (
        "UV",
        &[
            ("Seams, unwrap, relax", "Phase 8"),
            ("Pack and texel density", "Phase 8"),
        ],
    ),
];

/// The whole interface around the scene.
pub struct Workbench {
    info: AppInfo,
    dock: DockState<Panel>,
    /// The standard layout still needs Coordinates fitted to its content,
    /// which can only happen once the panels' sizes are known.
    fit_pending: bool,
    mode: Mode,
    /// The scene being worked on, with its undo history and selection.
    document: Document,
    objects: ObjectManager,
    attributes: AttributesPanel,
    /// Metres or centimetres, for every length shown or typed.
    units: Units,
    /// How much memory the undo history may use.
    undo_memory: UndoMemory,
    /// The Settings window.
    settings: SettingsWindow,
    /// Where the project is saved, whether it has unsaved changes, and
    /// the recent projects.
    file: ProjectFile,
    /// Actions to hand to the app on the next frame (what was waiting for
    /// a save to finish).
    queued: Vec<WorkbenchAction>,
    /// How many forgotten undo steps the status bar has already told
    /// about (it says so once, when forgetting starts).
    forgotten_told: bool,
    /// The `B` shape list.
    shape_picker: ShapePicker,
    /// The tool menu (E).
    tool_menu: ToolMenu,
    /// What the viewport draws, kept up to date with the document.
    drawing: DrawingCache,
    /// Polygons hidden from view (Shift + H, Ctrl + H; decision 0055).
    hidden: crate::drawing::Hidden,
    /// The objects copied for pasting (Ctrl + C, Alt + C; decision 0055).
    copied: Vec<scene::ObjectId>,
    /// The Points, Edges and Polygons modes: the element under the mouse
    /// and what is drawn over the edited objects.
    components: ComponentTracker,
    /// Selection commands from keys and the Select menu, carried out once
    /// the frame's drawing is known.
    select_requests: Vec<SelectRequest>,
    /// What a flipping brush stroke has flipped so far.
    stroke_memory: StrokeMemory,
    /// The tool in use: selecting, or a transform gizmo.
    tool: Tool,
    /// A change typed or dragged in the Coordinates panel, in progress.
    coordinates_session: Option<transform::CoordinatesSession>,
    /// A gizmo drag in progress.
    gizmo_session: Option<crate::transforming::Session>,
    /// A modelling tool's drag (extrude, inset) in progress.
    modelling_session: Option<modelling::ModellingSession>,
    /// A loop cut being placed (decision 0047).
    loop_cut_session: Option<loop_cut::LoopCutSession>,
    /// A slit's line being drawn (decision 0049).
    slit_session: Option<slit::SlitSession>,
    /// A Bridge or Weld drag in progress (decision 0051).
    pair_session: Option<joining::PairSession>,
    /// The question asked before dissolving across a sharp corner.
    dissolve_question: Option<cleaning::DissolveQuestion>,
    /// The change the gizmo drag last showed, so an unchanged frame does
    /// nothing.
    gizmo_shown: Option<tools::Change>,
    /// How many transform changes have been made (each gets its own undo
    /// merge key).
    sessions: u64,
    viewport: ViewportPanel,
    accent: Accent,
    about_open: bool,
    /// The keyboard shortcut viewer (Keys on the menu bar; decision 0056).
    shortcuts: shortcuts::ShortcutViewer,
    /// The keyboard guessed from the system language, drawn in the Keys
    /// window until one is picked there.
    system_keyboard: crate::KeyboardLayout,
    /// A message for the status bar, such as "Panel layout reset".
    note: Option<String>,
    /// When the next backup copy is due.
    autosaver: Autosaver,
    /// Where backup copies go, to show in Settings.
    autosave_folder: Option<PathBuf>,
    /// The question asked after a crash.
    recovery: RecoveryQuestion,
}

impl Workbench {
    /// A workbench with the saved panel layout, if it is usable, or the
    /// standard one.
    pub fn new(info: AppInfo, saved_layout: Option<DockState<Panel>>) -> Self {
        let had_saved = saved_layout.is_some();
        let (dock, note) = restore_or_default(saved_layout);
        // A saved layout keeps the user's own sizes; a new or replaced one
        // is fitted on its first frame.
        let fit_pending = !had_saved || note.is_some();
        Self {
            info,
            dock,
            fit_pending,
            mode: Mode::default(),
            document: Document::new(),
            objects: ObjectManager::default(),
            attributes: AttributesPanel::default(),
            units: Units::default(),
            undo_memory: UndoMemory::default(),
            forgotten_told: false,
            settings: SettingsWindow::default(),
            file: ProjectFile::default(),
            queued: Vec::new(),
            shape_picker: ShapePicker::default(),
            tool_menu: ToolMenu::default(),
            drawing: DrawingCache::default(),
            hidden: crate::drawing::Hidden::default(),
            copied: Vec::new(),
            components: ComponentTracker::default(),
            select_requests: Vec::new(),
            stroke_memory: StrokeMemory::default(),
            tool: Tool::default(),
            gizmo_session: None,
            modelling_session: None,
            loop_cut_session: None,
            slit_session: None,
            pair_session: None,
            dissolve_question: None,
            gizmo_shown: None,
            coordinates_session: None,
            sessions: 0,
            viewport: ViewportPanel::default(),
            accent: Accent::default(),
            about_open: false,
            shortcuts: shortcuts::ShortcutViewer::default(),
            system_keyboard: crate::KeyboardLayout::default(),
            note,
            autosaver: Autosaver::default(),
            autosave_folder: None,
            recovery: RecoveryQuestion::default(),
        }
    }

    /// The workbench with remembered autosave settings restored.
    #[must_use]
    pub fn with_autosave(mut self, settings: AutosaveSettings) -> Self {
        self.autosaver.settings = settings.clamped();
        self
    }

    /// The autosave settings, for remembering between sessions.
    pub fn autosave_settings(&self) -> AutosaveSettings {
        self.autosaver.settings
    }

    /// Where the app keeps backup copies, to show in Settings.
    pub fn set_autosave_folder(&mut self, folder: Option<PathBuf>) {
        self.autosave_folder = folder;
    }

    /// Work left behind by a crash: the workbench asks what to do with it
    /// (one at a time, newest first).
    pub fn offer_recovery(&mut self, offers: Vec<RecoveryOffer>) {
        self.recovery
            .offers
            .extend(offers.into_iter().filter(|o| !o.copies.is_empty()));
    }

    /// Whether the question about work left by a crash is showing.
    pub fn is_asking_to_recover(&self) -> bool {
        !self.recovery.offers.is_empty()
    }

    /// The app wrote the backup copy it was asked for.
    pub fn autosaved(&mut self) {
        self.autosaver.finished(true);
    }

    /// Writing a backup copy failed: says why (once), and tries again
    /// after the next interval.
    pub fn autosave_failed(&mut self, reason: &str) {
        self.autosaver.finished(false);
        self.note = Some(format!("Couldn't save a backup copy: {reason}."));
    }

    /// The app brought back `document` from a backup copy, for the
    /// project at `project` (if it had been saved): it replaces the
    /// current scene as unsaved work, so saving keeps it.
    pub fn recovered(&mut self, document: Document, project: Option<PathBuf>, when: &str) {
        self.replace_document(document);
        // Unsaved: the project file (if any) doesn't have this work.
        self.file.saved_revision = u64::MAX;
        self.file.path = project;
        // The app already kept it as this run's newest backup copy, and
        // the timer starts from it (`replace_document`).
        self.note = Some(format!(
            "Restored your work from {when}. Save it to keep it."
        ));
        self.viewport.frame(crate::Frame::All);
    }

    /// Restoring a backup copy failed: says why. The copies are kept.
    pub fn recovery_failed(&mut self, reason: &str) {
        self.note = Some(format!("Couldn't restore the backup copy: {reason}."));
    }

    /// Asks about work left by a crash, and when it is due, has a backup
    /// copy made.
    fn autosave_and_recovery(&mut self, ctx: &egui::Context, actions: &mut Vec<WorkbenchAction>) {
        if let Some((offer, answer)) = self.recovery.show(ctx) {
            let folder = offer.folder.clone();
            match answer {
                RecoveryAnswer::Restore(copy) => actions.push(WorkbenchAction::Restore {
                    folder,
                    copy,
                    project: offer.project,
                }),
                RecoveryAnswer::OpenSaved(path) => {
                    actions.push(WorkbenchAction::Open { path: Some(path) });
                    actions.push(WorkbenchAction::DiscardBackups { folder });
                }
                RecoveryAnswer::Discard => actions.push(WorkbenchAction::DiscardBackups { folder }),
            }
        }
        let now = ctx.input(|i| i.time);
        let revision = self.document.revision();
        match self.autosaver.tick(now, revision) {
            Tick::Idle => {}
            Tick::Wait(seconds) => {
                ctx.request_repaint_after(std::time::Duration::from_secs_f64(seconds));
            }
            Tick::Save => {
                self.autosaver.started(revision, now);
                let scene = self.document.scene().clone();
                actions.push(WorkbenchAction::Autosave {
                    document: Box::new(Document::from_scene(scene, self.document.selection())),
                    keep: self.autosaver.settings.copies as usize,
                });
            }
        }
    }

    /// The current panel layout, for saving between sessions.
    pub fn layout(&self) -> &DockState<Panel> {
        &self.dock
    }

    /// The text for the window's title bar.
    pub fn window_title(&self) -> String {
        format!("{} {}", self.info.name, self.info.version)
    }

    /// The current mode (Model, Object, Points and so on).
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// The status bar's message, if any.
    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }

    /// Whether the About box is showing.
    pub fn is_about_open(&self) -> bool {
        self.about_open
    }

    /// Puts every panel back in its standard place (View → Reset panel layout).
    pub fn reset_layout(&mut self) {
        self.dock = default_layout();
        self.fit_pending = true;
        self.note = Some("Panel layout reset to the standard arrangement.".into());
    }

    /// The scene being worked on.
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// The scene being worked on, for changing it in tests.
    #[cfg(test)]
    pub(crate) fn document_mut(&mut self) -> &mut Document {
        &mut self.document
    }

    /// The shape list's state, for tests.
    #[cfg(test)]
    pub(crate) fn shape_picker(&self) -> &ShapePicker {
        &self.shape_picker
    }

    /// The Objects panel's state, for tests.
    #[cfg(test)]
    pub(crate) fn object_manager(&self) -> &ObjectManager {
        &self.objects
    }

    /// The 3D viewport panel.
    pub fn viewport(&self) -> &ViewportPanel {
        &self.viewport
    }

    /// The 3D viewport panel, for changing it in tests.
    #[cfg(test)]
    pub(crate) fn viewport_mut(&mut self) -> &mut ViewportPanel {
        &mut self.viewport
    }

    /// The viewport's display choices, for saving between sessions.
    pub fn view_options(&self) -> ViewOptions {
        self.viewport.options()
    }

    /// Metres or centimetres, for saving between sessions.
    pub fn units(&self) -> Units {
        self.units
    }

    // ------------------------------------------------- the project file ---

    /// The window title: the project's name, then the app's name and
    /// version. With unsaved changes, a dot goes in front and "(unsaved)"
    /// after the name, so it shows at a glance even in the taskbar.
    pub fn title(&self) -> String {
        let (dot, changed) = if self.has_unsaved_changes() {
            ("● ", " (unsaved)")
        } else {
            ("", "")
        };
        format!(
            "{dot}{}{changed} — {} {}",
            self.file.name(),
            self.info.name,
            self.info.version
        )
    }

    /// Where the project is saved, if it has been.
    pub fn project_path(&self) -> Option<&Path> {
        self.file.path.as_deref()
    }

    /// Whether the scene has changed since it was last saved or opened.
    pub fn has_unsaved_changes(&self) -> bool {
        self.document.revision() != self.file.saved_revision
    }

    /// Recently saved or opened projects, newest first (for remembering
    /// between sessions).
    pub fn recent_projects(&self) -> &[PathBuf] {
        &self.file.recent
    }

    /// The workbench with remembered recent projects restored.
    #[must_use]
    pub fn with_recent_projects(mut self, recent: Vec<PathBuf>) -> Self {
        self.file.recent = recent;
        self.file.recent.truncate(crate::project::RECENT_COUNT);
        self
    }

    /// The app saved the document to `path`. Carries on with whatever was
    /// waiting for the save (a new project, opening another, closing).
    pub fn saved(&mut self, path: PathBuf) {
        self.note = Some(format!("Saved {}.", file_name(&path)));
        self.file.saved_revision = self.document.revision();
        self.file.remember(&path);
        self.file.path = Some(path);
        if let Some(next) = self.file.after_save.take() {
            self.go_ahead(next);
        }
    }

    /// Saving failed: says why, and stops whatever was waiting for it.
    pub fn save_failed(&mut self, reason: &str) {
        self.note = Some(format!("Couldn't save: {reason}."));
        self.file.after_save = None;
    }

    /// The user closed the Save dialog without choosing: nothing is saved,
    /// and whatever was waiting for the save doesn't happen.
    pub fn save_cancelled(&mut self) {
        self.file.after_save = None;
    }

    /// The app opened `document` from `path`: it replaces the current one
    /// (whose unsaved changes were already dealt with), with a fresh undo
    /// history, and the view frames it.
    pub fn opened(&mut self, document: Document, path: PathBuf) {
        self.replace_document(document);
        self.note = Some(format!("Opened {}.", file_name(&path)));
        self.file.remember(&path);
        self.file.path = Some(path);
        self.viewport.frame(crate::Frame::All);
    }

    /// Opening `path` failed: says why. A recent project that can no
    /// longer be found is taken off the list.
    pub fn open_failed(&mut self, path: &Path, reason: &str) {
        self.note = Some(format!("Couldn't open {}: {reason}.", file_name(path)));
        if !path.exists() {
            self.file.recent.retain(|p| p != path);
        }
    }

    /// The window's close button was pressed. Returns `true` if it may
    /// close now; with unsaved changes it asks first and returns `false`
    /// (a [`WorkbenchAction::Quit`] follows if the user agrees).
    pub fn request_close(&mut self) -> bool {
        if self.has_unsaved_changes() {
            self.file.asking = Some(Pending::Quit);
            false
        } else {
            true
        }
    }

    /// Does `next` now, or first asks whether to save unsaved changes.
    fn ask_then(&mut self, next: Pending, actions: &mut Vec<WorkbenchAction>) {
        if self.has_unsaved_changes() {
            self.file.asking = Some(next);
        } else {
            self.go_ahead(next);
            actions.append(&mut self.queued);
        }
    }

    /// Carries out `next`, unsaved changes being dealt with. A new project
    /// starts at once; opening and closing are queued for the app (handed
    /// over at once inside a frame, or on the next frame otherwise).
    fn go_ahead(&mut self, next: Pending) {
        match next {
            Pending::New => {
                self.replace_document(Document::new());
                self.file.path = None;
                self.note = Some("New project.".into());
            }
            Pending::Open(path) => self.queued.push(WorkbenchAction::Open { path }),
            Pending::Quit => self.queued.push(WorkbenchAction::Quit),
        }
    }

    /// Puts `document` in place of the current one, keeping the undo
    /// memory limit, and starts the view and panels afresh.
    fn replace_document(&mut self, mut document: Document) {
        document.set_history_limit(self.undo_memory.limit());
        self.file.saved_revision = document.revision();
        self.document = document;
        self.autosaver.restart();
        self.drawing.forget();
        self.hidden.faces.clear();
        self.hidden.revision += 1;
        self.copied.clear();
        self.forgotten_told = false;
    }

    /// The "Save changes before …?" question, while asked.
    fn unsaved_question(&mut self, ctx: &egui::Context, actions: &mut Vec<WorkbenchAction>) {
        let Some(pending) = self.file.asking.clone() else {
            return;
        };
        let mut answer = None;
        egui::Modal::new(egui::Id::new("unsaved_changes")).show(ctx, |ui| {
            ui.set_width(380.0);
            ui.label(
                egui::RichText::new(format!("Save changes to {}?", self.file.name()))
                    .strong()
                    .color(theme::TEXT_STRONG),
            );
            ui.add_space(4.0);
            ui.label(format!(
                "Your changes will be lost if you don't save them before {}.",
                pending.before()
            ));
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    answer = Some(UnsavedAnswer::Save);
                }
                if ui.button("Don't save").clicked() {
                    answer = Some(UnsavedAnswer::Discard);
                }
                if ui.button("Cancel").clicked() {
                    answer = Some(UnsavedAnswer::Cancel);
                }
            });
            ui.add_space(4.0);
            let discard = ui.ctx().format_shortcut(&DISCARD_SHORTCUT);
            ui.label(
                egui::RichText::new(format!("Enter: save · {discard}: don't save · Esc: cancel"))
                    .small()
                    .color(theme::TEXT_WEAK),
            );
        });
        let (save, discard, cancel) = ctx.input_mut(|i| {
            (
                i.consume_key(Modifiers::NONE, Key::Enter),
                i.consume_shortcut(&DISCARD_SHORTCUT),
                i.consume_key(Modifiers::NONE, Key::Escape),
            )
        });
        if save {
            answer = Some(UnsavedAnswer::Save);
        } else if discard {
            answer = Some(UnsavedAnswer::Discard);
        } else if cancel {
            answer = Some(UnsavedAnswer::Cancel);
        }
        match answer {
            Some(UnsavedAnswer::Save) => {
                self.file.asking = None;
                self.file.after_save = Some(pending);
                actions.push(WorkbenchAction::Save {
                    path: self.file.path.clone(),
                });
            }
            Some(UnsavedAnswer::Discard) => {
                self.file.asking = None;
                self.go_ahead(pending);
                actions.append(&mut self.queued);
            }
            Some(UnsavedAnswer::Cancel) => self.file.asking = None,
            None => {}
        }
    }

    /// Whether the "Save changes?" question is showing.
    pub fn is_asking_to_save(&self) -> bool {
        self.file.asking.is_some()
    }

    /// The File menu's project entries and their keys.
    fn file_keys(&mut self, ui: &Ui, actions: &mut Vec<WorkbenchAction>) {
        let (new, open, save_as, save) = ui.input_mut(|i| {
            (
                i.consume_shortcut(&NEW_SHORTCUT),
                i.consume_shortcut(&OPEN_SHORTCUT),
                // Ctrl+Shift+S first: Ctrl+S would match it too.
                i.consume_shortcut(&SAVE_AS_SHORTCUT),
                i.consume_shortcut(&SAVE_SHORTCUT),
            )
        });
        if new {
            self.ask_then(Pending::New, actions);
        }
        if open {
            self.ask_then(Pending::Open(None), actions);
        }
        if save_as {
            actions.push(WorkbenchAction::Save { path: None });
        } else if save {
            actions.push(WorkbenchAction::Save {
                path: self.file.path.clone(),
            });
        }
    }

    /// Whether the Settings window is open.
    pub fn is_settings_open(&self) -> bool {
        self.settings.open
    }

    /// Opens the Settings window.
    pub fn open_settings(&mut self) {
        self.settings.open = true;
    }

    /// How much memory the undo history may use.
    pub fn undo_memory(&self) -> UndoMemory {
        self.undo_memory
    }

    /// The workbench with a remembered undo memory choice restored.
    #[must_use]
    pub fn with_undo_memory(mut self, memory: UndoMemory) -> Self {
        self.set_undo_memory(memory);
        self
    }

    fn set_undo_memory(&mut self, memory: UndoMemory) {
        self.undo_memory = memory;
        self.document.set_history_limit(memory.limit());
    }

    /// The workbench with a remembered choice of units restored.
    #[must_use]
    pub fn with_units(mut self, units: Units) -> Self {
        self.units = units;
        self
    }

    /// The accent colour, for saving between sessions.
    pub fn accent(&self) -> Accent {
        self.accent
    }

    /// The workbench with a remembered accent colour restored.
    #[must_use]
    pub fn with_accent(mut self, accent: Accent) -> Self {
        self.accent = accent;
        self
    }

    /// Changes the accent colour (Window → Accent colour).
    pub fn set_accent(&mut self, accent: Accent) {
        self.accent = accent;
    }

    /// The workbench with remembered display choices restored.
    #[must_use]
    pub fn with_view_options(mut self, options: ViewOptions) -> Self {
        *self.viewport.options_mut() = options.migrated();
        self
    }

    /// Draws the whole window and returns what the user asked for.
    /// `renderer` draws the 3D viewport.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        renderer: &mut dyn ViewportRenderer,
    ) -> Vec<WorkbenchAction> {
        let mut actions = Vec::new();
        if theme::accent(ui.ctx()) != self.accent {
            theme::set_accent(ui.ctx(), self.accent);
        }
        // Double-clicks as forgiving as Windows' standard (egui's is 0.3 s).
        if ui.ctx().options(|o| o.input_options.max_double_click_delay) != DOUBLE_CLICK_SECONDS {
            ui.ctx().options_mut(|o| {
                o.input_options.max_double_click_delay = DOUBLE_CLICK_SECONDS;
            });
        }
        actions.append(&mut self.queued);
        if self.file.asking.is_none() {
            self.file_keys(ui, &mut actions);
        }
        if ui.input_mut(|i| i.consume_shortcut(&SETTINGS_SHORTCUT)) {
            self.settings.open = !self.settings.open;
        }
        let mut requests = Vec::new();
        // The shape list goes first: while it is open, Esc closes it
        // rather than deselecting.
        // While "Save changes?" (or the dissolve question) is asked, keys
        // answer it, not the scene.
        if self.file.asking.is_none() && self.dissolve_question.is_none() {
            let typing = ui.ctx().memory(|m| m.focused().is_some()) || self.objects.is_renaming();
            if let Some(kind) = self.shape_picker.keys(ui, typing) {
                requests.push(create(kind));
            }
            // The tool menu next: while it is open, Esc closes it, and
            // Enter or Space use its tool.
            let entries = self.tool_menu_entries();
            self.tool_menu_keys(ui, typing, &entries, &mut requests);
            self.showing_keys(ui);
            self.copying_keys(ui);
            self.document_keys(ui, &mut requests);
            self.mode_keys(ui);
            self.select_keys(ui);
            self.transform_keys(ui);
            self.modelling_keys(ui);
        }

        egui::Panel::top("menu_bar")
            .frame(theme::menu_bar_frame())
            .show(ui, |ui| {
                egui::MenuBar::new().ui(ui, |ui| self.menu_bar(ui, &mut actions, &mut requests));
            });
        let palette_state = PaletteState {
            undo: self.document.undo_label().map(str::to_string),
            redo: self.document.redo_label().map(str::to_string),
            select_tool: self.viewport.select_tool(),
            tool: self.tool,
            locked: self.viewport.options().locked,
            axes: self.viewport.options().axes,
            snapping: self.viewport.options().snap.on,
            symmetry: self.viewport.options().symmetry.on,
        };
        // The Symmetry menu changes a copy, kept if it changed (the
        // palette's switch changes the options directly).
        let symmetry_before = self.viewport.options().symmetry;
        let mut symmetry = symmetry_before;
        let units = self.units;
        egui::Panel::top("command_palette")
            .frame(theme::palette_frame())
            .exact_size(40.0)
            .show(ui, |ui| {
                if let Some(command) = command_palette(ui, &palette_state, &mut symmetry, units) {
                    requests.push(match command {
                        PaletteCommand::NextSelectTool => {
                            if self.tool == Tool::Select {
                                self.next_select_tool();
                            } else {
                                self.set_tool(Tool::Select);
                            }
                            return;
                        }
                        PaletteCommand::Tool(tool) => {
                            self.set_tool(tool);
                            return;
                        }
                        PaletteCommand::Lock(axis) => {
                            self.toggle_lock(axis);
                            return;
                        }
                        PaletteCommand::ToggleAxes => {
                            self.toggle_axes();
                            return;
                        }
                        PaletteCommand::ToggleSnapping => {
                            self.toggle_snapping();
                            return;
                        }
                        PaletteCommand::ToggleSymmetry => {
                            self.toggle_symmetry();
                            return;
                        }
                        PaletteCommand::WeldMiddle => {
                            self.weld_middle_now();
                            return;
                        }
                        PaletteCommand::Undo => DocRequest::Undo,
                        PaletteCommand::Redo => DocRequest::Redo,
                        PaletteCommand::CreateGroup => {
                            DocRequest::Run(Command::Create(ObjectKind::Group))
                        }
                        PaletteCommand::CreateShape(kind) => {
                            DocRequest::Run(Command::Create(ObjectKind::Primitive(kind.standard())))
                        }
                    });
                }
            });
        if symmetry != symmetry_before {
            self.viewport.options_mut().symmetry = symmetry;
        }
        egui::Panel::bottom("status_bar")
            .frame(theme::status_bar_frame())
            .exact_size(24.0)
            .show(ui, |ui| self.status_bar(ui));
        egui::Panel::left("mode_palette")
            .frame(theme::palette_frame())
            .exact_size(44.0)
            .resizable(false)
            .show(ui, |ui| {
                ui.add_space(6.0);
                match mode_palette(ui, self.mode, self.viewport.options_mut()) {
                    // With a modelling tool in use, picking a mode puts it
                    // away, as the mode keys do.
                    Some(crate::palette::ModePick::Mode(mode)) if self.tool.has_settings() => {
                        self.press_mode_key(mode);
                    }
                    Some(crate::palette::ModePick::Mode(mode)) => self.mode = mode,
                    Some(crate::palette::ModePick::Isolate) => {
                        self.hide(showing::Hiding::Others);
                    }
                    None => {}
                }
            });

        let editing = components::kind_for(self.mode);
        let mut summary: SceneSummary = self
            .drawing
            .update(&self.document, editing, &self.hidden)
            .clone();
        summary.overlay = self.components.overlay();
        self.components.freeze(
            self.gizmo_session.is_some()
                || self.coordinates_session.is_some()
                || self.modelling_session.is_some()
                || self.loop_cut_session.is_some(),
        );
        self.components.prepare(&summary.drawing);
        self.place_gizmo(&summary.drawing);
        self.place_drag_tool();
        let coordinates = self.coordinates_view(&summary.drawing);
        let mut coordinates_done = Interaction::default();
        self.attributes.tool = self.tool;
        hide_viewport_tab(&mut self.dock);
        let style = theme::dock_style(ui.style().as_ref(), self.accent);
        theme::dock_frame().show(ui, |ui| {
            DockArea::new(&mut self.dock)
                .style(style)
                .hidable_tab_bars(true)
                .tab_context_menus(false)
                .show_close_buttons(false)
                .show_add_buttons(false)
                .show_leaf_close_all_buttons(false)
                .show_leaf_collapse_buttons(false)
                .show_inside(
                    ui,
                    &mut PanelViewer {
                        viewport: &mut self.viewport,
                        renderer,
                        summary: &summary,
                        document: &mut self.document,
                        objects: &mut self.objects,
                        attributes: &mut self.attributes,
                        units: &mut self.units,
                        requests: &mut requests,
                        coordinates,
                        coordinates_done: &mut coordinates_done,
                    },
                );
        });
        // What is under the mouse now that the viewport knows where it is;
        // when that changes, draw again so the highlight follows at once.
        self.carry_out_selection(&summary.drawing);
        self.carry_out_gizmo(ui, &summary.drawing);
        let keys = ui.input(|i| i.modifiers);
        self.carry_out_tool_drag(&summary.drawing, keys);
        self.show_loop_cut_preview(&summary.drawing);
        self.show_slit(&summary.drawing);
        self.carry_out_coordinates(coordinates_done, &summary.drawing);
        let shown = summary.overlay.version;
        let options = self.viewport.options();
        let overlay = self.components.update(
            self.mode,
            &summary.drawing,
            &self.document,
            self.viewport.pointer(),
            crate::components::Guides {
                hard_edges: !options.hide_hard_edges,
                by_angle: options.auto_hard_angle(),
                normals: options.show_normals,
            },
        );
        if overlay.version != shown {
            ui.ctx().request_repaint();
        }
        if self.fit_pending && fit_coordinates(&mut self.dock) {
            self.fit_pending = false;
            ui.ctx().request_repaint();
        }

        if let Some(kind) = self.shape_picker.show(ui.ctx()) {
            requests.push(create(kind));
        }
        let entries = self.tool_menu_entries();
        self.tool_menu_show(ui.ctx(), &entries, &mut requests);
        if !requests.is_empty() {
            self.carry_out(requests);
            ui.ctx().request_repaint();
        }

        self.shortcut_window(ui.ctx());
        let mut about_open = self.about_open;
        egui::Window::new(format!("About {}", self.info.name))
            .open(&mut about_open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| about_contents(ui, &self.info));
        self.about_open = about_open;

        self.settings_window(ui.ctx());
        self.dissolve_question_window(ui.ctx());
        self.unsaved_question(ui.ctx(), &mut actions);
        self.autosave_and_recovery(ui.ctx(), &mut actions);

        actions
    }

    /// Draws the Settings window, if open, and applies what changed.
    fn settings_window(&mut self, ctx: &egui::Context) {
        let mut undo_memory = self.undo_memory;
        let history = (
            self.document.history_memory(),
            self.document.undo_steps() + self.document.redo_steps(),
        );
        let options = self.viewport.options_mut();
        let mut values = Values {
            accent: &mut self.accent,
            units: &mut self.units,
            navigation: &mut options.navigation,
            grid: &mut options.grid_detail,
            select_visible_only: &mut options.select_visible_only,
            select_keys: &mut options.select_keys,
            snap: &mut options.snap,
            undo_memory: &mut undo_memory,
            history,
            autosave: &mut self.autosaver.settings,
            autosave_folder: self.autosave_folder.as_deref(),
            tool_menu: &mut self.tool_menu.settings,
            dissolve_without_warning: &mut options.dissolve_without_warning,
        };
        self.settings.show(ctx, &mut values);
        if undo_memory != self.undo_memory {
            self.set_undo_memory(undo_memory);
        }
    }

    /// Keys that change the document: undo, redo, delete, deselect, group
    /// and ungroup. Ignored while typing in a text box, which has its own.
    fn document_keys(&mut self, ui: &Ui, out: &mut Vec<DocRequest>) {
        // (While a name is typed, Esc means "keep the old name", even on
        // the frame egui has already taken the keyboard away.)
        if ui.ctx().memory(|m| m.focused().is_some()) || self.objects.is_renaming() {
            return;
        }
        // While the gizmo is dragged, Esc cancels the drag, and nothing
        // else happens.
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
        // While a slit's line is drawn, Esc and Enter finish it (and Esc
        // deselects nothing).
        if self.slit_session.is_some()
            && ui.input_mut(|i| {
                i.consume_key(Modifiers::NONE, Key::Escape)
                    || i.consume_key(Modifiers::NONE, Key::Enter)
            })
        {
            self.finish_slit();
            return;
        }
        let popup_open = egui::Popup::is_any_open(ui.ctx());
        let anything = !self.document.selection().is_empty();
        let mode = self.mode;
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
            if i.consume_key(Modifiers::NONE, Key::Delete) && anything {
                commands.push(delete);
            }
            if i.consume_key(Modifiers::NONE, Key::Backspace) && anything {
                commands.push(dissolve);
            }
        });
        for command in commands {
            self.run_command(command, ui.ctx(), out);
        }
        // Esc closes an open menu first; otherwise it deselects: the
        // points, edges or polygons in those modes, else the objects.
        if !popup_open && ui.input_mut(|i| i.consume_shortcut(&objects::DESELECT_SHORTCUT)) {
            if components::kind_for(self.mode).is_some() {
                self.select_requests.push(SelectRequest::Clear);
            } else {
                self.document.clear_selection();
            }
        }
    }

    /// Carries out what the menus, keys and panels asked for this frame,
    /// saying in the status bar what happened when it isn't plain to see.
    fn carry_out(&mut self, requests: Vec<DocRequest>) {
        for request in requests {
            self.note = match request {
                DocRequest::Run(command) => {
                    let what = command.describe(self.document.scene());
                    match self.document.execute(command) {
                        Ok(()) => None,
                        Err(why) => Some(format!("Couldn't {}: {why}.", what.to_lowercase())),
                    }
                }
                DocRequest::RunMerging(command, key) => {
                    let what = command.describe(self.document.scene());
                    match self.document.execute_merging(command, key) {
                        Ok(()) => None,
                        Err(why) => Some(format!("Couldn't {}: {why}.", what.to_lowercase())),
                    }
                }
                DocRequest::Undo => Some(match self.document.undo() {
                    Some(what) => format!("Undone: {what}."),
                    None => "Nothing to undo.".into(),
                }),
                DocRequest::Redo => Some(match self.document.redo() {
                    Some(what) => format!("Redone: {what}."),
                    None => "Nothing to redo.".into(),
                }),
            };
        }
        if self.document.forgotten_steps() > 0 && !self.forgotten_told && self.note.is_none() {
            self.forgotten_told = true;
            self.note = Some(format!(
                "Undo memory is full ({}): the oldest steps are now forgotten as you work. \
                 Settings → Undo sets the limit.",
                self.undo_memory.short()
            ));
        }
    }

    /// The mirror planes edits to `id`'s shape are made across: with
    /// symmetry on, in the Points, Edges and Polygons modes
    /// ([decision 0059](../../../docs/decisions/0059-symmetry-for-every-edit.md)).
    pub(crate) fn edit_mirror(&self, id: scene::ObjectId) -> Option<mesh::symmetry::Mirror> {
        components::kind_for(self.mode)?;
        self.mirror_for(id)
    }

    /// With symmetry on, the mirror planes for `id`'s shape, in its own
    /// space: across its own axes, through the chosen centre moved by the
    /// offset.
    pub(crate) fn mirror_for(&self, id: scene::ObjectId) -> Option<mesh::symmetry::Mirror> {
        self.viewport.options().symmetry.active_axes()?;
        self.mirror_planes(id)
    }

    /// The mirror planes the Symmetry menu describes for `id`'s shape,
    /// whether symmetry is on or not (for Weld the middle now). `None`
    /// with no axis chosen.
    pub(crate) fn mirror_planes(&self, id: scene::ObjectId) -> Option<mesh::symmetry::Mirror> {
        let symmetry = self.viewport.options().symmetry;
        let axes = symmetry.axes;
        if !axes.contains(&true) {
            return None;
        }
        let offset = glam::Vec3::from_array(symmetry.offset);
        let offset = if offset.is_finite() {
            offset
        } else {
            glam::Vec3::ZERO
        };
        let scene = self.document.scene();
        let centre = match symmetry.centre {
            crate::SymmetryCentre::Pivot => offset,
            crate::SymmetryCentre::Middle => {
                let middle = scene
                    .evaluated(id)
                    .and_then(|shape| shape.mesh.bounds())
                    .map_or(glam::Vec3::ZERO, |b| b.centre());
                middle + offset
            }
            crate::SymmetryCentre::World => {
                let inverse = scene.world_matrix(id).inverse();
                if inverse.is_finite() {
                    inverse.transform_point3(offset)
                } else {
                    glam::Vec3::ZERO
                }
            }
        };
        Some(mesh::symmetry::Mirror {
            axes,
            centre,
            clip: symmetry.clip,
        })
    }

    /// The Symmetry menu's Weld the middle now: joins the halves of each
    /// selected shape at the middle, once, as one undo step (decision
    /// 0059). Points within the weld distance go onto the middle, and
    /// points meeting there become one.
    pub(crate) fn weld_middle_now(&mut self) {
        let distance = self.viewport.options().symmetry.weld_distance;
        let distance = if distance.is_finite() {
            distance.max(0.0)
        } else {
            0.0
        };
        let mut ops = Vec::new();
        let mut welded = 0;
        for id in self.document.selection() {
            let Some(shape) = self.document.scene().evaluated(id) else {
                continue;
            };
            let Some(mirror) = self.mirror_planes(id) else {
                self.note =
                    Some("Weld the middle: pick X, Y or Z in the Symmetry menu first.".into());
                return;
            };
            // Tried on a copy, so nothing is recorded when nothing joins.
            let mut trial = (*shape.mesh).clone();
            let n = mesh::symmetry::weld_middle(&mut trial, &mirror, distance);
            if n > 0 {
                welded += n;
                ops.push((id, scene::Operation::WeldMiddle { mirror, distance }));
            }
        }
        if ops.is_empty() {
            self.note = Some(format!(
                "Weld the middle: nothing within {} of the middle to join.",
                self.units.format(distance)
            ));
            return;
        }
        if self.record_each("Weld the middle", ops) {
            self.note = Some(format!(
                "Welded the middle ({welded} point{}). Ctrl + Z takes it back.",
                if welded == 1 { "" } else { "s" }
            ));
        }
    }

    /// Whether a selected object has a shape to edit (for the Points,
    /// Edges and Polygons modes).
    fn editing_anything(&self) -> bool {
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
    fn mode_keys(&mut self, ui: &Ui) {
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
    /// already there ([decision 0060](../../../docs/decisions/0060-mode-keys-put-tools-away.md)):
    /// pressing the mode key is how the project lead goes back to
    /// selecting. Otherwise the key switches to its mode, or back to the
    /// Model mode when already in it.
    pub(crate) fn press_mode_key(&mut self, mode: Mode) {
        if self.tool.has_settings() {
            let tool = self.tool.name();
            // Finishes a Slit line too, keeping what it cut.
            self.set_tool(Tool::Select);
            self.mode = mode;
            self.note = Some(format!(
                "{} mode: selecting ({tool} put away; its key brings it back).",
                mode.name()
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
            self.note = Some("Let go of the drag (or press Esc) first, then Tab.".into());
            return;
        }
        let menu = self.tool_menu.is_open();
        self.tool_menu.close();
        let tool = self.tool.has_settings();
        if tool {
            // Finishes a Slit line too, keeping what it cut.
            self.set_tool(Tool::Select);
        }
        let mode = self.mode != Mode::Model;
        self.mode = Mode::Model;
        self.note = Some(if mode || tool || menu {
            "Back in the Model mode: click an object to select it.".into()
        } else {
            "Already in the Model mode: Tab steps back here from editing points, edges or \
             polygons, or from a modelling tool."
                .into()
        });
    }

    fn status_bar(&self, ui: &mut Ui) {
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
                self.mode.description().to_lowercase()
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

    fn menu_bar(
        &mut self,
        ui: &mut Ui,
        actions: &mut Vec<WorkbenchAction>,
        requests: &mut Vec<DocRequest>,
    ) {
        ui.menu_button("File", |ui| {
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
                ui.menu_button("Open recent", |ui| {
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
            ui.add_enabled(false, egui::Button::new("Export…"))
                .on_disabled_hover_text("Export arrives in Phase 5");
            ui.separator();
            // No key for Quit, on purpose: Ctrl and Q are both used all the
            // time, and a slip shouldn't close the app.
            if ui.button("Quit").on_hover_text("Close the app").clicked() {
                self.ask_then(Pending::Quit, actions);
                ui.close();
            }
        });
        ui.menu_button("Edit", |ui| {
            objects::undo_entries(ui, &self.document, requests);
            ui.separator();
            objects::selection_entries(ui, &mut self.document, &mut self.objects, requests);
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
        ui.menu_button("Create", |ui| {
            for kind in PrimitiveKind::ALL {
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
                egui::RichText::new("Tip: press B for the shape list")
                    .small()
                    .color(theme::TEXT_WEAK),
            );
        });
        ui.menu_button("Modes", |ui| {
            for mode in Mode::ALL {
                let mut entry = egui::Button::selectable(self.mode == mode, mode.name());
                if let Some(key) = mode.key() {
                    entry = entry.shortcut_text(key.name());
                }
                if ui.add(entry).on_hover_text(mode.description()).clicked() {
                    self.mode = mode;
                    ui.close();
                }
            }
        });
        for (title, items) in COMING {
            ui.menu_button(title, |ui| {
                if title == "Select" {
                    self.select_menu(ui);
                }
                if title == "Tools" {
                    self.tools_menu(ui);
                }
                if title == "Mesh" {
                    objects::make_editable_entry(ui, &self.document, requests);
                    ui.separator();
                    if let Some(command) = commands_entries(ui, &crate::commands::MODELLING_RING) {
                        self.run_command(command, ui.ctx(), requests);
                        ui.close();
                    }
                    ui.separator();
                    if ui
                        .button("Split off")
                        .on_hover_text(
                            "Move the selected polygons into a new object of their own, in the \
                             same place (Polygons mode)",
                        )
                        .clicked()
                    {
                        self.run_split_off();
                        ui.close();
                    }
                    let join = crate::commands::CommandId::Join;
                    if ui
                        .button(join.name())
                        .on_hover_text(join.info().help)
                        .clicked()
                    {
                        self.run_command(join, ui.ctx(), requests);
                        ui.close();
                    }
                    ui.separator();
                    crate::viewport::chrome::auto_hard_edges_entry(ui, self.viewport.options_mut());
                    if ui
                        .button("Hard edges by angle")
                        .on_hover_text(
                            "In the selected objects, make every edge whose polygons meet more \
                             sharply than an angle (30° to start with) hard, and the rest soft: \
                             boxy corners crisp, curves smooth. Change the angle under Last edit.",
                        )
                        .clicked()
                    {
                        self.run_hard_by_angle();
                        ui.close();
                    }
                    if ui
                        .button("Fix facing")
                        .on_hover_text(
                            "Turn every polygon of the selected objects that faces the wrong \
                             way round: inside out compared with its neighbours, or facing \
                             into a closed shape. Select → Select flipped shows them first.",
                        )
                        .clicked()
                    {
                        self.run_fix_facing();
                        ui.close();
                    }
                }
                coming_entries(ui, items);
            });
        }
        ui.menu_button("View", |ui| {
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
            crate::viewport::chrome::grid_entries(ui, &mut self.viewport.options_mut().grid_detail);
            ui.separator();
            if ui.button("Reset panel layout").clicked() {
                self.reset_layout();
                ui.close();
            }
        });
        ui.menu_button("Window", |ui| {
            ui.add_enabled(false, egui::Button::new("UV Editor panel"))
                .on_disabled_hover_text("Arrives in Phase 8");
        });
        ui.menu_button("Help", |ui| {
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
            ui.separator();
            if ui.button(format!("About {}", self.info.name)).clicked() {
                self.about_open = true;
                ui.close();
            }
        });
        self.keys_button(ui);
        let unsaved = self.has_unsaved_changes();
        layout_tabs(ui, &self.file.name(), unsaved);
    }
}

/// How many polygons Help → Add a heavy test shape makes.
pub(crate) const HEAVY_TEST_POLYGONS: usize = 3_000_000;

impl Workbench {
    /// Adds a bumpy ball of [`HEAVY_TEST_POLYGONS`] polygons, 2 m tall, as
    /// one undo step: for trying how the app copes with heavy shapes
    /// (DESIGN.md §4.8).
    pub(crate) fn add_heavy_test_shape(&mut self) {
        self.add_test_shape(HEAVY_TEST_POLYGONS);
    }

    /// Adds a bumpy ball of about `polygons` polygons, as one undo step.
    pub(crate) fn add_test_shape(&mut self, polygons: usize) {
        /// Marks this step's merge key apart from every other.
        const HEAVY_SHAPE_KEY: u64 = 0x4845_4156_0000_0000;
        self.sessions += 1;
        let key = Some(HEAVY_SHAPE_KEY | self.sessions);
        let label = "Add heavy test shape";
        let ball = mesh::samples::heavy_ball(polygons, 100.0);
        let created = self.document.execute_labelled(
            Command::Create(ObjectKind::Mesh(std::sync::Arc::new(ball))),
            label,
            key,
        );
        if created.is_ok()
            && let Some(id) = self.document.selection().first().copied()
        {
            let _ = self.document.execute_labelled(
                Command::Rename {
                    id,
                    name: "Heavy test shape".into(),
                },
                label,
                key,
            );
        }
    }
}

/// The answers to "Save changes?".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnsavedAnswer {
    Save,
    Discard,
    Cancel,
}

/// "Don't save" in the "Save changes?" question, from the keyboard.
pub const DISCARD_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::D);

/// A path's file name, for messages.
fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// The request that creates a primitive of `kind` with standard settings.
fn create(kind: PrimitiveKind) -> DocRequest {
    DocRequest::Run(Command::Create(ObjectKind::Primitive(kind.standard())))
}

/// The status bar's standing hint: the navigation controls in use.
pub fn navigation_hint(scheme: tools::Scheme) -> String {
    format!("{}: {}", scheme.name(), scheme.summary())
}

/// The layout tabs on the right of the menu bar. Only the standard layout
/// exists so far; the others are shown greyed out with when they arrive.
fn layout_tabs(ui: &mut Ui, project: &str, unsaved: bool) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        ui.add_space(6.0);
        let tabs = LAYOUTS.iter().rev().collect::<Vec<_>>();
        layout_tab_labels(ui, &tabs);
        ui.add_space(12.0);
        project_badge(ui, project, unsaved);
    });
}

/// The project's name in the menu bar, with a badge in the accent colour
/// while there are unsaved changes: easier to see than a mark in the
/// window's title, which the system draws.
fn project_badge(ui: &mut Ui, project: &str, unsaved: bool) {
    ui.spacing_mut().item_spacing.x = 6.0;
    if unsaved {
        let accent = theme::accent(ui.ctx()).colors();
        let text = egui::RichText::new("Unsaved")
            .small()
            .strong()
            .color(theme::TEXT_STRONG);
        let galley = egui::WidgetText::from(text).into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::TextStyle::Small,
        );
        let size = galley.size() + egui::vec2(12.0, 4.0);
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
        ui.painter().rect_filled(rect, size.y / 2.0, accent.fill);
        ui.painter().galley(
            rect.center() - galley.size() / 2.0,
            galley,
            theme::TEXT_STRONG,
        );
        response
            .on_hover_text(format!(
                "Changes since the last save. {} saves.",
                ui.ctx().format_shortcut(&crate::SAVE_SHORTCUT)
            ))
            .widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Label, true, "Unsaved changes")
            });
    }
    ui.add(
        egui::Label::new(egui::RichText::new(project).color(theme::TEXT_WEAK)).selectable(false),
    )
    .on_hover_text("The project open now");
}

/// The layout tabs themselves, right to left.
fn layout_tab_labels(ui: &mut Ui, tabs: &[&(&str, Option<&str>)]) {
    for (name, coming) in tabs.iter().copied() {
        match coming {
            None => {
                let accent = theme::accent(ui.ctx()).colors();
                let response = ui.add(
                    egui::Label::new(egui::RichText::new(*name).color(theme::TEXT_STRONG))
                        .selectable(false),
                );
                let rect = response.rect;
                ui.painter().hline(
                    rect.x_range(),
                    rect.bottom() + 3.0,
                    egui::Stroke::new(2.0, accent.light),
                );
                response.on_hover_text("The layout in use");
            }
            Some(when) => {
                ui.add_enabled(false, egui::Label::new(*name).selectable(false))
                    .on_disabled_hover_text(*when);
            }
        }
    }
}

/// Menu entries for commands from the command list, each with its key;
/// one not built yet is greyed out, saying when it arrives. Returns the
/// command clicked, if any.
fn commands_entries(ui: &mut Ui, list: &[CommandId]) -> Option<CommandId> {
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

/// Menus whose commands are not built yet: each lists what is coming and
/// when, greyed out.
fn coming_entries(ui: &mut Ui, items: &[(&str, &str)]) {
    for (item, phase) in items {
        ui.add_enabled(false, egui::Button::new(*item))
            .on_disabled_hover_text(format!("Arrives in {phase}"));
    }
}

/// Draws each panel's contents.
struct PanelViewer<'a> {
    viewport: &'a mut ViewportPanel,
    renderer: &'a mut dyn ViewportRenderer,
    summary: &'a SceneSummary,
    document: &'a mut Document,
    objects: &'a mut ObjectManager,
    attributes: &'a mut AttributesPanel,
    units: &'a mut Units,
    requests: &'a mut Vec<DocRequest>,
    /// What the Coordinates panel shows, and what was done in it.
    coordinates: CoordinatesView,
    coordinates_done: &'a mut Interaction,
}

impl TabViewer for PanelViewer<'_> {
    type Tab = Panel;

    fn id(&mut self, tab: &mut Panel) -> Id {
        Id::new(("panel", *tab))
    }

    fn title(&mut self, tab: &mut Panel) -> WidgetText {
        tab.title().into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Panel) {
        if *tab == Panel::Viewport {
            // Edge to edge, with no padding.
            self.viewport.show(ui, self.renderer, self.summary);
            return;
        }
        match tab {
            Panel::Attributes => menu_line_with(ui, |ui| {
                let mut shared = Shared {
                    clipboard: &mut self.objects.clipboard,
                    units: self.units,
                };
                self.attributes
                    .menus(ui, self.document, &mut shared, self.requests);
            }),
            Panel::Objects => menu_line_with(ui, |ui| {
                objects::menus(ui, self.document, self.objects, self.requests);
            }),
            Panel::Viewport | Panel::Coordinates => {}
        }
        egui::Frame::new()
            .inner_margin(theme::panel_margin())
            .show(ui, |ui| match tab {
                Panel::Viewport => {}
                Panel::Attributes => {
                    self.attributes
                        .show(ui, self.document, *self.units, self.requests);
                }
                Panel::Objects => self.objects.show(ui, self.document, self.requests),
                Panel::Coordinates => {
                    *self.coordinates_done =
                        crate::coordinates::show(ui, *self.units, &self.coordinates);
                }
            });
    }

    fn is_closeable(&self, _tab: &Panel) -> bool {
        false
    }

    fn on_tab_button(&mut self, tab: &mut Panel, response: &egui::Response) {
        // Name the tab for screen readers (and UI tests); egui_dock paints
        // the title without labelling the button.
        let title = tab.title();
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, title));
    }

    fn allowed_in_windows(&self, _tab: &mut Panel) -> bool {
        // Panels dock inside the main window only: a floating window could
        // be closed and take its panel with it.
        false
    }

    fn clear_background(&self, tab: &Panel) -> bool {
        // The viewport paints its own background.
        *tab != Panel::Viewport
    }

    fn scroll_bars(&self, tab: &Panel) -> [bool; 2] {
        // The viewport fills its panel exactly.
        if *tab == Panel::Viewport {
            [false, false]
        } else {
            [true, true]
        }
    }
}

#[cfg(test)]
mod attribute_tests;
#[cfg(test)]
mod basics_tests;
mod cleaning;
#[cfg(test)]
mod cleaning_tests;
mod command;
mod copying;
mod hardness;
#[cfg(test)]
mod hardness_tests;
mod joining;
#[cfg(test)]
mod joining_tests;
mod loop_cut;
#[cfg(test)]
mod loop_cut_tests;
mod modelling;
#[cfg(test)]
mod modelling_tests;
mod parts;
mod selection;
mod shortcuts;
mod showing;
mod slit;
#[cfg(test)]
mod slit_tests;
#[cfg(test)]
mod symmetry_tests;
mod transform;

#[cfg(test)]
mod autosave_tests;
#[cfg(test)]
mod heavy_check;
#[cfg(test)]
mod mode_tests;
#[cfg(test)]
mod object_tests;
#[cfg(test)]
mod project_tests;
#[cfg(test)]
mod select_tests;
#[cfg(test)]
mod settings_tests;
#[cfg(test)]
mod shape_picker_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tool_menu_tests;
#[cfg(test)]
mod transform_tests;
