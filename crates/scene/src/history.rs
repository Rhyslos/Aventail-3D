//! The undo and redo lists.
//!
//! For now every step is kept for the whole session. Limiting the history
//! by the memory it uses, rather than by a number of steps, is a later
//! Phase 2 milestone (DESIGN.md §4.3).

use crate::tree::Edit;

/// One undoable step: a command's edits, in the order they were applied.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Step {
    /// What the step did, for the Edit menu ("Delete 2 objects").
    pub(crate) label: String,
    pub(crate) edits: Vec<Edit>,
}

/// Steps that can be undone, and undone steps that can be redone.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct History {
    undo: Vec<Step>,
    redo: Vec<Step>,
}

impl History {
    /// Records a newly made step. Anything that could be redone is
    /// forgotten, as in every editor.
    pub(crate) fn record(&mut self, step: Step) {
        self.redo.clear();
        self.undo.push(step);
    }

    pub(crate) fn take_undo(&mut self) -> Option<Step> {
        self.undo.pop()
    }

    pub(crate) fn take_redo(&mut self) -> Option<Step> {
        self.redo.pop()
    }

    /// Puts an undone step on the redo list.
    pub(crate) fn push_redo(&mut self, step: Step) {
        self.redo.push(step);
    }

    /// Puts a redone step back on the undo list, keeping the redo list.
    pub(crate) fn push_undo(&mut self, step: Step) {
        self.undo.push(step);
    }

    pub(crate) fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|s| s.label.as_str())
    }

    pub(crate) fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|s| s.label.as_str())
    }

    /// How many steps can be undone.
    pub(crate) fn undo_len(&self) -> usize {
        self.undo.len()
    }
}
