//! The Scripts panel (decisions 0301, 0302 and 0303): your scripts on the
//! left, the script being edited in the middle (coloured, its lines
//! numbered, its mistakes underlined as you type), the output under it,
//! and the console line at the bottom. Also the running script itself,
//! which the workbench asks about each frame.
//!
//! The panel asks the workbench for what it can't do itself (running a
//! script against the project, opening a file in another program) with
//! [`Request`]s, taken each frame.

mod code_view;
mod console;
mod editor;
mod list;
mod output;

pub(crate) use console::Entered;
#[cfg(test)]
pub(crate) use console::History;
pub(crate) use editor::Editor;
pub(crate) use list::GIVE_KEY;
#[cfg(test)]
pub(crate) use list::{ADD_TO_PALETTE, OFF_PALETTE};

use crate::text::tr;
use egui::{RichText, Ui};
use script::library::Library;
use script::{Ending, Event, Landing, Limits, Line, Names, Problem, Run, ScriptFile};
use std::path::PathBuf;
use std::time::Instant;

/// The most rows the output keeps: the oldest go first.
pub(crate) const MAX_ROWS: usize = 5_000;

/// One row of the output.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Row {
    /// A run started: the script's name.
    Started(String),
    /// A line typed on the console.
    Typed(String),
    /// A line a run said, or a note, warning or mistake.
    Line(Line),
    /// How a run ended, in words, and whether it finished well.
    Ended(String, bool),
}

/// What the panel asks of the workbench.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Request {
    /// Run `file`, called `name`; from the console, how many lines were
    /// put before what was typed (taken off its mistakes' lines).
    Run {
        file: ScriptFile,
        name: String,
        console: Option<u32>,
    },
    /// Open this file or folder in the program the computer uses for it.
    Outside(PathBuf),
}

/// A script running now.
pub(crate) struct Running {
    pub(crate) run: Run,
    /// The script's name in words.
    pub(crate) name: String,
    /// How far it has got, and what it's doing, if it said.
    pub(crate) progress: Option<(f32, String)>,
    /// What it changed, once it said (just before it ended).
    landing: Option<Box<Landing>>,
    /// From the console: its file's name, and how many lines were put
    /// before what was typed.
    console: Option<(String, u32)>,
}

/// How a run ended: in words, whether it finished well, and what it
/// changed (to land on the document as one undo step).
pub(crate) struct Ended {
    pub(crate) words: String,
    pub(crate) ok: bool,
    /// The script's name in words, for the undo step.
    pub(crate) name: String,
    pub(crate) landing: Option<Box<Landing>>,
}

/// The panel: the library, the editor, the output, the console line, and
/// the script running now, if any.
#[derive(Default)]
pub(crate) struct ScriptsPanel {
    pub(crate) rows: Vec<Row>,
    pub(crate) running: Option<Running>,
    /// The limits runs get.
    pub(crate) limits: Limits,
    /// The scripting folder's name, for checking.
    pub(crate) names: Option<Names>,
    /// Your script library, when the computer has a place for it.
    pub(crate) library: Option<Library>,
    /// Whether the description files have been written into it.
    prepared: bool,
    /// What went wrong reading or making it.
    pub(crate) library_trouble: Option<String>,
    /// The scripts in it, as last read.
    pub(crate) listed: Vec<PathBuf>,
    listed_at: Option<Instant>,
    /// A new script being named.
    pub(crate) new_script: Option<list::NewScript>,
    /// The script being edited.
    pub(crate) editor: Option<Editor>,
    pub(crate) console: console::Console,
    /// What the workbench is asked for.
    pub(crate) requests: Vec<Request>,
    /// Your library's scripts and the commands they make (decision 0305).
    pub(crate) registry: crate::script_commands::Registry,
    /// What you've set up for scripts: limits, the palette, keys, the
    /// scripts turned off and settings as last used.
    pub(crate) settings: crate::script_commands::ScriptSettings,
    /// The command running now, when the run is one of a script's
    /// commands.
    pub(crate) command_run: Option<crate::script_commands::CommandRun>,
    /// The command whose settings show in Attributes, while its run is
    /// still the last thing done.
    pub(crate) live: Option<crate::script_commands::LiveCommand>,
    /// The command being given a key, in its window.
    pub(crate) asking_key: Option<crate::script_commands::KeyAsk>,
    /// The script manager window.
    pub(crate) manager: crate::script_commands::Manager,
}

impl ScriptsPanel {
    /// A panel checking scripts with `names` and keeping them in
    /// `library`.
    pub(crate) fn new(names: Names, library: Option<PathBuf>) -> Self {
        Self {
            names: Some(names),
            library: library.map(Library::new),
            ..Default::default()
        }
    }

    /// The names for checking and running (empty in a panel made
    /// without).
    pub(crate) fn names(&self) -> Names {
        self.names.clone().unwrap_or_else(|| Names {
            folder: String::new(),
            made_by: String::new(),
        })
    }

    /// Every command of the scripts turned on, by name.
    pub(crate) fn commands(&self) -> Vec<crate::script_commands::ScriptCommand> {
        self.registry.commands(&self.settings)
    }

    /// Whether one of the scripts' windows is open (the script manager,
    /// or a command being given a key): keys go to it.
    pub(crate) fn window_open(&self) -> bool {
        self.manager.open || self.asking_key.is_some()
    }

    /// Starts showing `run` of the script `name`; `console` when it's a
    /// line typed on the console (its file's name and the lines put
    /// before it).
    pub(crate) fn start(&mut self, run: Run, name: String, console: Option<(String, u32)>) {
        if console.is_none() {
            self.push(Row::Started(name.clone()));
        }
        self.running = Some(Running {
            run,
            name,
            progress: None,
            landing: None,
            console,
        });
    }

    /// Takes in what the running script said since the last frame. Gives
    /// back how it ended once it has, with what it changed when it
    /// finished well.
    pub(crate) fn poll(&mut self) -> Option<Ended> {
        let running = self.running.as_mut()?;
        let mut ended = None;
        let mut rows = Vec::new();
        for event in running.run.events() {
            match event {
                Event::Line(Line::Mistake(problem)) => {
                    rows.push(Row::Line(Line::Mistake(placed(problem, &running.console))));
                }
                Event::Line(line) => rows.push(Row::Line(line)),
                Event::Progress(done, what) => running.progress = Some((done, what)),
                Event::Land(landing) => running.landing = Some(landing),
                Event::Ended(ending) => {
                    let ok = ending == Ending::Finished;
                    ended = Some(Ended {
                        words: script::ending_words(&ending, &running.name, self.limits),
                        ok,
                        name: running.name.clone(),
                        landing: if ok { running.landing.take() } else { None },
                    });
                }
            }
        }
        let console = running.console.is_some();
        for row in rows {
            self.push(row);
        }
        if let Some(ended) = &ended {
            // A console line that went well says nothing more.
            if !(console && ended.ok) {
                self.push(Row::Ended(ended.words.clone(), ended.ok));
            }
            self.running = None;
        }
        ended
    }

    /// Asks the running script to stop.
    pub(crate) fn stop(&mut self) {
        if let Some(running) = &self.running {
            running.run.stop();
        }
    }

    /// Whether a script is running.
    pub(crate) fn is_running(&self) -> bool {
        self.running.is_some()
    }

    /// Adds a row, letting the oldest go past [`MAX_ROWS`].
    pub(crate) fn push(&mut self, row: Row) {
        self.rows.push(row);
        if self.rows.len() > MAX_ROWS {
            let extra = self.rows.len() - MAX_ROWS;
            self.rows.drain(..extra);
        }
    }

    /// Puts what the editor said in the output.
    fn say(&mut self, notice: Option<editor::Notice>) {
        match notice {
            Some(editor::Notice::Note(words)) => self.push(Row::Line(Line::Note(words))),
            Some(editor::Notice::Trouble(words)) => self.say_trouble(words),
            None => {}
        }
    }

    /// Puts something that went wrong in the output.
    fn say_trouble(&mut self, words: String) {
        self.push(Row::Line(Line::Mistake(Problem {
            file: String::new(),
            line: 0,
            column: 0,
            message: words,
        })));
    }

    /// Does what's due without drawing: saving and checking the script
    /// being edited, and looking at its file. Called every frame, shown
    /// or not; gives back how soon to look again.
    pub(crate) fn tick(&mut self, now: Instant) -> Option<std::time::Duration> {
        let names = self.names();
        let editor = self.editor.as_mut()?;
        let notices = editor.tick(now, &names);
        let wait = editor.next_due(now);
        for notice in notices {
            self.say(Some(notice));
        }
        Some(wait)
    }

    /// Runs the script being edited: the lines chosen, or all of it.
    fn run_edited(&mut self, lines: Option<(usize, usize)>) {
        let Some(editor) = self.editor.as_mut() else {
            return;
        };
        let notice = editor.save_now();
        let whole = editor.file();
        self.say(notice);
        if self.is_running() {
            self.push(Row::Line(Line::Note(
                tr("A script is already running: stop it first (Esc).").to_string(),
            )));
            return;
        }
        let name = script::read_header(&whole.text).shown_name(&whole.name);
        let (file, name) = match lines {
            Some((first, last)) => (
                script::only_lines(&whole, first, last),
                if first == last {
                    crate::trf!("{} (line {})", name, first)
                } else {
                    crate::trf!("{} (lines {} to {})", name, first, last)
                },
            ),
            None => (whole, name),
        };
        self.requests.push(Request::Run {
            file,
            name,
            console: None,
        });
    }

    /// Runs a line typed on the console.
    fn run_typed(&mut self, typed: String) {
        let (file, before) = script::console_script(&typed, self.console.language, &self.names());
        self.push(Row::Typed(typed));
        self.requests.push(Request::Run {
            file,
            name: tr("Console line").to_string(),
            console: Some(before),
        });
    }

    /// Draws the panel.
    pub(crate) fn show(&mut self, ui: &mut Ui) {
        let now = Instant::now();
        self.refresh_list(now);
        let id = ui.id();
        egui::Panel::left(id.with(r"scripts list"))
            .resizable(true)
            .default_size(170.0)
            .size_range(110.0..=400.0)
            .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(0, 2)))
            .show(ui, |ui| self.show_list(ui, now));
        let running = self.is_running();
        let entered = egui::Panel::bottom(id.with(r"scripts console"))
            .resizable(false)
            .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(6, 4)))
            .show(ui, |ui| self.console.show(ui, running))
            .inner;
        match entered {
            Some(Entered::Line(typed)) => self.run_typed(typed),
            Some(Entered::Busy) => self.push(Row::Line(Line::Note(
                tr("A script is running: the line runs when you press Enter again after it ends.")
                    .to_string(),
            ))),
            None => {}
        }
        egui::Panel::bottom(id.with(r"scripts output"))
            .resizable(true)
            .default_size(90.0)
            .size_range(36.0..=600.0)
            .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(6, 2)))
            .show(ui, |ui| self.show_output(ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(4, 2)))
            .show(ui, |ui| self.show_editor(ui, now));
    }

    /// The editor, or what to do when no script is open.
    fn show_editor(&mut self, ui: &mut Ui, now: Instant) {
        let Some(editor) = self.editor.as_mut() else {
            ui.add_space(8.0);
            ui.label(
                RichText::new(tr(
                    "Double-click a script on the left to edit it here, or make a new one with +. Scripts save themselves as you type.",
                ))
                .weak(),
            );
            return;
        };
        let mut notice = None;
        let (mut close, mut outside, mut run_all, mut first_mistake) = (false, false, false, false);
        let running = self.running.is_some();
        ui.horizontal(|ui| {
            ui.label(RichText::new(&editor.file_name).strong());
            let weak = ui.visuals().weak_text_color();
            let red = ui.visuals().error_fg_color;
            let first = editor.problems.first();
            let (words, colour) = match (editor.problems.len(), editor.is_checking()) {
                (0, true) => (tr("Checking…").to_string(), weak),
                (0, false) => (tr("No mistakes found").to_string(), weak),
                (n, _) => {
                    let place = first.map(|p| p.to_string()).unwrap_or_default();
                    if n == 1 {
                        (place, red)
                    } else {
                        (crate::trf!("{} (and {} more)", place, n - 1), red)
                    }
                }
            };
            // Room is left for the buttons on the right.
            let room = (ui.available_width() - 330.0).max(80.0);
            let status = ui.add_sized(
                [room, ui.spacing().interact_size.y],
                egui::Label::new(RichText::new(words).color(colour))
                    .truncate()
                    .sense(egui::Sense::click()),
            );
            if let Some(first) = first {
                first_mistake = status
                    .on_hover_text(crate::trf!("{} (click to go there)", first))
                    .clicked();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                close = ui
                    .small_button("×")
                    .on_hover_text(tr("Close the script (it's saved)"))
                    .clicked();
                outside = ui
                    .small_button(tr("Open in your own editor"))
                    .on_hover_text(tr("Open it in the program your computer uses for this kind of file. Changes saved there are loaded here."))
                    .clicked();
                run_all = ui
                    .add_enabled(!running, egui::Button::new(tr("Run")).small())
                    .on_hover_text(tr("Run the whole script. Ctrl + Enter in the editor runs it too, or only the lines selected."))
                    .clicked();
            });
        });
        if first_mistake && let Some(line) = editor.problems.first().map(|p| p.line) {
            editor.go_to(line);
        }
        if outside {
            notice = editor.save_now();
            self.requests.push(Request::Outside(editor.path.clone()));
        }
        if editor.changed_outside {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(tr(
                        "This script was changed in another program while you were typing here.",
                    ))
                    .color(ui.visuals().warn_fg_color),
                );
                if ui.button(tr("Load theirs")).clicked() {
                    editor.take_theirs(now);
                }
                if ui.button(tr("Keep mine")).clicked() {
                    notice = editor.keep_mine();
                }
            });
        }
        let run = editor.show(ui, now);
        self.say(notice);
        if close {
            self.editor = None;
        } else if run_all {
            self.run_edited(None);
        } else if let Some(lines) = run {
            self.run_edited(lines);
        }
    }
}

/// A mistake from the console, at its line among the lines typed (the
/// lines put before them taken off).
fn placed(mut problem: Problem, console: &Option<(String, u32)>) -> Problem {
    if let Some((file, before)) = console
        && problem.file == *file
    {
        problem.line = problem.line.saturating_sub(*before);
        if problem.line == 0 {
            problem.column = 0;
        }
    }
    problem
}

#[cfg(test)]
mod tests;
