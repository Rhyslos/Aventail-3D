//! Carrying out changes on the canvas, each one undo step: edits checked
//! first and tidied after (`tidy`), the fixes a note offers, new boxes
//! from the search, comment frames, removing, copying and pasting, and
//! new graphs.

use super::MaterialEditor;
use super::drags::WireEnd;
use super::look::{self, COLUMN, FRAME_MARGIN, FRAME_TITLE};
use super::note::FixContext;
use super::search::Search;
use super::tidy;
use crate::text::tr;
use egui::Rect;
use glam::Vec2;
use matgraph::{
    BoxId, BoxInfo, Fit, Fix, Frame, FrameId, Graph, GraphBox, GraphEdit, PlugRef, Refusal,
    Registry, ValueKind, Wire,
};
use scene::{Command, Document, SceneError};
use std::collections::BTreeSet;

/// Boxes copied, to paste.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Copied {
    /// The boxes, as they were.
    boxes: Vec<(BoxId, GraphBox)>,
    /// The wires between them.
    wires: Vec<Wire>,
}

/// The key of a plug of `info` taking (`input`) or giving a `kind`: one
/// that fits as it is first, else one that converts.
fn plug_for(info: &BoxInfo, kind: ValueKind, input: bool) -> Option<String> {
    let plugs = if input { &info.inputs } else { &info.outputs };
    let fit = |p: &matgraph::PlugSpec| {
        if input {
            matgraph::kind::fit(kind, p.kind)
        } else {
            matgraph::kind::fit(p.kind, kind)
        }
    };
    plugs
        .iter()
        .find(|p| fit(p) == Ok(Fit::Fits))
        .or_else(|| plugs.iter().find(|p| fit(p).is_ok()))
        .map(|p| p.key.to_string())
}

/// What the output `plug` of `graph` carries.
fn output_kind(graph: &Graph, registry: &Registry, plug: &PlugRef) -> Option<ValueKind> {
    let b = graph.get(plug.node)?;
    Some(registry.get(&b.kind)?.info().output(&plug.plug)?.kind)
}

/// What the input `plug` of `graph` takes.
fn input_kind(graph: &Graph, registry: &Registry, plug: &PlugRef) -> Option<ValueKind> {
    let b = graph.get(plug.node)?;
    Some(registry.get(&b.kind)?.info().input(&plug.plug)?.kind)
}

impl MaterialEditor {
    /// Makes `edits` to the graph shown as one undo step called `label`
    /// (merged into the last under `key`), tidied after when `tidy`; or
    /// why they can't be made (nothing changes then).
    pub(crate) fn run(
        &mut self,
        document: &mut Document,
        edits: Vec<GraphEdit>,
        label: &str,
        key: Option<u64>,
        tidy: bool,
    ) -> Result<(), Refusal> {
        let no_graph = || Refusal::new(tr("Make a material graph first."));
        let id = self.graph.ok_or_else(no_graph)?;
        let graph = self.current(document).ok_or_else(no_graph)?;
        let registry = Registry::standard();
        let mut after = (**graph).clone();
        for edit in &edits {
            after.apply(registry, edit)?;
        }
        let mut all = edits;
        if tidy {
            all.extend(tidy::tidy(&after, registry));
        }
        if all.is_empty() {
            return Ok(());
        }
        let key = key.unwrap_or_else(|| self.next_key());
        let commands = all.into_iter().map(|edit| Command::EditGraph { id, edit });
        document
            .execute_all_labelled(commands, label, key)
            .map_err(|e| match e {
                SceneError::Graph(why) => why,
                other => Refusal::new(other.to_string()),
            })
    }

    /// Adds a box of kind `kind` where the search was opened, connected
    /// to the wire that opened it, if any.
    pub(crate) fn add_from_search(
        &mut self,
        document: &mut Document,
        kind: &str,
        search: &Search,
        rect: Rect,
        note: &mut Option<String>,
    ) {
        let Some(graph) = self.current(document).cloned() else {
            return;
        };
        let registry = Registry::standard();
        let new = BoxId(graph.next_id());
        let at = search.at - Vec2::new(look::BOX_WIDTH / 2.0, look::HEADER / 2.0);
        let mut edits = vec![GraphEdit::AddBox {
            kind: kind.to_string(),
            at,
        }];
        let info = registry.get(kind).map(|t| t.info());
        let mut context = FixContext::default();
        match (&search.wire, info) {
            (Some(WireEnd::Output(from, carries)), Some(info)) => {
                if let Some(key) = plug_for(info, *carries, true) {
                    let to = PlugRef::new(new, key);
                    context = FixContext::wire(from.clone(), to.clone());
                    edits.push(GraphEdit::Connect {
                        from: from.clone(),
                        to,
                    });
                }
            }
            (Some(WireEnd::Input(to, takes)), Some(info)) => {
                if let Some(key) = plug_for(info, *takes, false) {
                    let from = PlugRef::new(new, key);
                    context = FixContext::wire(from.clone(), to.clone());
                    edits.push(GraphEdit::Connect {
                        from,
                        to: to.clone(),
                    });
                }
            }
            _ => {}
        }
        let label = tr("Add a box");
        match self.run(document, edits, label, None, true) {
            Ok(()) => {
                self.canvas.selected = [new].into();
                self.canvas.frame = None;
            }
            Err(why) => {
                let screen = self.canvas.view.on_screen(rect, search.at);
                self.show_refusal(why, screen, rect, context, note);
            }
        }
    }

    /// Adds a box of kind `kind` after box `after` (one of its "usually
    /// comes next"), fed by its first output.
    pub(crate) fn add_next(
        &mut self,
        document: &mut Document,
        after: BoxId,
        kind: &str,
        note: &mut Option<String>,
    ) {
        let Some(graph) = self.current(document).cloned() else {
            return;
        };
        let registry = Registry::standard();
        let Some(b) = graph.get(after) else { return };
        let new = BoxId(graph.next_id());
        let mut edits = vec![GraphEdit::AddBox {
            kind: kind.to_string(),
            at: b.at + Vec2::new(COLUMN, 0.0),
        }];
        let from = registry
            .get(&b.kind)
            .and_then(|t| t.info().outputs.first())
            .map(|o| (PlugRef::new(after, o.key.to_string()), o.kind));
        if let (Some((from, carries)), Some(info)) = (from, registry.get(kind).map(|t| t.info()))
            && let Some(key) = plug_for(info, carries, true)
        {
            edits.push(GraphEdit::Connect {
                from,
                to: PlugRef::new(new, key),
            });
        }
        match self.run(document, edits, tr("Add a box"), None, true) {
            Ok(()) => self.canvas.selected = [new].into(),
            Err(why) => *note = Some(why.reason),
        }
    }

    /// Carries out a fix a note offered.
    pub(crate) fn carry_out_fix(
        &mut self,
        document: &mut Document,
        fix: &Fix,
        context: &FixContext,
        rect: Rect,
        note: &mut Option<String>,
    ) {
        let Some(graph) = self.current(document).cloned() else {
            return;
        };
        let registry = Registry::standard();
        let new = BoxId(graph.next_id());
        let place_of =
            |plug: &Option<PlugRef>| plug.as_ref().and_then(|p| graph.get(p.node)).map(|b| b.at);
        let edits = match fix {
            Fix::InsertBox { kind } => {
                let (Some(from), Some(to)) = (&context.from, &context.to) else {
                    return;
                };
                let info = registry.get(kind).map(|t| t.info());
                let carries = output_kind(&graph, registry, from);
                let takes = input_kind(&graph, registry, to);
                let (Some(info), Some(carries), Some(takes)) = (info, carries, takes) else {
                    return;
                };
                let at = match (place_of(&context.from), place_of(&context.to)) {
                    (Some(a), Some(b)) => (a + b) / 2.0,
                    (Some(a), None) | (None, Some(a)) => a,
                    (None, None) => Vec2::ZERO,
                };
                let mut edits = vec![GraphEdit::AddBox {
                    kind: kind.clone(),
                    at,
                }];
                if let Some(key) = plug_for(info, carries, true) {
                    edits.push(GraphEdit::Connect {
                        from: from.clone(),
                        to: PlugRef::new(new, key),
                    });
                }
                if let Some(key) = plug_for(info, takes, false) {
                    edits.push(GraphEdit::Connect {
                        from: PlugRef::new(new, key),
                        to: to.clone(),
                    });
                }
                edits
            }
            Fix::AddBox { kind } => {
                let at = place_of(&context.to).map_or_else(
                    || self.canvas.view.on_canvas(rect, rect.center()),
                    |a| a - Vec2::new(COLUMN, 0.0),
                );
                let mut edits = vec![GraphEdit::AddBox {
                    kind: kind.clone(),
                    at,
                }];
                if let Some(to) = &context.to
                    && let (Some(info), Some(takes)) = (
                        registry.get(kind).map(|t| t.info()),
                        input_kind(&graph, registry, to),
                    )
                    && let Some(key) = plug_for(info, takes, false)
                {
                    edits.push(GraphEdit::Connect {
                        from: PlugRef::new(new, key),
                        to: to.clone(),
                    });
                }
                edits
            }
            Fix::ConnectInstead { from } => match &context.to {
                Some(to) => vec![GraphEdit::Connect {
                    from: from.clone(),
                    to: to.clone(),
                }],
                None => return,
            },
            Fix::RemoveBox(id) => vec![GraphEdit::RemoveBoxes(vec![*id])],
            Fix::LookAt(id) => {
                self.canvas.selected = [*id].into();
                self.frame_view(&graph, rect, true);
                return;
            }
            Fix::UseSize(_) | Fix::UseNearest => return,
        };
        let label = fix.describe(registry, &graph);
        if let Err(why) = self.run(document, edits, &label, None, true) {
            let at = rect.center();
            self.show_refusal(why, at, rect, context.clone(), note);
        } else {
            *note = Some(crate::trf!("Done: {}.", label));
        }
    }

    /// A comment frame round the selected boxes, or an empty one at `at`
    /// (C); its title is ready to type.
    pub(crate) fn comment_frame(
        &mut self,
        document: &mut Document,
        at: Vec2,
        note: &mut Option<String>,
    ) {
        let Some(graph) = self.current(document).cloned() else {
            return;
        };
        let registry = Registry::standard();
        let mut lo = Vec2::splat(f32::INFINITY);
        let mut hi = Vec2::splat(f32::NEG_INFINITY);
        for id in &self.canvas.selected {
            if let Some((a, b)) = look::box_rect(&graph, registry, *id) {
                lo = lo.min(a);
                hi = hi.max(b);
            }
        }
        let (min, max) = if lo.is_finite() {
            (
                lo - Vec2::new(FRAME_MARGIN, FRAME_MARGIN + FRAME_TITLE),
                hi + Vec2::splat(FRAME_MARGIN),
            )
        } else {
            (at, at + Vec2::new(320.0, 200.0))
        };
        let title = tr("Comment").to_string();
        let id = FrameId(graph.next_id());
        let edit = GraphEdit::AddFrame(Frame::new(title.clone(), min, max));
        let label = edit.describe();
        match self.run(document, vec![edit], &label, None, false) {
            Ok(()) => {
                self.canvas.frame = Some(id);
                self.canvas.renaming = Some((id, title, true));
            }
            Err(why) => *note = Some(why.reason),
        }
    }

    /// Removes the selected boxes (Delete), or keeps the wire through each
    /// (Backspace); with no box selected, the selected frame.
    pub(crate) fn remove_selected(
        &mut self,
        document: &mut Document,
        dissolve: bool,
        note: &mut Option<String>,
    ) {
        let ids: Vec<BoxId> = self.canvas.selected.iter().copied().collect();
        let edits = if ids.is_empty() {
            match self.canvas.frame {
                Some(frame) => vec![GraphEdit::RemoveFrame(frame)],
                None => {
                    *note =
                        Some(tr("Select boxes first: click one, or drag round several.").into());
                    return;
                }
            }
        } else if dissolve {
            ids.iter().map(|id| GraphEdit::DissolveBox(*id)).collect()
        } else {
            vec![GraphEdit::RemoveBoxes(ids.clone())]
        };
        let label = match edits.as_slice() {
            [one] => one.describe(),
            _ => crate::trf!("Remove {} boxes, keeping their wires", ids.len()),
        };
        match self.run(document, edits, &label, None, false) {
            Ok(()) => {
                if ids.is_empty() {
                    self.canvas.frame = None;
                }
                if self.canvas.looking_at.is_some_and(|l| ids.contains(&l)) {
                    self.canvas.looking_at = None;
                }
                self.canvas.selected.clear();
            }
            Err(why) => *note = Some(why.reason),
        }
    }

    /// Copies the selected boxes and the wires between them (Ctrl + C;
    /// Alt + C for a linked copy, which pastes plain for now).
    pub(crate) fn copy(&mut self, document: &Document, linked: bool, note: &mut Option<String>) {
        let Some(graph) = self.current(document) else {
            return;
        };
        let ids: &BTreeSet<BoxId> = &self.canvas.selected;
        let boxes: Vec<(BoxId, GraphBox)> = ids
            .iter()
            .filter_map(|id| graph.get(*id).map(|b| (*id, b.clone())))
            .collect();
        if boxes.is_empty() {
            *note = Some(tr("Select boxes first: click one, or drag round several.").into());
            return;
        }
        let wires = graph
            .wires()
            .iter()
            .filter(|w| ids.contains(&w.from.node) && ids.contains(&w.to.node))
            .cloned()
            .collect();
        *note = Some(if linked {
            crate::trf!(
                "Copied {} boxes. Linked copies come with your own boxes; until then Alt + V \
                 pastes plain copies.",
                boxes.len()
            )
        } else {
            crate::trf!("Copied {} boxes.", boxes.len())
        });
        self.clipboard = Some(Copied { boxes, wires });
    }

    /// Pastes the copied boxes with their top left at `at` (Ctrl + V; Alt
    /// + V pastes plain copies until linked boxes come).
    pub(crate) fn paste(
        &mut self,
        document: &mut Document,
        at: Vec2,
        linked: bool,
        note: &mut Option<String>,
    ) {
        let (Some(copied), Some(graph)) = (self.clipboard.clone(), self.current(document).cloned())
        else {
            *note = Some(tr("Nothing copied yet: select boxes and press Ctrl + C.").into());
            return;
        };
        let corner = copied
            .boxes
            .iter()
            .map(|(_, b)| b.at)
            .fold(Vec2::splat(f32::INFINITY), Vec2::min);
        let mut new_ids = std::collections::BTreeMap::new();
        let mut edits = Vec::new();
        for (next, (old, b)) in (graph.next_id()..).zip(&copied.boxes) {
            new_ids.insert(*old, BoxId(next));
            edits.push(GraphEdit::AddBox {
                kind: b.kind.clone(),
                at: at + (b.at - corner),
            });
            for (key, value) in b.settings.iter() {
                edits.push(GraphEdit::SetSetting {
                    node: BoxId(next),
                    key: key.to_string(),
                    value,
                });
            }
        }
        for wire in &copied.wires {
            if let (Some(from), Some(to)) =
                (new_ids.get(&wire.from.node), new_ids.get(&wire.to.node))
            {
                edits.push(GraphEdit::Connect {
                    from: PlugRef::new(*from, wire.from.plug.clone()),
                    to: PlugRef::new(*to, wire.to.plug.clone()),
                });
            }
        }
        match self.run(document, edits, tr("Paste boxes"), None, true) {
            Ok(()) => {
                self.canvas.selected = new_ids.values().copied().collect();
                *note = Some(if linked {
                    tr("Pasted plain copies: linked copies come with your own boxes.").to_string()
                } else {
                    crate::trf!("Pasted {} boxes.", new_ids.len())
                });
            }
            Err(why) => *note = Some(why.reason),
        }
    }

    /// Makes a new graph holding a Material out box, and shows it.
    pub(crate) fn new_graph(&mut self, document: &mut Document, note: &mut Option<String>) {
        let store = document.scene().graphs();
        let names: BTreeSet<&str> = store.iter().map(|(_, g)| g.name()).collect();
        let name = (1..)
            .map(|n| crate::trf!("Material {}", n))
            .find(|n| !names.contains(n.as_str()))
            .unwrap_or_default();
        let id = store.next_id();
        let mut graph = Graph::new(name);
        let out = GraphEdit::AddBox {
            kind: matgraph::eval::MATERIAL_OUT.to_string(),
            at: Vec2::new(3.0 * COLUMN, 0.0),
        };
        if let Err(why) = graph.apply(Registry::standard(), &out) {
            *note = Some(why.reason);
        }
        match document.execute(Command::AddGraph(graph)) {
            Ok(()) => self.show_graph(Some(id)),
            Err(why) => *note = Some(why.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugs_that_fit_as_they_are_come_first() {
        let registry = Registry::standard();
        let blend = registry.get("blend").unwrap().info();
        assert_eq!(
            plug_for(blend, ValueKind::Shape, true).as_deref(),
            Some("where")
        );
        assert_eq!(
            plug_for(blend, ValueKind::Height, true).as_deref(),
            Some("bottom")
        );
        assert_eq!(plug_for(blend, ValueKind::Colour, true), None);
        let out = registry.get("material_from_height").unwrap().info();
        assert_eq!(
            plug_for(out, ValueKind::Colour, false).as_deref(),
            Some("material")
        );
    }
}
