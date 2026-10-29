//! The Edges switch ticking itself ([decision 0220](../../../../docs/decisions/0220-edges-switch.md)):
//! the Points, Edges and Polygons modes need the shape's edges to work
//! on, so entering one ticks the switch, and leaving them unticks it
//! again, unless the switch was changed by hand meanwhile or was already
//! on.

use super::Workbench;
use crate::palette::Mode;

/// What the switch was last seen with.
#[derive(Debug, Default)]
pub(super) struct EdgesSwitch {
    /// Whether the last frame was in one of the modes that tick it.
    editing: bool,
    /// Whether the switch was ticked on entering them (and is the app's
    /// to untick on leaving).
    ticked: bool,
}

/// Whether `mode` works on a shape's parts and so shows its edges.
fn ticks_edges(mode: Mode) -> bool {
    matches!(mode, Mode::Points | Mode::Edges | Mode::Polygons)
}

impl Workbench {
    /// Ticks or unticks the Edges switch as the mode changes, each frame.
    pub(super) fn update_edges_switch(&mut self) {
        let editing = ticks_edges(self.mode);
        let options = self.viewport.options_mut();
        let state = &mut self.edges_switch;
        if editing && !state.editing {
            if !options.show_edges {
                options.show_edges = true;
                state.ticked = true;
            }
        } else if !editing && state.editing {
            if state.ticked {
                options.show_edges = false;
            }
            state.ticked = false;
        } else if editing && state.ticked && !options.show_edges {
            // Unticked by hand while editing: the choice is theirs now.
            state.ticked = false;
        }
        state.editing = editing;
    }

    /// The Edges switch as the person chose it: off when it is only on
    /// because a mode ticked it, so the next session starts as they left
    /// it.
    pub(super) fn chosen_show_edges(&self) -> bool {
        self.viewport.options().show_edges && !self.edges_switch.ticked
    }
}
