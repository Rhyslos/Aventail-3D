//! The undo and redo lists, kept within a memory budget rather than a
//! number of steps (DESIGN.md §4.3, decision
//! [0022](../../../docs/decisions/0022-undo-memory-budget.md)): a hundred
//! renames cost next to nothing, while one step holding a large mesh can
//! cost a lot, so counting steps would be either too strict or useless.
//!
//! When the history grows past its budget, the oldest steps are forgotten
//! first. The newest step is always kept, however large, so the last
//! change can always be undone.

use crate::memory;
use crate::object::ObjectId;
use crate::tree::Edit;
use mesh::ElementSelection;
use std::collections::VecDeque;

/// How much memory the undo history may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryLimit {
    /// At most this many bytes (roughly; see [`crate::Document::history_memory`]).
    Bytes(usize),
    /// Keep every step, however much memory that takes.
    Unlimited,
}

impl HistoryLimit {
    /// The standard budget: 2 GB, enough for a long session of modelling
    /// on a computer with 8 GB or more.
    pub const STANDARD: Self = Self::Bytes(2 << 30);

    /// Whether `used` bytes are more than this limit allows.
    fn exceeded_by(self, used: usize) -> bool {
        match self {
            Self::Bytes(limit) => used > limit,
            Self::Unlimited => false,
        }
    }
}

impl Default for HistoryLimit {
    fn default() -> Self {
        Self::STANDARD
    }
}

/// A change to one object's selected points, edges and polygons.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SelectionEdit {
    pub(crate) id: ObjectId,
    pub(crate) before: ElementSelection,
    pub(crate) after: ElementSelection,
}

/// A change to which objects are selected.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ObjectsEdit {
    pub(crate) before: Vec<ObjectId>,
    pub(crate) after: Vec<ObjectId>,
}

/// One undoable step: a command's edits, in the order they were applied,
/// or a change to the selection: of elements, or of objects (decision
/// 0029).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Step {
    /// What the step did, for the Edit menu ("Delete 2 objects").
    pub(crate) label: String,
    pub(crate) edits: Vec<Edit>,
    /// Changes to selected elements, applied after `edits`.
    pub(crate) selections: Vec<SelectionEdit>,
    /// A change to which objects are selected.
    pub(crate) objects: Option<ObjectsEdit>,
    /// Steps made one after another with the same key become one step:
    /// a whole drag of a value is one undo, not one per frame.
    pub(crate) merge_key: Option<u64>,
}

/// A step with its measured size, so the total can be kept without
/// measuring everything again.
#[derive(Debug, Clone, PartialEq)]
struct Kept {
    step: Step,
    size: usize,
}

impl Kept {
    fn new(step: Step) -> Self {
        let size = memory::step(&step);
        Self { step, size }
    }
}

/// Steps that can be undone, and undone steps that can be redone.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct History {
    /// Oldest first.
    undo: VecDeque<Kept>,
    /// The next step to redo last.
    redo: Vec<Kept>,
    /// The size of every kept step, undo and redo.
    used: usize,
    limit: HistoryLimit,
    /// How many steps have been forgotten to stay within the limit.
    forgotten: usize,
}

impl History {
    /// Records a newly made step. Anything that could be redone is
    /// forgotten, as in every editor.
    pub(crate) fn record(&mut self, step: Step) {
        for gone in self.redo.drain(..) {
            self.used -= gone.size;
        }
        if let (Some(key), Some(last)) = (step.merge_key, self.undo.back_mut())
            && last.step.merge_key == Some(key)
        {
            for edit in step.edits {
                merge_edit(&mut last.step.edits, edit);
            }
            for change in step.selections {
                merge_selection(&mut last.step.selections, change);
            }
            if let Some(change) = step.objects {
                match &mut last.step.objects {
                    Some(earlier) => earlier.after = change.after,
                    None => last.step.objects = Some(change),
                }
            }
            last.step.label = step.label;
            self.used -= last.size;
            if last.step.edits.iter().all(Edit::changes_nothing)
                && last.step.selections.iter().all(|s| s.before == s.after)
                && last
                    .step
                    .objects
                    .as_ref()
                    .is_none_or(|o| o.before == o.after)
            {
                // Dragged back to where it started: nothing to undo.
                self.undo.pop_back();
            } else {
                last.size = memory::step(&last.step);
                self.used += last.size;
            }
        } else {
            let kept = Kept::new(step);
            self.used += kept.size;
            self.undo.push_back(kept);
        }
        self.trim();
    }

    /// Takes the last step to undo it. The step before it is closed: a
    /// later change never merges into a step that undo has gone back to.
    pub(crate) fn take_undo(&mut self) -> Option<Step> {
        let kept = self.undo.pop_back()?;
        self.used -= kept.size;
        if let Some(previous) = self.undo.back_mut() {
            previous.step.merge_key = None;
        }
        let mut step = kept.step;
        step.merge_key = None;
        Some(step)
    }

    pub(crate) fn take_redo(&mut self) -> Option<Step> {
        let kept = self.redo.pop()?;
        self.used -= kept.size;
        Some(kept.step)
    }

    /// Puts an undone step on the redo list.
    pub(crate) fn push_redo(&mut self, step: Step) {
        let kept = Kept::new(step);
        self.used += kept.size;
        self.redo.push(kept);
    }

    /// Puts a redone step back on the undo list, keeping the redo list.
    pub(crate) fn push_undo(&mut self, step: Step) {
        let kept = Kept::new(step);
        self.used += kept.size;
        self.undo.push_back(kept);
    }

    pub(crate) fn undo_label(&self) -> Option<&str> {
        self.undo.back().map(|k| k.step.label.as_str())
    }

    pub(crate) fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|k| k.step.label.as_str())
    }

    /// How many steps can be undone.
    pub(crate) fn undo_len(&self) -> usize {
        self.undo.len()
    }

    /// How many steps can be redone.
    pub(crate) fn redo_len(&self) -> usize {
        self.redo.len()
    }

    /// Roughly how many bytes the kept steps take up.
    pub(crate) fn used(&self) -> usize {
        self.used
    }

    pub(crate) fn limit(&self) -> HistoryLimit {
        self.limit
    }

    /// Changes the budget, forgetting old steps at once if they no longer
    /// fit.
    pub(crate) fn set_limit(&mut self, limit: HistoryLimit) {
        self.limit = limit;
        self.trim();
    }

    /// How many steps have been forgotten to stay within the budget.
    pub(crate) fn forgotten(&self) -> usize {
        self.forgotten
    }

    /// The size of every kept step, measured again from scratch (to check
    /// the running total).
    #[cfg(test)]
    pub(crate) fn measured(&self) -> usize {
        self.undo
            .iter()
            .chain(&self.redo)
            .map(|k| memory::step(&k.step))
            .sum()
    }

    /// Forgets steps until the history fits its budget: the oldest undo
    /// steps first, then the redo steps furthest away. The newest undo
    /// step is always kept.
    fn trim(&mut self) {
        while self.limit.exceeded_by(self.used) && self.undo.len() > 1 {
            if let Some(gone) = self.undo.pop_front() {
                self.used -= gone.size;
                self.forgotten += 1;
            }
        }
        while self.limit.exceeded_by(self.used) && !self.redo.is_empty() {
            let gone = self.redo.remove(0);
            self.used -= gone.size;
            self.forgotten += 1;
        }
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

/// Adds a selection change to a step's, folding it into an earlier one for
/// the same object (keeping where it started and where it ended), so a
/// whole brush stroke is one small step.
fn merge_selection(changes: &mut Vec<SelectionEdit>, change: SelectionEdit) {
    if let Some(earlier) = changes.iter_mut().find(|c| c.id == change.id) {
        earlier.after = change.after;
    } else {
        changes.push(change);
    }
}
