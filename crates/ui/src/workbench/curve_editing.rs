//! Editing curves' points in the Points mode
//! ([decision 0138](../../../../docs/decisions/0138-curves-sweep-lathe.md)).
//!
//! A selected curve shows its points, and the ends of the handles that
//! pull, as dots: they are picked, selected (click, box, Ctrl adds, Shift
//! removes) and moved with the gizmo as a mesh's points are, through the
//! drawing (`DrawItem::curve`). This module does what differs: Delete
//! (and Backspace) take points away, or take a handle back to nothing;
//! a double-click switches a point between sharp and smooth (in the
//! selector); and the Edges and Polygons modes say a curve has none.

use super::Workbench;
use crate::palette::Mode;
use crate::text::tr;
use scene::{Command, CurveEdit, CurvePart, ObjectId};

impl Workbench {
    /// The selected curves and their selected parts.
    fn selected_curve_parts(&self) -> Vec<(ObjectId, Vec<CurvePart>)> {
        let scene = self.document.scene();
        self.document
            .selection()
            .into_iter()
            .filter(|id| scene.object(*id).is_some_and(|o| o.kind.curve().is_some()))
            .filter_map(|id| {
                let parts: Vec<CurvePart> = self
                    .document
                    .elements(id)?
                    .points()
                    .map(|v| CurvePart::from_code(v.get()))
                    .collect();
                (!parts.is_empty()).then_some((id, parts))
            })
            .collect()
    }

    /// Delete or Backspace in the Points mode, for curves: their selected
    /// points go, and selected handles go back to nothing, as one undo
    /// step. Returns how many parts went, or `None` with no curve parts
    /// selected.
    pub(super) fn delete_curve_parts(&mut self) -> Option<usize> {
        if self.mode != Mode::Points {
            return None;
        }
        let chosen = self.selected_curve_parts();
        if chosen.is_empty() {
            return None;
        }
        let key = self.new_step_key();
        let count = chosen.iter().map(|(_, parts)| parts.len()).sum::<usize>();
        let commands = chosen.into_iter().map(|(id, parts)| Command::EditCurve {
            id,
            edit: CurveEdit::Delete { parts },
        });
        if let Err(why) =
            self.document
                .execute_all_labelled(commands, tr("Delete curve points"), key)
        {
            self.note = Some(crate::trf!("Couldn't do that: {}.", why));
            return Some(0);
        }
        self.note = Some(crate::trf!(
            "{} curve point{} or handle{} deleted. Ctrl + Z takes it back.",
            count,
            if count == 1 { "" } else { "s" },
            if count == 1 { "" } else { "s" }
        ));
        Some(count)
    }

    /// Says, when the mode changes to the Edges or Polygons mode with a
    /// curve selected, that a curve has neither.
    pub(super) fn note_curve_modes(&mut self) {
        if self.mode == self.curve_mode_noted {
            return;
        }
        self.curve_mode_noted = self.mode;
        if !matches!(self.mode, Mode::Edges | Mode::Polygons) {
            return;
        }
        // Lattice cages have none either (decision 0144).
        self.note_cage_modes();
        let scene = self.document.scene();
        let curves = self
            .document
            .selection()
            .iter()
            .any(|id| scene.object(*id).is_some_and(|o| o.kind.curve().is_some()));
        if curves {
            self.note = Some(
                tr(
                    "A curve has no edges or polygons of its own: press 1 to edit its points, \
                     or Bake into shape (Ctrl + E) to edit its solid as a mesh.",
                )
                .into(),
            );
        }
    }
}
