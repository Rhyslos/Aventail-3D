//! The open document: the scene, its undo history and the selection.

use crate::command::{self, Command};
use crate::history::{History, HistoryLimit, Step};
use crate::object::ObjectId;
use crate::tree::{Scene, SceneError};
use std::collections::BTreeSet;

/// Everything about the scene being worked on. The scene only changes
/// through [`Document::execute`], [`Document::undo`] and
/// [`Document::redo`], so every change can be taken back.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Document {
    scene: Scene,
    history: History,
    selection: BTreeSet<ObjectId>,
    /// Goes up whenever the scene changes, so whoever draws it knows when
    /// to rebuild what it shows.
    revision: u64,
}

impl Document {
    /// An empty document.
    pub fn new() -> Self {
        Self::default()
    }

    /// A document holding `scene` (as opened from a file), with `selection`
    /// selected (ids not in the scene are ignored) and an empty history.
    pub fn from_scene(scene: Scene, selection: impl IntoIterator<Item = ObjectId>) -> Self {
        let mut document = Self {
            scene,
            ..Self::default()
        };
        document.set_selection(selection);
        document
    }

    /// The scene, for reading.
    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    /// Carries out a command and records it so it can be undone. A command
    /// that changes nothing (renaming to the same name, say) records no
    /// step. On failure nothing changes and the reason is returned.
    pub fn execute(&mut self, command: Command) -> Result<(), SceneError> {
        self.run(command, None)
    }

    /// Like [`Self::execute`], but merged into the previous step when that
    /// was made with the same `key` and nothing came between: dragging a
    /// value changes it every frame, yet undoes in one go. Use a new key for
    /// each new drag or each new value typed.
    pub fn execute_merging(&mut self, command: Command, key: u64) -> Result<(), SceneError> {
        self.run(command, Some(key))
    }

    fn run(&mut self, command: Command, merge_key: Option<u64>) -> Result<(), SceneError> {
        let label = command.describe(&self.scene);
        let applied = command::apply(&mut self.scene, &command)?;
        if let Some(select) = applied.select {
            self.selection = select.into_iter().collect();
        }
        self.drop_missing_from_selection();
        if !applied.edits.is_empty() {
            self.revision += 1;
            self.history.record(Step {
                label,
                edits: applied.edits,
                merge_key,
            });
        }
        Ok(())
    }

    /// Takes back the last step. Returns what was undone, or `None` if
    /// there was nothing to undo.
    pub fn undo(&mut self) -> Option<String> {
        let step = self.history.take_undo()?;
        for (done, edit) in step.edits.iter().rev().enumerate() {
            if self.scene.apply(&edit.inverse()).is_err() {
                // Cannot happen while every change goes through commands;
                // if it ever does, put the step back as it was rather than
                // leave the scene half undone.
                for edit in step.edits.iter().rev().take(done).rev() {
                    let _ = self.scene.apply(edit);
                }
                self.history.push_undo(step);
                return None;
            }
        }
        self.drop_missing_from_selection();
        self.revision += 1;
        let label = step.label.clone();
        self.history.push_redo(step);
        Some(label)
    }

    /// Does the last undone step again. Returns what was redone, or `None`
    /// if there was nothing to redo.
    pub fn redo(&mut self) -> Option<String> {
        let step = self.history.take_redo()?;
        for (done, edit) in step.edits.iter().enumerate() {
            if self.scene.apply(edit).is_err() {
                for edit in step.edits.iter().take(done).rev() {
                    let _ = self.scene.apply(&edit.inverse());
                }
                self.history.push_redo(step);
                return None;
            }
        }
        self.drop_missing_from_selection();
        self.revision += 1;
        let label = step.label.clone();
        self.history.push_undo(step);
        Some(label)
    }

    /// What Undo would take back, for the Edit menu.
    pub fn undo_label(&self) -> Option<&str> {
        self.history.undo_label()
    }

    /// What Redo would do again, for the Edit menu.
    pub fn redo_label(&self) -> Option<&str> {
        self.history.redo_label()
    }

    /// How many steps can be undone.
    pub fn undo_steps(&self) -> usize {
        self.history.undo_len()
    }

    /// How many undone steps can be done again.
    pub fn redo_steps(&self) -> usize {
        self.history.redo_len()
    }

    /// How much memory the undo history may use. Standard:
    /// [`HistoryLimit::STANDARD`] (2 GB).
    pub fn history_limit(&self) -> HistoryLimit {
        self.history.limit()
    }

    /// Changes how much memory the undo history may use. If the steps
    /// kept no longer fit, the oldest are forgotten at once (the newest
    /// is always kept).
    pub fn set_history_limit(&mut self, limit: HistoryLimit) {
        self.history.set_limit(limit);
    }

    /// Roughly how many bytes the undo history uses now, undo and redo
    /// steps together. An estimate: it counts names, settings, meshes and
    /// recorded edits, but not the memory system's own overheads.
    pub fn history_memory(&self) -> usize {
        self.history.used()
    }

    /// How many steps have been forgotten so far to keep the history
    /// within its limit. Goes up, never down.
    pub fn forgotten_steps(&self) -> usize {
        self.history.forgotten()
    }

    /// A number that goes up every time the scene changes (not the
    /// selection): a cheap way to tell whether anything needs redrawing.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// The history, for tests that check its bookkeeping.
    #[cfg(test)]
    pub(crate) fn history(&self) -> &History {
        &self.history
    }

    /// The selected objects, top to bottom as the Object Manager lists
    /// them.
    pub fn selection(&self) -> Vec<ObjectId> {
        self.scene
            .walk()
            .into_iter()
            .map(|(id, _)| id)
            .filter(|id| self.selection.contains(id))
            .collect()
    }

    /// Whether the object is selected.
    pub fn is_selected(&self, id: ObjectId) -> bool {
        self.selection.contains(&id)
    }

    /// Selects exactly these objects (unknown ones are ignored). Selecting
    /// is not an undo step for now.
    pub fn set_selection(&mut self, ids: impl IntoIterator<Item = ObjectId>) {
        self.selection = ids
            .into_iter()
            .filter(|id| self.scene.contains(*id))
            .collect();
    }

    /// Adds the object to the selection, or takes it out if it was in.
    pub fn toggle_selected(&mut self, id: ObjectId) {
        if !self.selection.remove(&id) && self.scene.contains(id) {
            self.selection.insert(id);
        }
    }

    /// Selects nothing.
    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }

    fn drop_missing_from_selection(&mut self) {
        let scene = &self.scene;
        self.selection.retain(|id| scene.contains(*id));
    }
}
