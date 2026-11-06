//! What the pointer does on the canvas (the project lead's controls,
//! decision 0256): click to select (Ctrl adds, Shift takes out), drag a
//! box around several, drag boxes to move them, drag from a plug's dot to
//! another to connect, drag a wire's end away to disconnect, drag a
//! frame's title to move it with everything inside, drag its edges to
//! size it, double-click its title to rename it, click a box's picture to
//! look through it, right-click empty canvas for the box search.

use super::MaterialEditor;
use super::canvas::{Edges, Hit, Places};
use egui::{Modifiers, PointerButton, Pos2, Rect, Ui};
use glam::Vec2;
use matgraph::{
    BoxId, Fit, FrameId, Graph, GraphEdit, PlugRef, Refusal, Registry, ValueKind, Wire,
};
use scene::Document;
use std::collections::{BTreeMap, BTreeSet};

/// How far (on screen) a press must move to be a drag.
const DRAG_START: f32 = 4.0;
/// The smallest a frame can be sized to.
const SMALLEST_FRAME: Vec2 = Vec2::new(80.0, 60.0);

/// A press not yet a click or a drag.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Press {
    pub(crate) at: Pos2,
    pub(crate) hit: Hit,
    pub(crate) modifiers: Modifiers,
}

/// The fixed end of a wire being dragged: the output it comes from, or
/// the input it goes into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WireEnd {
    Output(PlugRef, ValueKind),
    Input(PlugRef, ValueKind),
}

impl WireEnd {
    /// The plug and what it carries.
    pub(crate) fn plug(&self) -> (&PlugRef, ValueKind, bool) {
        match self {
            Self::Output(p, k) => (p, *k, true),
            Self::Input(p, k) => (p, *k, false),
        }
    }

    /// The wire from this end to the plug `other`, as (output, input),
    /// when `other` is the other way round.
    fn pair(&self, other: &PlugRef, other_is_output: bool) -> Option<(PlugRef, PlugRef)> {
        match self {
            Self::Output(from, _) if !other_is_output => Some((from.clone(), other.clone())),
            Self::Input(to, _) if other_is_output => Some((other.clone(), to.clone())),
            _ => None,
        }
    }
}

/// A plug the dragged wire is over, and whether it fits.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Target {
    pub(crate) from: PlugRef,
    pub(crate) to: PlugRef,
    pub(crate) fits: Result<Fit, Refusal>,
}

/// What is being dragged.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Drag {
    /// Boxes, moved by the pointer.
    Move {
        ids: Vec<BoxId>,
        last: Vec2,
        key: u64,
        moved: bool,
    },
    /// A wire, from its fixed end to the pointer; `detached` is the wire
    /// picked up by its input end.
    Wire {
        end: WireEnd,
        detached: Option<Wire>,
        target: Option<Target>,
        pointer: Pos2,
    },
    /// A box drawn round boxes to select them.
    Select {
        from: Pos2,
        to: Pos2,
        modifiers: Modifiers,
    },
    /// A frame by its title, with the boxes and frames inside.
    Frame {
        id: FrameId,
        boxes: Vec<BoxId>,
        frames: Vec<FrameId>,
        last: Vec2,
        key: u64,
        moved: bool,
    },
    /// A frame's edges.
    Resize { id: FrameId, edges: Edges, key: u64 },
}

impl MaterialEditor {
    /// The boxes in `graph` that couldn't run last time, with why.
    pub(crate) fn problems(&self, graph: &Graph) -> BTreeMap<BoxId, Refusal> {
        self.pictures.shown().map_or_else(BTreeMap::new, |done| {
            done.evaluation
                .problems()
                .filter(|(id, _)| graph.get(*id).is_some())
                .map(|(id, why)| (id, why.clone()))
                .collect()
        })
    }

    /// Presses, drags and clicks on the canvas this frame.
    #[expect(clippy::too_many_arguments, reason = "the canvas's whole frame")]
    pub(super) fn pointer(
        &mut self,
        ui: &Ui,
        rect: Rect,
        response: &egui::Response,
        graph: &Graph,
        places: &Places,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let (pos, pressed, down, released, modifiers, double) = ui.input(|i| {
            (
                i.pointer.interact_pos(),
                i.pointer.primary_pressed(),
                i.pointer.primary_down(),
                i.pointer.primary_released(),
                i.modifiers,
                i.pointer.button_double_clicked(PointerButton::Primary),
            )
        });
        let problems: BTreeSet<BoxId> = self.problems(graph).into_keys().collect();
        let overlays = self.canvas.overlays.clone();
        let free = |p: Pos2| {
            rect.contains(p)
                && !overlays.iter().any(|r| r.contains(p))
                && !egui::Popup::is_any_open(ui.ctx())
        };
        if pressed
            && let Some(at) = pos
            && free(at)
            && self.canvas.drag.is_none()
        {
            self.canvas.press = Some(Press {
                at,
                hit: places.hit(at, &problems),
                modifiers,
            });
            self.canvas.search = None;
            self.canvas.note = None;
        }
        if down
            && self.canvas.drag.is_none()
            && let (Some(press), Some(at)) = (self.canvas.press.clone(), pos)
            && press.at.distance(at) > DRAG_START
        {
            self.start_drag(&press, rect, graph, places, document);
        }
        if let Some(at) = pos
            && self.canvas.drag.is_some()
        {
            self.carry_on_drag(at, rect, graph, places, document, note);
        }
        if released {
            if let Some(drag) = self.canvas.drag.take() {
                self.finish_drag(drag, pos, rect, graph, places, &problems, document, note);
            } else if let Some(press) = self.canvas.press.take() {
                self.click(&press, double, rect, graph, document);
            }
            self.canvas.press = None;
        }
        if response.secondary_clicked()
            && let Some(at) = response.interact_pointer_pos()
            && free(at)
            && places.hit(at, &problems) == Hit::Empty
        {
            let on_canvas = self.canvas.view.on_canvas(rect, at);
            self.open_search(on_canvas, None, document);
        }
        if let Some(at) = response.hover_pos() {
            ui.ctx()
                .set_cursor_icon(cursor(&places.hit(at, &problems), &self.canvas.drag));
        }
    }

    /// Starts dragging what was pressed.
    fn start_drag(
        &mut self,
        press: &Press,
        rect: Rect,
        graph: &Graph,
        places: &Places,
        document: &Document,
    ) {
        let at = self.canvas.view.on_canvas(rect, press.at);
        let drag = match &press.hit {
            Hit::Plug(plug, true, kind) => Drag::Wire {
                end: WireEnd::Output(plug.clone(), *kind),
                detached: None,
                target: None,
                pointer: press.at,
            },
            Hit::Plug(plug, false, kind) => match graph.wire_into(plug) {
                // Picked up by its input end: the wire comes away.
                Some(wire) => {
                    let from_kind = places.plug(&wire.from, true).map_or(*kind, |p| p.kind);
                    Drag::Wire {
                        end: WireEnd::Output(wire.from.clone(), from_kind),
                        detached: Some(wire.clone()),
                        target: None,
                        pointer: press.at,
                    }
                }
                None => Drag::Wire {
                    end: WireEnd::Input(plug.clone(), *kind),
                    detached: None,
                    target: None,
                    pointer: press.at,
                },
            },
            Hit::Picture(id) | Hit::Box(id) | Hit::Badge(id) => {
                if !self.canvas.selected.contains(id) {
                    if !press.modifiers.command {
                        self.canvas.selected.clear();
                    }
                    self.canvas.selected.insert(*id);
                }
                self.canvas.frame = None;
                Drag::Move {
                    ids: self.canvas.selected.iter().copied().collect(),
                    last: at,
                    key: self.next_key(),
                    moved: false,
                }
            }
            Hit::FrameTitle(id) => {
                let Some(frame) = graph.frame(*id) else {
                    return;
                };
                self.canvas.frame = Some(*id);
                let frames = graph
                    .frames()
                    .filter(|(other, f)| {
                        other != id && frame.contains(f.min) && frame.contains(f.max)
                    })
                    .map(|(other, _)| other)
                    .collect();
                Drag::Frame {
                    id: *id,
                    boxes: graph.boxes_in(*id),
                    frames,
                    last: at,
                    key: self.next_key(),
                    moved: false,
                }
            }
            Hit::FrameEdge(id, edges) => {
                self.canvas.frame = Some(*id);
                Drag::Resize {
                    id: *id,
                    edges: *edges,
                    key: self.next_key(),
                }
            }
            Hit::Empty => Drag::Select {
                from: press.at,
                to: press.at,
                modifiers: press.modifiers,
            },
        };
        let _ = document;
        self.canvas.drag = Some(drag);
    }

    /// Carries a drag on to the pointer at `at`.
    fn carry_on_drag(
        &mut self,
        at: Pos2,
        rect: Rect,
        graph: &Graph,
        places: &Places,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let here = self.canvas.view.on_canvas(rect, at);
        let Some(mut drag) = self.canvas.drag.take() else {
            return;
        };
        match &mut drag {
            Drag::Move {
                ids,
                last,
                key,
                moved,
            } => {
                let by = here - *last;
                if by.length_squared() > 0.0 && !ids.is_empty() {
                    let edit = GraphEdit::MoveBoxes {
                        ids: ids.clone(),
                        by,
                    };
                    let label = edit.describe();
                    if self
                        .run(document, vec![edit], &label, Some(*key), false)
                        .is_ok()
                    {
                        *last = here;
                        *moved = true;
                    }
                }
            }
            Drag::Wire {
                end,
                detached,
                target,
                pointer,
            } => {
                *pointer = at;
                *target = wire_target(end, detached.as_ref(), at, graph, places);
                if let Some(Target { fits: Err(why), .. }) = target {
                    // The status bar repeats why it won't fit.
                    *note = Some(why.reason.clone());
                }
            }
            Drag::Select { to, .. } => *to = at,
            Drag::Frame {
                id,
                boxes,
                frames,
                last,
                key,
                moved,
            } => {
                let by = here - *last;
                if by.length_squared() > 0.0 {
                    let mut edits = Vec::new();
                    if !boxes.is_empty() {
                        edits.push(GraphEdit::MoveBoxes {
                            ids: boxes.clone(),
                            by,
                        });
                    }
                    for frame_id in std::iter::once(*id).chain(frames.iter().copied()) {
                        if let Some(f) = graph.frame(frame_id) {
                            let mut f = f.clone();
                            f.min += by;
                            f.max += by;
                            edits.push(GraphEdit::ChangeFrame {
                                id: frame_id,
                                frame: f,
                            });
                        }
                    }
                    let label = crate::text::tr("Move a comment frame");
                    if self.run(document, edits, label, Some(*key), false).is_ok() {
                        *last = here;
                        *moved = true;
                    }
                }
            }
            Drag::Resize { id, edges, key } => {
                if let Some(f) = graph.frame(*id) {
                    let mut f = f.clone();
                    if edges.left {
                        f.min.x = here.x.min(f.max.x - SMALLEST_FRAME.x);
                    }
                    if edges.right {
                        f.max.x = here.x.max(f.min.x + SMALLEST_FRAME.x);
                    }
                    if edges.bottom {
                        f.max.y = here.y.max(f.min.y + SMALLEST_FRAME.y);
                    }
                    let edit = GraphEdit::ChangeFrame { id: *id, frame: f };
                    let label = crate::text::tr("Size a comment frame");
                    let _ = self.run(document, vec![edit], label, Some(*key), false);
                }
            }
        }
        self.canvas.drag = Some(drag);
    }
}

/// The plug the wire is over at `at`, and whether it fits.
pub(super) fn wire_target(
    end: &WireEnd,
    detached: Option<&Wire>,
    at: Pos2,
    graph: &Graph,
    places: &Places,
) -> Option<Target> {
    let Hit::Plug(plug, output, _) = places.hit(at, &BTreeSet::new()) else {
        return None;
    };
    let (from, to) = end.pair(&plug, output)?;
    if detached.is_some_and(|w| w.from == from && w.to == to) {
        return Some(Target {
            from,
            to,
            fits: Ok(Fit::Fits),
        });
    }
    let fits = graph.check_connection(Registry::standard(), &from, &to);
    Some(Target { from, to, fits })
}

/// Changes `selected` by the app's rule (decision 0133): a plain click
/// or box picks exactly `these`, Ctrl adds them, Shift takes them out.
pub(crate) fn select(selected: &mut BTreeSet<BoxId>, these: BTreeSet<BoxId>, modifiers: Modifiers) {
    if modifiers.command {
        selected.extend(these);
    } else if modifiers.shift {
        selected.retain(|id| !these.contains(id));
    } else {
        *selected = these;
    }
}

/// The pointer's look over what it is over.
fn cursor(hit: &Hit, drag: &Option<Drag>) -> egui::CursorIcon {
    use egui::CursorIcon;
    match (drag, hit) {
        (Some(Drag::Move { .. } | Drag::Frame { .. }), _) => CursorIcon::Grabbing,
        (Some(_), _) => CursorIcon::Default,
        (None, Hit::Plug(..)) => CursorIcon::Crosshair,
        (None, Hit::Picture(_) | Hit::Badge(_)) => CursorIcon::PointingHand,
        (None, Hit::Box(_) | Hit::FrameTitle(_)) => CursorIcon::Grab,
        (None, Hit::FrameEdge(_, e)) if e.bottom && (e.left || e.right) => {
            if e.left {
                CursorIcon::ResizeSouthWest
            } else {
                CursorIcon::ResizeSouthEast
            }
        }
        (None, Hit::FrameEdge(_, e)) if e.bottom => CursorIcon::ResizeVertical,
        (None, Hit::FrameEdge(..)) => CursorIcon::ResizeHorizontal,
        (None, Hit::Empty) => CursorIcon::Default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selecting_follows_the_apps_rule() {
        let mut selected: BTreeSet<BoxId> = [BoxId(1), BoxId(2)].into();
        select(&mut selected, [BoxId(3)].into(), Modifiers::COMMAND);
        assert_eq!(selected.len(), 3);
        select(&mut selected, [BoxId(1)].into(), Modifiers::SHIFT);
        assert!(!selected.contains(&BoxId(1)));
        select(&mut selected, [BoxId(9)].into(), Modifiers::NONE);
        assert_eq!(selected, [BoxId(9)].into());
    }

    #[test]
    fn a_wire_pairs_only_with_the_other_kind_of_plug() {
        let a = PlugRef::new(BoxId(1), "out");
        let b = PlugRef::new(BoxId(2), "in");
        let from_output = WireEnd::Output(a.clone(), ValueKind::Height);
        assert_eq!(from_output.pair(&b, false), Some((a.clone(), b.clone())));
        assert_eq!(from_output.pair(&b, true), None);
        let from_input = WireEnd::Input(b.clone(), ValueKind::Height);
        assert_eq!(from_input.pair(&a, true), Some((a, b)));
    }
}
