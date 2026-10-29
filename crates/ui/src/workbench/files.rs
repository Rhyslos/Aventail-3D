//! Project files: the window's title, saving and opening (with the
//! answers the app sends back), recent projects, and the "Save changes?"
//! question before work would be lost.

use super::exporting::TargetOutcome;
use super::{Workbench, WorkbenchAction, exporting};
use crate::project::{NEW_SHORTCUT, OPEN_SHORTCUT, Pending, SAVE_AS_SHORTCUT, SAVE_SHORTCUT};
use crate::text::tr;
use egui::{Key, KeyboardShortcut, Modifiers, Ui};
use scene::Document;
use std::path::{Path, PathBuf};

impl Workbench {
    /// The window title: the project's name, then the app's name and
    /// version. With unsaved changes, a dot goes in front and "(unsaved)"
    /// after the name, so it shows at a glance even in the taskbar.
    pub fn title(&self) -> String {
        let (dot, changed) = if self.has_unsaved_changes() {
            ("● ", tr(" (unsaved)"))
        } else {
            ("", "")
        };
        crate::trf!(
            "{}{}{} — {} {}",
            dot,
            self.file.name(),
            changed,
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
        self.saved_with_export(path, None);
    }

    /// The app saved the project to `path`, and then exported it into its
    /// game project folder (decision 0076), with this outcome: the
    /// file written and what it held, or why not.
    pub fn saved_with_export(&mut self, path: PathBuf, export: Option<TargetOutcome>) {
        self.saved_at(path, self.document.revision(), export);
    }

    /// The app saved the project as it was at `revision` to `path` (on
    /// another thread, decision 0163), then exported it into its game
    /// project folder with `export`'s outcome, if it does that on every
    /// save. Changes made since are still unsaved.
    pub fn saved_at(&mut self, path: PathBuf, revision: u64, export: Option<TargetOutcome>) {
        self.file_work_done();
        let mut note = crate::trf!("Saved {}.", file_name(&path));
        if revision != self.document.revision() {
            note.push(' ');
            note.push_str(tr("Changes made while it was saving aren't in it yet."));
        }
        if let Some(outcome) = export {
            note.push(' ');
            note.push_str(&exporting::target_note(&outcome));
        }
        self.note = Some(note);
        self.file.saved_revision = revision;
        self.file.remember(&path);
        self.file.path = Some(path);
        if let Some(next) = self.file.after_save.take() {
            self.go_ahead(next);
        }
    }

    /// Saving failed: says why, and stops whatever was waiting for it.
    pub fn save_failed(&mut self, reason: &str) {
        self.file_work_done();
        self.note = Some(crate::trf!("Couldn't save: {}.", reason));
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
        self.opened_noting(document, path, &[]);
    }

    /// [`Self::opened`], saying in the status bar what was left out so the
    /// rest could open (`notes`, each a sentence without its full stop,
    /// from `fileio::project::open_noting`).
    pub fn opened_noting(&mut self, document: Document, path: PathBuf, notes: &[String]) {
        self.replace_document(document);
        let mut note = crate::trf!("Opened {}.", file_name(&path));
        for left_out in notes {
            note.push(' ');
            note.push_str(left_out);
            note.push('.');
        }
        self.note = Some(note);
        self.file.remember(&path);
        self.file.path = Some(path);
        self.viewport.frame(crate::Frame::All);
    }

    /// Opening `path` failed: says why. A recent project that can no
    /// longer be found is taken off the list.
    pub fn open_failed(&mut self, path: &Path, reason: &str) {
        self.note = Some(crate::trf!(
            "Couldn't open {}: {}.",
            file_name(path),
            reason
        ));
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
    pub(super) fn ask_then(&mut self, next: Pending, actions: &mut Vec<WorkbenchAction>) {
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
        if self.waits_for_file_work(&next) {
            return;
        }
        match next {
            Pending::New => {
                self.replace_document(self.new_document());
                self.file.path = None;
                self.note = Some(tr("New project.").into());
            }
            Pending::Open(path) => self.queued.push(WorkbenchAction::Open { path }),
            Pending::Quit => self.queued.push(WorkbenchAction::Quit),
        }
    }

    /// Puts `document` in place of the current one, keeping the undo
    /// memory limit, and starts the view and panels afresh.
    pub(super) fn replace_document(&mut self, mut document: Document) {
        document.set_history_limit(self.undo_memory.limit());
        self.file.saved_revision = document.revision();
        self.document = document;
        self.autosaver.restart();
        self.drawing.forget();
        self.components.forget_collision();
        self.hidden.faces.clear();
        self.hidden.revision += 1;
        self.copied.clear();
        self.forgotten_told = false;
        // Maps baked for the last project aren't this one's.
        self.cancel_bake();
        self.bakes = Default::default();
    }

    /// The "Save changes before …?" question, while asked.
    pub(super) fn unsaved_question(
        &mut self,
        ctx: &egui::Context,
        actions: &mut Vec<WorkbenchAction>,
    ) {
        let Some(pending) = self.file.asking.clone() else {
            return;
        };
        let mut answer = None;
        let discard = ctx.format_shortcut(&DISCARD_SHORTCUT);
        let keys = crate::trf!("Enter: save · {}: don't save · Esc: cancel", discard);
        crate::dialog::Dialog::new(crate::trf!("Save changes to {}?", self.file.name()), 440.0)
            .id("unsaved_changes")
            .modal()
            .show(ctx, |d| {
                d.header(false, |_| {});
                d.body(|ui| {
                    ui.label(crate::trf!(
                        "Your changes will be lost if you don't save them before {}.",
                        pending.before()
                    ));
                    crate::dialog::help(ui, &keys);
                });
                d.footer(None, |ui| {
                    if crate::dialog::main_button(ui, tr("Save")).clicked() {
                        answer = Some(UnsavedAnswer::Save);
                    }
                    if crate::dialog::plain_button(ui, tr("Cancel")).clicked() {
                        answer = Some(UnsavedAnswer::Cancel);
                    }
                    if crate::dialog::plain_button(ui, tr("Don't save")).clicked() {
                        answer = Some(UnsavedAnswer::Discard);
                    }
                });
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
    pub(super) fn file_keys(&mut self, ui: &Ui, actions: &mut Vec<WorkbenchAction>) {
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
}

/// The answers to "Save changes?".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnsavedAnswer {
    Save,
    Discard,
    Cancel,
}

/// "Don't save" in the "Save changes?" question, from the keyboard.
pub const DISCARD_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::ALT, Key::N);

/// A path's file name, for messages.
pub(super) fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}
