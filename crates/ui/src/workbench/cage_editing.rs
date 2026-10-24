//! Editing lattice cages' points in the Points mode
//! ([decision 0144](../../../../docs/decisions/0144-lattice-cage.md)).
//!
//! A selected cage shows its points as dots: they are picked, selected
//! (click, box, Ctrl adds, Shift removes) and moved with the gizmo and
//! Coordinates as a mesh's points are, through the drawing
//! (`DrawItem::cage`) and the transform target (`PointsStart::cage`).
//! This module says what differs: a cage's points can't be deleted (its
//! divisions say how many it has), and it has no edges or polygons.

use super::Workbench;
use crate::palette::Mode;
use crate::text::tr;

impl Workbench {
    /// Whether any selected cage has points selected.
    pub(super) fn cage_points_selected(&self) -> bool {
        let scene = self.document.scene();
        self.document.selection().into_iter().any(|id| {
            scene.object(id).is_some_and(|o| o.kind.lattice().is_some())
                && self
                    .document
                    .elements(id)
                    .is_some_and(|e| e.points().next().is_some())
        })
    }

    /// Says that a cage's points stay (Delete and Backspace on them).
    pub(super) fn note_cage_points_stay(&mut self) {
        self.note = Some(
            tr(
                "A lattice cage's points can't be deleted: change its Divisions in the \
                 Attributes panel for more or fewer, or Reset points to put them back.",
            )
            .into(),
        );
    }

    /// Says, when the mode changes to the Edges or Polygons mode with a
    /// cage selected, that a cage has neither.
    pub(super) fn note_cage_modes(&mut self) {
        if !matches!(self.mode, Mode::Edges | Mode::Polygons) {
            return;
        }
        let scene = self.document.scene();
        let cages = self.document.selection().iter().any(|id| {
            scene
                .object(*id)
                .is_some_and(|o| o.kind.lattice().is_some())
        });
        if cages {
            self.note = Some(
                tr(
                    "A lattice cage has no edges or polygons: press 1 to move its points, and \
                     the shapes it bends follow.",
                )
                .into(),
            );
        }
    }
}
