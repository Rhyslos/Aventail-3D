//! The shape canvas (Phase 12 milestone 11, decision 0273): where a Shape
//! box's drawing is made. It opens from the box (double-click it, or Open
//! the canvas in Attributes) in place of the node canvas, and Tab goes
//! back to the graph.
//!
//! Every change makes a new `shapes::Drawing` and is carried out as one
//! `GraphEdit::SetDrawing` through `MaterialEditor::run`, so undo works
//! and the drawing is saved with the project; a drag is one undo step
//! (one merge key) and Esc abandons it.
//!
//! - this file: the canvas's state, its view, its keys, opening and
//!   closing, and changing the drawing.
//! - `changes`: the mirror line, removing, notes and their fixes, and
//!   making pen lines and basic shapes into pieces.
//! - `geometry`: the gizmo's parts, the change a drag makes, a point
//!   dragged.
//! - `pointer`: what the pointer does with each tool (the pen, select,
//!   move, rotate, scale, all, the mirror line, editing points).
//! - `paint`: drawing it all on screen.
//! - `panels`: the bar over it, the pieces list, the strip showing what
//!   the box gives, the basic shapes list and the notes; `rows`: the
//!   chosen piece's rows and the reference image's.

mod changes;
mod geometry;
mod paint;
mod panels;
mod pointer;
mod rows;

use super::MaterialEditor;
use crate::text::tr;
use egui::{Context, Key, Modifiers, Pos2, Rect};
use glam::Vec2;
use matgraph::{BoxId, GraphEdit, Refusal};
use scene::Document;
use shapes::{Drawing, Knot, PieceId};
use std::collections::BTreeSet;
use std::sync::Arc;

pub(crate) use pointer::{Gizmo, Pen};

/// The canvas panel's accessible name while the shape canvas is open.
pub(crate) const SHAPE_CANVAS_LABEL: &str = crate::text!("Shape canvas");
/// The button going back to the graph.
pub(crate) const BACK_LABEL: &str = crate::text!("Back to the graph (Tab)");
/// The button opening the canvas, in Attributes.
pub(crate) const OPEN_LABEL: &str = crate::text!("Open the canvas");

/// P: the pen.
const PEN_KEY: Key = Key::P;
/// B: the basic shapes list.
const SHAPES_KEY: Key = Key::B;
/// M: the mirror line on or off.
const MIRROR_KEY: Key = Key::M;

/// The shape canvas's tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ShapeTool {
    /// Q: click a piece to select it; drag round pieces to select them.
    #[default]
    Select,
    /// W: drag to move the selected pieces.
    Move,
    /// R: drag to turn them.
    Rotate,
    /// S: drag to size them.
    Scale,
    /// T: move, turn and size with one gizmo.
    All,
    /// P: draw points: click for a corner, drag for a curve.
    Pen,
}

impl ShapeTool {
    /// Its plain name and key.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Select => crate::text!("Select (Q)"),
            Self::Move => crate::text!("Move (W)"),
            Self::Rotate => crate::text!("Rotate (R)"),
            Self::Scale => crate::text!("Scale (S)"),
            Self::All => crate::text!("All (T)"),
            Self::Pen => crate::text!("Pen (P)"),
        }
    }

    /// The gizmo it shows round the selected pieces.
    pub(crate) fn gizmo(self) -> Option<Gizmo> {
        match self {
            Self::Move => Some(Gizmo::Move),
            Self::Rotate => Some(Gizmo::Rotate),
            Self::Scale => Some(Gizmo::Scale),
            Self::All => Some(Gizmo::All),
            Self::Select | Self::Pen => None,
        }
    }
}

/// A key pressed over the shape canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShapeKey {
    Tool(ShapeTool),
    Shapes,
    Mirror,
    Remove,
    Finish,
    Escape,
}

/// Pan and zoom: the texture square's middle sits at the middle of the
/// drawing area, moved by `pan` (in texture widths), `zoom` times the
/// size that fits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ShapeView {
    pub(crate) pan: Vec2,
    pub(crate) zoom: f32,
}

impl Default for ShapeView {
    fn default() -> Self {
        Self {
            pan: Vec2::ZERO,
            zoom: 1.0,
        }
    }
}

impl ShapeView {
    /// Screen pixels per texture width in `area`.
    pub(crate) fn scale(&self, area: Rect) -> f32 {
        area.width().min(area.height()).max(1.0) * 0.8 * self.zoom
    }

    /// Where the canvas place `p` is on screen.
    pub(crate) fn on_screen(&self, area: Rect, p: Vec2) -> Pos2 {
        let s = self.scale(area);
        let q = (p - Vec2::splat(0.5) - self.pan) * s;
        area.center() + egui::vec2(q.x, q.y)
    }

    /// The canvas place under the screen point `q`.
    pub(crate) fn on_canvas(&self, area: Rect, q: Pos2) -> Vec2 {
        let d = q - area.center();
        Vec2::new(d.x, d.y) / self.scale(area) + Vec2::splat(0.5) + self.pan
    }

    /// The wheel turned by `delta` with the pointer at `at`: zooms round
    /// it.
    pub(crate) fn wheel(&mut self, area: Rect, at: Pos2, delta: f32) {
        let before = self.on_canvas(area, at);
        self.zoom = (self.zoom * (delta * 0.002).exp()).clamp(0.2, 20.0);
        let after = self.on_canvas(area, at);
        self.pan += before - after;
    }
}

/// A note on the canvas: why something won't work, with the ways to make
/// it work.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ShapeNote {
    pub(crate) reason: String,
    pub(crate) fixes: Vec<NoteFix>,
}

/// A way to make it work, offered in a note.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NoteFix {
    /// Join the pen line's last point to its first, and fill it.
    CloseIt,
    /// Keep the pen line as a line.
    KeepAsLine,
    /// Throw the pen line away.
    ThrowAway,
}

impl NoteFix {
    pub(crate) fn describe(self) -> &'static str {
        match self {
            Self::CloseIt => crate::text!("Close it"),
            Self::KeepAsLine => crate::text!("Keep it as a line"),
            Self::ThrowAway => crate::text!("Throw it away"),
        }
    }
}

/// The shape canvas open on one Shape box.
#[derive(Debug)]
pub(crate) struct ShapeCanvas {
    /// The Shape box drawn on, in the graph shown.
    pub(crate) node: BoxId,
    pub(crate) view: ShapeView,
    pub(crate) tool: ShapeTool,
    /// The Add / Cut switch: new shapes cut out (Alt does the same while
    /// drawing).
    pub(crate) cut: bool,
    pub(crate) selected: BTreeSet<PieceId>,
    /// The piece whose points are being edited (double-click).
    pub(crate) editing: Option<PieceId>,
    /// The point chosen while editing.
    pub(crate) point: Option<usize>,
    /// A pen line being drawn.
    pub(crate) pen: Option<Pen>,
    pub(crate) press: Option<pointer::ShapePress>,
    pub(crate) drag: Option<pointer::ShapeDrag>,
    /// The basic shapes list, open at this screen point.
    pub(crate) list: Option<Pos2>,
    /// Sides of a polygon, points of a star, in the list.
    pub(crate) sides: u32,
    pub(crate) note: Option<ShapeNote>,
    /// Where the whole canvas and its drawing area were drawn.
    pub(crate) rect: Option<Rect>,
    pub(crate) area: Option<Rect>,
    /// The note and the list as drawn: presses there aren't on the canvas.
    pub(crate) overlays: Vec<Rect>,
    pub(crate) keys: Vec<ShapeKey>,
    pub(crate) textures: paint::Textures,
}

impl ShapeCanvas {
    fn new(node: BoxId) -> Self {
        Self {
            node,
            view: ShapeView::default(),
            tool: ShapeTool::default(),
            cut: false,
            selected: BTreeSet::new(),
            editing: None,
            point: None,
            pen: None,
            press: None,
            drag: None,
            list: None,
            sides: 6,
            note: None,
            rect: None,
            area: None,
            overlays: Vec::new(),
            keys: Vec::new(),
            textures: paint::Textures::default(),
        }
    }

    /// Whether Esc has something to cancel: a pen line, a drag, the
    /// basic shapes list or a note.
    pub(crate) fn cancellable(&self) -> bool {
        self.pen.is_some() || self.drag.is_some() || self.list.is_some() || self.note.is_some()
    }

    /// A new undo merge key for one change or drag here.
    fn key(&mut self, editor_key: u64) -> u64 {
        /// Marks the shape canvas's merge keys apart from every other.
        const SHAPE_KEYS: u64 = 0x5348_4150_0000_0000;
        SHAPE_KEYS ^ editor_key
    }
}

impl MaterialEditor {
    /// Opens the shape canvas on Shape box `node` of the graph shown.
    pub(crate) fn open_shape_canvas(&mut self, node: BoxId) {
        self.shape = Some(ShapeCanvas::new(node));
        self.canvas.search = None;
        self.canvas.note = None;
        self.canvas.selected = [node].into();
    }

    /// Whether box `id` of the graph shown is drawn on the shape canvas.
    pub(crate) fn is_drawn(&self, document: &Document, id: BoxId) -> bool {
        self.current(document)
            .and_then(|g| g.get(id))
            .and_then(|b| self.registry_of(document).get(&b.kind).cloned())
            .is_some_and(|k| k.takes_drawing())
    }

    /// The drawing on the open canvas's box as it is in `document` (empty
    /// for a box with nothing drawn yet); `None` when the box is gone.
    pub(crate) fn shape_drawing(&self, document: &Document) -> Option<Arc<Drawing>> {
        let node = self.shape.as_ref()?.node;
        let b = self.current(document)?.get(node)?;
        Some(
            b.drawing
                .clone()
                .unwrap_or_else(|| Arc::new(Drawing::new())),
        )
    }

    /// Makes `drawing` the open canvas's drawing: one undo step labelled
    /// `label`, merged with others of key `key` (a drag), or why not.
    pub(crate) fn set_drawing(
        &mut self,
        document: &mut Document,
        drawing: Drawing,
        label: &str,
        key: Option<u64>,
    ) -> Result<(), Refusal> {
        let Some(node) = self.shape.as_ref().map(|s| s.node) else {
            return Ok(());
        };
        let edit = GraphEdit::SetDrawing {
            node,
            drawing: Arc::new(drawing),
        };
        self.run(document, vec![edit], label, key, false)
    }

    /// A fresh merge key for a change on the shape canvas.
    pub(crate) fn shape_key(&mut self) -> u64 {
        let k = self.next_key();
        self.shape.as_mut().map_or(k, |s| s.key(k))
    }

    /// Takes the shape canvas's keys while the pointer is over it (and
    /// nothing is being typed): its tools, the shapes list, the mirror
    /// line, Delete, Enter and Esc. True when the canvas is open (the
    /// node canvas's keys then wait).
    pub(crate) fn take_shape_keys(&mut self, ctx: &Context, typing: bool) -> bool {
        let Some(shape) = self.shape.as_mut() else {
            return false;
        };
        shape.keys.clear();
        let over = shape
            .rect
            .zip(ctx.pointer_hover_pos())
            .is_some_and(|(rect, at)| rect.contains(at));
        if typing || !over {
            return true;
        }
        let plain = [
            (Key::Q, ShapeKey::Tool(ShapeTool::Select)),
            (Key::W, ShapeKey::Tool(ShapeTool::Move)),
            (Key::R, ShapeKey::Tool(ShapeTool::Rotate)),
            (Key::S, ShapeKey::Tool(ShapeTool::Scale)),
            (Key::T, ShapeKey::Tool(ShapeTool::All)),
            (PEN_KEY, ShapeKey::Tool(ShapeTool::Pen)),
            (SHAPES_KEY, ShapeKey::Shapes),
            (MIRROR_KEY, ShapeKey::Mirror),
            (Key::Delete, ShapeKey::Remove),
            (Key::Enter, ShapeKey::Finish),
        ];
        let cancel = shape.cancellable();
        ctx.input_mut(|i| {
            for (key, what) in plain {
                if i.consume_key(Modifiers::NONE, key) {
                    shape.keys.push(what);
                }
            }
            if cancel && i.consume_key(Modifiers::NONE, Key::Escape) {
                shape.keys.push(ShapeKey::Escape);
            }
        });
        true
    }

    /// Tab on the shape canvas: out of editing points, or back to the
    /// graph; says what happened (or why not yet) in `note`.
    pub(crate) fn shape_step_back(&mut self, note: &mut Option<String>) {
        let Some(shape) = self.shape.as_mut() else {
            return;
        };
        if shape.drag.is_some() {
            *note = Some(tr("Let go of the drag (or press Esc) first, then Tab.").into());
        } else if shape.pen.is_some() {
            *note = Some(
                tr(
                    "Finish the pen line first (click its first point, or Enter), or Esc to \
                     throw it away, then Tab.",
                )
                .into(),
            );
        } else if shape.editing.take().is_some() {
            shape.point = None;
            *note = Some(tr("Done editing points: Tab again goes back to the graph.").into());
        } else {
            self.shape = None;
            *note = Some(tr("Back to the graph.").into());
        }
    }

    /// Carries out the keys pressed over the shape canvas this frame.
    fn use_shape_keys(
        &mut self,
        ctx: &Context,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let keys = self
            .shape
            .as_mut()
            .map(|s| std::mem::take(&mut s.keys))
            .unwrap_or_default();
        for key in keys {
            match key {
                ShapeKey::Tool(tool) => self.choose_tool(tool),
                ShapeKey::Shapes => {
                    let at = ctx.pointer_hover_pos();
                    if let Some(s) = self.shape.as_mut() {
                        s.list = at.or(s.area.map(|a| a.center()));
                    }
                }
                ShapeKey::Mirror => self.toggle_mirror(document, note),
                ShapeKey::Remove => self.remove_chosen(document, note),
                ShapeKey::Finish => self.finish_pen(document, false, note),
                ShapeKey::Escape => self.shape_escape(document),
            }
        }
    }

    /// Picks `tool` (a pen line under way is kept until finished or
    /// cancelled).
    pub(crate) fn choose_tool(&mut self, tool: ShapeTool) {
        if let Some(s) = self.shape.as_mut() {
            s.tool = tool;
            s.list = None;
            if tool != ShapeTool::Pen {
                s.editing = None;
                s.point = None;
            }
        }
    }

    /// Esc: cancels what is under way, and only that.
    fn shape_escape(&mut self, document: &mut Document) {
        let Some(s) = self.shape.as_mut() else { return };
        if let Some(drag) = s.drag.take() {
            s.press = None;
            if let Some(key) = drag.key() {
                document.abandon(key);
            }
        } else if s.pen.take().is_some() || s.list.take().is_none() {
            // The pen line goes with any note about it; with neither a
            // line nor the list, Esc closes the note.
            s.note = None;
        }
    }

    /// The mirror line's place, snapping the pen's points onto it when
    /// they are close (on screen), so a half drawn against it closes.
    pub(crate) fn snapped(&self, drawing: &Drawing, area: Rect, view: &ShapeView, p: Vec2) -> Vec2 {
        let mirror = drawing.mirror();
        let reach = 8.0 / view.scale(area);
        if mirror.on && (p.x - mirror.x).abs() < reach {
            Vec2::new(mirror.x, p.y)
        } else {
            p
        }
    }
}

/// Puts a refusal's reason in the status bar.
pub(crate) fn report(done: Result<(), Refusal>, note: &mut Option<String>) {
    if let Err(why) = done {
        *note = Some(why.reason);
    }
}

/// A smooth point's handles made from its neighbours (a third of the way
/// to each, along the line between them), for turning a corner smooth.
pub(crate) fn smoothed(prev: Vec2, at: Vec2, next: Vec2) -> Knot {
    let along = (next - prev).normalize_or_zero();
    let out = along * (next - at).length() / 3.0;
    let inn = -along * (at - prev).length() / 3.0;
    Knot { at, inn, out }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_view_turns_canvas_places_to_screen_and_back() {
        let area = Rect::from_min_size(Pos2::new(10.0, 20.0), egui::vec2(500.0, 400.0));
        let mut view = ShapeView::default();
        assert_eq!(view.on_screen(area, Vec2::splat(0.5)), area.center());
        let p = Vec2::new(0.2, 0.9);
        assert!(view.on_canvas(area, view.on_screen(area, p)).distance(p) < 1e-5);
        // Zooming round a point keeps it under the pointer.
        let at = Pos2::new(100.0, 100.0);
        let before = view.on_canvas(area, at);
        view.wheel(area, at, 300.0);
        assert!(view.zoom > 1.0);
        assert!(view.on_canvas(area, at).distance(before) < 1e-4);
    }

    #[test]
    fn a_corner_turned_smooth_gets_handles_along_its_neighbours() {
        let k = smoothed(Vec2::ZERO, Vec2::new(1.0, 1.0), Vec2::new(2.0, 0.0));
        assert!(k.inn.y.abs() < 1e-6 && k.out.y.abs() < 1e-6);
        assert!(k.out.x > 0.0 && k.inn.x < 0.0);
        assert_eq!(smoothed(Vec2::ONE, Vec2::ONE, Vec2::ONE).out, Vec2::ZERO);
    }
}
