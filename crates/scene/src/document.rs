//! The open document: the scene, its undo history and the selection.

use crate::command::{self, Command};
use crate::history::{History, Step};
use crate::object::{Object, ObjectId, ObjectKind};
use crate::tree::{Place, Scene, SceneError};
use std::collections::BTreeSet;

/// Everything about the scene being worked on. The scene only changes
/// through [`Document::execute`], [`Document::undo`] and
/// [`Document::redo`], so every change can be taken back.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Document {
    scene: Scene,
    history: History,
    selection: BTreeSet<ObjectId>,
}

impl Document {
    /// An empty document.
    pub fn new() -> Self {
        Self::default()
    }

    /// The scene the app opens with until projects can be opened: the
    /// test cube, with nothing to undo.
    pub fn starting_scene() -> Self {
        let mut document = Self::new();
        let cube = Command::Insert {
            object: Object::new(ObjectKind::TestCube),
            place: Place {
                parent: None,
                index: 0,
            },
        };
        // Inserting into an empty scene cannot fail.
        let _ = document.execute(cube);
        document.history = History::default();
        document.selection.clear();
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
        let label = command.describe(&self.scene);
        let applied = command::apply(&mut self.scene, &command)?;
        if let Some(select) = applied.select {
            self.selection = select.into_iter().collect();
        }
        self.drop_missing_from_selection();
        if !applied.edits.is_empty() {
            self.history.record(Step {
                label,
                edits: applied.edits,
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
