//! The workbench's part in the game bevel
//! ([decision 0156](../../../../docs/decisions/0156-game-bevel.md)): the
//! Game bevel command on the modifiers palette, in Mesh → Modifiers
//! (*Game ready*) and in command search gives the selected shapes a Live
//! bevel with soft bevels and Weighted normals last, as one undo step
//! (`scene::Document::game_bevel`). No keys or mouse actions.

use super::Workbench;
use crate::text::tr;

impl Workbench {
    /// Gives the selected shapes the game bevel, and says what it did (or
    /// why it couldn't) in the status bar.
    pub(super) fn run_game_bevel(&mut self) {
        let ids = match crate::modifiers::targets(&self.document) {
            Ok(ids) => ids,
            Err(why) => {
                self.note = Some(format!("{why}."));
                return;
            }
        };
        let label = if ids.len() == 1 {
            tr("Game bevel").to_string()
        } else {
            crate::trf!("Game bevel {} objects", ids.len())
        };
        let key = self.new_step_key();
        match self.document.game_bevel(&ids, &label, key) {
            Ok(done) => {
                let what = if done.objects == 1 {
                    tr("the shape").to_string()
                } else {
                    crate::trf!("{} shapes", done.objects)
                };
                let mut note = crate::trf!(
                    "Game bevel: {} bevelled 2 cm at every edge sharper than 30°, with round \
                     bevels and flat faces (Live bevel, then Weighted normals). Change either in \
                     the Attributes panel; Ctrl + Z takes it all back.",
                    what
                );
                if done.switched > 0 {
                    note.push(' ');
                    note.push_str(tr(
                        "Smoothing now follows the marked edges, with the shape's look kept as \
                         marks.",
                    ));
                }
                self.note = Some(note);
            }
            Err(why) => self.note = Some(crate::trf!("Couldn't add the game bevel: {}.", why)),
        }
    }
}
