//! What the pointer does on the shape canvas (the controls in the Phase
//! 12 plan): the pen (click for a corner, drag for a curve, click the
//! first point to close; Alt while starting makes a cut-out), select
//! (click, Ctrl adds, Shift takes out, drag round pieces), move, rotate,
//! scale and all with their gizmos, dragging the mirror line, and editing
//! points (double-click a piece; drag a point or handle, double-click a
//! point to make it smooth or a corner, double-click a side to add one).
//!
//! Drags follow the pull: a move goes with the pointer, a turn by the
//! angle round the pieces' middle, a size by the distance from it.

use super::geometry::{change_for, choose, gizmo_parts, knot_moved, middle_of, self_snap};
use super::report;
use super::{ShapeTool, smoothed};
use crate::material::MaterialEditor;
use crate::text::tr;
use egui::{Modifiers, PointerButton, Pos2, Rect, Ui};
use glam::Vec2;
use scene::Document;
use shapes::{Drawing, Knot, PieceId, PieceKind};
use std::sync::Arc;

/// How far (on screen) a press must move to be a drag.
pub(crate) const DRAG_START: f32 = 4.0;
/// How near (on screen) the pointer must be to a point, a handle or a
/// side to take it.
pub(crate) const REACH: f32 = 7.0;
/// How long the gizmo's arrows are, and the rotate ring's radius, on
/// screen.
pub(crate) const ARROW: f32 = 60.0;
pub(crate) const RING: f32 = 78.0;

/// A pen line being drawn.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Pen {
    pub(crate) knots: Vec<Knot>,
    /// Whether it will cut out (the Add / Cut switch, or Alt held at the
    /// first point).
    pub(crate) cut: bool,
    /// Whether the last point is being dragged into a curve.
    pub(crate) shaping: Option<Pos2>,
}

/// The gizmo round the selected pieces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Gizmo {
    Move,
    Rotate,
    Scale,
    All,
}

/// A part of the gizmo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Part {
    MoveX,
    MoveY,
    Ring,
    ScaleX,
    ScaleY,
}

/// What the pointer is over.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ShapeHit {
    /// A point of the piece being edited.
    Knot(usize),
    /// A handle of one (its outgoing one when true).
    Handle(usize, bool),
    /// A side of it, and how far along.
    Side(usize, f32),
    Gizmo(Part),
    MirrorLine,
    Piece(PieceId),
    Empty,
}

/// A press not yet a click or a drag.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ShapePress {
    pub(crate) at: Pos2,
    pub(crate) hit: ShapeHit,
    pub(crate) modifiers: Modifiers,
}

/// How pieces are being changed by a drag.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum How {
    /// Moved, along one axis (0 across, 1 down) or freely.
    Move(Option<usize>),
    Rotate,
    /// Sized, along one axis or evenly.
    Scale(Option<usize>),
}

/// What is being dragged.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ShapeDrag {
    /// Pieces moved, turned or sized: from the drawing as it was, so each
    /// frame makes the whole change again under one merge key.
    Change {
        before: Arc<Drawing>,
        ids: Vec<PieceId>,
        how: How,
        middle: Vec2,
        start: Vec2,
        key: u64,
    },
    /// A point (no handle) or one of its handles.
    Knot {
        before: Arc<Drawing>,
        id: PieceId,
        index: usize,
        handle: Option<bool>,
        key: u64,
    },
    MirrorLine {
        before: Arc<Drawing>,
        key: u64,
    },
    /// A box drawn round pieces to select them.
    Select {
        from: Pos2,
        to: Pos2,
        modifiers: Modifiers,
    },
}

impl ShapeDrag {
    /// Its undo merge key, for one that changes the drawing.
    pub(crate) fn key(&self) -> Option<u64> {
        match self {
            Self::Change { key, .. } | Self::Knot { key, .. } | Self::MirrorLine { key, .. } => {
                Some(*key)
            }
            Self::Select { .. } => None,
        }
    }
}

impl MaterialEditor {
    /// What is under the screen point `at` on the shape canvas.
    pub(crate) fn shape_hit(&self, drawing: &Drawing, area: Rect, at: Pos2) -> ShapeHit {
        let Some(s) = self.shape.as_ref() else {
            return ShapeHit::Empty;
        };
        let view = s.view;
        let scale = view.scale(area);
        let screen = |p: Vec2| view.on_screen(area, p);
        let p = view.on_canvas(area, at);
        if let Some(piece) = s.editing.and_then(|id| drawing.piece(id)) {
            for (i, k) in piece.path.knots().iter().enumerate() {
                for (out, h) in [(false, k.inn), (true, k.out)] {
                    if h != Vec2::ZERO && screen(k.at + h).distance(at) <= REACH {
                        return ShapeHit::Handle(i, out);
                    }
                }
            }
            for (i, k) in piece.path.knots().iter().enumerate() {
                if screen(k.at).distance(at) <= REACH {
                    return ShapeHit::Knot(i);
                }
            }
            if let Some((side, t, d)) = piece.path.nearest(p)
                && d * scale <= REACH
            {
                return ShapeHit::Side(side, t);
            }
        }
        if let Some(gizmo) = s.tool.gizmo()
            && let Some(m) = middle_of(drawing, s.selected.iter().copied())
        {
            let m = screen(m);
            for (part, spot) in gizmo_parts(gizmo, m) {
                let hit = match part {
                    Part::Ring => (spot.distance(at) - RING).abs() <= REACH,
                    _ => spot.distance(at) <= REACH + 3.0,
                };
                if hit {
                    return ShapeHit::Gizmo(part);
                }
            }
        }
        let mirror = drawing.mirror();
        if mirror.on && s.tool != ShapeTool::Pen {
            let x = screen(Vec2::new(mirror.x, 0.0)).x;
            let (top, bottom) = (screen(Vec2::ZERO).y, screen(Vec2::ONE).y);
            if (at.x - x).abs() <= 5.0 && at.y >= top - 20.0 && at.y <= bottom + 20.0 {
                return ShapeHit::MirrorLine;
            }
        }
        for piece in drawing.pieces().iter().rev() {
            for path in drawing.drawn_paths(piece) {
                let near = path.nearest(p).is_some_and(|(_, _, d)| d * scale <= REACH);
                let inside = piece.kind != PieceKind::Line && path.contains(p);
                if inside || near {
                    return ShapeHit::Piece(piece.id);
                }
            }
        }
        ShapeHit::Empty
    }

    /// Presses, drags and clicks on the shape canvas this frame.
    pub(crate) fn shape_pointer(
        &mut self,
        ui: &Ui,
        area: Rect,
        drawing: &Arc<Drawing>,
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
        let Some(s) = self.shape.as_ref() else { return };
        let overlays = s.overlays.clone();
        let free = |p: Pos2| {
            area.contains(p)
                && !overlays.iter().any(|r| r.contains(p))
                && !egui::Popup::is_any_open(ui.ctx())
        };
        let pen = s.tool == ShapeTool::Pen && s.editing.is_none();
        if pressed
            && let Some(at) = pos
            && free(at)
            && s.drag.is_none()
        {
            if let Some(s) = self.shape.as_mut() {
                s.list = None;
                s.note = None;
            }
            if pen {
                self.pen_press(drawing, area, at, modifiers, document, note);
            } else {
                let hit = self.shape_hit(drawing, area, at);
                if let Some(s) = self.shape.as_mut() {
                    s.press = Some(ShapePress { at, hit, modifiers });
                }
            }
        }
        if let Some(at) = pos {
            self.pen_shape(area, at, down);
        }
        let start = self.shape.as_ref().and_then(|s| {
            let press = s.press.clone()?;
            (down && s.drag.is_none() && pos?.distance(press.at) > DRAG_START).then_some(press)
        });
        if let Some(press) = start {
            self.start_shape_drag(&press, area, drawing, pos.unwrap_or(press.at));
        }
        if let Some(at) = pos {
            self.carry_on_shape_drag(area, at, document, note);
        }
        if released {
            let drag = self.shape.as_mut().and_then(|s| s.drag.take());
            let press = self.shape.as_mut().and_then(|s| s.press.take());
            match (drag, press) {
                (
                    Some(ShapeDrag::Select {
                        from,
                        to,
                        modifiers,
                    }),
                    _,
                ) => {
                    self.select_round(drawing, area, Rect::from_two_pos(from, to), modifiers);
                }
                (Some(_), _) => {}
                (None, Some(press)) => self.shape_click(&press, double, drawing, document, note),
                (None, None) => {}
            }
        }
    }

    /// A press with the pen: closes the line on its first point, or adds
    /// a point (a corner until dragged).
    fn pen_press(
        &mut self,
        drawing: &Drawing,
        area: Rect,
        at: Pos2,
        modifiers: Modifiers,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let Some(s) = self.shape.as_ref() else { return };
        let view = s.view;
        let closes = s.pen.as_ref().is_some_and(|pen| {
            pen.knots.len() >= 2
                && pen
                    .knots
                    .first()
                    .is_some_and(|k| view.on_screen(area, k.at).distance(at) <= REACH)
        });
        if closes {
            self.finish_pen(document, true, note);
            return;
        }
        let p = self.snapped(drawing, area, &view, view.on_canvas(area, at));
        let switch = s.cut;
        if let Some(s) = self.shape.as_mut() {
            let pen = s.pen.get_or_insert(Pen {
                knots: Vec::new(),
                cut: switch != modifiers.alt,
                shaping: None,
            });
            pen.knots.push(Knot::corner(p));
            pen.shaping = Some(at);
        }
    }

    /// While the pen's last point is held: dragging pulls out its handles
    /// (a curve); letting go keeps it.
    fn pen_shape(&mut self, area: Rect, at: Pos2, down: bool) {
        let Some(s) = self.shape.as_mut() else { return };
        let view = s.view;
        let Some(pen) = s.pen.as_mut() else { return };
        let Some(from) = pen.shaping else { return };
        if !down {
            pen.shaping = None;
            return;
        }
        if let Some(last) = pen.knots.last_mut()
            && from.distance(at) > DRAG_START
        {
            let pulled = view.on_canvas(area, at) - last.at;
            *last = Knot::smooth(last.at, pulled);
        }
    }

    /// Starts dragging what was pressed.
    fn start_shape_drag(
        &mut self,
        press: &ShapePress,
        area: Rect,
        drawing: &Arc<Drawing>,
        now: Pos2,
    ) {
        let key = self.shape_key();
        let Some(s) = self.shape.as_mut() else { return };
        let view = s.view;
        let start = view.on_canvas(area, press.at);
        let tool = s.tool;
        let change = |s: &mut super::ShapeCanvas, how: How| {
            let ids: Vec<PieceId> = s.selected.iter().copied().collect();
            let middle = middle_of(drawing, ids.iter().copied())?;
            Some(ShapeDrag::Change {
                before: drawing.clone(),
                ids,
                how,
                middle,
                start,
                key,
            })
        };
        let select = ShapeDrag::Select {
            from: press.at,
            to: now,
            modifiers: press.modifiers,
        };
        let drag = match press.hit {
            ShapeHit::Knot(index) | ShapeHit::Handle(index, _) => {
                s.editing.map(|id| ShapeDrag::Knot {
                    before: drawing.clone(),
                    id,
                    index,
                    handle: match press.hit {
                        ShapeHit::Handle(_, out) => Some(out),
                        _ => None,
                    },
                    key,
                })
            }
            ShapeHit::Gizmo(part) => {
                let how = match part {
                    Part::MoveX => How::Move(Some(0)),
                    Part::MoveY => How::Move(Some(1)),
                    Part::Ring => How::Rotate,
                    Part::ScaleX => How::Scale(Some(0)),
                    Part::ScaleY => How::Scale(Some(1)),
                };
                change(s, how)
            }
            ShapeHit::MirrorLine => Some(ShapeDrag::MirrorLine {
                before: drawing.clone(),
                key,
            }),
            ShapeHit::Piece(id) if s.editing.is_none() => {
                if !s.selected.contains(&id) {
                    s.selected = [id].into();
                }
                let how = match tool {
                    ShapeTool::Rotate => How::Rotate,
                    ShapeTool::Scale => How::Scale(None),
                    _ => How::Move(None),
                };
                change(s, how)
            }
            _ if s.editing.is_none() && !s.selected.is_empty() => match tool {
                ShapeTool::Move => change(s, How::Move(None)),
                ShapeTool::Rotate => change(s, How::Rotate),
                ShapeTool::Scale => change(s, How::Scale(None)),
                _ => Some(select),
            },
            _ => Some(select),
        };
        s.drag = drag;
        if s.drag.is_none() {
            s.press = None;
        }
    }

    /// Carries a drag on to the pointer at `at`: the whole change made
    /// again from the drawing as it was, under the drag's merge key.
    fn carry_on_shape_drag(
        &mut self,
        area: Rect,
        at: Pos2,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let Some(s) = self.shape.as_mut() else { return };
        let view = s.view;
        let p = view.on_canvas(area, at);
        let changed = match &mut s.drag {
            None => return,
            Some(ShapeDrag::Select { to, .. }) => {
                *to = at;
                return;
            }
            Some(ShapeDrag::Change {
                before,
                ids,
                how,
                middle,
                start,
                key,
            }) => {
                let change = change_for(*how, *middle, *start, p);
                let label = match how {
                    How::Move(_) => tr("Move pieces"),
                    How::Rotate => tr("Turn pieces"),
                    How::Scale(_) => tr("Size pieces"),
                };
                before.with_change(ids, &change).map(|d| (d, label, *key))
            }
            Some(ShapeDrag::Knot {
                before,
                id,
                index,
                handle,
                key,
            }) => {
                let p = if handle.is_none() {
                    self_snap(before, area, &view, p)
                } else {
                    p
                };
                let label = if handle.is_some() {
                    tr("Move a handle")
                } else {
                    tr("Move a point")
                };
                knot_moved(before, *id, *index, *handle, p).map(|d| (d, label, *key))
            }
            Some(ShapeDrag::MirrorLine { before, key }) => {
                let mut mirror = before.mirror();
                mirror.x = p.x;
                before
                    .with_mirror(mirror)
                    .map(|d| (d, tr("Move the mirror line"), *key))
            }
        };
        // A change that can't be made (a point too far out, a shape with
        // no room left inside) stops where it last could be.
        if let Ok((drawing, label, key)) = changed {
            let done = self.set_drawing(document, drawing, label, Some(key));
            report(done, note);
        }
    }

    /// A press let go without moving.
    fn shape_click(
        &mut self,
        press: &ShapePress,
        double: bool,
        drawing: &Drawing,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let Some(s) = self.shape.as_mut() else { return };
        let Some(editing) = s.editing else {
            match press.hit {
                ShapeHit::Piece(id) => {
                    choose(&mut s.selected, id, press.modifiers);
                    if double {
                        s.editing = Some(id);
                        s.point = None;
                        s.selected = [id].into();
                        *note = Some(
                            tr(
                                "Editing points: drag a point or its handles; double-click a \
                                 point to make it smooth or a corner, or a side to add one. Tab \
                                 when done.",
                            )
                            .into(),
                        );
                    }
                }
                ShapeHit::Empty if !press.modifiers.command && !press.modifiers.shift => {
                    s.selected.clear();
                }
                _ => {}
            }
            return;
        };
        let Some(piece) = drawing.piece(editing) else {
            s.editing = None;
            return;
        };
        let path = &piece.path;
        let changed = match press.hit {
            ShapeHit::Knot(index) => {
                s.point = Some(index);
                if !double {
                    return;
                }
                let knots = path.knots();
                let n = knots.len();
                let k = knots[index];
                let made = if k.is_corner() {
                    let prev = knots[(index + n - 1) % n].at;
                    let next = knots[(index + 1) % n].at;
                    (smoothed(prev, k.at, next), tr("Make a point smooth"))
                } else {
                    (Knot::corner(k.at), tr("Make a point a corner"))
                };
                drawing
                    .with_path(editing, path.with_knot(index, made.0))
                    .map(|d| (d, made.1))
            }
            ShapeHit::Side(side, t) if double => {
                s.point = Some(side + 1);
                drawing
                    .with_path(editing, path.with_knot_on_side(side, t))
                    .map(|d| (d, tr("Add a point")))
            }
            ShapeHit::Piece(other) if double && other != editing => {
                s.editing = Some(other);
                s.point = None;
                s.selected = [other].into();
                return;
            }
            ShapeHit::Empty => {
                s.point = None;
                return;
            }
            _ => return,
        };
        match changed {
            Ok((d, label)) => {
                let done = self.set_drawing(document, d, label, None);
                report(done, note);
            }
            Err(why) => self.shape_note(tr(why.describe()), Vec::new(), note),
        }
    }

    /// Selects the pieces whose middles are inside `round` (on screen).
    fn select_round(&mut self, drawing: &Drawing, area: Rect, round: Rect, modifiers: Modifiers) {
        let Some(s) = self.shape.as_mut() else { return };
        let view = s.view;
        let inside: Vec<PieceId> = drawing
            .pieces()
            .iter()
            .filter(|p| round.contains(view.on_screen(area, p.path.middle())))
            .map(|p| p.id)
            .collect();
        if modifiers.shift {
            for id in inside {
                s.selected.remove(&id);
            }
        } else {
            if !modifiers.command {
                s.selected.clear();
            }
            s.selected.extend(inside);
        }
    }
}
