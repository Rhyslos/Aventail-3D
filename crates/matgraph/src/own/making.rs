//! Making your own box from selected boxes (Alt + G), and opening one
//! back up into its boxes (Shift + G).
//!
//! Both are worked out here as plain changes: a new [`OwnBox`] and the
//! [`GraphEdit`]s that replace the selection with one box of it, or the
//! edits that replace a placed own box with copies of the boxes inside.
//! The scene makes them one undo step. Neither changes the picture: the
//! boxes inside run with the graph's seed and real-world size, and the
//! sliders start at the inner boxes' values.

use super::{MAX_DEPTH, MAX_PLUGS, MAX_SLIDERS, OwnBox, OwnInput, OwnOutput, OwnSlider};
use crate::graph::{BoxId, Graph, GraphEdit, GraphParts, MAX_NAME, PlugRef};
use crate::kind::ValueKind;
use crate::refusal::{Fix, Refusal};
use crate::registry::{Group, Registry};
use glam::Vec2;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use words::{tr, trf};

/// An input the new box would have: a wire coming into the selection.
#[derive(Debug, Clone, PartialEq)]
pub struct ProposedInput {
    /// Its name (the first plug it feeds, to start with).
    pub name: String,
    /// What it takes: what the wire carries.
    pub kind: ValueKind,
    /// The output outside feeding it.
    pub from: PlugRef,
    /// The selected boxes' inputs it feeds.
    pub to: Vec<PlugRef>,
    /// Whether to keep it (left out, those inputs get nothing).
    pub keep: bool,
}

/// An output the new box would have: a wire leaving the selection, or a
/// selected box feeding nothing.
#[derive(Debug, Clone, PartialEq)]
pub struct ProposedOutput {
    /// Its name (the plug it comes from, to start with).
    pub name: String,
    /// What it gives.
    pub kind: ValueKind,
    /// The selected box's output.
    pub from: PlugRef,
    /// The inputs outside it feeds.
    pub to: Vec<PlugRef>,
    /// Whether to keep it (left out, those inputs lose their wire).
    pub keep: bool,
}

/// A setting of a selected box that could become a slider.
#[derive(Debug, Clone, PartialEq)]
pub struct ProposedSlider {
    /// The box.
    pub node: BoxId,
    /// Its setting's key.
    pub setting: String,
    /// The box's name, to tell settings of the same name apart.
    pub box_name: String,
    /// The slider's name.
    pub name: String,
    /// Whether it becomes a slider.
    pub chosen: bool,
}

/// What making the selection into your own box would give, to be changed
/// before it is made: its name and words, its plugs and its sliders.
#[derive(Debug, Clone, PartialEq)]
pub struct Proposal {
    /// The boxes going inside.
    pub boxes: BTreeSet<BoxId>,
    /// Its name.
    pub name: String,
    /// "What does this do?".
    pub about: String,
    /// Its inputs.
    pub inputs: Vec<ProposedInput>,
    /// Its outputs.
    pub outputs: Vec<ProposedOutput>,
    /// The settings that could become sliders.
    pub sliders: Vec<ProposedSlider>,
}

/// How many own boxes go inside each other in kind `kind`: 0 for a
/// standard box, 1 for an own box holding none.
pub(crate) fn depth(registry: &Registry, kind: &str) -> usize {
    fn inner(registry: &Registry, kind: &str, left: usize) -> usize {
        let Some(own) = registry.get(kind).and_then(|k| k.own().cloned()) else {
            return 0;
        };
        if left == 0 {
            return MAX_DEPTH + 1;
        }
        1 + own
            .uses()
            .iter()
            .map(|u| inner(registry, u, left - 1))
            .max()
            .unwrap_or(0)
    }
    inner(registry, kind, MAX_DEPTH + 1)
}

/// Plain names made different by numbering the repeats ("Height",
/// "Height 2").
fn numbered(names: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    names
        .into_iter()
        .map(|n| {
            let count = seen.entry(n.clone()).or_insert(0);
            *count += 1;
            if *count == 1 {
                n
            } else {
                format!("{n} {count}")
            }
        })
        .collect()
}

impl Proposal {
    /// What making `selection` of `graph` into your own box would give:
    /// an input for each output outside feeding the selection, an output
    /// for each selected output feeding a box outside (or, with none, for
    /// each selected box feeding nothing), and each selected box's
    /// settings to choose sliders from (those shown in the painter
    /// chosen). Refused, with why, for nothing selected, Material out or
    /// a From the model box in it, a box outside both fed by it and
    /// feeding it (a loop), no output at all, or too many plugs.
    pub fn of(
        graph: &Graph,
        registry: &Registry,
        selection: &BTreeSet<BoxId>,
    ) -> Result<Self, Refusal> {
        let boxes: BTreeSet<BoxId> = selection
            .iter()
            .copied()
            .filter(|id| graph.get(*id).is_some())
            .collect();
        if boxes.is_empty() {
            return Err(Refusal::new(tr(
                "Select the boxes to make into your own box first: click one, or drag round \
                 several.",
            )));
        }
        for id in &boxes {
            let info = graph.info_of(registry, *id)?;
            if info.one_per_graph || info.outputs.is_empty() {
                return Err(Refusal::new(trf!(
                    "{} can't go inside your own box: it is where the graph's material comes \
                     out. Leave it out of the selection (Shift + click it).",
                    info.name.show()
                ))
                .with_fixes([Fix::LookAt(*id)]));
            }
            if info.group == Group::Model {
                return Err(Refusal::new(trf!(
                    "{} can't go inside your own box yet: it reads the model's own maps in the \
                     painter, and a box of your own works on one repeat of the material. Leave \
                     it outside and plug it into your box instead.",
                    info.name.show()
                ))
                .with_fixes([Fix::LookAt(*id)]));
            }
        }
        if let Some(between) = loop_through(graph, &boxes) {
            let name = graph
                .info_of(registry, between)
                .map_or_else(|_| tr("a box").to_string(), |i| i.name.show());
            return Err(Refusal::new(trf!(
                "{} is fed by the selected boxes and feeds them too, so your own box would feed \
                 itself in a loop. Select {} as well, or leave out the boxes after it.",
                name,
                name
            ))
            .with_fixes([Fix::LookAt(between)]));
        }
        let plug_name = |plug: &PlugRef, output: bool| {
            graph
                .info_of(registry, plug.node)
                .ok()
                .and_then(|info| {
                    if output {
                        info.output(&plug.plug)
                    } else {
                        info.input(&plug.plug)
                    }
                })
                .map(|p| (p.name.show(), p.kind))
        };
        // Wires coming in, by the output outside feeding them.
        let mut coming: BTreeMap<PlugRef, Vec<PlugRef>> = BTreeMap::new();
        // Wires going out, by the selected output.
        let mut going: BTreeMap<PlugRef, Vec<PlugRef>> = BTreeMap::new();
        for w in graph.wires() {
            match (boxes.contains(&w.from.node), boxes.contains(&w.to.node)) {
                (false, true) => coming.entry(w.from.clone()).or_default().push(w.to.clone()),
                (true, false) => going.entry(w.from.clone()).or_default().push(w.to.clone()),
                _ => {}
            }
        }
        let mut inputs = Vec::new();
        for (from, to) in coming {
            let kind = plug_name(&from, true).map(|(_, k)| k);
            let name = to.first().and_then(|t| plug_name(t, false)).map(|(n, _)| n);
            let (Some(kind), Some(name)) = (kind, name) else {
                continue;
            };
            inputs.push(ProposedInput {
                name,
                kind,
                from,
                to,
                keep: true,
            });
        }
        let mut outputs = Vec::new();
        for (from, to) in going {
            if let Some((name, kind)) = plug_name(&from, true) {
                outputs.push(ProposedOutput {
                    name,
                    kind,
                    from,
                    to,
                    keep: true,
                });
            }
        }
        if outputs.is_empty() {
            // Nothing leaves the selection: the boxes at its end give out.
            for id in &boxes {
                let feeds_inside = graph
                    .wires()
                    .iter()
                    .any(|w| w.from.node == *id && boxes.contains(&w.to.node));
                let first = graph
                    .info_of(registry, *id)
                    .ok()
                    .and_then(|i| i.outputs.first());
                if let (false, Some(first)) = (feeds_inside, first) {
                    outputs.push(ProposedOutput {
                        name: first.name.show(),
                        kind: first.kind,
                        from: PlugRef::new(*id, first.key.to_string()),
                        to: Vec::new(),
                        keep: true,
                    });
                }
            }
        }
        if outputs.is_empty() {
            return Err(Refusal::new(tr(
                "Your own box would give nothing out: every selected box only feeds another \
                 selected box. Leave out the last box, or connect a selected box to one after \
                 it.",
            )));
        }
        let names = numbered(inputs.iter().map(|i| i.name.clone()));
        for (input, name) in inputs.iter_mut().zip(names) {
            input.name = name;
        }
        let names = numbered(outputs.iter().map(|o| o.name.clone()));
        for (output, name) in outputs.iter_mut().zip(names) {
            output.name = name;
        }
        let mut sliders = Vec::new();
        for id in &boxes {
            let Some(b) = graph.get(*id) else { continue };
            let Ok(info) = graph.info_of(registry, *id) else {
                continue;
            };
            for spec in &info.settings {
                sliders.push(ProposedSlider {
                    node: *id,
                    setting: spec.key.to_string(),
                    box_name: info.name.show(),
                    name: spec.name.show(),
                    chosen: b.shown.contains(spec.key.as_ref()),
                });
            }
        }
        // Settings of the same name on several boxes say which box.
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for s in &sliders {
            *counts.entry(s.name.clone()).or_insert(0) += 1;
        }
        for s in &mut sliders {
            if counts.get(&s.name).is_some_and(|n| *n > 1) {
                s.name = format!("{} ({})", s.name, s.box_name);
            }
        }
        let proposal = Self {
            boxes,
            name: tr("My box").to_string(),
            about: String::new(),
            inputs,
            outputs,
            sliders,
        };
        proposal.plugs_fit()?;
        Ok(proposal)
    }

    /// Too many plugs or sliders kept, as a refusal.
    fn plugs_fit(&self) -> Result<(), Refusal> {
        let ins = self.inputs.iter().filter(|i| i.keep).count();
        let outs = self.outputs.iter().filter(|o| o.keep).count();
        let sliders = self.sliders.iter().filter(|s| s.chosen).count();
        if ins > MAX_PLUGS || outs > MAX_PLUGS {
            return Err(Refusal::new(trf!(
                "Your own box would have {} inputs and {} outputs; a box can have up to {} \
                 of each. Select fewer boxes, or the boxes feeding them too.",
                ins,
                outs,
                MAX_PLUGS
            )));
        }
        if sliders > MAX_SLIDERS {
            return Err(Refusal::new(trf!(
                "A box can have up to {} sliders; {} are chosen. Untick some.",
                MAX_SLIDERS,
                sliders
            )));
        }
        Ok(())
    }

    /// Makes it: the own box with id `id` and `picture`, and the edits
    /// that replace the selected boxes in `graph` with one box of it,
    /// wired as they were. The edits need a registry knowing the new
    /// box (the scene adds it first). Refused, with why, for a missing
    /// name, no output kept, too many plugs, or own boxes too deep
    /// inside each other.
    pub fn make(
        &self,
        graph: &Graph,
        registry: &Registry,
        id: String,
        picture: Option<Vec<u8>>,
    ) -> Result<(OwnBox, Vec<GraphEdit>), Refusal> {
        let name = self.name.trim();
        if name.is_empty() || name.len() > MAX_NAME {
            return Err(Refusal::new(trf!(
                "Your own box needs a name of up to {} letters.",
                MAX_NAME
            )));
        }
        if !self.outputs.iter().any(|o| o.keep) {
            return Err(Refusal::new(tr(
                "Keep at least one output: a box that gives nothing out does nothing.",
            )));
        }
        self.plugs_fit()?;
        let deepest = self
            .boxes
            .iter()
            .filter_map(|b| graph.get(*b))
            .map(|b| super::making::depth(registry, &b.kind))
            .max()
            .unwrap_or(0);
        if deepest + 1 > MAX_DEPTH {
            return Err(Refusal::new(trf!(
                "Your own boxes would go more than {} deep inside each other. Open one of the \
                 selected boxes back up first (Shift + G).",
                MAX_DEPTH
            )));
        }
        let all = graph.to_parts();
        let inside: BTreeSet<_> = self.boxes.clone();
        let frames = all
            .frames
            .iter()
            .filter(|(f, _)| {
                let held = graph.boxes_in(*f);
                !held.is_empty() && held.iter().all(|b| inside.contains(b))
            })
            .cloned()
            .collect();
        let parts = GraphParts {
            name: name.to_string(),
            seed: 0,
            next_id: all.next_id,
            boxes: all
                .boxes
                .into_iter()
                .filter(|(id, _)| inside.contains(id))
                .map(|(id, mut b)| {
                    b.shown.clear();
                    (id, b)
                })
                .collect(),
            wires: all
                .wires
                .into_iter()
                .filter(|w| inside.contains(&w.from.node) && inside.contains(&w.to.node))
                .collect(),
            frames,
            // Inside, animated boxes run at the outer graph's moment.
            moment: 0.0,
        };
        let inner = Graph::from_parts(parts, registry).map_err(Refusal::new)?;
        let kept_inputs: Vec<&ProposedInput> = self.inputs.iter().filter(|i| i.keep).collect();
        let kept_outputs: Vec<&ProposedOutput> = self.outputs.iter().filter(|o| o.keep).collect();
        let chosen: Vec<&ProposedSlider> = self.sliders.iter().filter(|s| s.chosen).collect();
        let own = OwnBox {
            id: id.clone(),
            name: name.to_string(),
            about: self.about.trim().to_string(),
            picture,
            inner: Arc::new(inner),
            inputs: kept_inputs
                .iter()
                .enumerate()
                .map(|(i, p)| OwnInput {
                    key: format!("in{}", i + 1),
                    name: p.name.clone(),
                    kind: p.kind,
                    to: p.to.clone(),
                })
                .collect(),
            outputs: kept_outputs
                .iter()
                .enumerate()
                .map(|(i, p)| OwnOutput {
                    key: format!("out{}", i + 1),
                    name: p.name.clone(),
                    kind: p.kind,
                    from: Some(p.from.clone()),
                })
                .collect(),
            sliders: chosen
                .iter()
                .enumerate()
                .map(|(i, s)| OwnSlider {
                    key: format!("s{}", i + 1),
                    name: s.name.clone(),
                    node: s.node,
                    setting: s.setting.clone(),
                })
                .collect(),
        };
        own.check(registry)
            .map_err(|why| Refusal::new(trf!("Your own box can't be kept: {}.", why)))?;
        let placed = BoxId(graph.next_id());
        let count = self.boxes.len() as f32;
        let at = self
            .boxes
            .iter()
            .filter_map(|b| graph.get(*b))
            .fold(Vec2::ZERO, |sum, b| sum + b.at)
            / count.max(1.0);
        let mut edits = vec![
            GraphEdit::RemoveBoxes(self.boxes.iter().copied().collect()),
            GraphEdit::AddBox { kind: id, at },
        ];
        for (input, p) in own.inputs.iter().zip(&kept_inputs) {
            edits.push(GraphEdit::Connect {
                from: p.from.clone(),
                to: PlugRef::new(placed, input.key.clone()),
            });
        }
        for (output, p) in own.outputs.iter().zip(&kept_outputs) {
            for to in &p.to {
                edits.push(GraphEdit::Connect {
                    from: PlugRef::new(placed, output.key.clone()),
                    to: to.clone(),
                });
            }
        }
        // A setting shown in the painter stays shown, as its slider.
        for slider in &own.sliders {
            let shown = graph
                .get(slider.node)
                .is_some_and(|b| b.shown.contains(&slider.setting));
            if shown {
                edits.push(GraphEdit::ShowInPainter {
                    node: placed,
                    key: slider.key.clone(),
                    shown: true,
                });
            }
        }
        Ok((own, edits))
    }
}

/// A box outside `boxes` that they feed and that feeds them, if any.
fn loop_through(graph: &Graph, boxes: &BTreeSet<BoxId>) -> Option<BoxId> {
    // Everything after the selection, outside it.
    let mut after = BTreeSet::new();
    let mut todo: Vec<BoxId> = boxes.iter().copied().collect();
    while let Some(id) = todo.pop() {
        for w in graph.wires().iter().filter(|w| w.from.node == id) {
            let next = w.to.node;
            if !boxes.contains(&next) && after.insert(next) {
                todo.push(next);
            }
        }
    }
    after
        .into_iter()
        .find(|x| boxes.iter().any(|b| graph.feeds(*x, *b)))
}

/// What opening a placed own box back up does: the edits, and the boxes
/// it gives (to select).
#[derive(Debug, Clone, PartialEq)]
pub struct OpenedUp {
    /// The edits, in order.
    pub edits: Vec<GraphEdit>,
    /// The new boxes' ids.
    pub boxes: Vec<BoxId>,
}

/// Opening own box `placed` of `graph` back up (Shift + G): copies of the
/// boxes inside (with its sliders' values) and their comment frames
/// where it sat, wired to what it was wired to. Refused, with why, for a
/// box that isn't one of your own.
pub fn open_up(graph: &Graph, registry: &Registry, placed: BoxId) -> Result<OpenedUp, Refusal> {
    let info = graph.info_of(registry, placed)?;
    let b = graph
        .get(placed)
        .ok_or_else(|| Refusal::new(tr("That box isn't in the graph any more.")))?;
    let Some(own) = registry.get(&b.kind).and_then(|k| k.own().cloned()) else {
        return Err(Refusal::new(trf!(
            "{} is one of the app's own boxes, so there is nothing inside it to open up. \
             Shift + G opens up boxes you made with Alt + G.",
            info.name.show()
        )));
    };
    let settings = b.settings.resolved(&info.settings);
    let value_of = |key: &str| {
        let at = info.settings.iter().position(|s| s.key == key)?;
        settings.get(at).copied()
    };
    let inner = own.inner_with(registry, value_of, graph.seed());
    let corner = inner
        .boxes()
        .map(|(_, b)| b.at)
        .fold(Vec2::splat(f32::INFINITY), Vec2::min);
    let offset = if corner.is_finite() {
        b.at - corner
    } else {
        Vec2::ZERO
    };
    let mut edits = vec![GraphEdit::RemoveBoxes(vec![placed])];
    let mut map = BTreeMap::new();
    for (next, (old, inner_box)) in (graph.next_id()..).zip(inner.boxes()) {
        let new = BoxId(next);
        map.insert(old, new);
        edits.push(GraphEdit::AddBox {
            kind: inner_box.kind.clone(),
            at: inner_box.at + offset,
        });
        edits.extend(inner_box.copy_edits(new));
    }
    for (_, frame) in inner.frames() {
        let mut frame = frame.clone();
        frame.min += offset;
        frame.max += offset;
        edits.push(GraphEdit::AddFrame(frame));
    }
    let mapped = |p: &PlugRef| map.get(&p.node).map(|n| PlugRef::new(*n, p.plug.clone()));
    for w in inner.wires() {
        if let (Some(from), Some(to)) = (mapped(&w.from), mapped(&w.to)) {
            edits.push(GraphEdit::Connect { from, to });
        }
    }
    for input in &own.inputs {
        let Some(wire) = graph.wire_into(&PlugRef::new(placed, input.key.clone())) else {
            continue;
        };
        for to in input.to.iter().filter_map(mapped) {
            edits.push(GraphEdit::Connect {
                from: wire.from.clone(),
                to,
            });
        }
    }
    for output in &own.outputs {
        let Some(from) = output.from.as_ref().and_then(mapped) else {
            continue;
        };
        for w in graph
            .wires()
            .iter()
            .filter(|w| w.from.node == placed && w.from.plug == output.key)
        {
            edits.push(GraphEdit::Connect {
                from: from.clone(),
                to: w.to.clone(),
            });
        }
    }
    // A slider shown in the painter shows its setting again.
    for slider in &own.sliders {
        if let (true, Some(node)) = (b.shown.contains(&slider.key), map.get(&slider.node)) {
            edits.push(GraphEdit::ShowInPainter {
                node: *node,
                key: slider.setting.clone(),
                shown: true,
            });
        }
    }
    Ok(OpenedUp {
        edits,
        boxes: map.values().copied().collect(),
    })
}
