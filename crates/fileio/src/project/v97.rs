//! Version 97 of the project file ([decision 0261](../../../../docs/decisions/0261-graphs-in-the-painter.md)):
//! version 96's, with graphs in the painter:
//!
//! - graph fill layers (`v64::LayerV64::graph`, stored as
//!   [`GraphFillV97`]): which graph, the layer's own values of the
//!   graph's shown settings, and what its pixels were made from. Their
//!   pixels are saved as for any paint layer, so they open as they were
//!   left without being worked out again;
//! - the settings a graph shows in the painter (`v95::BoxV95::shown`).
//!
//! Files before version 97 hold neither; an older app would drop them, so
//! the version goes up.

use super::v1::Problem;
use super::v95::ValueV95;
use crate::limits::Bounded;
use matgraph::{BoxId, GraphId};
use paint::{GraphFill, GraphValue};
use serde::{Deserialize, Serialize};
use words::tr;

/// A stored graph fill.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct GraphFillV97 {
    /// The project's graph it runs.
    pub(crate) graph: u64,
    /// Its own values.
    pub(crate) values: Bounded<GraphValueV97>,
    /// What its pixels were made from (a hash; 0 until made).
    pub(crate) made: u64,
}

/// One of a graph fill's own values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct GraphValueV97 {
    /// The box, by its id in the graph.
    pub(crate) node: u64,
    /// The setting's key.
    pub(crate) key: String,
    /// The value.
    pub(crate) value: ValueV95,
}

/// `fill` as stored.
pub(crate) fn graph_fill_of(fill: &GraphFill) -> GraphFillV97 {
    GraphFillV97 {
        graph: fill.graph.0,
        values: fill
            .values
            .iter()
            .map(|v| GraphValueV97 {
                node: v.node.0,
                key: v.key.clone(),
                value: ValueV95::of(v.value),
            })
            .collect(),
        made: fill.made,
    }
}

/// A stored graph fill, checked: too many values, a key too long, a
/// damaged gradient or numbers that aren't real are refused. A value for
/// a box or setting the graph no longer has is kept (and ignored when the
/// layer is made), as is a graph the project no longer has (the layer
/// keeps its pixels and says so).
pub(crate) fn graph_fill_from(stored: GraphFillV97) -> Result<GraphFill, Problem> {
    let damaged = || -> Problem { tr("its painted textures are damaged").into() };
    let mut values = Vec::with_capacity(stored.values.len());
    for v in stored.values.0 {
        values.push(GraphValue {
            node: BoxId(v.node),
            key: v.key,
            value: v.value.value().ok_or_else(damaged)?,
        });
    }
    let fill = GraphFill {
        graph: GraphId(stored.graph),
        values,
        made: stored.made,
    };
    if fill.is_valid() {
        Ok(fill)
    } else {
        Err(damaged())
    }
}
