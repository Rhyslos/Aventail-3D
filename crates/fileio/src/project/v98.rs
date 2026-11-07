//! Version 98 of the project file ([decision 0263](../../../../docs/decisions/0263-your-own-boxes.md)):
//! version 97's, with the project's own boxes (`v95::GraphsV95::own`,
//! stored as [`OwnBoxV98`]): each one's id, name, words and picture, the
//! graph inside (stored as a graph is), its plugs and its sliders. Graphs
//! place them by id. Files before version 98 hold none; an older app
//! couldn't run a graph using one, so the version goes up.
//!
//! Box files (`crate::own_box_file`) store own boxes the same way.

use super::v1::Problem;
use super::v95::GraphV95;
use crate::limits::Bounded;
use matgraph::own::{MAX_OWN_BOXES, OwnBox, OwnInput, OwnOutput, OwnParts, OwnSlider, is_own};
use matgraph::{BoxId, GraphId, PlugRef, ValueKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::Arc;
use words::{tr, trf};

/// A stored own box.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct OwnBoxV98 {
    /// Its id (`own:` and hex digits).
    pub(crate) id: String,
    /// Its name.
    pub(crate) name: String,
    /// "What does this do?".
    pub(crate) about: String,
    /// Its picture: sRGB bytes, row by row.
    #[serde(default)]
    pub(crate) picture: Option<Bounded<u8>>,
    /// The graph inside (its id unused).
    pub(crate) inner: GraphV95,
    /// Its inputs.
    pub(crate) inputs: Bounded<InputV98>,
    /// Its outputs.
    pub(crate) outputs: Bounded<OutputV98>,
    /// Its sliders.
    pub(crate) sliders: Bounded<SliderV98>,
}

/// A plug inside: a box and its plug's key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct PlugV98 {
    /// The box inside.
    pub(crate) node: u64,
    /// Its plug.
    pub(crate) plug: String,
}

/// An input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct InputV98 {
    /// Its key.
    pub(crate) key: String,
    /// Its name.
    pub(crate) name: String,
    /// What it takes ([`kind_of`]).
    pub(crate) kind: u8,
    /// The plugs inside it feeds.
    pub(crate) to: Bounded<PlugV98>,
}

/// An output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct OutputV98 {
    /// Its key.
    pub(crate) key: String,
    /// Its name.
    pub(crate) name: String,
    /// What it gives ([`kind_of`]).
    pub(crate) kind: u8,
    /// The plug inside it gives.
    pub(crate) from: Option<PlugV98>,
}

/// A slider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct SliderV98 {
    /// Its key.
    pub(crate) key: String,
    /// Its name.
    pub(crate) name: String,
    /// The box inside.
    pub(crate) node: u64,
    /// That box's setting.
    pub(crate) setting: String,
}

/// A value kind as stored: 0 shape, 1 height, 2 colour, 3 material.
fn number_of(kind: ValueKind) -> u8 {
    match kind {
        ValueKind::Shape => 0,
        ValueKind::Height => 1,
        ValueKind::Colour => 2,
        ValueKind::Material => 3,
    }
}

/// A stored value kind; `None` for a number this version doesn't know.
fn kind_of(number: u8) -> Option<ValueKind> {
    Some(match number {
        0 => ValueKind::Shape,
        1 => ValueKind::Height,
        2 => ValueKind::Colour,
        3 => ValueKind::Material,
        _ => return None,
    })
}

fn plug_of(p: &PlugRef) -> PlugV98 {
    PlugV98 {
        node: p.node.0,
        plug: p.plug.clone(),
    }
}

fn plug_from(p: PlugV98) -> PlugRef {
    PlugRef::new(BoxId(p.node), p.plug)
}

/// `own` as stored.
pub(crate) fn own_box_of(own: &OwnBox) -> OwnBoxV98 {
    OwnBoxV98 {
        id: own.id.clone(),
        name: own.name.clone(),
        about: own.about.clone(),
        picture: own.picture.clone().map(Bounded),
        inner: GraphV95::of(GraphId(0), &own.inner),
        inputs: own
            .inputs
            .iter()
            .map(|i| InputV98 {
                key: i.key.clone(),
                name: i.name.clone(),
                kind: number_of(i.kind),
                to: i.to.iter().map(plug_of).collect(),
            })
            .collect(),
        outputs: own
            .outputs
            .iter()
            .map(|o| OutputV98 {
                key: o.key.clone(),
                name: o.name.clone(),
                kind: number_of(o.kind),
                from: o.from.as_ref().map(plug_of),
            })
            .collect(),
        sliders: own
            .sliders
            .iter()
            .map(|s| SliderV98 {
                key: s.key.clone(),
                name: s.name.clone(),
                node: s.node.0,
                setting: s.setting.clone(),
            })
            .collect(),
    }
}

/// A stored own box's parts, unchecked; a kind number this version
/// doesn't know is refused.
fn parts_from(stored: OwnBoxV98) -> Result<OwnParts, Problem> {
    let damaged = || -> Problem { tr("its own boxes are damaged").into() };
    let (_, inner) = stored.inner.into_parts()?;
    let mut inputs = Vec::with_capacity(stored.inputs.len());
    for i in stored.inputs.0 {
        inputs.push(OwnInput {
            key: i.key,
            name: i.name,
            kind: kind_of(i.kind).ok_or_else(damaged)?,
            to: i.to.0.into_iter().map(plug_from).collect(),
        });
    }
    let mut outputs = Vec::with_capacity(stored.outputs.len());
    for o in stored.outputs.0 {
        outputs.push(OwnOutput {
            key: o.key,
            name: o.name,
            kind: kind_of(o.kind).ok_or_else(damaged)?,
            from: o.from.map(plug_from),
        });
    }
    Ok(OwnParts {
        id: stored.id,
        name: stored.name,
        about: stored.about,
        picture: stored.picture.map(|p| p.0),
        inner,
        inputs,
        outputs,
        sliders: stored
            .sliders
            .0
            .into_iter()
            .map(|s| OwnSlider {
                key: s.key,
                name: s.name,
                node: BoxId(s.node),
                setting: s.setting,
            })
            .collect(),
    })
}

/// Stored own boxes, each checked, in an order where every own box comes
/// after those inside it (their ids unique, at most
/// [`MAX_OWN_BOXES`]). Own boxes inside each other in a loop are refused.
pub(crate) fn own_boxes_from(stored: Bounded<OwnBoxV98>) -> Result<Vec<OwnBox>, Problem> {
    if stored.len() > MAX_OWN_BOXES {
        return Err(tr("its own boxes are damaged").into());
    }
    let mut waiting = stored
        .0
        .into_iter()
        .map(parts_from)
        .collect::<Result<Vec<_>, _>>()?;
    let ids: BTreeSet<String> = waiting.iter().map(|p| p.id.clone()).collect();
    if ids.len() != waiting.len() {
        return Err(tr("two of its own boxes share an id").into());
    }
    let mut done: Vec<OwnBox> = Vec::new();
    let mut registry = matgraph::Registry::standard().clone();
    while !waiting.is_empty() {
        let (ready, later): (Vec<OwnParts>, Vec<OwnParts>) = waiting.into_iter().partition(|p| {
            p.inner
                .boxes
                .iter()
                .filter(|(_, b)| is_own(&b.kind) && ids.contains(&b.kind))
                .all(|(_, b)| done.iter().any(|d| d.id == b.kind))
        });
        if ready.is_empty() {
            return Err(tr("its own boxes hold each other in a loop").into());
        }
        for parts in ready {
            let own = OwnBox::from_parts(parts, &registry)
                .map_err(|why| trf!("one of its own boxes is damaged: {}", why))?;
            done.push(own.clone());
            matgraph::own::add_own(&mut registry, Arc::new(own)).map_err(|why| why.reason)?;
        }
        waiting = later;
    }
    Ok(done)
}
