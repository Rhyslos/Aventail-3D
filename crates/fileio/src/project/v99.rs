//! Version 99 of the project file ([decision 0270](../../../../docs/decisions/0270-tiling-and-moving-colours.md)):
//! version 98's, with each graph fill's **Tiling** (`v97::GraphFillV97::tiling`):
//! how many times more often its material repeats on the model than at
//! its real-world size. Files before version 99 hold none, so their graph
//! fills open at 1, as they were; an older app would drop it (and show
//! the material at another size), so the version goes up.
//!
//! Transform and Repeat taking colours and materials (the same decision)
//! needs nothing here: they gained plugs, and their released ids and
//! keys are unchanged.

use paint::graph_fill::{MAX_TILING, MIN_TILING};

/// The Tiling of a graph fill stored before version 99.
pub(crate) fn standard_tiling() -> f32 {
    1.0
}

/// A stored Tiling, brought within the limits; `None` (damaged) when it
/// isn't a real number.
pub(crate) fn tiling_from(stored: f32) -> Option<f32> {
    stored
        .is_finite()
        .then(|| stored.clamp(MIN_TILING, MAX_TILING))
}
