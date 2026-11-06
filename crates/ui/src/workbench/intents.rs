//! UV intents in the window ([decision 0248](../../../../docs/decisions/0248-uv-intents.md)):
//! Straighten, Set texel density and Put on trim row tell the pieces what
//! to be (`Operation::SetUvIntent` beside their own edit), so they are made
//! so again after modelling changes them. UV → Pieces → Intents adds Keep
//! this way up and Clear intents; the UV Editor says on hover what a piece
//! keeps.

use super::Workbench;
use super::joining::counted;
use super::unwrapping::{anchors, targets};
use crate::text::tr;
use egui::Ui;
use mesh::uv_intents::UvIntent;
use scene::{ObjectId, Operation};

/// The submenu's names.
pub(crate) const INTENTS_LABEL: &str = crate::text!("Intents");
pub(crate) const UPRIGHT_LABEL: &str = crate::text!("Keep this way up");
pub(crate) const CLEAR_INTENTS_LABEL: &str = crate::text!("Clear intents");

/// What an intent says, for the UV Editor.
pub(crate) fn intent_words(intent: UvIntent, texture: scene::TextureSettings) -> Option<String> {
    Some(match intent {
        UvIntent::None => return None,
        UvIntent::Strip => tr("Keeps: a straight strip (Straighten)").to_string(),
        UvIntent::Density(d) => crate::trf!(
            "Keeps: {:.0} px/m (Set texel density)",
            texture.pixels_per_metre(d)
        ),
        UvIntent::TrimRow { bottom, top } => crate::trf!(
            "Keeps: on the trim row from {:.0}% to {:.0}% up (Put on trim row)",
            bottom * 100.0,
            top * 100.0
        ),
        UvIntent::Upright => {
            tr("Keeps: this way up (the world's up runs up the texture)").to_string()
        }
    })
}

impl Workbench {
    /// UV → Pieces → Intents.
    pub(super) fn intent_entries(&mut self, ui: &mut Ui) {
        ui.label(
            egui::RichText::new(tr(
                "Straighten, Set texel density and Put on trim row make pieces keep what they \
                 were told after modelling changes them",
            ))
            .small(),
        );
        if ui
            .button(tr(UPRIGHT_LABEL))
            .on_hover_text(tr(
                "Turn the selected pieces so the world's up runs up the texture, and keep them \
                 so after edits: for wood grain, writing, drips",
            ))
            .clicked()
        {
            self.run_set_intent(UvIntent::Upright);
            ui.close();
        }
        if ui
            .button(tr(CLEAR_INTENTS_LABEL))
            .on_hover_text(tr(
                "The selected pieces forget what they were told: edits lay them out as mending \
                 does, and nothing more",
            ))
            .clicked()
        {
            self.run_set_intent(UvIntent::None);
            ui.close();
        }
    }

    /// Gives the selected pieces (whole objects without a selection) the
    /// intent `intent`, one undo step. Keep this way up leaves out pieces
    /// lying flat (they have no up), Clear intents pieces that keep none;
    /// when that leaves nothing, nothing is recorded and the note says
    /// why.
    pub(crate) fn run_set_intent(&mut self, intent: UvIntent) {
        let name = if intent == UvIntent::None {
            tr(CLEAR_INTENTS_LABEL)
        } else {
            tr(UPRIGHT_LABEL)
        };
        let mut count = 0;
        let mut flat = 0;
        let mut any = false;
        let ops: Vec<(ObjectId, Operation)> = self
            .own_shapes()
            .into_iter()
            .filter(|(_, mesh)| mesh::uvs::has_uvs(mesh))
            .filter_map(|(id, mesh)| {
                any = true;
                let faces = targets(&mesh, self.document.elements(id), self.mode);
                // Whole pieces, each one that it changes.
                let faces: Vec<scene::FaceId> = uv::pack::islands_of(&mesh, &faces)
                    .into_iter()
                    .filter(|piece| match intent {
                        UvIntent::None => piece
                            .iter()
                            .any(|f| mesh::uv_intents::intent_of(&mesh, *f) != UvIntent::None),
                        UvIntent::Upright if !uv::intent::has_up(&mesh, piece) => {
                            flat += 1;
                            false
                        }
                        _ => true,
                    })
                    .flatten()
                    .collect();
                if faces.is_empty() {
                    return None;
                }
                count += faces.len();
                Some((
                    id,
                    Operation::SetUvIntent {
                        anchors: anchors(&mesh, &faces),
                        faces,
                        intent,
                    },
                ))
            })
            .collect();
        if !any {
            self.note = Some(crate::trf!("{}: select objects with UVs first.", name));
            return;
        }
        let flat_words = match flat {
            0 => None,
            1 => Some(tr("1 piece lies flat, with no up to keep (turn it by hand).").to_string()),
            n => Some(crate::trf!(
                "{} pieces lie flat, with no up to keep (turn those by hand).",
                n
            )),
        };
        if ops.is_empty() {
            self.note = Some(match flat_words {
                Some(words) => crate::trf!("{}: nothing changed. {}", name, words),
                None => crate::trf!(
                    "{}: nothing changed, the selected pieces keep no intents.",
                    name
                ),
            });
            return;
        }
        let label = counted(name, count, tr("polygon"));
        if self.record_each(&label, ops) {
            let mut note = crate::trf!("{}. Ctrl + Z takes it back.", label);
            if let Some(words) = flat_words {
                note.push(' ');
                note.push_str(&words);
            }
            self.note = Some(note);
        }
    }
}
