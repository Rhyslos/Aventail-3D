//! The Attributes panel's Modifiers section
//! ([decision 0142](../../../../docs/decisions/0142-modifier-stack-and-remesh.md)):
//! each of the object's modifiers, first to last, with its on/off switch,
//! Up, Down and Remove, its settings, whether the viewport shows it, and
//! why it can't do its work, if it can't. Every change is a command; one
//! drag of a value is one undo step.

use super::curve_rows::Rows;
use super::{AttributesPanel, LABEL_WIDTH, label, section, stepper};
use crate::objects::DocRequest;
use crate::text::tr;
use crate::theme;
use crate::units::Units;
use egui::{RichText, Ui, vec2};
use scene::{Command, Document, Modifier, ModifierKind, ObjectId, RemeshOptions};

/// The accessible name of a modifier's on/off switch.
pub(crate) const ON_LABEL: &str = crate::text!("Modifier on");

impl AttributesPanel {
    /// The Modifiers section for object `id`, if it has any.
    pub(super) fn modifiers_section(
        &mut self,
        ui: &mut Ui,
        doc: &Document,
        id: ObjectId,
        units: Units,
        out: &mut Vec<DocRequest>,
    ) {
        let scene = doc.scene();
        let modifiers = scene.modifiers(id).to_vec();
        if modifiers.is_empty() {
            return;
        }
        section(ui, tr("Modifiers"));
        let problems = scene.modifier_problems(id);
        let count = modifiers.len();
        for (index, modifier) in modifiers.iter().enumerate() {
            ui.push_id(("modifier", id, index), |ui| {
                self.modifier_header(ui, id, index, count, modifier, out);
                self.modifier_rows(ui, id, index, modifier, units, out);
                if let Some((_, why)) = problems.iter().find(|(i, _)| *i == index) {
                    ui.label(
                        RichText::new(crate::trf!("Not worked out: {}.", why))
                            .color(theme::WARNING),
                    );
                }
            });
            ui.add_space(4.0);
        }
        ui.label(
            RichText::new(tr(
                "Modifiers are worked out first to last, after the edits, booleans and mirror. \
                 Bake into shape (Ctrl + E) makes those that are on part of the shape for good.",
            ))
            .small()
            .color(theme::TEXT_WEAK),
        );
    }

    /// A modifier's first line: its switch and name, Up, Down and Remove.
    fn modifier_header(
        &mut self,
        ui: &mut Ui,
        id: ObjectId,
        index: usize,
        count: usize,
        modifier: &Modifier,
        out: &mut Vec<DocRequest>,
    ) {
        ui.horizontal(|ui| {
            let mut on = modifier.on;
            let switch = ui
                .checkbox(&mut on, RichText::new(modifier.kind.name()).strong())
                .on_hover_text(tr(
                    "Switch this modifier off to see the shape without it; it stays, with its \
                     settings",
                ));
            switch.widget_info(|| {
                egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, on, tr(ON_LABEL))
            });
            if switch.changed() {
                out.push(change(
                    id,
                    index,
                    Modifier {
                        on,
                        ..modifier.clone()
                    },
                ));
            }
            let up = ui
                .add_enabled(index > 0, egui::Button::new(tr("Up")).small())
                .on_hover_text(tr("Work this modifier out before the one above it"));
            if up.clicked() {
                out.push(DocRequest::Run(Command::MoveModifier {
                    id,
                    from: index,
                    to: index - 1,
                }));
            }
            let down = ui
                .add_enabled(index + 1 < count, egui::Button::new(tr("Down")).small())
                .on_hover_text(tr("Work this modifier out after the one below it"));
            if down.clicked() {
                out.push(DocRequest::Run(Command::MoveModifier {
                    id,
                    from: index,
                    to: index + 1,
                }));
            }
            if ui
                .small_button(tr("Remove"))
                .on_hover_text(tr("Take this modifier away"))
                .clicked()
            {
                out.push(DocRequest::Run(Command::RemoveModifier { id, index }));
            }
        });
    }

    /// A modifier's settings, and whether the viewport shows it.
    fn modifier_rows(
        &mut self,
        ui: &mut Ui,
        id: ObjectId,
        index: usize,
        modifier: &Modifier,
        units: Units,
        out: &mut Vec<DocRequest>,
    ) {
        let mut edited = modifier.clone();
        let mut rows = Rows::default();
        egui::Grid::new(("attributes_modifier", id, index))
            .num_columns(2)
            .min_col_width(LABEL_WIDTH)
            .spacing(vec2(12.0, 6.0))
            .show(ui, |ui| {
                kind_rows(ui, &mut edited.kind, units, &mut rows);
                label(
                    ui,
                    tr("Show in viewport"),
                    tr(
                        "Off, the viewport skips this modifier (handy while it is slow); exports \
                         and Bake into shape still use it",
                    ),
                );
                rows.toggle(ui, &mut edited.in_viewport, tr("Show in viewport"));
                ui.end_row();
            });
        if rows.began {
            self.gesture += 1;
        }
        if rows.changed && edited != *modifier {
            out.push(DocRequest::RunMerging(
                Command::ChangeModifier {
                    id,
                    index,
                    modifier: edited,
                },
                self.gesture,
            ));
        }
    }
}

/// The rows for a kind's own settings.
fn kind_rows(ui: &mut Ui, kind: &mut ModifierKind, units: Units, rows: &mut Rows) {
    match kind {
        ModifierKind::Remesh(options) => remesh_rows(ui, options, units, rows),
    }
}

/// Remesh's settings: Voxel size and Smooth.
fn remesh_rows(ui: &mut Ui, options: &mut RemeshOptions, units: Units, rows: &mut Rows) {
    label(
        ui,
        tr("Voxel size"),
        tr(
            "How far apart the grid the shape is rebuilt on is: about the size of the new \
             polygons. Smaller keeps more detail and makes more polygons.",
        ),
    );
    rows.take(stepper::length(
        ui,
        &mut options.voxel_size,
        (RemeshOptions::MIN_VOXEL, RemeshOptions::MAX_VOXEL),
        units,
        tr("Voxel size"),
    ));
    ui.end_row();
    label(
        ui,
        tr("Smooth"),
        tr("Relax passes evening out the new surface without shrinking it: 0 for none"),
    );
    rows.take(stepper::count(
        ui,
        &mut options.smooth,
        (0, RemeshOptions::MAX_SMOOTH),
        tr("Smooth"),
    ));
    ui.end_row();
}

fn change(id: ObjectId, index: usize, modifier: Modifier) -> DocRequest {
    DocRequest::Run(Command::ChangeModifier {
        id,
        index,
        modifier,
    })
}
