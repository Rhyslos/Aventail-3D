//! Version 104 of the project file ([decision 0278](../../../../docs/decisions/0278-bypass-and-reroute-dots.md)):
//! version 103's, with bypassed boxes and reroute dots. Each box
//! (`v95::BoxV95`) may be marked bypassed (`bypassed`, stored only when
//! it is), and each graph (`v95::GraphV95`) keeps the reroute dots its
//! wires bend through (`bends`, [`BendsV104`], stored only when there are
//! any). Files before version 104 hold neither: no box is bypassed and
//! every wire is straight, which is how they worked; an older app would
//! drop both (switching bypassed boxes back on without a word), so the
//! version goes up.
//!
//! Dots are checked by `matgraph::Graph::from_parts`: at most
//! `matgraph::MAX_BENDS` a wire, each a real place within a graph's
//! limits; dots of a wire the file doesn't hold are dropped.

use crate::limits::Bounded;
use glam::Vec2;
use matgraph::{BoxId, PlugRef};
use serde::{Deserialize, Serialize};

/// The reroute dots of one wire, by the input it goes into.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub(crate) struct BendsV104 {
    /// The box the wire goes into.
    pub(crate) to: u64,
    /// That box's input.
    pub(crate) to_plug: String,
    /// The dots, in order from the wire's output.
    pub(crate) points: Bounded<[f32; 2]>,
}

/// A graph's dots as stored.
pub(crate) fn bends_of(bends: Vec<(PlugRef, Vec<Vec2>)>) -> Bounded<BendsV104> {
    bends
        .into_iter()
        .map(|(to, points)| BendsV104 {
            to: to.node.0,
            to_plug: to.plug,
            points: points.iter().map(|p| p.to_array()).collect(),
        })
        .collect()
}

/// The dots stored as `stored`, unchecked.
pub(crate) fn bends_from(stored: Bounded<BendsV104>) -> Vec<(PlugRef, Vec<Vec2>)> {
    stored
        .0
        .into_iter()
        .map(|b| {
            let to = PlugRef::new(BoxId(b.to), b.to_plug);
            (to, b.points.0.into_iter().map(Vec2::from_array).collect())
        })
        .collect()
}

/// Whether a graph has no dots (none are stored then).
pub(crate) fn no_bends(bends: &Bounded<BendsV104>) -> bool {
    bends.is_empty()
}
