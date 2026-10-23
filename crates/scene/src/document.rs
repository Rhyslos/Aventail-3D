//! The open document: the scene, its undo history and the selection.

use crate::command::{self, Command};
use crate::history::{History, HistoryLimit, ObjectsEdit, SelectionEdit, Step};
use crate::object::ObjectId;
use crate::tree::{Scene, SceneError};
use mesh::ElementSelection;
use std::collections::{BTreeMap, BTreeSet};
use words::{tr, trf};

/// Everything about the scene being worked on. The scene only changes
/// through [`Document::execute`], [`Document::undo`] and
/// [`Document::redo`], so every change can be taken back.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Document {
    scene: Scene,
    history: History,
    selection: BTreeSet<ObjectId>,
    /// Each object's selected points, edges and polygons (decision 0029).
    /// Objects with nothing selected have no entry.
    elements: BTreeMap<ObjectId, ElementSelection>,
    /// Goes up whenever the scene changes, so whoever draws it knows when
    /// to rebuild what it shows.
    revision: u64,
    /// Goes up whenever the selected elements change.
    element_revision: u64,
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
        let scene = &document.scene;
        document.selection = selection
            .into_iter()
            .filter(|id| scene.contains(*id))
            .collect();
        document
    }

    /// The same document measured in `units`, for a project being opened
    /// (not a step to undo: use [`Command::SetUnits`] for that).
    #[must_use]
    pub fn with_units(mut self, units: crate::Units) -> Self {
        self.scene = std::mem::take(&mut self.scene).with_units(units);
        self
    }

    /// The same document with texture settings `texture`, for a project
    /// being opened (not a step to undo: use [`Command::SetTexture`]).
    #[must_use]
    pub fn with_texture(mut self, texture: crate::TextureSettings) -> Self {
        self.scene = std::mem::take(&mut self.scene).with_texture(texture);
        self
    }

    /// The same document with mending settings `mending`, for a project
    /// being opened (not a step to undo: use [`Command::SetMending`]).
    #[must_use]
    pub fn with_mending(mut self, mending: crate::MendOptions) -> Self {
        self.scene = std::mem::take(&mut self.scene).with_mending(mending);
        self
    }

    /// The same document with layout snapshots `snapshots`, for a project
    /// being opened (not a step to undo: use [`Command::SetSnapshots`]).
    #[must_use]
    pub fn with_snapshots(mut self, snapshots: Vec<crate::UvSnapshot>) -> Self {
        self.scene = std::mem::take(&mut self.scene).with_snapshots(snapshots);
        self
    }

    /// The same document with object `id`'s smoothing and mirror space set,
    /// for a project being opened (see [`crate::Scene::with_object_settings`]).
    pub fn with_object_settings(
        mut self,
        id: ObjectId,
        hardness: mesh::Hardness,
        mirror_on_object: bool,
    ) -> Result<Self, &'static str> {
        self.scene =
            std::mem::take(&mut self.scene).with_object_settings(id, hardness, mirror_on_object)?;
        Ok(self)
    }

    /// The same document with object `id` given back the primitive it was
    /// built from (see [`Scene::with_primitive`]), for a project being
    /// opened.
    pub fn with_primitive(
        mut self,
        id: ObjectId,
        primitive: crate::Primitive,
    ) -> Result<Self, &'static str> {
        self.scene = std::mem::take(&mut self.scene).with_primitive(id, primitive)?;
        Ok(self)
    }

    /// The same document with trim sheet `trim`, for a project being
    /// opened (not a step to undo: use [`Command::SetTrim`]).
    #[must_use]
    pub fn with_trim(mut self, trim: crate::TrimSheet) -> Self {
        self.scene = std::mem::take(&mut self.scene).with_trim(trim);
        self
    }

    /// The same document exporting to `target`, for a project being
    /// opened (not a step to undo: use [`Command::SetExportTarget`]).
    #[must_use]
    pub fn with_export_target(mut self, target: Option<crate::ExportTarget>) -> Self {
        self.scene = std::mem::take(&mut self.scene).with_export_target(target);
        self
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

    /// Like [`Self::execute`], with a label of the caller's choosing for
    /// the Edit menu ("Rotate Cube"), and a `merge_key` as in
    /// [`Self::execute_merging`] if wanted.
    pub fn execute_labelled(
        &mut self,
        command: Command,
        label: impl Into<String>,
        merge_key: Option<u64>,
    ) -> Result<(), SceneError> {
        self.run_labelled(command, Some(label.into()), merge_key)
    }

    fn run(&mut self, command: Command, merge_key: Option<u64>) -> Result<(), SceneError> {
        self.run_labelled(command, None, merge_key)
    }

    fn run_labelled(
        &mut self,
        command: Command,
        label: Option<String>,
        merge_key: Option<u64>,
    ) -> Result<(), SceneError> {
        let label = label.unwrap_or_else(|| command.describe(&self.scene));
        let before: Vec<ObjectId> = self.selection.iter().copied().collect();
        let applied = command::apply(&mut self.scene, &command)?;
        if let Some(select) = applied.select {
            self.selection = select.into_iter().collect();
        }
        self.drop_missing_from_selection();
        if !applied.edits.is_empty() {
            self.revision += 1;
            // A step that changes the selection itself (Create, Delete,
            // Group, Duplicate…) keeps the change, so undo and redo put
            // the selection back as it was before and after.
            let after: Vec<ObjectId> = self.selection.iter().copied().collect();
            let objects = (after != before).then_some(ObjectsEdit { before, after });
            self.history.record(Step {
                label,
                edits: applied.edits,
                selections: Vec::new(),
                objects,
                merge_key,
            });
        }
        Ok(())
    }

    /// Takes back the last step if it was made with `merge_key`, and
    /// forgets it: it can't be redone (decision 0030). A drag of the
    /// gizmo uses this to replace what it did a frame ago with what it
    /// does now, and to cancel. Returns whether a step was taken back.
    pub fn abandon(&mut self, merge_key: u64) -> bool {
        if self.history.last_merge_key() != Some(merge_key) {
            return false;
        }
        let Some(step) = self.history.take_undo() else {
            return false;
        };
        if !self.revert(&step) {
            self.history.push_undo(step);
            return false;
        }
        true
    }

    /// Applies the step's edits backwards. On failure, leaves the scene as
    /// it was and returns `false`.
    fn revert(&mut self, step: &Step) -> bool {
        for (done, edit) in step.edits.iter().rev().enumerate() {
            if self.scene.apply(&edit.inverse()).is_err() {
                for edit in step.edits.iter().rev().take(done).rev() {
                    let _ = self.scene.apply(edit);
                }
                return false;
            }
        }
        for change in step.selections.iter().rev() {
            self.put_elements(change.id, change.before.clone());
        }
        if let Some(change) = &step.objects {
            self.selection = change.before.iter().copied().collect();
        }
        self.drop_missing_from_selection();
        if !step.edits.is_empty() {
            self.revision += 1;
        }
        true
    }

    /// Takes back the last step. Returns what was undone, or `None` if
    /// there was nothing to undo.
    pub fn undo(&mut self) -> Option<String> {
        let step = self.history.take_undo()?;
        if !self.revert(&step) {
            // Cannot happen while every change goes through commands; if
            // it ever does, the step is put back as it was rather than
            // leave the scene half undone.
            self.history.push_undo(step);
            return None;
        }
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
        for change in &step.selections {
            self.put_elements(change.id, change.after.clone());
        }
        if let Some(change) = &step.objects {
            self.selection = change.after.iter().copied().collect();
        }
        self.drop_missing_from_selection();
        if !step.edits.is_empty() {
            self.revision += 1;
        }
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

    /// The selected objects, top to bottom as the Objects panel lists
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

    /// Selects exactly these objects (unknown ones are ignored), as an
    /// undo step (decision 0029). Selecting what is already selected
    /// records nothing.
    pub fn set_selection(&mut self, ids: impl IntoIterator<Item = ObjectId>) {
        self.select_objects(ids, None);
    }

    /// Like [`Self::set_selection`], with a `merge_key` that folds the
    /// change into the previous step made with the same key (a brush
    /// stroke). Returns whether the selection changed.
    pub fn select_objects(
        &mut self,
        ids: impl IntoIterator<Item = ObjectId>,
        merge_key: Option<u64>,
    ) -> bool {
        let after: BTreeSet<ObjectId> = ids
            .into_iter()
            .filter(|id| self.scene.contains(*id))
            .collect();
        if after == self.selection {
            return false;
        }
        let before: Vec<ObjectId> = self.selection.iter().copied().collect();
        self.selection = after;
        let label = self.selection_label();
        self.history.record(Step {
            label,
            edits: Vec::new(),
            selections: Vec::new(),
            objects: Some(ObjectsEdit {
                before,
                after: self.selection.iter().copied().collect(),
            }),
            merge_key,
        });
        true
    }

    /// What a selection step is called: "Select Cube", "Select 3
    /// objects" or "Deselect all".
    fn selection_label(&self) -> String {
        match self.selection.len() {
            0 => tr("Deselect all").to_string(),
            1 => match self
                .selection
                .iter()
                .next()
                .and_then(|id| self.scene.object(*id))
            {
                Some(object) => trf!("Select {}", object.name),
                None => tr("Select 1 object").to_string(),
            },
            n => trf!("Select {} objects", n),
        }
    }

    /// Adds the object to the selection, or takes it out if it was in (an
    /// undo step).
    pub fn toggle_selected(&mut self, id: ObjectId) {
        let mut next = self.selection.clone();
        if !next.remove(&id) {
            next.insert(id);
        }
        self.select_objects(next, None);
    }

    /// Selects nothing (an undo step, if anything was selected).
    pub fn clear_selection(&mut self) {
        self.select_objects([], None);
    }

    /// The object's selected points, edges and polygons, as last set:
    /// it may name elements its shape no longer has (see
    /// [`ElementSelection::valid_in`]). `None` when nothing is selected.
    pub fn elements(&self, id: ObjectId) -> Option<&ElementSelection> {
        self.elements.get(&id)
    }

    /// Changes the selected elements of one or more objects, as one undo
    /// step called `label` ("Select 4 points"). Like
    /// [`Self::execute_merging`], a `merge_key` folds the change into the
    /// previous step made with the same key, so a whole brush stroke
    /// undoes in one go. Objects not in the scene are skipped; when nothing
    /// changes, no step is recorded. Returns whether anything changed.
    pub fn select_elements(
        &mut self,
        changes: impl IntoIterator<Item = (ObjectId, ElementSelection)>,
        label: impl Into<String>,
        merge_key: Option<u64>,
    ) -> bool {
        let mut edits = Vec::new();
        for (id, after) in changes {
            if !self.scene.contains(id) {
                continue;
            }
            let before = self.elements.get(&id).cloned().unwrap_or_default();
            if before != after {
                self.put_elements(id, after.clone());
                edits.push(SelectionEdit { id, before, after });
            }
        }
        if edits.is_empty() {
            return false;
        }
        self.history.record(Step {
            label: label.into(),
            edits: Vec::new(),
            selections: edits,
            objects: None,
            merge_key,
        });
        true
    }

    /// A number that goes up every time the selected elements change
    /// (through [`Self::select_elements`], undo or redo).
    pub fn element_revision(&self) -> u64 {
        self.element_revision
    }

    fn put_elements(&mut self, id: ObjectId, selection: ElementSelection) {
        if selection.is_empty() {
            self.elements.remove(&id);
        } else {
            self.elements.insert(id, selection);
        }
        self.element_revision += 1;
    }

    fn drop_missing_from_selection(&mut self) {
        let scene = &self.scene;
        self.selection.retain(|id| scene.contains(*id));
        let before = self.elements.len();
        self.elements.retain(|id, _| scene.contains(*id));
        if self.elements.len() != before {
            self.element_revision += 1;
        }
    }
}
