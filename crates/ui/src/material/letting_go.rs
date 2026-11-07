//! Letting go on the canvas: the end of a drag (a wire connected,
//! refused or dropped on empty canvas for the box search, a box drawn
//! round boxes, moved boxes tidied), a click (select, look through a
//! picture, a badge's note, a frame's title), Esc during a drag, and
//! showing a refusal (decision 0258).

use super::MaterialEditor;
use super::canvas::{Hit, Places, badge_rect};
use super::drags::{Drag, Press, Target, WireEnd, select, wire_target};
use super::note::{FixContext, Note};
use egui::{Modifiers, Pos2, Rect};
use matgraph::{BoxId, Graph, GraphEdit, PlugRef, Refusal, Wire};
use scene::Document;
use std::collections::BTreeSet;

impl MaterialEditor {
    /// Lets go of a drag at `pos`.
    #[expect(clippy::too_many_arguments, reason = "the canvas's whole frame")]
    pub(super) fn finish_drag(
        &mut self,
        drag: Drag,
        pos: Option<Pos2>,
        rect: Rect,
        graph: &Graph,
        places: &Places,
        problems: &BTreeSet<BoxId>,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        match drag {
            Drag::Move { key, moved, .. } if moved => {
                // The app tidies round what was moved, in the same step.
                let label = crate::text::tr("Move boxes");
                let _ = self.run(document, Vec::new(), label, Some(key), true);
            }
            Drag::Frame { key, moved, .. } if moved => {
                let label = crate::text::tr("Move a comment frame");
                let _ = self.run(document, Vec::new(), label, Some(key), true);
            }
            Drag::Move { .. } | Drag::Frame { .. } | Drag::Resize { .. } => {}
            Drag::Select {
                from,
                to,
                modifiers,
            } => {
                let band = Rect::from_two_pos(from, to);
                let inside: BTreeSet<BoxId> = places
                    .boxes
                    .iter()
                    .filter(|b| b.rect.intersects(band))
                    .map(|b| b.id)
                    .collect();
                select(&mut self.canvas.selected, inside, modifiers);
                self.canvas.frame = None;
            }
            Drag::Wire {
                end,
                detached,
                target,
                pointer,
            } => {
                let at = pos.unwrap_or(pointer);
                let target = target.or_else(|| {
                    wire_target(&end, detached.as_ref(), at, graph, places, self.registry())
                });
                self.drop_wire(
                    end, detached, target, at, rect, places, problems, document, note,
                );
            }
        }
    }

    /// Lets go of a wire at `at`, over `target` if any.
    #[expect(clippy::too_many_arguments, reason = "the canvas's whole frame")]
    fn drop_wire(
        &mut self,
        end: WireEnd,
        detached: Option<Wire>,
        target: Option<Target>,
        at: Pos2,
        rect: Rect,
        places: &Places,
        problems: &BTreeSet<BoxId>,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let disconnect = |this: &mut Self, document: &mut Document, wire: &Wire| {
            let edit = GraphEdit::Disconnect(wire.to.clone());
            let label = edit.describe();
            let _ = this.run(document, vec![edit], &label, None, false);
        };
        match target {
            Some(Target {
                from,
                to,
                fits: Ok(_),
            }) => {
                if detached
                    .as_ref()
                    .is_some_and(|w| w.from == from && w.to == to)
                {
                    return;
                }
                let mut edits = Vec::new();
                if let Some(wire) = &detached
                    && wire.to != to
                {
                    edits.push(GraphEdit::Disconnect(wire.to.clone()));
                }
                edits.push(GraphEdit::Connect {
                    from: from.clone(),
                    to: to.clone(),
                });
                let label = crate::text::tr("Connect boxes");
                if let Err(why) = self.run(document, edits, label, None, true) {
                    let context = FixContext::wire(from, to);
                    self.show_refusal(why, at, rect, context, note);
                }
            }
            Some(Target {
                from,
                to,
                fits: Err(why),
            }) => {
                // Letting go cancels; the note stays with its fixes, and
                // the status bar repeats why.
                let context = FixContext::wire(from, to);
                self.show_refusal(why, at, rect, context, note);
            }
            None => {
                let over_box = !matches!(places.hit(at, problems), Hit::Empty);
                match (&detached, over_box) {
                    (Some(wire), _) => disconnect(self, document, wire),
                    (None, false) => {
                        let on_canvas = self.canvas.view.on_canvas(rect, at);
                        self.open_search(on_canvas, Some(end), document);
                    }
                    (None, true) => {}
                }
            }
        }
    }

    /// A press let go without moving.
    pub(super) fn click(
        &mut self,
        press: &Press,
        double: bool,
        rect: Rect,
        graph: &Graph,
        document: &Document,
    ) {
        match &press.hit {
            Hit::Picture(id) => {
                select(&mut self.canvas.selected, [*id].into(), press.modifiers);
                self.canvas.frame = None;
                // Look through the box in the preview; again to go back.
                self.canvas.looking_at = if self.canvas.looking_at == Some(*id) {
                    None
                } else {
                    Some(*id)
                };
            }
            Hit::Box(id) | Hit::Plug(PlugRef { node: id, .. }, _, _) => {
                select(&mut self.canvas.selected, [*id].into(), press.modifiers);
                self.canvas.frame = None;
                // Double-click your own box: see inside it.
                if double && matches!(press.hit, Hit::Box(_)) {
                    let mut status = None;
                    self.open_box(document, *id, &mut status);
                }
            }
            Hit::Badge(id) => {
                select(&mut self.canvas.selected, [*id].into(), Modifiers::NONE);
                if let Some(why) = self.problems(graph).remove(id) {
                    let context = FixContext::problem(graph, self.registry(), *id);
                    let at = Places::of(graph, self.registry(), &self.canvas.view, rect)
                        .find(*id)
                        .map_or(press.at, |b| badge_rect(b.rect).right_bottom());
                    let mut status = None;
                    self.show_refusal(why, at, rect, context, &mut status);
                }
            }
            Hit::FrameTitle(id) | Hit::FrameEdge(id, _) => {
                self.canvas.frame = Some(*id);
                if !press.modifiers.command && !press.modifiers.shift {
                    self.canvas.selected.clear();
                }
                if double
                    && matches!(press.hit, Hit::FrameTitle(_))
                    && let Some(f) = graph.frame(*id)
                {
                    self.canvas.renaming = Some((*id, f.title.clone(), true));
                }
            }
            Hit::Empty => {
                if !press.modifiers.command && !press.modifiers.shift {
                    self.canvas.selected.clear();
                    self.canvas.frame = None;
                }
            }
        }
        let _ = document;
    }

    /// Esc during a drag: what it changed is taken back.
    pub(crate) fn cancel_drag(&mut self, document: &mut Document) {
        self.canvas.press = None;
        match self.canvas.drag.take() {
            Some(
                Drag::Move {
                    key, moved: true, ..
                }
                | Drag::Frame {
                    key, moved: true, ..
                },
            ) => {
                document.abandon(key);
            }
            Some(Drag::Resize { key, .. }) => {
                document.abandon(key);
            }
            _ => {}
        }
    }

    /// Shows `why` in a note at the screen point `at`, with its fixes, and
    /// repeats it in the status bar.
    pub(crate) fn show_refusal(
        &mut self,
        why: Refusal,
        at: Pos2,
        rect: Rect,
        context: FixContext,
        note: &mut Option<String>,
    ) {
        *note = Some(why.reason.clone());
        self.canvas.note = Some(Note {
            refusal: why,
            at: self.canvas.view.on_canvas(rect, at),
            context,
        });
    }
}
