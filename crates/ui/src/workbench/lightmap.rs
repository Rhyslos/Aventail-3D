//! UV → Lightmap UVs ([decision 0242](../../../../docs/decisions/0242-lightmap-uvs.md)):
//! one button lays out each selected object's `lightmap` UV set for
//! Godot's baked lighting, exported as UV2. Recorded as
//! `Operation::MakeLightmap` with the object's smoothing, one undo step.

use super::Workbench;
use super::joining::counted;
use crate::text::tr;
use egui::Ui;
use scene::{ObjectId, Operation};

/// The menu's names, and what the button does.
pub(crate) const LIGHTMAP_LABEL: &str = crate::text!("Lightmap UVs");
pub(crate) const MAKE_LIGHTMAP_LABEL: &str = crate::text!("Make lightmap UVs");
pub(crate) const MAKE_LIGHTMAP_HELP: &str = crate::text!(
    "Lay out a second UV set called lightmap for Godot's baked lighting: cut along the seams \
     and hard edges, every piece at its real size with its own pixels and room round it. \
     Exported as UV2; the UVs you paint with stay as they are."
);

impl Workbench {
    /// The submenu: the size and padding, and the button.
    pub(super) fn lightmap_entries(&mut self, ui: &mut Ui) {
        ui.label(egui::RichText::new(tr("Lightmap size")).strong());
        for size in uv::lightmap::LIGHTMAP_SIZES {
            ui.radio_value(
                &mut self.uv_settings.lightmap_size,
                size,
                crate::trf!("{} × {} px", size, size),
            )
            .on_hover_text(tr(
                "The lightmap Godot will bake, per object: 512 suits most props; bigger for \
                 large walls and floors",
            ));
        }
        ui.horizontal(|ui| {
            ui.label(tr("Room round pieces"));
            ui.add(
                egui::DragValue::new(&mut self.uv_settings.lightmap_padding)
                    .range(1..=64)
                    .suffix(tr(" px")),
            )
            .on_hover_text(tr(
                "Pixels between pieces in the lightmap, so baked light doesn't bleed from one to \
                 the next (4 standard)",
            ));
        });
        ui.separator();
        if ui
            .button(tr(MAKE_LIGHTMAP_LABEL))
            .on_hover_text(tr(MAKE_LIGHTMAP_HELP))
            .clicked()
        {
            self.run_make_lightmap();
            ui.close();
        }
    }

    /// Makes lightmap UVs on every selected object, one undo step.
    /// Objects that already have as many UV sets as a shape can, none of
    /// them the lightmap, are left out and named in the note; with only
    /// those, nothing is recorded.
    pub(crate) fn run_make_lightmap(&mut self) {
        let size = self.uv_settings.lightmap_size;
        let padding = self.uv_settings.lightmap_padding.clamp(1, 64);
        let shapes: Vec<_> = self
            .own_shapes()
            .into_iter()
            .filter(|(_, mesh)| mesh.face_count() > 0)
            .collect();
        if shapes.is_empty() {
            self.note = Some(crate::trf!(
                "{}: select the objects first.",
                tr(MAKE_LIGHTMAP_LABEL)
            ));
            return;
        }
        let (room, full): (Vec<_>, Vec<_>) = shapes
            .into_iter()
            .partition(|(_, mesh)| mesh::uv_sets::has_room_for(mesh, mesh::uv_sets::LIGHTMAP_NAME));
        let full_words = match full.len() {
            0 => None,
            1 => Some(crate::trf!(
                "1 object already has {} UV sets, as many as a shape can have: delete one (UV → \
                 UV sets) to make room for the lightmap.",
                mesh::uv_sets::MAX_SETS
            )),
            n => Some(crate::trf!(
                "{} objects already have {} UV sets, as many as a shape can have: delete one \
                 (UV → UV sets) to make room for the lightmap.",
                n,
                mesh::uv_sets::MAX_SETS
            )),
        };
        if room.is_empty() {
            self.note = Some(crate::trf!(
                "{}: nothing changed. {}",
                tr(MAKE_LIGHTMAP_LABEL),
                full_words.unwrap_or_default()
            ));
            return;
        }
        let ops: Vec<(ObjectId, Operation)> = room
            .into_iter()
            .map(|(id, _)| {
                (
                    id,
                    Operation::MakeLightmap {
                        size,
                        padding,
                        hardness: self.smoothing_of(id),
                    },
                )
            })
            .collect();
        let count = ops.len();
        let label = counted(tr("Make lightmap UVs for"), count, tr("object"));
        if self.record_each(&label, ops) {
            let mut note = crate::trf!(
                "{}: a UV set called lightmap, exported as UV2. Ctrl + Z takes it back.",
                label
            );
            if let Some(words) = full_words {
                note.push(' ');
                note.push_str(&words);
            }
            self.note = Some(note);
        }
    }
}
