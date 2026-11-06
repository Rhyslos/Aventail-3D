//! UV → Pieces → Packing for the selection ([decision 0240](../../../../docs/decisions/0240-packing-control.md)):
//! giving the selected pieces a packing group, a grain and a texture
//! scale, recorded as `Operation::SetPackValues` (one undo step each).

use super::Workbench;
use super::joining::counted;
use super::unwrapping::{anchors, targets};
use crate::text::tr;
use egui::Ui;
use mesh::uvs::{PackParts, PackValues};
use scene::{FaceId, ObjectId, Operation};

/// The submenu's name.
pub(crate) const PACK_VALUES_LABEL: &str = crate::text!("Packing for the selection");

impl Workbench {
    /// The submenu's entries.
    pub(super) fn pack_values_entries(&mut self, ui: &mut Ui) {
        ui.label(egui::RichText::new(tr("Packing group")).strong());
        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut self.uv_settings.piece_group)
                    .range(1..=mesh::uvs::MAX_PACK_GROUP),
            );
            if ui
                .button(tr("Put in this group"))
                .on_hover_text(tr(
                    "Pieces in one group are packed as one block, as they lie together: a \
                     belt and its buckle, a door and its frame",
                ))
                .clicked()
            {
                let group = self.uv_settings.piece_group;
                self.run_pack_values(
                    PackValues {
                        group,
                        ..PackValues::default()
                    },
                    PackParts {
                        group: true,
                        ..PackParts::default()
                    },
                );
                ui.close();
            }
        });
        if ui.button(tr("Take out of its group")).clicked() {
            self.run_pack_values(
                PackValues::default(),
                PackParts {
                    group: true,
                    ..PackParts::default()
                },
            );
            ui.close();
        }
        ui.separator();
        ui.label(egui::RichText::new(tr("Grain")).strong());
        for (label, help, grain) in [
            (
                tr("Keep the grain"),
                tr("Never turned when packed: wood grain, brick rows and text stay as laid"),
                1,
            ),
            (tr("Free to turn"), tr("May be turned when packed, whatever the app's setting"), 2),
            (tr("As the app's setting"), tr("Follow UV → Settings → Packing → Keep the grain"), 0),
        ] {
            if ui.button(label).on_hover_text(help).clicked() {
                self.run_pack_values(
                    PackValues {
                        grain,
                        ..PackValues::default()
                    },
                    PackParts {
                        grain: true,
                        ..PackParts::default()
                    },
                );
                ui.close();
            }
        }
        ui.separator();
        ui.label(egui::RichText::new(tr("Texture scale")).strong());
        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut self.uv_settings.piece_scale)
                    .range(mesh::uvs::PACK_SCALE_RANGE)
                    .speed(0.05)
                    .suffix("×"),
            );
            if ui
                .button(tr("Set"))
                .on_hover_text(tr(
                    "How much texture these pieces get when packed, against the rest: 2× is twice \
                     as wide (four times the pixels), for a face or a logo seen up close; 0.5× \
                     for parts hardly seen. Select a whole object for a scale per object.",
                ))
                .clicked()
            {
                let scale = self.uv_settings.piece_scale;
                self.run_pack_values(
                    PackValues {
                        scale,
                        ..PackValues::default()
                    },
                    PackParts {
                        scale: true,
                        ..PackParts::default()
                    },
                );
                ui.close();
            }
        });
        if ui.button(tr("Standard texture scale")).clicked() {
            self.run_pack_values(
                PackValues::default(),
                PackParts {
                    scale: true,
                    ..PackParts::default()
                },
            );
            ui.close();
        }
    }

    /// Gives the whole pieces holding the selection `values` (the parts
    /// in `which`), as one undo step.
    pub(crate) fn run_pack_values(&mut self, values: PackValues, which: PackParts) {
        let mut ops: Vec<(ObjectId, Operation)> = Vec::new();
        let mut count = 0;
        for (id, mesh) in self.own_shapes() {
            if !mesh::uvs::has_uvs(&mesh) {
                continue;
            }
            let faces = targets(&mesh, self.document.elements(id), self.mode);
            let whole: Vec<FaceId> = uv::pack::islands_of(&mesh, &faces)
                .into_iter()
                .flatten()
                .collect();
            if whole.is_empty() {
                continue;
            }
            count += whole.len();
            let anchors = anchors(&mesh, &whole);
            ops.push((
                id,
                Operation::SetPackValues {
                    faces: whole,
                    values,
                    which,
                    anchors,
                },
            ));
        }
        if ops.is_empty() {
            self.note = Some(crate::trf!(
                "{}: select objects with UVs (or some of their pieces) first.",
                tr(PACK_VALUES_LABEL)
            ));
            return;
        }
        let label = counted(tr("Set how to pack"), count, tr("polygon"));
        if self.record_each(&label, ops) {
            self.note = Some(crate::trf!(
                "{}. Pack to see it. Ctrl + Z takes it back.",
                label
            ));
        }
    }
}
