//! The script being edited in the Scripts panel (plan, milestone 3,
//! choice 11): its text, saved by itself a moment after typing stops;
//! checked on a thread of its own a moment after typing stops, its
//! mistakes underlined; and loaded again when its file is changed by
//! another program (looked at once a second).
//!
//! Checking never runs the script: it only reads it (`script::check`),
//! and never on the interface thread, so typing stays smooth however long
//! the script is.

use super::code_view::{Coloured, gutter_width, paint_line_numbers};
use egui::{Key, Modifiers, Ui};
use script::library::{Stamp, save, stamp};
use script::{Language, Names, Problem, ScriptFile};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

/// How long after the last key a script is checked.
pub(crate) const CHECK_PAUSE: Duration = Duration::from_millis(400);
/// How long after the last key a script is saved.
pub(crate) const SAVE_PAUSE: Duration = Duration::from_secs(1);
/// How often the file is looked at for changes made by other programs.
pub(crate) const LOOK_EVERY: Duration = Duration::from_secs(1);

/// What the editor tells the output.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Notice {
    /// Something to note, in words.
    Note(String),
    /// Something that went wrong, in words.
    Trouble(String),
}

/// The script open in the editor.
pub(crate) struct Editor {
    /// Its file.
    pub(crate) path: PathBuf,
    /// The file's name (`bevel.lsc`), as mistakes name it.
    pub(crate) file_name: String,
    pub(crate) language: Language,
    /// The text being edited.
    pub(crate) text: String,
    /// The text on disk, as last read or written.
    saved: String,
    /// The file's stamp when last read or written.
    stamp: Option<Stamp>,
    /// When the text was last typed in, if it hasn't been saved since.
    typed_at: Option<Instant>,
    /// When a check is due, if the text changed since the last one.
    check_due: Option<Instant>,
    /// The check under way.
    checking: Option<Receiver<Vec<Problem>>>,
    /// When the file was last looked at.
    looked_at: Instant,
    /// The mistakes the last check found.
    pub(crate) problems: Vec<Problem>,
    /// The file changed on disk while there was typing not yet saved:
    /// the person chooses which to keep.
    pub(crate) changed_outside: bool,
    /// The last save that failed, so it's said once.
    save_failed: Option<String>,
    /// A line to put the cursor on, next frame.
    go_to: Option<u32>,
    /// The colouring.
    coloured: Coloured,
    /// The text box's id, once drawn.
    pub(crate) id: Option<egui::Id>,
}

impl Editor {
    /// Opens the script at `path`, its check due at once.
    pub(crate) fn open(path: &Path, now: Instant) -> Result<Self, String> {
        let file = ScriptFile::read(path)?;
        let language = file.language().unwrap_or(Language::Lua);
        Ok(Self {
            path: path.to_path_buf(),
            file_name: file.name,
            language,
            saved: file.text.clone(),
            text: file.text,
            stamp: stamp(path),
            typed_at: None,
            check_due: Some(now),
            checking: None,
            looked_at: now,
            problems: Vec::new(),
            changed_outside: false,
            save_failed: None,
            go_to: None,
            coloured: Coloured::default(),
            id: None,
        })
    }

    /// The script as it is now, as a file to run or check.
    pub(crate) fn file(&self) -> ScriptFile {
        ScriptFile {
            name: self.file_name.clone(),
            text: self.text.clone(),
        }
    }

    /// Whether there's typing not yet saved.
    pub(crate) fn unsaved(&self) -> bool {
        self.text != self.saved
    }

    /// The text was typed in at `now`.
    pub(crate) fn typed(&mut self, now: Instant) {
        self.typed_at = Some(now);
        self.check_due = Some(now + CHECK_PAUSE);
    }

    /// Puts the cursor on `line` (from 1) next time it's drawn.
    pub(crate) fn go_to(&mut self, line: u32) {
        self.go_to = Some(line.max(1));
    }

    /// Saves the text now, if it changed. What went wrong is given back,
    /// once.
    pub(crate) fn save_now(&mut self) -> Option<Notice> {
        self.typed_at = None;
        if !self.unsaved() || self.changed_outside {
            return None;
        }
        match save(&self.path, &self.text) {
            Ok(()) => {
                self.saved = self.text.clone();
                self.stamp = stamp(&self.path);
                self.save_failed = None;
                None
            }
            Err(why) => {
                let said = self.save_failed.as_ref() == Some(&why);
                self.save_failed = Some(why.clone());
                (!said).then_some(Notice::Trouble(why))
            }
        }
    }

    /// Does what's due at `now`: saving after a pause in typing, looking
    /// at the file for changes, starting a check and taking in its
    /// answer. Gives back what the output should say.
    pub(crate) fn tick(&mut self, now: Instant, names: &Names) -> Vec<Notice> {
        let mut notices = Vec::new();
        // Looked at before saving, so a change made outside is never
        // written over unseen.
        if now.duration_since(self.looked_at) >= LOOK_EVERY {
            self.looked_at = now;
            notices.extend(self.look_at_file(now));
        }
        if self
            .typed_at
            .is_some_and(|at| now.duration_since(at) >= SAVE_PAUSE)
        {
            notices.extend(self.save_now());
        }
        self.take_check();
        if self.checking.is_none() && self.check_due.is_some_and(|due| now >= due) {
            self.check_due = None;
            self.start_check(names);
        }
        notices
    }

    /// How long until something is due, for the next frame.
    pub(crate) fn next_due(&self, now: Instant) -> Duration {
        let mut wait = LOOK_EVERY.saturating_sub(now.duration_since(self.looked_at));
        if let Some(at) = self.typed_at {
            wait = wait.min(SAVE_PAUSE.saturating_sub(now.duration_since(at)));
        }
        if let Some(due) = self.check_due {
            wait = wait.min(due.saturating_duration_since(now));
        }
        if self.checking.is_some() {
            wait = wait.min(Duration::from_millis(50));
        }
        wait
    }

    /// Whether a check is under way.
    pub(crate) fn is_checking(&self) -> bool {
        self.checking.is_some()
    }

    /// Starts checking the text as it is now, on a thread of its own.
    fn start_check(&mut self, names: &Names) {
        let (send, answer) = std::sync::mpsc::channel();
        let file = self.file();
        let names = names.clone();
        let started = std::thread::Builder::new()
            .name(r"script editor check".into())
            .spawn(move || {
                let _ = send.send(script::check(&file, &names));
            });
        if started.is_ok() {
            self.checking = Some(answer);
        }
    }

    /// Takes in the check's answer, if it has come.
    fn take_check(&mut self) {
        let Some(answer) = &self.checking else {
            return;
        };
        match answer.try_recv() {
            Ok(problems) => {
                self.problems = problems;
                self.checking = None;
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => self.checking = None,
        }
    }

    /// Looks at the file: if another program changed it, loads it again,
    /// or, while there's typing not yet saved, asks which to keep.
    fn look_at_file(&mut self, now: Instant) -> Option<Notice> {
        let found = stamp(&self.path);
        if found == self.stamp || found.is_none() {
            // Unchanged, or gone: a missing file is written again on the
            // next save.
            if found.is_none() && self.stamp.is_some() {
                self.stamp = None;
                self.saved.clear();
                self.typed_at.get_or_insert(now);
                return Some(Notice::Note(crate::trf!(
                    "{} was removed from its folder by another program: it's still open here, and is saved again.",
                    self.file_name
                )));
            }
            return None;
        }
        let file = match ScriptFile::read(&self.path) {
            Ok(file) => file,
            Err(why) => return Some(Notice::Trouble(why)),
        };
        self.stamp = found;
        if file.text == self.text {
            self.saved = file.text;
            return None;
        }
        if self.unsaved() {
            self.saved = file.text;
            self.changed_outside = true;
            return None;
        }
        self.load(file.text, now);
        Some(Notice::Note(crate::trf!(
            "{} was changed in another program, so it was loaded again.",
            self.file_name
        )))
    }

    /// Takes `text` as the script's, as on disk.
    fn load(&mut self, text: String, now: Instant) {
        self.saved = text.clone();
        self.text = text;
        self.typed_at = None;
        self.check_due = Some(now);
    }

    /// Keeps the text on disk, dropping the typing not yet saved.
    pub(crate) fn take_theirs(&mut self, now: Instant) {
        let theirs = self.saved.clone();
        self.load(theirs, now);
        self.changed_outside = false;
    }

    /// Keeps the typing, writing it over the file.
    pub(crate) fn keep_mine(&mut self) -> Option<Notice> {
        self.changed_outside = false;
        self.save_now()
    }

    /// Draws the editor filling `ui`. Gives back the lines chosen to run
    /// when Ctrl + Enter was pressed in it (the whole script when none
    /// are selected).
    pub(crate) fn show(&mut self, ui: &mut Ui, now: Instant) -> Option<Option<(usize, usize)>> {
        let id = ui.id().with(r"script editor").with(&self.path);
        self.id = Some(id);
        let focused = ui.ctx().memory(|m| m.has_focus(id));
        let mut run = None;
        if focused && ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Enter)) {
            run = Some(selected_lines(&self.text, id, ui.ctx()));
        }
        let font = egui::TextStyle::Monospace.resolve(ui.style());
        let lines = self.text.lines().count().max(1);
        let gutter = gutter_width(ui, lines, &font);
        let Self {
            text,
            language,
            file_name,
            problems,
            coloured,
            ..
        } = self;
        let language = *language;
        let mut layouter = |ui: &Ui, buffer: &dyn egui::TextBuffer, _wrap: f32| {
            coloured.galley(
                ui.ctx(),
                buffer.as_str(),
                language,
                file_name,
                problems,
                &font,
            )
        };
        let output = egui::ScrollArea::both()
            .id_salt(id.with(r"scroll"))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    let (gutter_rect, _) =
                        ui.allocate_exact_size(egui::vec2(gutter, 0.0), egui::Sense::hover());
                    let size = ui.available_size();
                    let output = egui::TextEdit::multiline(text)
                        .id(id)
                        .code_editor()
                        .lock_focus(true)
                        .desired_width(f32::INFINITY)
                        .min_size(size)
                        .frame(egui::Frame::NONE)
                        .layouter(&mut layouter)
                        .show(ui);
                    paint_line_numbers(
                        ui,
                        &output.galley,
                        output.galley_pos,
                        gutter_rect.right() - 6.0,
                        &font_of(ui),
                    );
                    output
                })
                .inner
            })
            .inner;
        if output.response.response.changed() {
            self.typed(now);
        }
        if let Some(line) = self.go_to.take() {
            self.place_cursor(ui, id, line, &output);
        }
        run
    }

    /// Puts the cursor at the start of `line` and shows it.
    fn place_cursor(
        &self,
        ui: &Ui,
        id: egui::Id,
        line: u32,
        output: &egui::text_edit::TextEditOutput,
    ) {
        let starts = super::code_view::line_starts(&self.text);
        let byte = starts
            .get(line as usize - 1)
            .copied()
            .unwrap_or(self.text.len());
        let index = self.text[..byte].chars().count();
        let mut state = output.state.clone();
        let cursor = egui::text::CCursor::new(index);
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::one(cursor)));
        state.store(ui.ctx(), id);
        ui.ctx().memory_mut(|m| m.request_focus(id));
        let row = output.galley.pos_from_cursor(cursor);
        ui.scroll_to_rect(
            row.translate(output.galley_pos.to_vec2()).expand(20.0),
            Some(egui::Align::Center),
        );
    }
}

impl Drop for Editor {
    /// Typing not yet saved is saved when the editor closes (another
    /// script opened, the tab or the app closed).
    fn drop(&mut self) {
        let _ = self.save_now();
    }
}

/// The monospace font in `ui`.
fn font_of(ui: &Ui) -> egui::FontId {
    egui::TextStyle::Monospace.resolve(ui.style())
}

/// The lines selected in the editor `id` (from 1, first and last), or
/// `None` when nothing is selected.
fn selected_lines(text: &str, id: egui::Id, ctx: &egui::Context) -> Option<(usize, usize)> {
    let state = egui::TextEdit::load_state(ctx, id)?;
    let range = state.cursor.char_range()?;
    lines_of(
        text,
        range.primary.index.into(),
        range.secondary.index.into(),
    )
}

/// The lines (from 1) holding the characters between `a` and `b`, or
/// `None` when they're the same place. A selection ending at a line's
/// very start doesn't take that line.
pub(crate) fn lines_of(text: &str, a: usize, b: usize) -> Option<(usize, usize)> {
    if a == b {
        return None;
    }
    let (from, to) = (a.min(b), a.max(b));
    let mut first = 1;
    let mut last = 1;
    let mut previous = None;
    for (index, c) in text.chars().enumerate() {
        if index == to {
            break;
        }
        if c == '\n' {
            if index < from {
                first += 1;
            }
            last += 1;
        }
        previous = Some(c);
    }
    if previous == Some('\n') && last > first {
        last -= 1;
    }
    Some((first, last))
}
