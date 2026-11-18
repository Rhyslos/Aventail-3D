//! Running scripts (decisions 0301, 0302 and 0303): the Scripts menu,
//! starting a run against a copy of the scene (a script file, the
//! script being edited, or a console line), taking in what it says each
//! frame, Stop in the status bar, Esc while one runs and F11 for the
//! panel.
//!
//! While a script runs, the project's document is **held**: every change
//! to it is turned away with a note, so it is exactly as the script's copy
//! started when the run ends. A run that ends well lands everything it
//! changed at once, as one undo step named after the script; one that
//! fails, is stopped, or runs out of time or memory lands nothing. Other
//! tabs stay free, and a run in a tab not in use still lands when it ends.

use super::{Workbench, WorkbenchAction};
use crate::palette::Mode;
use crate::panel::Panel;
use crate::scripts_panel::{Ended, Request, Row};
use crate::text::tr;
use egui::{Key, Modifiers, Ui};
use script::{Landing, Line, Problem, ScriptFile, Snapshot};
use std::path::Path;

/// How often the frame is drawn while a script runs, to show what it says.
const POLL_EVERY: std::time::Duration = std::time::Duration::from_millis(50);

/// The key that shows or puts away the Scripts panel (choice 17).
pub(crate) const SCRIPTS_PANEL_KEY: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(Modifiers::NONE, Key::F11);

/// What carrying out a run's changes came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Landed {
    /// It changed nothing (or didn't finish).
    Nothing,
    /// Its changes were made, as one undo step.
    Made,
    /// They couldn't be: none were kept.
    Refused,
}

/// The Scripts menu's entry opening the script manager.
pub(crate) const MANAGER_ENTRY: &str = crate::text!("Script manager…");

/// The Scripts menu's list of example scripts.
pub(crate) const EXAMPLES_ENTRY: &str = crate::text!("Example scripts");

/// The Scripts menu's entry that runs a file.
pub(super) const RUN_FILE_ENTRY: &str = crate::text!("Run a script file…");

impl Workbench {
    /// Reads the script file at `path` and runs it (the app hands it over
    /// after Scripts → Run a script file…). What can't be read is said in
    /// the Scripts panel and the status bar.
    pub fn run_script_file(&mut self, path: &Path) {
        match ScriptFile::read(path) {
            Ok(file) => {
                let name = script::read_header(&file.text).shown_name(&file.name);
                let source = crate::script_sharing::Source::File(path.to_path_buf());
                self.start_run(file, name, None, None, source);
            }
            Err(why) => {
                crate::layout::show_beside_coordinates(&mut self.dock, Panel::Scripts);
                self.scripts.push(Row::Line(Line::Mistake(Problem {
                    file: String::new(),
                    line: 0,
                    column: 0,
                    message: why.clone(),
                })));
                self.note = Some(why);
            }
        }
    }

    /// Runs `file` against a copy of the scene, showing the Scripts panel.
    /// It reaches no files (a file from disk goes through
    /// [`Self::run_script_file`]). For tests: the app runs through
    /// `start_run` with where the script comes from.
    #[cfg(test)]
    pub(crate) fn run_script(&mut self, file: ScriptFile) {
        let name = script::read_header(&file.text).shown_name(&file.name);
        self.run_script_as(file, name, None);
    }

    /// Runs `file`, called `name`, against a copy of the scene, showing
    /// the Scripts panel, reaching no files. `console`: how many lines
    /// were put before what was typed on the console line, when it's from
    /// there. For tests.
    #[cfg(test)]
    pub(crate) fn run_script_as(&mut self, file: ScriptFile, name: String, console: Option<u32>) {
        self.start_run(
            file,
            name,
            console,
            None,
            crate::script_sharing::Source::Console,
        );
    }

    /// The copy of the project a script runs against.
    pub(super) fn script_snapshot(&self) -> Snapshot {
        let scene = self.document.scene();
        let elements = scene
            .walk()
            .into_iter()
            .filter_map(|(id, _)| self.document.elements(id).map(|e| (id, e.clone())))
            .collect();
        Snapshot {
            scene: scene.copy_with_results(),
            selection: self.document.selection_order().to_vec(),
            elements,
            mode: script_mode(self.mode),
            project_name: self.project_name(),
            project_folder: self
                .project_path()
                .and_then(Path::parent)
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            unwrap: self.unwrap_options(None),
            // What it may reach is set once its permissions are known.
            access: script::Access::default(),
            moment: None,
            export: None,
        }
    }

    /// Takes in what the running script said since the last frame, and
    /// keeps the frame drawn while it runs.
    /// Called for every tab, the one in use or not, so a run in a tab
    /// left for another still lands when it ends.
    pub(crate) fn poll_scripts(&mut self, ctx: &egui::Context) {
        if let Some(ended) = self.scripts.poll() {
            self.note = Some(ended.words.clone());
            let ok = ended.ok;
            let moment = self.scripts.moments.running;
            if moment {
                self.moment_run_ended(&ended);
            }
            let landed = self.land_script(ended);
            if !moment {
                self.command_ended(ok && landed != Landed::Refused, landed == Landed::Made);
            }
        }
        self.poll_script_commands();
        self.poll_moments();
        if self.scripts.is_running() {
            ctx.request_repaint_after(POLL_EVERY);
        }
        if let Some(wait) = self.scripts.tick(std::time::Instant::now()) {
            ctx.request_repaint_after(wait);
        }
        for request in std::mem::take(&mut self.scripts.requests) {
            match request {
                Request::Run {
                    file,
                    name,
                    console,
                    source,
                } => {
                    self.start_run(file, name, console, None, source);
                }
                Request::Outside(path) => self.queued.push(WorkbenchAction::OpenOutside { path }),
                Request::Keep(path) => self.keep_with_project(&path),
                Request::TakeOut(name) => self.take_out_of_project(&name),
                Request::CopyIn(name) => self.copy_into_library(&name),
                Request::Export { file, project } => {
                    self.queued.push(WorkbenchAction::ExportScript {
                        file,
                        project,
                        path: None,
                    });
                }
                Request::TurnOn => self.turn_on_project_scripts(),
            }
        }
    }

    /// Shows the Scripts panel, or puts it away (F11, Window → Scripts).
    pub(crate) fn toggle_scripts_panel(&mut self) {
        match self.dock.find_tab(&Panel::Scripts) {
            Some(path) => {
                self.dock.remove_tab(path);
                // Coordinates alone again: back to its slim band.
                crate::layout::fit_coordinates(&mut self.dock);
            }
            None => crate::layout::show_beside_coordinates(&mut self.dock, Panel::Scripts),
        }
    }

    /// Lets the document go once a run has ended, and carries out what
    /// it changed, if it finished well, as one undo step named after it.
    fn land_script(&mut self, ended: Ended) -> Landed {
        let held = self.document.is_held();
        self.document.release();
        let Some(landing) = ended.landing else {
            return Landed::Nothing;
        };
        if !held {
            // The project in this tab was replaced while the script ran
            // (opened again, say): what it did belongs to the old one.
            let why = crate::trf!(
                "{}'s changes weren't kept: the project changed under it while it ran.",
                ended.name
            );
            self.scripts.push(Row::Line(Line::Note(why.clone())));
            self.note = Some(why);
            return Landed::Refused;
        }
        let Landing { changes, scene } = *landing;
        let count = changes.len();
        match self.document.execute_batch(changes, &ended.name) {
            Ok(()) => {
                if *self.document.scene() != scene {
                    // Commands are deterministic, so this can't happen;
                    // if it ever does, the step is still one undo away.
                    self.scripts.push(Row::Line(Line::Note(crate::trf!(
                        "{}'s changes came out differently from how the script saw them. This is a bug; please report it. Ctrl + Z takes them back.",
                        ended.name
                    ))));
                }
                self.note = Some(crate::trf!(
                    "{}: {} changes made, as one step. Ctrl + Z takes them back.",
                    ended.words,
                    count
                ));
                Landed::Made
            }
            Err(refused) => {
                let why = crate::trf!(
                    "{}'s changes couldn't be made, so none were kept: {}.",
                    ended.name,
                    refused.why
                );
                self.scripts.push(Row::Line(Line::Note(why.clone())));
                self.note = Some(why);
                Landed::Refused
            }
        }
    }

    /// Stops the running script, if any.
    pub(crate) fn stop_script(&mut self) {
        if self.scripts.is_running() {
            self.scripts.stop();
            self.note = Some(tr("Stopping the script…").to_string());
        }
    }

    /// F11 shows or puts away the Scripts panel, even while typing (no
    /// text box uses it); Esc stops a running script (only while one
    /// runs, and not while typing in a text box, where Esc belongs to the
    /// text).
    pub(super) fn script_keys(&mut self, ui: &Ui) {
        if !self.window_open()
            && !self.scripts.window_open()
            && ui.input_mut(|i| i.consume_shortcut(&SCRIPTS_PANEL_KEY))
        {
            self.toggle_scripts_panel();
        }
        self.script_command_keys(ui);
        if !self.scripts.is_running() || ui.ctx().memory(|m| m.focused().is_some()) {
            return;
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.stop_script();
        }
    }

    /// The Scripts menu's entries.
    pub(super) fn scripts_menu(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        let running = self.scripts.is_running();
        if ui
            .add_enabled(!running, egui::Button::new(tr(RUN_FILE_ENTRY)))
            .on_hover_text(tr(
                "Run a LustScript (.lsc) or Lua (.lua) file on this project. What it changes lands as one step you can undo; what it prints shows in the Scripts panel.",
            ))
            .on_disabled_hover_text(tr("A script is running: stop it first."))
            .clicked()
        {
            actions.push(WorkbenchAction::RunScript { path: None });
            ui.close();
        }
        if ui
            .add_enabled(
                running,
                egui::Button::new(tr("Stop the script")).shortcut_text(tr("Esc")),
            )
            .on_hover_text(tr("Stop the script that's running"))
            .clicked()
        {
            self.stop_script();
            ui.close();
        }
        ui.separator();
        if ui
            .button(tr(super::script_sharing::INSTALL_ENTRY))
            .on_hover_text(tr(
                "Add a script someone gave you (a .lsc or .lua file) to your library. Its name, what it asks for and its code are shown first; nothing runs.",
            ))
            .clicked()
        {
            actions.push(WorkbenchAction::InstallScript { path: None });
            ui.close();
        }
        self.export_menu(ui, actions);
        self.examples_menu(ui);
        ui.separator();
        self.scripts_panel_entry(ui, tr("Scripts panel"));
        if ui
            .button(tr(MANAGER_ENTRY))
            .on_hover_text(tr(
                "Your library's scripts: what each does and asks for, its commands, their keys and places on the command palette; turn one off, rename, remove or reload it.",
            ))
            .clicked()
        {
            self.scripts.manager.open = true;
            ui.close();
        }
        ui.separator();
        self.script_command_entries(ui);
    }

    /// Export a script: a list of your scripts and the project's, each
    /// saved as a file of its own to give to someone.
    fn export_menu(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        let yours: Vec<String> = self
            .scripts
            .registry
            .scripts
            .iter()
            .map(|s| s.file.clone())
            .collect();
        let theirs: Vec<String> = self
            .scripts
            .project
            .entries
            .iter()
            .map(|e| e.name.clone())
            .collect();
        ui.add_enabled_ui(!yours.is_empty() || !theirs.is_empty(), |ui| {
            ui.menu_button(tr("Export a script"), |ui| {
                for (list, project) in [(yours, false), (theirs, true)] {
                    if project && !list.is_empty() {
                        ui.separator();
                        ui.label(egui::RichText::new(tr("In this project")).weak().small());
                    }
                    for file in list {
                        if ui.button(&file).clicked() {
                            actions.push(WorkbenchAction::ExportScript {
                                file,
                                project,
                                path: None,
                            });
                            ui.close();
                        }
                    }
                }
            })
            .response
            .on_hover_text(tr(
                "Save one of your scripts as a file of its own, to give to someone",
            ));
        });
    }

    /// Example scripts: each added to your library and opened in the
    /// editor (decision 0309).
    fn examples_menu(&mut self, ui: &mut Ui) {
        let examples = script::examples(&self.scripts.names());
        ui.add_enabled_ui(self.scripts.library.is_some(), |ui| {
            ui.menu_button(tr(EXAMPLES_ENTRY), |ui| {
                for example in examples {
                    let header = script::read_header(&example.text);
                    let name = header.shown_name(&example.name);
                    if ui
                        .button(&name)
                        .on_hover_text(header.about.clone().unwrap_or_default())
                        .clicked()
                    {
                        self.add_example(&example);
                        ui.close();
                    }
                }
            })
            .response
            .on_hover_text(tr(
                "Scripts to learn from and change: each is added to your library and opened in the editor. The manual's Scripting page explains each one.",
            ));
        });
    }

    /// Adds `example` to your library (with a number when you have one of
    /// its name) and opens it in the editor.
    pub(crate) fn add_example(&mut self, example: &ScriptFile) {
        let Some(library) = self.scripts.library.clone() else {
            return;
        };
        match library.add(example, &self.scripts.names()) {
            Ok(path) => {
                let now = std::time::Instant::now();
                self.scripts.list_again(now);
                crate::layout::show_beside_coordinates(&mut self.dock, Panel::Scripts);
                self.scripts.edit(&path, now);
                self.note = Some(crate::trf!(
                    "{} is in your library now: read it in the editor, then run it, or find its command in the Scripts menu.",
                    super::files::file_name(&path)
                ));
            }
            Err(why) => self.note = Some(why),
        }
    }

    /// The entry showing or putting away the Scripts panel, in the
    /// Scripts and Window menus.
    pub(super) fn scripts_panel_entry(&mut self, ui: &mut Ui, label: &str) {
        let shown = self.dock.find_tab(&Panel::Scripts).is_some();
        if ui
            .add(
                egui::Button::selectable(shown, label)
                    .shortcut_text(ui.ctx().format_shortcut(&SCRIPTS_PANEL_KEY)),
            )
            .on_hover_text(tr(
                "Show the Scripts panel beside Coordinates, or put it away: your scripts, the editor, what scripts print and the console line.",
            ))
            .clicked()
        {
            self.toggle_scripts_panel();
            ui.close();
        }
    }

    /// The status bar's part for a running script: its progress, if it
    /// says, and Stop. Drawn from the right.
    pub(super) fn script_status(&mut self, ui: &mut Ui) {
        let Some(running) = &self.scripts.running else {
            return;
        };
        let progress = running.progress.clone();
        let name = running.name.clone();
        if ui
            .small_button(tr("Stop"))
            .on_hover_text(crate::trf!("Stop {} (Esc)", name))
            .clicked()
        {
            self.stop_script();
        }
        if let Some((done, what)) = progress {
            ui.add(egui::ProgressBar::new(done).desired_width(120.0).text(what));
        }
        ui.add(egui::Spinner::new().size(12.0));
    }
}

/// The app's mode as scripts see it.
fn script_mode(mode: Mode) -> script::Mode {
    match mode {
        Mode::Model => script::Mode::Model,
        Mode::Object => script::Mode::Object,
        Mode::Points => script::Mode::Points,
        Mode::Edges => script::Mode::Edges,
        Mode::Polygons => script::Mode::Polygons,
        Mode::Uv => script::Mode::Uv,
        Mode::Paint => script::Mode::Paint,
        Mode::Sculpt => script::Mode::Sculpt,
        Mode::Material => script::Mode::Material,
    }
}
