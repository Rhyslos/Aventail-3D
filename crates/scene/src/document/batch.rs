//! Many changes as one undo step, and holding the document while a script
//! works on a copy of it (decision 0302).
//!
//! A script runs on a copy of the document on a thread of its own, and
//! what it did lands on the real one at once, as one step named after
//! the script. While it runs the real document is **held**: every change
//! is turned away ([`SceneError::Held`]), so it is still exactly as the
//! script's copy started when the changes land. Commands are
//! deterministic (DESIGN.md §4.2), so the same changes give the same
//! scene on both.

use super::Document;
use crate::command::{self, Command};
use crate::history::{ObjectsEdit, SelectionEdit, Step};
use crate::object::ObjectId;
use crate::tree::SceneError;
use mesh::ElementSelection;

/// One change in a batch ([`Document::execute_batch`]).
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    /// A command, as [`Document::execute`] carries it out.
    Command(Command),
    /// Selects exactly these objects (unknown ones are ignored), as
    /// [`Document::set_selection`].
    SelectObjects(Vec<ObjectId>),
    /// Gives an object these selected points, edges and polygons, as
    /// [`Document::select_elements`]. Skipped for an object not in the
    /// scene.
    SelectElements(ObjectId, ElementSelection),
}

/// What a batch was refused for: which change (from 0) and why. Nothing
/// it did is kept.
#[derive(Debug, Clone, PartialEq)]
pub struct Refused {
    /// The change refused, counted from 0.
    pub at: usize,
    /// Why.
    pub why: SceneError,
}

impl Document {
    /// Holds the document while a script works on a copy of it: every
    /// change is turned away until [`Self::release`] (the scene, the undo
    /// history and the selection stay as they are).
    pub fn hold(&mut self) {
        self.held = true;
    }

    /// Lets the document change again.
    pub fn release(&mut self) {
        self.held = false;
    }

    /// Whether the document is held ([`Self::hold`]).
    pub fn is_held(&self) -> bool {
        self.held
    }

    pub(super) fn check_not_held(&self) -> Result<(), SceneError> {
        if self.held {
            Err(SceneError::Held)
        } else {
            Ok(())
        }
    }

    /// Carries out `changes` one after another as one undo step called
    /// `label`: commands, and changes to which objects and elements are
    /// selected. If one is refused, everything before it is taken back
    /// too, so nothing is left half done, and the refusal says which. A
    /// batch that changes nothing records no step. Refused while held.
    pub fn execute_batch(
        &mut self,
        changes: impl IntoIterator<Item = Change>,
        label: &str,
    ) -> Result<(), Refused> {
        if self.held {
            return Err(Refused {
                at: 0,
                why: SceneError::Held,
            });
        }
        let order_before = self.order.clone();
        let selection_before = self.selection.clone();
        let elements_before = self.elements.clone();
        let mut edits = Vec::new();
        let mut selections: Vec<SelectionEdit> = Vec::new();
        for (at, change) in changes.into_iter().enumerate() {
            let done = match change {
                Change::Command(command) => {
                    command::apply(&mut self.scene, &command).map(|applied| {
                        edits.extend(applied.edits);
                        if let Some(select) = applied.select {
                            self.put_selection(select);
                        }
                        self.drop_missing_from_selection();
                    })
                }
                Change::SelectObjects(ids) => {
                    let scene = &self.scene;
                    let wanted: Vec<ObjectId> =
                        ids.into_iter().filter(|id| scene.contains(*id)).collect();
                    self.put_selection(wanted);
                    Ok(())
                }
                Change::SelectElements(id, after) => {
                    if self.scene.contains(id) {
                        let before = self.elements.get(&id).cloned().unwrap_or_default();
                        if before != after {
                            self.put_elements(id, after.clone());
                            match selections.iter_mut().find(|s| s.id == id) {
                                Some(earlier) => earlier.after = after,
                                None => selections.push(SelectionEdit { id, before, after }),
                            }
                        }
                    }
                    Ok(())
                }
            };
            if let Err(why) = done {
                for edit in edits.iter().rev() {
                    // Each edit was just applied, so its opposite applies.
                    let _ = self.scene.apply(&edit.inverse());
                }
                self.selection = selection_before;
                self.order = order_before;
                self.elements = elements_before;
                self.element_revision += 1;
                return Err(Refused { at, why });
            }
        }
        selections.retain(|s| s.before != s.after);
        let objects = (self.order != order_before).then(|| ObjectsEdit {
            before: order_before,
            after: self.order.clone(),
        });
        if edits.is_empty() && selections.is_empty() && objects.is_none() {
            return Ok(());
        }
        if !edits.is_empty() {
            self.revision += 1;
        }
        self.changes_made += 1;
        self.history.record(Step {
            label: label.to_string(),
            edits,
            selections,
            objects,
            merge_key: None,
        });
        Ok(())
    }
}

#[cfg(test)]
#[path = "batch_tests.rs"]
mod tests;
