//! Version 105 of the project file ([decision 0280](../../../../docs/decisions/0280-variations-and-the-dice.md)):
//! version 104's, with variations. Each graph (`v95::GraphV95`) keeps its
//! named looks (`variations`, [`VariationV105`], stored only when there
//! are any): a name, the values of the settings shown in the painter and
//! of every seed, the graph's seed and the moment baked. Each graph fill
//! (`v97::GraphFillV97`) may name the variation it shows (`variation`,
//! stored only when it picks one). Files before version 105 hold neither:
//! no graph has variations and every layer shows its graph as it is,
//! which is how they worked; an older app would drop both, so the
//! version goes up.
//!
//! Variations are checked by `matgraph::Graph::from_parts`: at most
//! `matgraph::MAX_VARIATIONS` a graph, ids unique and below the graph's
//! counter, a name, real numbers, the moment in range. A layer naming a
//! variation its graph no longer has shows the graph as it is.

use super::v1::Problem;
use super::v95::ValueV95;
use super::v97::GraphValueV97;
use crate::limits::Bounded;
use matgraph::{BoxId, Variation, VariationId, VariationValue};
use serde::{Deserialize, Serialize};
use words::tr;

/// One stored variation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct VariationV105 {
    /// Its id in the graph.
    pub(crate) id: u64,
    /// Its name.
    pub(crate) name: String,
    /// The settings' values it holds.
    pub(crate) values: Bounded<GraphValueV97>,
    /// The graph's seed.
    pub(crate) seed: u32,
    /// The moment baked (none: the start).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) moment: Option<f32>,
}

impl Default for VariationV105 {
    /// Not stored: serde needs it to start an empty list.
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            values: Bounded::new(),
            seed: 0,
            moment: None,
        }
    }
}

/// A graph's variations as stored.
pub(crate) fn variations_of(variations: Vec<(VariationId, Variation)>) -> Bounded<VariationV105> {
    variations
        .into_iter()
        .map(|(id, v)| VariationV105 {
            id: id.0,
            name: v.name,
            values: v
                .values
                .into_iter()
                .map(|x| GraphValueV97 {
                    node: x.node.0,
                    key: x.key,
                    value: ValueV95::of(x.value),
                })
                .collect(),
            seed: v.seed,
            moment: super::v103::moment_of(v.moment),
        })
        .collect()
}

/// The variations stored as `stored`, unchecked (their gradients are).
pub(crate) fn variations_from(
    stored: Bounded<VariationV105>,
) -> Result<Vec<(VariationId, Variation)>, Problem> {
    let damaged = || -> Problem { tr("a material graph's gradient is damaged").into() };
    let mut out = Vec::with_capacity(stored.len());
    for v in stored.0 {
        let mut values = Vec::with_capacity(v.values.len());
        for x in v.values.0 {
            values.push(VariationValue {
                node: BoxId(x.node),
                key: x.key,
                value: x.value.value().ok_or_else(damaged)?,
            });
        }
        let variation = Variation {
            name: v.name,
            values,
            seed: v.seed,
            moment: super::v103::moment_from(v.moment),
        };
        out.push((VariationId(v.id), variation));
    }
    Ok(out)
}

/// Whether a graph has no variations (none are stored then).
pub(crate) fn no_variations(variations: &Bounded<VariationV105>) -> bool {
    variations.is_empty()
}
