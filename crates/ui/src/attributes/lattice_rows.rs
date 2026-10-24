//! The Attributes panel's Lattice section, for a selected lattice cage
//! ([decision 0144](../../../../docs/decisions/0144-lattice-cage.md)):
//! its divisions along each side, how it blends its points, putting its
//! points back, and the shapes it bends. Every change is a command; one
//! drag of a value is one undo step.

use super::curve_rows::Rows;
use super::{AttributesPanel, LABEL_WIDTH, label, section, stepper};
use crate::objects::DocRequest;
use crate::text::tr;
use crate::theme;
use egui::{RichText, Ui, vec2};
use scene::{Command, Document, Interpolation, Lattice, LatticeEdit, ModifierKind, ObjectId};

impl AttributesPanel {
    /// The Lattice section for cage `id`.
    pub(super) fn lattice_section(
        &mut self,
        ui: &mut Ui,
        doc: &Document,
        id: ObjectId,
        out: &mut Vec<DocRequest>,
    ) {
        let Some(lattice) = doc.scene().object(id).and_then(|o| o.kind.lattice()) else {
            return;
        };
        section(ui, tr("Lattice"));
        let mut divisions = lattice.divisions();
        let mut rows = Rows::default();
        let mut picked = None;
        egui::Grid::new("attributes_lattice")
            .num_columns(2)
            .min_col_width(LABEL_WIDTH)
            .spacing(vec2(12.0, 6.0))
            .show(ui, |ui| {
                for (axis, value) in ["X", "Y", "Z"].iter().zip(divisions.iter_mut()) {
                    let name = crate::trf!("Divisions {}", axis);
                    label(
                        ui,
                        &name,
                        tr(
                            "How many points the cage has along this side, 2 to 16: 2 is a \
                             point at each end; more bend the shape in more places",
                        ),
                    );
                    rows.take(stepper::count(
                        ui,
                        value,
                        (Lattice::MIN_DIVISIONS, Lattice::MAX_DIVISIONS),
                        &name,
                    ));
                    ui.end_row();
                }
                label(
                    ui,
                    tr("Interpolation"),
                    tr(
                        "How the cage's points pull the shape: Smooth blends them with no \
                         creases (the shape follows a little less closely); Linear follows \
                         straight between points, creasing along the cage's lines",
                    ),
                );
                ui.horizontal(|ui| {
                    for choice in Interpolation::ALL {
                        let on = lattice.interpolation() == choice;
                        if ui.selectable_label(on, choice.name()).clicked() && !on {
                            picked = Some(choice);
                        }
                    }
                });
                ui.end_row();
            });
        if rows.began {
            self.gesture += 1;
        }
        if rows.changed && divisions != lattice.divisions() {
            out.push(DocRequest::RunMerging(
                Command::EditLattice {
                    id,
                    edit: LatticeEdit::Divisions(divisions),
                },
                self.gesture,
            ));
        }
        if let Some(choice) = picked {
            out.push(edit(id, LatticeEdit::Interpolation(choice)));
        }
        let reset = ui.add_enabled(!lattice.is_at_rest(), egui::Button::new(tr("Reset points")));
        if reset
            .on_hover_text(tr(
                "Put every point back where it started, so the cage bends nothing",
            ))
            .clicked()
        {
            out.push(edit(id, LatticeEdit::Reset));
        }
        bent_shapes(ui, doc, id);
    }
}

/// The shapes cage `id` bends, and how to edit it.
fn bent_shapes(ui: &mut Ui, doc: &Document, id: ObjectId) {
    let scene = doc.scene();
    let names: Vec<String> = scene
        .walk()
        .iter()
        .filter(|(shape, _)| {
            scene
                .modifiers(*shape)
                .iter()
                .any(|m| matches!(m.kind, ModifierKind::Lattice(o) if o.cage == id))
        })
        .filter_map(|(shape, _)| Some(scene.object(*shape)?.name.clone()))
        .collect();
    let note = if names.is_empty() {
        tr("It bends no shapes now: select shapes and use Add lattice for a new cage.").to_string()
    } else {
        crate::trf!(
            "Bends: {}. Press 1 and move its points to bend them.",
            names.join(", ")
        )
    };
    ui.label(RichText::new(note).small().color(theme::TEXT_WEAK));
}

fn edit(id: ObjectId, edit: LatticeEdit) -> DocRequest {
    DocRequest::Run(Command::EditLattice { id, edit })
}
