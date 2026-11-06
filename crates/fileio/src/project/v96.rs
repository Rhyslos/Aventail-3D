//! Version 96 of the project file ([decision 0259](../../../../docs/decisions/0259-first-material-boxes.md)):
//! version 95's, with gradient settings in material graphs
//! (`v95::ValueV95::Gradient`, Gradient map's colours): each colour of a
//! gradient and where it sits. Files before version 96 hold none; an older
//! app can't read a graph holding one, so the version goes up.

use crate::limits::Bounded;
use matgraph::{ColourGradient, GradientStop};
use serde::{Deserialize, Serialize};

/// A stored gradient: its colours, in the order they were added.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct GradientV96 {
    /// Its colours.
    pub(crate) stops: Bounded<StopV96>,
}

/// One colour of a gradient.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct StopV96 {
    /// Where it sits, 0 to 1.
    pub(crate) at: f32,
    /// Its colour (sRGB, 0 to 1).
    pub(crate) colour: [f32; 3],
}

/// `gradient` as stored.
pub(crate) fn gradient_of(gradient: &ColourGradient) -> GradientV96 {
    GradientV96 {
        stops: gradient
            .stops()
            .iter()
            .map(|s| StopV96 {
                at: s.at,
                colour: s.colour,
            })
            .collect(),
    }
}

/// The stored gradient; `None` unless it has 2 to
/// [`matgraph::MAX_STOPS`] colours. Places and colours out of range are
/// brought into range with the rest of the graph's settings.
pub(crate) fn gradient_from(stored: GradientV96) -> Option<ColourGradient> {
    let stops: Vec<GradientStop> = stored
        .stops
        .0
        .into_iter()
        .map(|s| GradientStop {
            at: s.at,
            colour: s.colour,
        })
        .collect();
    ColourGradient::new(&stops)
}
