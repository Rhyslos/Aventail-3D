//! The last edit made (decision 0120): its settings stay in the panel
//! while it is still the object's top edit, and Change it opens it again
//! once something else is selected.

use super::edits::OpenedEdit;
use super::{AttributesPanel, section};
use crate::objects::DocRequest;
use crate::theme;
use crate::transforming::Tool;
use crate::units::Units;
use egui::{RichText, Ui};
use scene::{Document, ObjectId, Operation, Stack};

impl AttributesPanel {
    /// Keeps track of the last edit and the selection it was made with
    /// (decision 0120). Called by the workbench every frame, before
    /// anything that changes the last edit.
    pub(crate) fn track_last_edit(&mut self, doc: &Document) {
        let selection = doc.selection();
        let [id] = selection.as_slice() else {
            return;
        };
        let Some(next) = doc.scene().object(*id).map(|o| o.stack.next_edit_id()) else {
            return;
        };
        if self
            .last_anchors
            .get(id)
            .is_none_or(|(newest, _)| *newest != next)
        {
            self.last_anchors
                .insert(*id, (next, doc.element_revision()));
        }
    }

    /// Takes the selection as it is now as the one the last edit was made
    /// with: for a tool that moves the selection on after its edit (Extrude
    /// moves it to the new outer edges).
    pub(crate) fn keep_last_edit_live(&mut self, doc: &Document) {
        for id in doc.selection() {
            self.last_anchors.remove(&id);
        }
        self.track_last_edit(doc);
    }

    /// Whether object `id`'s last edit can still be changed from the
    /// panel and with Ctrl + wheel (decision 0120): nothing else has been
    /// selected since it was made.
    pub(crate) fn last_edit_is_live(&self, doc: &Document, id: ObjectId) -> bool {
        let Some(next) = doc.scene().object(id).map(|o| o.stack.next_edit_id()) else {
            return false;
        };
        self.last_anchors
            .get(&id)
            .is_some_and(|(newest, revision)| {
                *newest == next && *revision == doc.element_revision()
            })
    }

    /// The last edit's settings, if it is an extrude, inset, bevel or loop
    /// cut: they
    /// can still be changed, and the shape follows (decisions 0038 and
    /// 0040). Once another edit is made on top, it is fixed. Changes here
    /// also become the tool's settings for the next drag.
    pub(super) fn last_edit_section(
        &mut self,
        ui: &mut Ui,
        id: ObjectId,
        stack: &Stack,
        units: Units,
        stretch: glam::Vec3,
        out: &mut Vec<DocRequest>,
    ) {
        let Some((layer, index, operation)) = last_edit(stack) else {
            return;
        };
        if operation.settings().is_none() {
            return;
        }
        section(ui, &format!("Last edit: {}", operation.describe()));
        self.edit_settings(ui, id, (layer, index), operation, units, stretch, out);
    }

    /// Once something else is selected after an edit with settings, a
    /// line instead of its settings (decision 0120): changing them now
    /// would change that edit, not the next one. A button opens it, to
    /// change it on purpose.
    pub(super) fn left_edit_line(
        &mut self,
        ui: &mut Ui,
        doc: &Document,
        id: ObjectId,
        stack: &Stack,
    ) {
        let Some((layer, index, operation)) = last_edit(stack) else {
            return;
        };
        if operation.settings().is_none() {
            return;
        }
        let edit = stack.layers()[layer].entries[index].id;
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("Last edit: {}", operation.describe()))
                    .small()
                    .color(theme::TEXT_WEAK),
            );
            if ui
                .small_button(CHANGE_LAST_EDIT_LABEL)
                .on_hover_text(
                    "You've selected something else since, so the settings above are for your \
                     next use of the tool. Open this edit to change it.",
                )
                .clicked()
            {
                self.opened = Some(OpenedEdit::new(doc, id, edit));
            }
        });
    }
}

/// The button that opens the last edit once something else is selected
/// (decision 0120).
pub(crate) const CHANGE_LAST_EDIT_LABEL: &str = "Change it";

/// The top edit of `stack` (where it is, and what), if any.
pub(crate) fn last_edit(stack: &Stack) -> Option<(usize, usize, &Operation)> {
    let (layer, top) = stack
        .layers()
        .iter()
        .enumerate()
        .rev()
        .find(|(_, l)| !l.entries.is_empty())?;
    let index = top.entries.len() - 1;
    Some((layer, index, &top.entries[index].operation))
}

/// Whether `operation` is the kind of edit `tool` makes.
pub(crate) fn made_by(operation: &Operation, tool: Tool) -> bool {
    matches!(
        (operation.inner(), tool),
        (
            Operation::ExtrudePolygons { .. }
                | Operation::ExtrudeBent { .. }
                | Operation::ExtrudeEdges { .. },
            Tool::Extrude
        ) | (Operation::InsetPolygons { .. }, Tool::Inset)
            | (
                Operation::BevelEdges { .. } | Operation::BevelPoints { .. },
                Tool::Bevel
            )
            | (Operation::Groove { .. }, Tool::Groove)
            | (Operation::LoopCut { .. }, Tool::LoopCut)
            | (Operation::Slit { .. }, Tool::Slit)
            | (Operation::Bridge { .. }, Tool::Bridge)
            | (Operation::Weld { .. }, Tool::Weld)
    )
}
