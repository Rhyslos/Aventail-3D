//! The console line under the output (plan, milestone 3): type a line,
//! press Enter to run it, Up and Down to go through earlier lines,
//! Shift + Enter for a new line. Each run is a run of its own, on a copy
//! of the scene, landing as one undo step; nothing carries over from one
//! line to the next (choice 31).

use crate::text::tr;
use egui::{Key, KeyboardShortcut, Modifiers, Ui};
use script::Language;

/// The most earlier lines kept.
pub(crate) const MAX_HISTORY: usize = 100;

/// The lines run before, and where Up and Down have got to.
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct History {
    /// Earlier lines, oldest first.
    lines: Vec<String>,
    /// The line shown, while going through them: an index into `lines`.
    at: Option<usize>,
    /// What was being typed before going back, given back past the
    /// newest.
    draft: String,
}

impl History {
    /// Keeps `line`, run just now (not again straight after itself, and
    /// not when only space), and starts again from the newest.
    pub(crate) fn push(&mut self, line: &str) {
        self.at = None;
        self.draft.clear();
        if line.trim().is_empty() || self.lines.last().is_some_and(|l| l == line) {
            return;
        }
        self.lines.push(line.to_string());
        if self.lines.len() > MAX_HISTORY {
            self.lines.remove(0);
        }
    }

    /// One line further back from `typed` (what's on the line now), if
    /// there is one.
    pub(crate) fn back(&mut self, typed: &str) -> Option<String> {
        let at = match self.at {
            None if self.lines.is_empty() => return None,
            None => {
                self.draft = typed.to_string();
                self.lines.len() - 1
            }
            Some(0) => return None,
            Some(at) => at - 1,
        };
        self.at = Some(at);
        self.lines.get(at).cloned()
    }

    /// One line forward, and past the newest what was being typed.
    pub(crate) fn forward(&mut self) -> Option<String> {
        let at = self.at?;
        if at + 1 < self.lines.len() {
            self.at = Some(at + 1);
            self.lines.get(at + 1).cloned()
        } else {
            self.at = None;
            Some(std::mem::take(&mut self.draft))
        }
    }

    /// The earlier lines, oldest first.
    #[cfg(test)]
    pub(crate) fn lines(&self) -> &[String] {
        &self.lines
    }
}

/// What Enter on the console line asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Entered {
    /// Run this.
    Line(String),
    /// A script is running: the line is kept, to run once it ends.
    Busy,
}

/// The console line.
pub(crate) struct Console {
    /// What's typed.
    pub(crate) text: String,
    /// The language it's typed in.
    pub(crate) language: Language,
    pub(crate) history: History,
    /// The cursor goes to the end of the line next frame: after Up or
    /// Down.
    move_to_end: bool,
    /// The text box's id, once drawn.
    pub(crate) id: Option<egui::Id>,
}

impl Default for Console {
    fn default() -> Self {
        Self {
            text: String::new(),
            language: Language::LustScript,
            history: History::default(),
            move_to_end: false,
            id: None,
        }
    }
}

impl Console {
    /// Draws the console line. Gives back what Enter asked for, when it
    /// was pressed on it (`running` turns it away while a script runs).
    pub(crate) fn show(&mut self, ui: &mut Ui, running: bool) -> Option<Entered> {
        let id = ui.id().with(r"script console");
        self.id = Some(id);
        let focused = ui.ctx().memory(|m| m.has_focus(id));
        let mut entered = None;
        if focused {
            entered = self.keys(ui, id, running);
        }
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt(id.with(r"language"))
                .width(84.0)
                .selected_text(language_name(self.language))
                .show_ui(ui, |ui| {
                    for language in [Language::LustScript, Language::Lua] {
                        ui.selectable_value(&mut self.language, language, language_name(language));
                    }
                })
                .response
                .on_hover_text(tr("The language the console line is written in"));
            let rows = self.text.lines().count().clamp(1, 6);
            let output = egui::TextEdit::multiline(&mut self.text)
                .id(id)
                .code_editor()
                .desired_rows(rows)
                .desired_width(f32::INFINITY)
                .return_key(KeyboardShortcut::new(Modifiers::SHIFT, Key::Enter))
                .hint_text(tr(
                    "Type a line and press Enter to run it. Up and Down go through earlier lines; Shift + Enter adds a line.",
                ))
                .show(ui);
            if std::mem::take(&mut self.move_to_end) {
                let mut state = output.state.clone();
                let end = egui::text::CCursor::new(self.text.chars().count());
                state
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::one(end)));
                state.store(ui.ctx(), id);
            }
        });
        entered
    }

    /// Enter, Up and Down while the line has the keyboard.
    fn keys(&mut self, ui: &mut Ui, id: egui::Id, running: bool) -> Option<Entered> {
        if ui.input_mut(|i| plain(i) && i.consume_key(Modifiers::NONE, Key::Enter)) {
            if self.text.trim().is_empty() {
                return None;
            }
            if running {
                return Some(Entered::Busy);
            }
            let typed = std::mem::take(&mut self.text);
            self.history.push(&typed);
            return Some(Entered::Line(typed));
        }
        let (on_first, on_last) = cursor_rows(&self.text, id, ui.ctx());
        if on_first && ui.input_mut(|i| plain(i) && i.consume_key(Modifiers::NONE, Key::ArrowUp)) {
            if let Some(line) = self.history.back(&self.text) {
                self.text = line;
                self.move_to_end = true;
            }
        } else if on_last && ui.input_mut(|i| plain(i) && i.consume_key(Modifiers::NONE, Key::ArrowDown)) {
            if let Some(line) = self.history.forward() {
                self.text = line;
                self.move_to_end = true;
            }
        }
        None
    }
}

/// Whether no Shift or Alt is held: egui's own matching lets them
/// through, and Shift + Enter must add a line, Shift + Up select.
fn plain(input: &egui::InputState) -> bool {
    !input.modifiers.shift && !input.modifiers.alt
}

/// Whether the console's cursor is on its first line, and on its last:
/// Up and Down go through earlier lines only from there, and move the
/// cursor otherwise.
fn cursor_rows(text: &str, id: egui::Id, ctx: &egui::Context) -> (bool, bool) {
    let index = egui::TextEdit::load_state(ctx, id)
        .and_then(|s| s.cursor.char_range())
        .map_or(text.chars().count(), |r| r.primary.index.into());
    let before = text.chars().take(index).filter(|c| *c == '\n').count();
    let all = text.matches('\n').count();
    (before == 0, before == all)
}

/// A language's name, as the console's choice shows it.
pub(crate) fn language_name(language: Language) -> &'static str {
    match language {
        Language::LustScript => tr("LustScript"),
        Language::Lua => tr("Lua"),
    }
}
