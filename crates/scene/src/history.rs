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
    /// Steps made one after another with the same key become one step:
    /// a whole drag of a value is one undo, not one per frame.
    pub(crate) merge_key: Option<u64>,
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
        if let (Some(key), Some(last)) = (step.merge_key, self.undo.last_mut())
            && last.merge_key == Some(key)
        {
            for edit in step.edits {
                merge_edit(&mut last.edits, edit);
            }
            if last.edits.iter().all(Edit::changes_nothing) {
                // Dragged back to where it started: nothing to undo.
                self.undo.pop();
            }
            return;
        }
        self.undo.push(step);
    }

    /// Takes the last step to undo it. The step before it is closed: a
    /// later change never merges into a step that undo has gone back to.
    pub(crate) fn take_undo(&mut self) -> Option<Step> {
        let mut step = self.undo.pop()?;
        step.merge_key = None;
        if let Some(previous) = self.undo.last_mut() {
            previous.merge_key = None;
        }
        Some(step)
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

/// Adds `edit` to a step's edits, folding a settings change into an
/// earlier one for the same object (keeping where it started and where it
/// ended), so a long drag stays one small step.
fn merge_edit(edits: &mut Vec<Edit>, edit: Edit) {
    if let Edit::SetKind { id, after, .. } = &edit
        && let Some(Edit::SetKind { after: earlier, .. }) = edits
            .iter_mut()
            .find(|e| matches!(e, Edit::SetKind { id: other, .. } if other == id))
    {
        earlier.clone_from(after);
        return;
    }
    edits.push(edit);
}
