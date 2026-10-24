//! The object's edits in the Attributes panel: the Edits section (how many,
//! and any that no longer fit the shape), and an edit opened from the Edit
//! layers panel, with its settings to change in place, Close, Bake into
//! shape up to here and Delete edit (decision 0092).

use super::tool_rows::{
    angle_row, bend_rows, bevel_option_rows, bridge_rows, cuts_row, extrude_rows, groove_rows,
    keep_together, position_row, profile_row, segments_row, through_row,
};
use super::tool_section::{HardAngle, grouping_of};
use super::{AttributesPanel, LABEL_WIDTH, label, section, stepper};
use crate::objects::DocRequest;
use crate::text::tr;
use crate::theme;
use crate::units::Units;
use egui::{RichText, Ui, vec2};
use mesh::primitives::MAX_SIZE;
use scene::{Command, Document, EditId, Grouping, ObjectId, Operation, Stack};

/// An edit opened from the edit list (decision 0092).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OpenedEdit {
    /// Its object.
    pub(crate) object: ObjectId,
    /// The edit.
    pub(crate) edit: EditId,
    /// The id the object's next edit would get when it was opened: once a
    /// new edit is made, the panel goes back to showing the last edit.
    newest: EditId,
}

impl OpenedEdit {
    /// Whether this is edit `edit` of object `object`, still open (no new
    /// edit made since).
    pub(crate) fn is(&self, doc: &Document, object: ObjectId, edit: EditId) -> bool {
        self.object == object
            && self.edit == edit
            && doc
                .scene()
                .object(object)
                .is_some_and(|o| o.stack.next_edit_id() == self.newest)
    }

    /// Edit `edit` of object `object`, opened now.
    pub(crate) fn new(doc: &Document, object: ObjectId, edit: EditId) -> Self {
        let newest = doc
            .scene()
            .object(object)
            .map_or(edit, |o| o.stack.next_edit_id());
        Self {
            object,
            edit,
            newest,
        }
    }
}

impl AttributesPanel {
    /// The edit opened from the edit list on object `id`, if it is still
    /// there.
    pub(crate) fn opened_edit(&self, doc: &Document, id: ObjectId) -> Option<EditId> {
        let opened = self.opened?;
        let stack = &doc.scene().object(id)?.stack;
        (opened.object == id
            && stack.entry(opened.edit).is_some()
            && stack.next_edit_id() == opened.newest)
            .then_some(opened.edit)
    }

    /// An edit opened from the edit list (decision 0092): its switches,
    /// its settings (if it has any), and Freeze up to here and Delete edit.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn opened_edit_section(
        &mut self,
        ui: &mut Ui,
        doc: &Document,
        id: ObjectId,
        edit: EditId,
        stack: &Stack,
        units: Units,
        stretch: glam::Vec3,
        out: &mut Vec<DocRequest>,
    ) {
        let Some((layer, index, entry)) = stack.entry(edit) else {
            return;
        };
        let entry = entry.clone();
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(crate::trf!("Edit: {}", entry.operation.describe())).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button(tr(CLOSE_EDIT_LABEL))
                    .on_hover_text(tr("Go back to showing the last edit"))
                    .clicked()
                {
                    self.opened = None;
                }
            });
        });
        ui.separator();
        let layer_name = stack.layers()[layer].name.clone();
        let total = stack.layers()[layer].entries.len();
        ui.label(
            RichText::new(crate::trf!(
                "Edit {} of {} in {}. Change it and the edits after it are made \
                 again on the new shape.",
                index + 1,
                total,
                layer_name
            ))
            .small()
            .color(theme::TEXT_WEAK),
        );
        if let Some(problem) = doc
            .scene()
            .evaluated(id)
            .and_then(|e| edit_warning(&e.warnings, layer, index))
        {
            ui.label(RichText::new(problem).color(theme::WARNING));
        }
        ui.push_id("opened_edit_switches", |ui| {
            egui::Grid::new("attributes_opened_edit")
                .num_columns(2)
                .min_col_width(LABEL_WIDTH)
                .spacing(vec2(12.0, 6.0))
                .show(ui, |ui| {
                    label(
                        ui,
                        tr("On"),
                        tr("Off: the shape is made as if this edit wasn't there. Nothing is lost."),
                    );
                    let mut on = entry.enabled;
                    let switch = ui.checkbox(&mut on, "");
                    switch.widget_info(|| {
                        egui::WidgetInfo::selected(
                            egui::WidgetType::Checkbox,
                            true,
                            on,
                            tr("Edit on"),
                        )
                    });
                    if switch.changed() {
                        out.push(DocRequest::Run(Command::SetEditEnabled {
                            id,
                            edit,
                            enabled: on,
                        }));
                    }
                    ui.end_row();
                });
        });
        if entry.operation.settings().is_some() {
            self.edit_settings(
                ui,
                id,
                (layer, index),
                &entry.operation,
                units,
                stretch,
                out,
            );
        } else {
            ui.label(
                RichText::new(tr("This kind of edit has no settings to change."))
                    .small()
                    .color(theme::TEXT_WEAK),
            );
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let problem = stack.freeze_problem(edit);
            let count = scene::frozen_count(stack, edit);
            let help = match problem {
                Some(why) => crate::trf!("Can't bake into the shape: {}.", why),
                None => crate::trf!(
                    "Bake the shape's settings and the {} up to and including this one into \
                     the shape, so they no longer have to be made again: big models stay quick. \
                     The shape doesn't change, but those edits can't be changed any more.",
                    if count == 1 {
                        tr("edit").to_string()
                    } else {
                        crate::trf!("{} edits", count)
                    }
                ),
            };
            if ui
                .add_enabled(problem.is_none(), egui::Button::new(tr(FREEZE_LABEL)))
                .on_hover_text(&help)
                .on_disabled_hover_text(&help)
                .clicked()
            {
                out.push(DocRequest::Run(Command::Freeze { id, edit }));
                self.opened = None;
            }
            if ui
                .button(tr(DELETE_EDIT_LABEL))
                .on_hover_text(tr(
                    "Take this edit out. The edits after it are made again without it \
                     (Ctrl + Z brings it back).",
                ))
                .clicked()
            {
                out.push(DocRequest::Run(Command::DeleteEdit { id, edit }));
                self.opened = None;
            }
        });
    }

    /// The settings rows of the edit at `at` (layer, place), changed in
    /// place with [`Command::ReplaceOperation`]: the Last edit section's,
    /// and an edit opened from the edit list's (decision 0092).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn edit_settings(
        &mut self,
        ui: &mut Ui,
        id: ObjectId,
        (layer, index): (usize, usize),
        operation: &Operation,
        units: Units,
        stretch: glam::Vec3,
        out: &mut Vec<DocRequest>,
    ) {
        let Some(settings) = operation.settings() else {
            return;
        };
        let mut edited = settings;
        let mut changed = false;
        // A mirrored edit shows the edit's own settings.
        let kind = operation.inner();
        let bevel = matches!(
            kind,
            Operation::BevelEdges { .. } | Operation::BevelPoints { .. }
        );
        let low = match kind {
            Operation::ExtrudePolygons { .. }
            | Operation::ExtrudeEdges { .. }
            | Operation::Groove { .. } => -MAX_SIZE,
            _ => 0.0,
        };
        // Its own id scope: its Width field mustn't share ids with the
        // shape's Width below.
        ui.push_id("last_edit", |ui| {
            egui::Grid::new("attributes_last_edit")
                .num_columns(2)
                .min_col_width(LABEL_WIDTH)
                .spacing(vec2(12.0, 6.0))
                .show(ui, |ui| {
                    if let Some(mut cuts) = edited.cuts {
                        let touched = cuts_row(ui, &mut cuts);
                        if touched.began {
                            self.gesture += 1;
                        }
                        if touched.changed {
                            edited.cuts = Some(cuts);
                            if matches!(kind, Operation::Connect { .. }) {
                                self.join.connect_cuts = cuts;
                            } else {
                                self.loop_cut.cuts = cuts;
                            }
                            changed = true;
                        }
                        ui.end_row();
                    }
                    if let Some(mut angle) = edited.angle {
                        let touched = angle_row(ui, &mut angle);
                        if touched.began {
                            self.gesture += 1;
                        }
                        if touched.changed {
                            edited.angle = Some(angle);
                            self.hard_angle = HardAngle(angle);
                            changed = true;
                        }
                        ui.end_row();
                    }
                    if let Some(mut bridge) = edited.bridge {
                        let touched = bridge_rows(ui, &mut bridge);
                        if touched.began {
                            self.gesture += 1;
                        }
                        if touched.changed {
                            edited.bridge = Some(bridge);
                            self.join.bridge = bridge;
                            changed = true;
                        }
                    }
                    if let Some(mut through) = edited.through {
                        if through_row(ui, &mut through) {
                            self.gesture += 1;
                            edited.through = Some(through);
                            self.slit_through = through;
                            changed = true;
                        }
                        ui.end_row();
                    }
                    if let Some(mut position) = edited.position {
                        let touched = position_row(ui, &mut position);
                        if touched.began {
                            self.gesture += 1;
                        }
                        if touched.changed {
                            edited.position = Some(position);
                            changed = true;
                        }
                        ui.end_row();
                    }
                    let Some(mut distance) = edited.distance else {
                        return;
                    };
                    let groove = matches!(kind, Operation::Groove { .. });
                    let (name, help) = if bevel {
                        (
                            tr("Width"),
                            tr("How far the bevel reaches into each polygon (in the object's own \
                         size). Change it and the shape follows."),
                        )
                    } else if groove {
                        (
                            tr("Depth"),
                            tr("How far the groove goes in (below 0) or the ridge comes out (above \
                             0), in the object's own size. Change it and the shape follows."),
                        )
                    } else {
                        (
                            tr("Distance"),
                            tr("How far it was pushed (in the object's own size). Change it and the \
                         shape follows."),
                        )
                    };
                    label(ui, name, help);
                    let touched = stepper::length(ui, &mut distance, (low, MAX_SIZE), units, name);
                    if touched.began {
                        self.gesture += 1;
                    }
                    if touched.changed {
                        edited.distance = Some(distance);
                        changed = true;
                    }
                    ui.end_row();
                    if let Some(grouping) = edited.grouping {
                        let mut together = grouping == Grouping::Together;
                        if keep_together(ui, &mut together) {
                            self.gesture += 1;
                            self.grouping = grouping_of(together);
                            edited.grouping = Some(self.grouping);
                            changed = true;
                        }
                        ui.end_row();
                    }
                    if let Some(mut shape) = edited.extrude {
                        let touched = extrude_rows(ui, &mut shape, true);
                        if touched.began {
                            self.gesture += 1;
                        }
                        if touched.changed {
                            edited.extrude = Some(shape);
                            self.extrude = shape;
                            changed = true;
                        }
                    } else if matches!(kind, Operation::ExtrudeBent { .. }) {
                        // The shape rows stay, greyed out, so the bend rows
                        // below don't jump up the moment the Angle leaves 0.
                        // They show the shape it takes if the bend is taken
                        // back to nothing: then it becomes a straight
                        // extrude shaped as the tool is.
                        let mut shape = self.extrude;
                        extrude_rows(ui, &mut shape, false);
                        edited.extrude = Some(self.extrude);
                    }
                    if let Some(bend) = edited.bend {
                        // A straight extrude shows the tool's bend, turned
                        // to no angle: give it an angle and it bends so.
                        let mut bend = if matches!(kind, Operation::ExtrudeBent { .. }) {
                            bend
                        } else {
                            scene::Bend {
                                angle: 0.0,
                                ..self.bend
                            }
                        };
                        let touched = bend_rows(ui, &mut bend, units);
                        if touched.began {
                            self.gesture += 1;
                        }
                        if touched.changed {
                            edited.bend = Some(bend);
                            // The rest carries on to the next extrude, but
                            // not the turn: that starts straight.
                            self.bend = scene::Bend { angle: 0.0, ..bend };
                            changed = true;
                        }
                    }
                    if let Some(mut segments) = edited.segments {
                        let touched = segments_row(ui, &mut segments);
                        if touched.began {
                            self.gesture += 1;
                        }
                        if touched.changed {
                            edited.segments = Some(segments);
                            self.bevel.segments = segments;
                            changed = true;
                        }
                        ui.end_row();
                    }
                    if let Some(mut profile) = edited.profile {
                        let touched = profile_row(ui, &mut profile);
                        if touched.began {
                            self.gesture += 1;
                        }
                        if touched.changed {
                            edited.profile = Some(profile);
                            self.bevel.profile = profile;
                            changed = true;
                        }
                        ui.end_row();
                    }
                    if let Some(mut shape) = edited.groove {
                        let touched = groove_rows(ui, &mut shape.width, &mut shape.sides, units);
                        if touched.began {
                            self.gesture += 1;
                        }
                        if touched.changed {
                            edited.groove = Some(shape);
                            self.groove.sides = shape.sides;
                            changed = true;
                        }
                    }
                    if let Some(mut options) = edited.bevel_options {
                        let touched = bevel_option_rows(ui, &mut options, units);
                        if touched.began {
                            self.gesture += 1;
                        }
                        if touched.changed {
                            edited.bevel_options = Some(options);
                            // The next bevel is always improved, even
                            // when this one was made before (decision 0141).
                            self.bevel.options = scene::BevelOptions {
                                improved: true,
                                ..options
                            };
                            changed = true;
                        }
                    }
                });
        });
        // Any change is worked out as the object is stretched now, so the
        // edit keeps its shape as seen (decisions 0081 and 0082).
        edited.stretch = Some(stretch);
        // A bevel made before decision 0141 is improved once changed.
        if changed {
            edited = edited.improved();
        }
        let after = operation.with(edited);
        if changed && after != *operation {
            out.push(DocRequest::RunMerging(
                Command::ReplaceOperation {
                    id,
                    layer,
                    index,
                    operation: after,
                },
                self.gesture,
            ));
        }
    }
}

/// The button that closes an opened edit.
pub(crate) const CLOSE_EDIT_LABEL: &str = crate::text!("Close");

/// Freezes the edits up to the opened one (decision 0092).
pub(crate) const FREEZE_LABEL: &str = crate::text!("Bake into shape up to here");

/// Deletes the opened edit.
pub(crate) const DELETE_EDIT_LABEL: &str = crate::text!("Delete edit");

/// What is wrong with edit `index` of layer `layer`, in plain words, if
/// anything truly is: points it can't find, or skipped as too big. (An
/// edit that found its points again by place is fine.)
pub(crate) fn edit_warning(
    warnings: &[scene::StackWarning],
    layer: usize,
    index: usize,
) -> Option<String> {
    let warning = warnings
        .iter()
        .find(|w| w.layer == layer && w.operation == index)?;
    if warning.too_big {
        Some(tr("Skipped: it would make the shape too large.").into())
    } else if warning.missing > 0 {
        Some(match warning.missing {
            1 => tr("1 of its parts can't be found on the shape any more; it applies to the rest.")
                .into(),
            n => crate::trf!(
                "{} of its parts can't be found on the shape any more; it applies to the rest.",
                n
            ),
        })
    } else {
        None
    }
}

/// What is recorded on top of the shape: how many edits, and any that no
/// longer fully match it.
pub(super) fn edits_section(ui: &mut Ui, doc: &Document, id: ObjectId, stack: &Stack) {
    section(ui, tr("Edits"));
    let count = stack.operation_count();
    let mut text = match count {
        1 => tr("1 edit recorded on top of the shape").to_string(),
        n => crate::trf!("{} edits recorded on top of the shape", n),
    };
    // Several layers: the Edit layers panel lists them (decision 0091).
    let layers = stack.layers().len();
    if layers > 1 {
        text.push_str(&crate::trf!(", in {} layers", layers));
    }
    ui.label(text).on_hover_text(tr(
        "Edits are kept apart from the shape's settings, so the settings stay \
         editable: change them and the edits are made again on the new shape.",
    ));
    let warnings = doc
        .scene()
        .evaluated(id)
        .map(|shape| shape.warnings)
        .unwrap_or_default();
    let not_fitting = warnings.iter().filter(|w| w.missing > 0).count();
    let found_again = warnings
        .iter()
        .filter(|w| w.found_again && w.missing == 0)
        .count();
    if found_again > 0 {
        let text = match found_again {
            1 => tr("1 edit found its points again on the changed shape").to_string(),
            n => crate::trf!("{} edits found their points again on the changed shape", n),
        };
        ui.label(RichText::new(text).color(theme::TEXT_WEAK))
            .on_hover_text(tr(
                "The shape's points were renumbered (more or fewer segments, say), \
                 so these edits looked for the points, edges and polygons they were \
                 made on by where they are. A polygon split into smaller ones moves \
                 as a whole. Nothing to fix.",
            ));
    }
    let too_big = warnings.iter().filter(|w| w.too_big).count();
    if too_big > 0 {
        let text = match too_big {
            1 => tr("1 edit was skipped: the shape would be too large").to_string(),
            n => crate::trf!("{} edits were skipped: the shape would be too large", n),
        };
        ui.label(RichText::new(text).color(theme::WARNING))
            .on_hover_text(crate::trf!(
                "These edits would have made the shape larger than {} million polygons, \
                 more than the app can work with safely, so they were left out. Undo \
                 them, or use fewer rows or a smaller selection.",
                scene::MAX_SHAPE_POLYGONS / 1_000_000
            ));
    }
    if not_fitting > 0 {
        let text = match not_fitting {
            1 => tr("1 edit no longer fully fits the shape").to_string(),
            n => crate::trf!("{} edits no longer fully fit the shape", n),
        };
        ui.label(RichText::new(text).color(theme::WARNING))
            .on_hover_text(tr(
                "Some of the points these edits move have no place on the shape now \
                 (for example after giving it fewer segments). The edits still apply \
                 to what remains; nothing else is lost, and going back to the earlier \
                 settings brings the rest back.",
            ));
    }
}
