//! One material graph: its boxes, the wires between their plugs, and its
//! comment frames.
//!
//! A graph changes only through a [`GraphEdit`] ([`Graph::apply`]), which
//! checks it and says why when it can't be made; the scene keeps graphs
//! and records each edit as an undo step. Box and frame ids are given out
//! in order from one counter and never reused, so replaying the same
//! edits gives the same ids.
//!
//! The layout is part of the graph (where each box sits, the frames'
//! rectangles) so a graph opens as it was left. Frames are data only:
//! a box is in a frame when it sits inside its rectangle, and frames sit
//! inside frames the same way.

#[cfg(test)]
mod bend_tests;
mod bends;
pub(crate) mod connect;
mod edit;
#[cfg(test)]
mod tests;

pub use edit::GraphEdit;

use crate::registry::Registry;
use crate::settings::Settings;
use glam::Vec2;
use std::collections::{BTreeMap, BTreeSet};
use words::{text, tr, trf};

/// The most boxes a graph may hold.
pub const MAX_BOXES: usize = 2000;
/// The most comment frames a graph may hold.
pub const MAX_FRAMES: usize = 500;
/// The longest name or frame title, in bytes.
pub const MAX_NAME: usize = 200;
/// The longest frame note, in bytes.
pub const MAX_NOTE: usize = 4000;
/// The farthest from the middle a box or frame may sit (canvas units).
pub const MAX_PLACE: f32 = 1.0e6;
/// The latest moment of an animation a graph can be baked at, in
/// seconds (an hour).
pub const MAX_MOMENT: f32 = 3600.0;
/// The most settings kept on one box, and the longest key, in bytes.
const MAX_SETTINGS: usize = 64;
const MAX_KEY: usize = 64;
/// The most wires into one box (for kinds this app doesn't know).
const MAX_INPUTS: usize = 16;
/// The most reroute dots one wire may bend through.
pub const MAX_BENDS: usize = 16;

/// A box's id in its graph: never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoxId(pub u64);

/// A comment frame's id in its graph: never reused (boxes and frames
/// share one counter).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrameId(pub u64);

/// One box in a graph.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphBox {
    /// Its kind's id ([`crate::BoxInfo::id`]). A kind this app doesn't know
    /// (from a newer app) is kept, and refuses to run with a reason.
    pub kind: String,
    /// Where it sits on the canvas: its top left corner.
    pub at: Vec2,
    /// Its settings changed from their standard.
    pub settings: Settings,
    /// The keys of its settings shown in the painter, on every layer
    /// using the graph (milestone 5): there each layer can set them its
    /// own way.
    pub shown: BTreeSet<String>,
    /// The keys of its settings marked "change in the game" (milestone
    /// 15, [decision 0277](../../../docs/decisions/0277-animated-boxes.md)):
    /// kept for the engine translators, which will make them settings a
    /// game can change while it runs. Until then they are baked at their
    /// value, and the export check lists them.
    pub in_game: BTreeSet<String>,
    /// What is drawn on its shape canvas, for a box drawn there (the
    /// Shape box, milestone 11); `None` for every other box, and for a
    /// Shape box with nothing drawn yet.
    pub drawing: Option<std::sync::Arc<shapes::Drawing>>,
    /// Whether it is bypassed (milestone 16): switched off for a moment,
    /// what goes in passes straight through to its outputs (see
    /// [`crate::eval`]), so the graph can be seen with and without it.
    pub bypassed: bool,
}

impl GraphBox {
    /// A box of kind `kind` at `at`, with its standard settings, none
    /// shown in the painter.
    pub fn new(kind: impl Into<String>, at: Vec2) -> Self {
        Self {
            kind: kind.into(),
            at,
            settings: Settings::new(),
            shown: BTreeSet::new(),
            in_game: BTreeSet::new(),
            drawing: None,
            bypassed: false,
        }
    }

    /// The edits giving box `node` (just added, of the same kind) this
    /// box's settings, drawing, marks and bypass: for copies, pastes and
    /// opening your own box back up.
    pub fn copy_edits(&self, node: BoxId) -> Vec<GraphEdit> {
        let mut edits: Vec<GraphEdit> = self
            .settings
            .iter()
            .map(|(key, value)| GraphEdit::SetSetting {
                node,
                key: key.to_string(),
                value,
            })
            .collect();
        if let Some(drawing) = &self.drawing {
            edits.push(GraphEdit::SetDrawing {
                node,
                drawing: drawing.clone(),
            });
        }
        edits.extend(self.in_game.iter().map(|key| GraphEdit::ChangeInGame {
            node,
            key: key.clone(),
            on: true,
        }));
        if self.bypassed {
            edits.push(GraphEdit::Bypass { node, on: true });
        }
        edits
    }
}

/// One setting of a graph shown in the painter: which box, which setting.
#[derive(Debug, Clone, PartialEq)]
pub struct Shown<'a> {
    /// The box.
    pub node: BoxId,
    /// Its kind.
    pub info: &'a crate::BoxInfo,
    /// The setting.
    pub spec: &'a crate::SettingSpec,
    /// Its value in the graph.
    pub value: crate::SettingValue,
}

/// Whether setting `spec` can be marked "change in the game": a number,
/// a length, a whole number, a colour or a switch (what a game's shader
/// can be given). A choice, a gradient or a seed changes how the box
/// works, so it is always baked.
pub fn changes_in_game(spec: &crate::SettingSpec) -> bool {
    use crate::SettingKind as K;
    matches!(
        spec.kind,
        K::Number { .. } | K::Length { .. } | K::Whole { .. } | K::Colour { .. } | K::Switch { .. }
    )
}

/// A plug on a box: the box and the plug's key.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlugRef {
    /// The box.
    pub node: BoxId,
    /// The plug's key ([`crate::PlugSpec::key`]).
    pub plug: String,
}

impl PlugRef {
    /// Plug `plug` on box `node`.
    pub fn new(node: BoxId, plug: impl Into<String>) -> Self {
        Self {
            node,
            plug: plug.into(),
        }
    }
}

/// A wire from an output to an input. An input takes at most one wire;
/// an output can feed many.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Wire {
    /// The output it comes from.
    pub from: PlugRef,
    /// The input it goes into.
    pub to: PlugRef,
}

/// A comment frame: a coloured rectangle with a title, grouping boxes.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// Its title.
    pub title: String,
    /// Its colour (sRGB).
    pub colour: [u8; 3],
    /// Its top left corner.
    pub min: Vec2,
    /// Its bottom right corner.
    pub max: Vec2,
    /// A longer note, shown in Attributes and under its title on the
    /// canvas.
    pub note: String,
}

impl Frame {
    /// A frame titled `title` over the rectangle from `min` to `max`.
    pub fn new(title: impl Into<String>, min: Vec2, max: Vec2) -> Self {
        Self {
            title: title.into(),
            colour: [86, 110, 140],
            min,
            max,
            note: String::new(),
        }
    }

    /// Whether `point` is inside it.
    pub fn contains(&self, point: Vec2) -> bool {
        point.cmpge(self.min).all() && point.cmple(self.max).all()
    }

    /// Why it can't be kept, if it can't.
    fn problem(&self) -> Option<&'static str> {
        let finite = [self.min, self.max]
            .iter()
            .all(|p| p.is_finite() && p.abs().max_element() <= MAX_PLACE);
        if !finite || self.min.cmpgt(self.max).any() {
            Some(text!("a frame's corners are out of place"))
        } else if self.title.len() > MAX_NAME {
            Some(text!("a frame's title is too long"))
        } else if self.note.len() > MAX_NOTE {
            Some(text!("a frame's note is too long"))
        } else {
            None
        }
    }
}

/// A graph as plain parts, to save or to rebuild one from a file
/// ([`Graph::from_parts`] checks everything).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GraphParts {
    /// Its name.
    pub name: String,
    /// Its seed, mixed into every box's own.
    pub seed: u32,
    /// The next id to give a box or frame.
    pub next_id: u64,
    /// Its boxes.
    pub boxes: Vec<(BoxId, GraphBox)>,
    /// Its wires.
    pub wires: Vec<Wire>,
    /// Its comment frames.
    pub frames: Vec<(FrameId, Frame)>,
    /// The moment of its animation it is baked at, in seconds
    /// ([`Graph::moment`]).
    pub moment: f32,
    /// The reroute dots each wire bends through, by the input it goes
    /// into ([`Graph::bends`]).
    pub bends: Vec<(PlugRef, Vec<Vec2>)>,
}

/// `seconds` as a moment of an animation: from 0 to [`MAX_MOMENT`], and
/// 0 when it isn't a real number.
pub fn usable_moment(seconds: f32) -> f32 {
    if seconds.is_finite() {
        seconds.clamp(0.0, MAX_MOMENT)
    } else {
        0.0
    }
}

/// One material graph.
#[derive(Debug, Clone, PartialEq)]
pub struct Graph {
    name: String,
    seed: u32,
    next_id: u64,
    boxes: BTreeMap<BoxId, GraphBox>,
    /// Kept sorted, one per input.
    wires: Vec<Wire>,
    frames: BTreeMap<FrameId, Frame>,
    moment: f32,
    /// Reroute dots by the input their wire goes into: only wires that
    /// are there have any, and none has an empty list.
    bends: BTreeMap<PlugRef, Vec<Vec2>>,
}

impl Graph {
    /// An empty graph called `name`.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            seed: 0,
            next_id: 1,
            boxes: BTreeMap::new(),
            wires: Vec::new(),
            frames: BTreeMap::new(),
            moment: 0.0,
            bends: BTreeMap::new(),
        }
    }

    /// The moment of its animation, in seconds, that exports and the
    /// painter bake (milestone 15): animated boxes are worked out at it
    /// unless the preview's player asks for another
    /// ([`crate::Evaluator::set_time`]). 0 to start with.
    pub fn moment(&self) -> f32 {
        self.moment
    }

    /// Whether anything in it moves over time: an animated box
    /// ([`crate::BoxInfo::animated`], including an own box holding one).
    /// A bypassed box (milestone 16) doesn't move: it passes its input
    /// on.
    pub fn is_animated(&self, registry: &Registry) -> bool {
        self.boxes
            .values()
            .any(|b| !b.bypassed && registry.get(&b.kind).is_some_and(|t| t.info().animated))
    }

    /// Its name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Its seed.
    pub fn seed(&self) -> u32 {
        self.seed
    }

    /// The id the next box or frame added gets.
    pub fn next_id(&self) -> u64 {
        self.next_id
    }

    /// Box `id`.
    pub fn get(&self, id: BoxId) -> Option<&GraphBox> {
        self.boxes.get(&id)
    }

    /// Every box, by id.
    pub fn boxes(&self) -> impl Iterator<Item = (BoxId, &GraphBox)> {
        self.boxes.iter().map(|(id, b)| (*id, b))
    }

    /// How many boxes it holds.
    pub fn box_count(&self) -> usize {
        self.boxes.len()
    }

    /// Every wire, sorted.
    pub fn wires(&self) -> &[Wire] {
        &self.wires
    }

    /// The wire into input `to`, if any.
    pub fn wire_into(&self, to: &PlugRef) -> Option<&Wire> {
        self.wires.iter().find(|w| w.to == *to)
    }

    /// Frame `id`.
    pub fn frame(&self, id: FrameId) -> Option<&Frame> {
        self.frames.get(&id)
    }

    /// Every comment frame, by id.
    pub fn frames(&self) -> impl Iterator<Item = (FrameId, &Frame)> {
        self.frames.iter().map(|(id, f)| (*id, f))
    }

    /// The boxes inside frame `id` (their corner inside its rectangle).
    pub fn boxes_in(&self, id: FrameId) -> Vec<BoxId> {
        self.frames.get(&id).map_or_else(Vec::new, |f| {
            self.boxes()
                .filter(|(_, b)| f.contains(b.at))
                .map(|(id, _)| id)
                .collect()
        })
    }

    /// The first box of kind `kind`.
    pub fn first_of(&self, kind: &str) -> Option<BoxId> {
        self.boxes().find(|(_, b)| b.kind == kind).map(|(id, _)| id)
    }

    /// The boxes feeding box `id` directly, through its wires.
    pub fn sources_of(&self, id: BoxId) -> impl Iterator<Item = BoxId> + '_ {
        self.wires
            .iter()
            .filter(move |w| w.to.node == id)
            .map(|w| w.from.node)
    }

    /// Whether `to` is fed by `from`, through any number of boxes (or is
    /// it).
    pub fn feeds(&self, from: BoxId, to: BoxId) -> bool {
        let mut seen = BTreeSet::new();
        let mut todo = vec![to];
        while let Some(id) = todo.pop() {
            if id == from {
                return true;
            }
            if seen.insert(id) {
                todo.extend(self.sources_of(id));
            }
        }
        false
    }

    /// `wanted` and every box feeding them, in an order where each box
    /// comes after everything feeding it; every box when `wanted` is
    /// empty. A box in a loop (only in a damaged graph) is left out.
    pub fn order(&self, wanted: &[BoxId]) -> Vec<BoxId> {
        let starts: Vec<BoxId> = if wanted.is_empty() {
            self.boxes.keys().copied().collect()
        } else {
            wanted.to_vec()
        };
        // Depth first, children before parents; "open" marks a box on the
        // current path, so a loop is noticed and skipped.
        let mut done = BTreeSet::new();
        let mut open = BTreeSet::new();
        let mut out = Vec::new();
        for start in starts {
            if !self.boxes.contains_key(&start) {
                continue;
            }
            let mut stack = vec![(start, false)];
            while let Some((id, children_done)) = stack.pop() {
                if children_done {
                    open.remove(&id);
                    if done.insert(id) {
                        out.push(id);
                    }
                    continue;
                }
                if done.contains(&id) || !open.insert(id) {
                    continue;
                }
                stack.push((id, true));
                for source in self.sources_of(id) {
                    if !done.contains(&source) && !open.contains(&source) {
                        stack.push((source, false));
                    }
                }
            }
        }
        out
    }

    /// Whether any box feeds itself.
    pub fn has_loop(&self) -> bool {
        self.boxes
            .keys()
            .any(|id| self.sources_of(*id).any(|s| self.feeds(*id, s)))
    }

    /// The settings shown in the painter (milestone 5), box by box in
    /// the order the graph runs them, each box's in its settings' order.
    /// Boxes of kinds `registry` doesn't know are left out.
    pub fn shown<'r>(&self, registry: &'r Registry) -> Vec<Shown<'r>> {
        let mut out = Vec::new();
        for id in self.order(&[]) {
            let Some(b) = self.get(id) else { continue };
            if b.shown.is_empty() {
                continue;
            }
            let Some(info) = registry.get(&b.kind).map(|t| t.info()) else {
                continue;
            };
            for spec in &info.settings {
                if b.shown.contains(spec.key.as_ref()) {
                    let value = b.settings.get(&spec.key).unwrap_or_else(|| spec.standard());
                    out.push(Shown {
                        node: id,
                        info,
                        spec,
                        value: spec.nearest(value),
                    });
                }
            }
        }
        out
    }

    /// The settings marked "change in the game" (milestone 15), box by
    /// box in the order the graph runs them, each box's in its settings'
    /// order. Boxes of kinds `registry` doesn't know are left out.
    pub fn in_game<'r>(&self, registry: &'r Registry) -> Vec<Shown<'r>> {
        let mut out = Vec::new();
        for id in self.order(&[]) {
            let Some(b) = self.get(id) else { continue };
            if b.in_game.is_empty() {
                continue;
            }
            let Some(info) = registry.get(&b.kind).map(|t| t.info()) else {
                continue;
            };
            for spec in &info.settings {
                if b.in_game.contains(spec.key.as_ref()) {
                    let value = b.settings.get(&spec.key).unwrap_or_else(|| spec.standard());
                    out.push(Shown {
                        node: id,
                        info,
                        spec,
                        value: spec.nearest(value),
                    });
                }
            }
        }
        out
    }

    /// This graph with settings changed: each `(box, key, value)` set as
    /// [`GraphEdit::SetSetting`] would. One that is refused (a box or
    /// setting no longer there, a value of the wrong kind) is left out, so
    /// a layer's own values outlive changes to the graph.
    pub fn with_settings<'v>(
        &self,
        registry: &Registry,
        values: impl IntoIterator<Item = (BoxId, &'v str, crate::SettingValue)>,
    ) -> Graph {
        let mut out = self.clone();
        for (node, key, value) in values {
            let edit = GraphEdit::SetSetting {
                node,
                key: key.to_string(),
                value,
            };
            let _ = out.apply(registry, &edit);
        }
        out
    }

    /// Its plain parts, to save.
    pub fn to_parts(&self) -> GraphParts {
        GraphParts {
            name: self.name.clone(),
            seed: self.seed,
            next_id: self.next_id,
            boxes: self.boxes().map(|(id, b)| (id, b.clone())).collect(),
            wires: self.wires.clone(),
            frames: self.frames().map(|(id, f)| (id, f.clone())).collect(),
            moment: self.moment,
            bends: self
                .bends
                .iter()
                .map(|(to, points)| (to.clone(), points.clone()))
                .collect(),
        }
    }

    /// Rebuilds a graph from its parts (from a file, so anything may be
    /// wrong), checked against `registry`: ids unique and below
    /// `next_id`, wires between plugs the boxes have and fitting, one per
    /// input, no loops, places real, within the limits. Settings are made
    /// allowed (kept in range); settings a box doesn't have are dropped.
    /// Boxes of kinds `registry` doesn't know are kept as they are.
    pub fn from_parts(parts: GraphParts, registry: &Registry) -> Result<Self, String> {
        let damaged = |why: &'static str| Err(tr(why).to_string());
        if parts.boxes.len() > MAX_BOXES
            || parts.frames.len() > MAX_FRAMES
            || parts.wires.len() > MAX_BOXES * MAX_INPUTS
        {
            return damaged(text!("it has more boxes or frames than a graph may hold"));
        }
        if parts.name.trim().is_empty() || parts.name.len() > MAX_NAME {
            return damaged(text!("its name is missing or too long"));
        }
        let mut graph = Self::new(parts.name);
        graph.seed = parts.seed;
        graph.next_id = parts.next_id;
        graph.moment = usable_moment(parts.moment);
        for (id, mut b) in parts.boxes {
            if id.0 == 0 || id.0 >= parts.next_id || graph.boxes.contains_key(&id) {
                return damaged(text!("two boxes share an id, or one's id is out of range"));
            }
            if !b.at.is_finite() || b.at.abs().max_element() > MAX_PLACE {
                return damaged(text!("a box is out of place"));
            }
            if b.kind.is_empty() || b.kind.len() > MAX_KEY || b.settings.len() > MAX_SETTINGS {
                return damaged(text!("a box's kind or settings are damaged"));
            }
            if b.settings.iter().any(|(k, _)| k.len() > MAX_KEY) {
                return damaged(text!("a box's kind or settings are damaged"));
            }
            let too_long = |keys: &BTreeSet<String>| {
                keys.len() > MAX_SETTINGS || keys.iter().any(|k| k.len() > MAX_KEY)
            };
            if too_long(&b.shown) || too_long(&b.in_game) {
                return damaged(text!("a box's kind or settings are damaged"));
            }
            if let Some(kind) = registry.get(&b.kind) {
                b.settings = b.settings.normalised(&kind.info().settings);
                // Settings a box doesn't have can't be shown.
                b.shown.retain(|k| kind.info().setting(k).is_some());
                b.in_game
                    .retain(|k| kind.info().setting(k).is_some_and(changes_in_game));
                // Only a box drawn on the shape canvas keeps a drawing.
                if !kind.takes_drawing() {
                    b.drawing = None;
                }
            }
            graph.boxes.insert(id, b);
        }
        let box_ids = graph.boxes.keys().copied().collect::<BTreeSet<_>>();
        for (id, frame) in parts.frames {
            if id.0 == 0
                || id.0 >= parts.next_id
                || box_ids.contains(&BoxId(id.0))
                || graph.frames.contains_key(&id)
            {
                return damaged(text!("two frames share an id, or one's id is out of range"));
            }
            if let Some(why) = frame.problem() {
                return damaged(why);
            }
            graph.frames.insert(id, frame);
        }
        for wire in parts.wires {
            if let Err(why) = graph.wire_problem(registry, &wire) {
                return Err(trf!("a wire is damaged: {}", why));
            }
            graph.wires.push(wire);
        }
        graph.wires.sort();
        if graph.has_loop() {
            return damaged(text!("its wires go round in a loop"));
        }
        graph.bends = bends::from_parts(&graph, parts.bends).map_err(|why| tr(why).to_string())?;
        Ok(graph)
    }

    /// Why `wire` can't be added to the graph as it is (it is checked as
    /// a file's wire: kinds converting is fine).
    fn wire_problem(&self, registry: &Registry, wire: &Wire) -> Result<(), String> {
        let (Some(from), Some(to)) = (self.get(wire.from.node), self.get(wire.to.node)) else {
            return Err(tr("it joins a box that isn't there").to_string());
        };
        if wire.from.node == wire.to.node {
            return Err(tr("it joins a box to itself").to_string());
        }
        if self.wire_into(&wire.to).is_some() {
            return Err(tr("two wires go into one plug").to_string());
        }
        if let (Some(f), Some(t)) = (registry.get(&from.kind), registry.get(&to.kind)) {
            let (Some(out), Some(input)) = (
                f.info().output(&wire.from.plug),
                t.info().input(&wire.to.plug),
            ) else {
                return Err(tr("it joins a plug the box doesn't have").to_string());
            };
            crate::kind::fit(out.kind, input.kind).map_err(str::to_string)?;
        } else if wire.from.plug.len() > MAX_KEY || wire.to.plug.len() > MAX_KEY {
            return Err(tr("it joins a plug the box doesn't have").to_string());
        }
        Ok(())
    }

    /// Roughly how much memory it takes, in bytes (for the undo budget).
    pub fn memory_size(&self) -> usize {
        let boxes: usize = self
            .boxes
            .values()
            .map(|b| {
                64 + b.kind.len()
                    + b.settings.len() * 48
                    + (b.shown.len() + b.in_game.len()) * 32
                    + b.drawing.as_ref().map_or(0, |d| d.memory_size())
            })
            .sum();
        let wires: usize = self
            .wires
            .iter()
            .map(|w| 32 + w.from.plug.len() + w.to.plug.len())
            .sum();
        let frames: usize = self
            .frames
            .values()
            .map(|f| 64 + f.title.len() + f.note.len())
            .sum();
        let bends: usize = self.bends.values().map(|p| 48 + p.len() * 8).sum();
        64 + self.name.len() + boxes + wires + frames + bends
    }
}
