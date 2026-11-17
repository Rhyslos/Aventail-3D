//! Running scripts (decisions 0301 and 0302): the Scripts menu, starting
//! a run against a copy of the scene, taking in what it says each frame,
//! Stop in the status bar and Esc while one runs.
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
use crate::scripts_panel::{Ended, Row};
use crate::text::tr;
use egui::{Key, Modifiers, Ui};
use script::{Landing, Line, Problem, Run, ScriptFile, Snapshot};
use std::path::Path;

/// How often the frame is drawn while a script runs, to show what it says.
const POLL_EVERY: std::time::Duration = std::time::Duration::from_millis(50);

/// The Scripts menu's entry that runs a file.
pub(super) const RUN_FILE_ENTRY: &str = crate::text!("Run a script file…");

impl Workbench {
    /// Reads the script file at `path` and runs it (the app hands it over
    /// after Scripts → Run a script file…). What can't be read is said in
    /// the Scripts panel and the status bar.
    pub fn run_script_file(&mut self, path: &Path) {
        match ScriptFile::read(path) {
            Ok(file) => self.run_script(file),
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
    pub(crate) fn run_script(&mut self, file: ScriptFile) {
        if self.scripts.is_running() {
            self.note = Some(
                tr("A script is already running: stop it first (Esc, or Stop in the status bar).")
                    .to_string(),
            );
            return;
        }
        let name = script::read_header(&file.text).shown_name(&file.name);
        let names = script::Names {
            folder: self.info.script_name.clone(),
            made_by: format!("{} {}", self.info.name, self.info.version),
        };
        let run = Run::start(file, self.script_snapshot(), self.scripts.limits, names);
        self.document.hold();
        crate::layout::show_beside_coordinates(&mut self.dock, Panel::Scripts);
        self.note = Some(crate::trf!("Running {}… (Esc stops it)", name));
        self.scripts.start(run, name);
    }

    /// The copy of the project a script runs against.
    fn script_snapshot(&self) -> Snapshot {
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
        }
    }

    /// Takes in what the running script said since the last frame, and
    /// keeps the frame drawn while it runs.
    /// Called for every tab, the one in use or not, so a run in a tab
    /// left for another still lands when it ends.
    pub(crate) fn poll_scripts(&mut self, ctx: &egui::Context) {
        if let Some(ended) = self.scripts.poll() {
            self.note = Some(ended.words.clone());
            self.land_script(ended);
        }
        if self.scripts.is_running() {
            ctx.request_repaint_after(POLL_EVERY);
        }
    }

    /// Lets the document go once a run has ended, and carries out what
    /// it changed, if it finished well, as one undo step named after it.
    fn land_script(&mut self, ended: Ended) {
        let held = self.document.is_held();
        self.document.release();
        let Some(landing) = ended.landing else {
            return;
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
            return;
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
            }
            Err(refused) => {
                let why = crate::trf!(
                    "{}'s changes couldn't be made, so none were kept: {}.",
                    ended.name,
                    refused.why
                );
                self.scripts.push(Row::Line(Line::Note(why.clone())));
                self.note = Some(why);
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

    /// Esc stops a running script (only while one runs, and not while
    /// typing in a text box, where Esc belongs to the text).
    pub(super) fn script_keys(&mut self, ui: &Ui) {
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
        let shown = self.dock.find_tab(&Panel::Scripts).is_some();
        if ui
            .add(egui::Button::selectable(shown, tr("Scripts panel")))
            .on_hover_text(tr(
                "Show the Scripts panel beside Coordinates, or put it away: what scripts print, and their mistakes.",
            ))
            .clicked()
        {
            match self.dock.find_tab(&Panel::Scripts) {
                Some(path) => {
                    self.dock.remove_tab(path);
                }
                None => crate::layout::show_beside_coordinates(&mut self.dock, Panel::Scripts),
            }
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
        Mode::Material => script::Mode::Material,
    }
}
