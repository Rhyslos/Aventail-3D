//! The workbench's part in locked pieces and pieces' own unwrap settings
//! ([decision 0112](../../../../docs/decisions/0112-locked-pieces-and-piece-settings.md)).
//!
//! No keys: the UV menu holds Lock pieces and Unlock pieces, and the Piece
//! settings submenu. Both work on whole pieces: those holding the
//! selection (whole objects with nothing selected inside them).

use super::Workbench;
use super::joining::counted;
use super::unwrapping::UnwrapWay;
use scene::{FaceId, ObjectId, Operation};
use std::collections::BTreeSet;

/// The UV menu's names for locking, and what it does.
pub(crate) const LOCK_LABEL: &str = "Lock pieces";
pub(crate) const LOCK_HELP: &str = "No automatic tool changes the pieces holding the selection: \
     Unwrap, Relax, Straighten, Pack, texel density, trim rows, Suggest seams and mending all \
     leave them as they are. You can still move, turn and scale them by hand. Drawn hatched in \
     the UV Editor.";
pub(crate) const UNLOCK_LABEL: &str = "Unlock pieces";
pub(crate) const UNLOCK_HELP: &str = "Let the automatic tools change the selected pieces again";

/// The UV menu's name for the piece settings submenu, and its parts.
pub(crate) const PIECE_SETTINGS_LABEL: &str = "Piece settings";
pub(crate) const APPS_OWN: &str = "The app's";
pub(crate) const SET_PIECE_LABEL: &str = "Give the selection these settings";
pub(crate) const CLEAR_PIECE_LABEL: &str = "Clear the selection's settings";
pub(crate) const OWN_DENSITY_LABEL: &str = "Its own texel density";

impl Workbench {
    /// The whole pieces (by the shape's cuts) holding the selection, per
    /// object; whole objects with nothing selected inside them.
    pub(super) fn chosen_pieces(
        &self,
    ) -> Vec<(ObjectId, std::sync::Arc<scene::Mesh>, Vec<FaceId>)> {
        self.own_shapes()
            .into_iter()
            .filter_map(|(id, mesh)| {
                let chosen =
                    super::unwrapping::targets(&mesh, self.document.elements(id), self.mode);
                let wanted: BTreeSet<FaceId> = chosen.into_iter().collect();
                let cuts = uv::unwrap::cuts_of(&mesh);
                let faces: Vec<FaceId> = uv::seams::pieces(&mesh, &cuts)
                    .into_iter()
                    .filter(|p| p.iter().any(|f| wanted.contains(f)))
                    .flatten()
                    .collect();
                (!faces.is_empty()).then_some((id, mesh, faces))
            })
            .collect()
    }

    /// Lock pieces (`lock`) or Unlock pieces, as one undo step.
    pub(crate) fn run_lock(&mut self, lock: bool) {
        let name = if lock { LOCK_LABEL } else { UNLOCK_LABEL };
        if self.document.selection().is_empty() {
            self.note = Some(format!("{name}: select the objects first."));
            return;
        }
        let mut ops = Vec::new();
        let mut count = 0;
        for (id, mesh, faces) in self.chosen_pieces() {
            let now = mesh::uvs::locked(&mesh);
            let changing = faces.iter().any(|f| {
                now.as_ref()
                    .and_then(|l| l.get(f.get() as usize).copied())
                    .unwrap_or(false)
                    != lock
            });
            if !changing {
                continue;
            }
            count += faces.len();
            let anchors = super::unwrapping::anchors(&mesh, &faces);
            ops.push((
                id,
                Operation::SetUvLock {
                    faces,
                    lock,
                    anchors,
                },
            ));
        }
        if ops.is_empty() {
            let how = if lock { "locked" } else { "unlocked" };
            self.note = Some(format!("{name}: those pieces are {how} already."));
            return;
        }
        let verb = if lock {
            "Lock the UVs of"
        } else {
            "Unlock the UVs of"
        };
        let label = counted(verb, count, "polygon");
        if self.record_each(&label, ops) {
            let why = if lock {
                " No automatic tool changes them; move them by hand as usual."
            } else {
                ""
            };
            self.note = Some(format!("{label}. Ctrl + Z takes it back.{why}"));
        }
    }

    /// The piece settings the submenu would give, in the tools' units.
    pub(crate) fn chosen_piece_settings(&self) -> scene::PieceSettings {
        let s = &self.uv_settings;
        let texture = self.document.scene().texture();
        scene::PieceSettings {
            method: s.piece_method.map(UnwrapWay::method),
            strips: s.piece_strips,
            density: s
                .piece_density
                .filter(|d| d.is_finite() && *d > 0.0)
                .map(|d| d.min(scene::MAX_DENSITY) / texture.size as f32 / 100.0),
        }
    }

    /// Gives the selected pieces `settings` (all `None`: clears them), as
    /// one undo step.
    pub(crate) fn run_piece_settings(&mut self, settings: scene::PieceSettings) {
        if self.document.selection().is_empty() {
            self.note = Some(format!("{PIECE_SETTINGS_LABEL}: select the objects first."));
            return;
        }
        let mut ops = Vec::new();
        let mut count = 0;
        for (id, mesh, faces) in self.chosen_pieces() {
            count += faces.len();
            let anchors = super::unwrapping::anchors(&mesh, &faces);
            ops.push((
                id,
                Operation::SetPieceSettings {
                    faces,
                    settings,
                    anchors,
                },
            ));
        }
        if ops.is_empty() {
            self.note = Some(format!(
                "{PIECE_SETTINGS_LABEL}: the selected objects have no polygons."
            ));
            return;
        }
        let verb = if settings.is_empty() {
            "Clear the piece settings of"
        } else {
            "Set the piece settings of"
        };
        let label = counted(verb, count, "polygon");
        if self.record_each(&label, ops) {
            let why = if settings.is_empty() {
                " They follow the app's settings again."
            } else {
                " Every unwrap of these pieces uses them."
            };
            self.note = Some(format!("{label}. Ctrl + Z takes it back.{why}"));
        }
    }

    /// The UV menu's Piece settings submenu.
    pub(crate) fn piece_settings_menu(&mut self, ui: &mut egui::Ui) {
        let s = &mut self.uv_settings;
        ui.label("Method")
            .on_hover_text("How these pieces are laid flat, whatever UV → Unwrapping says");
        ui.radio_value(&mut s.piece_method, None, APPS_OWN);
        for way in UnwrapWay::ALL {
            ui.radio_value(&mut s.piece_method, Some(way), way.name())
                .on_hover_text(way.help());
        }
        ui.separator();
        ui.label("Straighten as a strip").on_hover_text(
            "Whether these pieces, when they're strips of square quads, come out as straight grids",
        );
        ui.radio_value(&mut s.piece_strips, None, APPS_OWN);
        ui.radio_value(&mut s.piece_strips, Some(true), "Straighten");
        ui.radio_value(&mut s.piece_strips, Some(false), "Don't straighten");
        ui.separator();
        let mut own = s.piece_density.is_some();
        ui.horizontal(|ui| {
            if ui
                .checkbox(&mut own, OWN_DENSITY_LABEL)
                .on_hover_text(
                    "Unwrap sizes these pieces to this texel density instead of matching the rest: \
                     more for a part seen up close (a face, a label), less for one hidden away",
                )
                .changed()
            {
                s.piece_density = own.then_some(1024.0);
            }
            if let Some(density) = s.piece_density.as_mut() {
                ui.add(
                    egui::DragValue::new(density)
                        .range(1.0..=scene::MAX_DENSITY)
                        .speed(1.0)
                        .suffix(" px/m"),
                );
            }
        });
        ui.separator();
        if ui
            .button(SET_PIECE_LABEL)
            .on_hover_text("The pieces holding the selection (or whole objects) keep these settings for every unwrap")
            .clicked()
        {
            let settings = self.chosen_piece_settings();
            self.run_piece_settings(settings);
            ui.close();
        }
        if ui
            .button(CLEAR_PIECE_LABEL)
            .on_hover_text("The pieces holding the selection follow the app's settings again")
            .clicked()
        {
            self.run_piece_settings(scene::PieceSettings::default());
            ui.close();
        }
    }
}
