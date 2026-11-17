//! The Scripts panel (decision 0301): for now only the output of the
//! scripts run in this project, what they printed, their warnings and
//! their mistakes with file and line, and how each run ended. The editor
//! and the console line come later (plan, milestone 3). Also the running
//! script itself, which the workbench asks about each frame.

use crate::text::tr;
use egui::{RichText, Ui};
use script::{Ending, Event, Limits, Line, Run};

/// The most rows the output keeps: the oldest go first.
pub(crate) const MAX_ROWS: usize = 5_000;

/// One row of the output.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Row {
    /// A run started: the script's name.
    Started(String),
    /// A line a run said, or a note, warning or mistake.
    Line(Line),
    /// How a run ended, in words, and whether it finished well.
    Ended(String, bool),
}

/// A script running now.
pub(crate) struct Running {
    pub(crate) run: Run,
    /// The script's name in words.
    pub(crate) name: String,
    /// How far it has got, and what it's doing, if it said.
    pub(crate) progress: Option<(f32, String)>,
}

/// The output, and the script running now, if any.
#[derive(Default)]
pub(crate) struct ScriptsPanel {
    pub(crate) rows: Vec<Row>,
    pub(crate) running: Option<Running>,
    /// The limits runs get.
    pub(crate) limits: Limits,
}

impl ScriptsPanel {
    /// Starts showing `run` of the script `name`.
    pub(crate) fn start(&mut self, run: Run, name: String) {
        self.push(Row::Started(name.clone()));
        self.running = Some(Running {
            run,
            name,
            progress: None,
        });
    }

    /// Takes in what the running script said since the last frame. Gives
    /// back how it ended, in words, once it has.
    pub(crate) fn poll(&mut self) -> Option<(String, bool)> {
        let running = self.running.as_mut()?;
        let mut ended = None;
        let mut rows = Vec::new();
        for event in running.run.events() {
            match event {
                Event::Line(line) => rows.push(Row::Line(line)),
                Event::Progress(done, what) => running.progress = Some((done, what)),
                Event::Ended(ending) => {
                    let words = script::ending_words(&ending, &running.name, self.limits);
                    ended = Some((words, ending == Ending::Finished));
                }
            }
        }
        for row in rows {
            self.push(row);
        }
        if let Some((words, ok)) = &ended {
            self.push(Row::Ended(words.clone(), *ok));
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

    /// Draws the panel.
    pub(crate) fn show(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr("Output")).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(!self.rows.is_empty(), egui::Button::new(tr("Clear")))
                    .on_hover_text(tr("Clear the output"))
                    .clicked()
                {
                    self.rows.clear();
                }
            });
        });
        ui.separator();
        if self.rows.is_empty() {
            ui.label(
                RichText::new(tr(
                    "What scripts print, and their mistakes, show here. Run one with Scripts → Run a script file…",
                ))
                .weak(),
            );
            return;
        }
        let warning = ui.visuals().warn_fg_color;
        let mistake = ui.visuals().error_fg_color;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for row in &self.rows {
                    let text = match row {
                        Row::Started(name) => {
                            RichText::new(crate::trf!("Running {}", name)).strong()
                        }
                        Row::Line(Line::Said(text)) => RichText::new(text).monospace(),
                        Row::Line(Line::Warning(text)) => {
                            RichText::new(crate::trf!("Warning: {}", text)).color(warning)
                        }
                        Row::Line(Line::Note(text)) => RichText::new(text).weak(),
                        Row::Line(Line::Mistake(problem)) => {
                            RichText::new(problem.to_string()).color(mistake)
                        }
                        Row::Ended(words, true) => RichText::new(words).weak(),
                        Row::Ended(words, false) => RichText::new(words).color(mistake),
                    };
                    ui.add(egui::Label::new(text).wrap());
                }
            });
    }
}
