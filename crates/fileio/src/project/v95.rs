//! Version 95 of the project file ([decision 0257](../../../../docs/decisions/0257-material-graphs-core.md)):
//! version 94's, with the project's material graphs
//! ([`super::v43::ProjectV43::graphs`], stored as [`GraphsV95`]): each
//! graph's boxes (kind, place, changed settings by key), wires (between
//! plug keys) and comment frames. Files before version 95 hold none.
//!
//! The model is the file's own, kept apart from `matgraph`'s types, so
//! those can change without breaking old files. Everything read is
//! checked by `matgraph::Graph::from_parts`.

use super::v1::Problem;
use crate::limits::Bounded;
use glam::Vec2;
use matgraph::{
    BoxId, Frame, FrameId, Graph, GraphBox, GraphId, GraphParts, GraphStore, PlugRef, Registry,
    SettingValue, Settings, Wire,
};
use scene::Document;
use serde::{Deserialize, Serialize};
use words::{tr, trf};

/// The project's graphs as stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct GraphsV95 {
    /// The id the next graph gets.
    pub(crate) next_id: u64,
    /// Each graph.
    pub(crate) graphs: Bounded<GraphV95>,
    /// The project's own boxes (version 98, see `v98`).
    #[serde(default = "Bounded::new")]
    pub(crate) own: Bounded<super::v98::OwnBoxV98>,
}

/// One graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct GraphV95 {
    /// Its id in the project.
    pub(crate) id: u64,
    /// Its name.
    pub(crate) name: String,
    /// Its seed.
    pub(crate) seed: u32,
    /// The id its next box or frame gets.
    pub(crate) next_id: u64,
    /// Its boxes.
    pub(crate) boxes: Bounded<BoxV95>,
    /// Its wires.
    pub(crate) wires: Bounded<WireV95>,
    /// Its comment frames.
    pub(crate) frames: Bounded<FrameV95>,
    /// The moment of its animation it is baked at (version 103, see
    /// `v103`); none: the start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) moment: Option<f32>,
    /// The reroute dots its wires bend through (version 104, see `v104`).
    #[serde(default, skip_serializing_if = "super::v104::no_bends")]
    pub(crate) bends: Bounded<super::v104::BendsV104>,
    /// Its variations (version 105, see `v105`).
    #[serde(default, skip_serializing_if = "super::v105::no_variations")]
    pub(crate) variations: Bounded<super::v105::VariationV105>,
}

/// One box.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct BoxV95 {
    /// Its id in the graph.
    pub(crate) id: u64,
    /// Its kind's id.
    pub(crate) kind: String,
    /// Where it sits.
    pub(crate) at: [f32; 2],
    /// Its settings changed from their standard.
    pub(crate) settings: Bounded<SettingV95>,
    /// The keys of its settings shown in the painter (version 97).
    #[serde(default = "Bounded::new")]
    pub(crate) shown: Bounded<String>,
    /// The keys of its settings marked "change in the game" (version 103,
    /// see `v103`).
    #[serde(default = "Bounded::new", skip_serializing_if = "super::v103::no_keys")]
    pub(crate) in_game: Bounded<String>,
    /// What is drawn on its shape canvas, for a Shape box (version 100,
    /// see `v100`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) drawing: Option<super::v100::DrawingV100>,
    /// Whether it is bypassed (version 104, see `v104`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub(crate) bypassed: bool,
}

/// One setting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct SettingV95 {
    /// Its key.
    pub(crate) key: String,
    /// Its value.
    pub(crate) value: ValueV95,
}

/// A setting's value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum ValueV95 {
    /// A number.
    Number(f32),
    /// A whole number or a seed.
    Whole(i32),
    /// A colour.
    Colour([f32; 3]),
    /// A choice, by index.
    Choice(u32),
    /// On or off.
    Switch(bool),
    /// A gradient (version 96, see `v96`).
    Gradient(super::v96::GradientV96),
    /// One of the project's palettes, by name (version 106, see `v106`).
    Palette(String),
}

/// One wire.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct WireV95 {
    /// The box it comes from.
    pub(crate) from: u64,
    /// That box's output.
    pub(crate) from_plug: String,
    /// The box it goes into.
    pub(crate) to: u64,
    /// That box's input.
    pub(crate) to_plug: String,
}

/// One comment frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct FrameV95 {
    /// Its id in the graph.
    pub(crate) id: u64,
    /// Its title.
    pub(crate) title: String,
    /// Its colour (sRGB).
    pub(crate) colour: [u8; 3],
    /// Its top left corner.
    pub(crate) min: [f32; 2],
    /// Its bottom right corner.
    pub(crate) max: [f32; 2],
    /// Its note.
    pub(crate) note: String,
}

impl ValueV95 {
    pub(crate) fn of(value: SettingValue) -> Self {
        match value {
            SettingValue::Number(n) => Self::Number(n),
            SettingValue::Whole(n) => Self::Whole(n),
            SettingValue::Colour(c) => Self::Colour(c),
            SettingValue::Choice(c) => Self::Choice(c),
            SettingValue::Switch(b) => Self::Switch(b),
            SettingValue::Gradient(g) => Self::Gradient(super::v96::gradient_of(&g)),
            SettingValue::Palette(name) => Self::Palette(name),
        }
    }

    /// The value; `None` for a gradient without 2 to 8 colours.
    pub(crate) fn value(self) -> Option<SettingValue> {
        Some(match self {
            Self::Number(n) => SettingValue::Number(n),
            Self::Whole(n) => SettingValue::Whole(n),
            Self::Colour(c) => SettingValue::Colour(c),
            Self::Choice(c) => SettingValue::Choice(c),
            Self::Switch(b) => SettingValue::Switch(b),
            Self::Gradient(g) => SettingValue::Gradient(super::v96::gradient_from(g)?),
            Self::Palette(name) => SettingValue::Palette(name),
        })
    }
}

impl GraphV95 {
    pub(crate) fn of(id: GraphId, graph: &Graph) -> Self {
        let parts = graph.to_parts();
        Self {
            id: id.0,
            name: parts.name,
            seed: parts.seed,
            next_id: parts.next_id,
            boxes: parts
                .boxes
                .into_iter()
                .map(|(id, b)| BoxV95 {
                    id: id.0,
                    kind: b.kind,
                    at: b.at.to_array(),
                    settings: b
                        .settings
                        .iter()
                        .map(|(key, value)| SettingV95 {
                            key: key.to_string(),
                            value: ValueV95::of(value),
                        })
                        .collect(),
                    shown: b.shown.into_iter().collect(),
                    in_game: b.in_game.into_iter().collect(),
                    drawing: b.drawing.as_deref().map(super::v100::drawing_of),
                    bypassed: b.bypassed,
                })
                .collect(),
            wires: parts
                .wires
                .into_iter()
                .map(|w| WireV95 {
                    from: w.from.node.0,
                    from_plug: w.from.plug,
                    to: w.to.node.0,
                    to_plug: w.to.plug,
                })
                .collect(),
            frames: parts
                .frames
                .into_iter()
                .map(|(id, f)| FrameV95 {
                    id: id.0,
                    title: f.title,
                    colour: f.colour,
                    min: f.min.to_array(),
                    max: f.max.to_array(),
                    note: f.note,
                })
                .collect(),
            moment: super::v103::moment_of(parts.moment),
            bends: super::v104::bends_of(parts.bends),
            variations: super::v105::variations_of(parts.variations),
        }
    }

    /// The graph, checked against `registry` (the standard boxes and the
    /// project's own).
    fn into_graph(self, registry: &Registry) -> Result<(GraphId, Graph), Problem> {
        let (id, parts) = self.into_parts()?;
        let graph = Graph::from_parts(parts, registry)
            .map_err(|why| trf!("a material graph is damaged: {}", why))?;
        Ok((id, graph))
    }

    /// The graph's parts, unchecked (their gradients are).
    pub(crate) fn into_parts(self) -> Result<(GraphId, GraphParts), Problem> {
        let mut boxes = Vec::with_capacity(self.boxes.len());
        for b in self.boxes.0 {
            let mut settings = Settings::new();
            for s in b.settings.0 {
                let value = s
                    .value
                    .value()
                    .ok_or_else(|| tr("a material graph's gradient is damaged").to_string())?;
                settings.set(s.key, value);
            }
            let place = GraphBox {
                kind: b.kind,
                at: Vec2::from_array(b.at),
                settings,
                shown: b.shown.0.into_iter().collect(),
                in_game: b.in_game.0.into_iter().collect(),
                drawing: b
                    .drawing
                    .map(super::v100::drawing_from)
                    .transpose()?
                    .map(std::sync::Arc::new),
                bypassed: b.bypassed,
            };
            boxes.push((BoxId(b.id), place));
        }
        let parts = GraphParts {
            name: self.name,
            seed: self.seed,
            next_id: self.next_id,
            boxes,
            wires: self
                .wires
                .0
                .into_iter()
                .map(|w| Wire {
                    from: PlugRef::new(BoxId(w.from), w.from_plug),
                    to: PlugRef::new(BoxId(w.to), w.to_plug),
                })
                .collect(),
            frames: self
                .frames
                .0
                .into_iter()
                .map(|f| {
                    let frame = Frame {
                        title: f.title,
                        colour: f.colour,
                        min: Vec2::from_array(f.min),
                        max: Vec2::from_array(f.max),
                        note: f.note,
                    };
                    (FrameId(f.id), frame)
                })
                .collect(),
            moment: super::v103::moment_from(self.moment),
            bends: super::v104::bends_from(self.bends),
            variations: super::v105::variations_from(self.variations)?,
        };
        Ok((GraphId(self.id), parts))
    }
}

/// The graphs to store; none for a project without any.
pub(crate) fn graphs_of(document: &Document) -> Option<GraphsV95> {
    let store = document.scene().graphs();
    let any = !store.is_empty() || store.next_id() != GraphId(1) || store.own_count() > 0;
    any.then(|| GraphsV95 {
        next_id: store.next_id().0,
        graphs: store.iter().map(|(id, g)| GraphV95::of(id, g)).collect(),
        own: store
            .own_boxes()
            .map(|o| super::v98::own_box_of(o))
            .collect(),
    })
}

/// `document` with the stored graphs, each checked.
pub(crate) fn with_graphs(
    document: Document,
    stored: Option<GraphsV95>,
) -> Result<Document, Problem> {
    let Some(stored) = stored else {
        return Ok(document);
    };
    // Own boxes first: the graphs may use them.
    let own = super::v98::own_boxes_from(stored.own)?;
    let empty = GraphStore::new()
        .with_own(own.clone())
        .ok_or_else(|| tr("its own boxes are damaged").to_string())?;
    let registry = empty.registry().clone();
    let graphs = stored
        .graphs
        .0
        .into_iter()
        .map(|g| g.into_graph(&registry))
        .collect::<Result<Vec<_>, _>>()?;
    let store = GraphStore::from_graphs(graphs, stored.next_id)
        .and_then(|s| s.with_own(own))
        .ok_or_else(|| tr("its material graphs are damaged").to_string())?;
    Ok(document.with_graphs(store))
}
