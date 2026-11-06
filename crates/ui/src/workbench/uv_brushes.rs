//! UV brushes in the UV Editor ([decision 0238](../../../../docs/decisions/0238-uv-brushes.md)):
//! Grab, Relax and Pinch, picked from the UV Editor's Brush menu (or UV →
//! Pieces → UV brush), then dragged over the UVs. The controls are the
//! ones the project lead chose for UV brushes: **Ctrl + drag** does the
//! opposite (Pinch pushes out; Grab and Relax relax gently instead),
//! **[** and **]** change the size, **Esc** goes back to selecting (or,
//! mid-stroke, takes the stroke back). Each stroke is one undo step: one
//! `Operation::SetUvs` per object with the corners' final UVs. Only the
//! left button brushes: the view's own drags (a middle drag pans) work
//! under the brush, and a stroke touches only the pieces the editor
//! shows (not those hidden by Isolate).

use super::Workbench;
use crate::palette::Mode;
use crate::text::tr;
use egui::{Key, Modifiers, Sense, Stroke, Ui};
use glam::Vec2;
use scene::{ObjectId, Operation};
use uv::brush::{Spots, UvBrushKind};

/// The menu's name, and each brush's name and help.
pub(crate) const UV_BRUSH_MENU: &str = crate::text!("Brush");
pub(crate) const UV_BRUSH_HELP: &str = crate::text!(
    "Shape UVs with a soft round brush: drag over them in the UV Editor. Ctrl + drag does the \
     opposite, [ and ] change the size, Esc goes back to selecting."
);

/// A brush's words in the window.
pub(crate) trait BrushWords {
    /// Its name.
    fn name(self) -> &'static str;
    /// What it does.
    fn help(self) -> &'static str;
}

impl BrushWords for UvBrushKind {
    /// Its name.
    fn name(self) -> &'static str {
        match self {
            UvBrushKind::Grab => tr("Grab UVs"),
            UvBrushKind::Relax => tr("Relax UVs"),
            UvBrushKind::Pinch => tr("Pinch UVs"),
        }
    }

    /// What it does.
    fn help(self) -> &'static str {
        match self {
            UvBrushKind::Grab => tr(
                "Drag the UVs under the brush along with the mouse, most in the middle: for \
                 nudging a bulge or bending a piece into place",
            ),
            UvBrushKind::Relax => tr(
                "Even the UVs under the brush out, each pulled towards its neighbours: for \
                 smoothing out stretch or a crumpled patch",
            ),
            UvBrushKind::Pinch => tr(
                "Pull the UVs under the brush towards its middle (Ctrl: push them out): to give \
                 a detail less, or more, of the texture",
            ),
        }
    }
}

/// The brush's size limits and standard, in screen points.
pub(crate) const UV_BRUSH_MIN: f32 = 6.0;
pub(crate) const UV_BRUSH_MAX: f32 = 300.0;
pub(crate) const UV_BRUSH_RADIUS: f32 = 40.0;
/// How much `[` and `]` change the size.
const SIZE_STEP: f32 = 1.2;
/// How far Relax and Pinch go each dab.
const STRENGTH: f32 = 0.3;

/// The UV brush while in use.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct UvBrushing {
    pub(super) kind: UvBrushKind,
    /// Its radius, in screen points.
    pub(super) radius: f32,
    stroke: Option<BrushStroke>,
}

/// A stroke in progress.
#[derive(Debug, Clone, PartialEq)]
struct BrushStroke {
    key: u64,
    /// Where the brush was last dab, in the texture.
    last: Vec2,
    /// Whether Ctrl was held as it began.
    other_way: bool,
    /// Each shown object's spots, as they are now in the stroke.
    parts: Vec<(ObjectId, Spots)>,
}

impl Workbench {
    /// Whether a UV brush is in use.
    pub(crate) fn uv_brush(&self) -> Option<UvBrushKind> {
        self.uv_brushing.as_ref().map(|b| b.kind)
    }

    /// The Brush menu's entries (in the UV Editor and UV → Pieces).
    pub(super) fn uv_brush_entries(&mut self, ui: &mut Ui) {
        let now = self.uv_brush();
        for kind in UvBrushKind::ALL {
            if ui
                .selectable_label(now == Some(kind), kind.name())
                .on_hover_text(kind.help())
                .clicked()
            {
                if now == Some(kind) {
                    self.stop_uv_brush();
                } else {
                    self.start_uv_brush(kind);
                }
                ui.close();
            }
        }
    }

    /// Picks up `kind`.
    pub(crate) fn start_uv_brush(&mut self, kind: UvBrushKind) {
        if self.mode == Mode::Paint {
            return;
        }
        self.stop_lattice();
        let radius = self
            .uv_brushing
            .as_ref()
            .map_or(UV_BRUSH_RADIUS, |b| b.radius);
        self.uv_brushing = Some(UvBrushing {
            kind,
            radius,
            stroke: None,
        });
        self.note = Some(crate::trf!(
            "{}: drag over the UVs in the UV Editor. Ctrl + drag does the opposite; [ and ] \
             change the size; Esc goes back to selecting.",
            kind.name()
        ));
    }

    /// Puts the brush down.
    pub(crate) fn stop_uv_brush(&mut self) {
        if self.uv_brushing.take().is_some() {
            self.note = Some(tr("UV brush put down: dragging selects again.").into());
        }
    }

    /// Each frame while a brush is up: Esc, `[` and `]`, and over the UV
    /// Editor the circle and the strokes.
    pub(super) fn use_uv_brush(&mut self, ui: &Ui) {
        if self.uv_brushing.is_none() {
            return;
        }
        if self.mode == Mode::Paint {
            self.stop_uv_brush();
            return;
        }
        let typing = ui.ctx().egui_wants_keyboard_input();
        if !typing {
            let (esc, smaller, bigger) = ui.input_mut(|i| {
                (
                    i.consume_key(Modifiers::NONE, Key::Escape),
                    i.consume_key(Modifiers::NONE, Key::OpenBracket),
                    i.consume_key(Modifiers::NONE, Key::CloseBracket),
                )
            });
            // Esc mid-stroke takes the stroke back, as it cancels a gizmo
            // drag; otherwise it puts the brush down.
            if esc && self.uv_stroking() {
                self.cancel_uv_stroke();
            } else if esc {
                self.stop_uv_brush();
                return;
            }
            if let Some(b) = &mut self.uv_brushing {
                if smaller {
                    b.radius = (b.radius / SIZE_STEP).clamp(UV_BRUSH_MIN, UV_BRUSH_MAX);
                }
                if bigger {
                    b.radius = (b.radius * SIZE_STEP).clamp(UV_BRUSH_MIN, UV_BRUSH_MAX);
                }
            }
        }
        let Some(rect) = self.uv_editor.drawn() else {
            return;
        };
        if !self.uv_editor.hovered(ui.ctx())
            && self
                .uv_brushing
                .as_ref()
                .is_some_and(|b| b.stroke.is_none())
        {
            return;
        }
        let radius_px = self
            .uv_brushing
            .as_ref()
            .map_or(UV_BRUSH_RADIUS, |b| b.radius);
        let accent = crate::theme::accent(ui.ctx()).colors();
        let response = egui::Area::new(egui::Id::new("uv-brush"))
            .fixed_pos(rect.min)
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                let response = ui.allocate_rect(rect, Sense::drag());
                if let Some(at) = response.hover_pos().or(response.interact_pointer_pos()) {
                    ui.painter_at(rect).circle_stroke(
                        at,
                        radius_px,
                        Stroke::new(1.0, accent.light),
                    );
                }
                response
            })
            .inner;
        // Panning and zooming work under the brush as over the editor (a
        // middle drag pans): they take the drag, and only the left button
        // brushes.
        let navigation = self.viewport.options().navigation;
        self.uv_editor.navigate(ui, &response, rect, &navigation);
        let ctrl = ui.input(|i| i.modifiers.command);
        if response.drag_started_by(egui::PointerButton::Primary)
            && !self.uv_editor.navigating()
            && let Some(at) = response.interact_pointer_pos()
            && let Some(uv) = self.uv_editor.uv_of(at)
        {
            self.begin_uv_stroke(uv, ctrl);
        }
        if response.dragged()
            && let Some(at) = response.interact_pointer_pos()
            && let Some(uv) = self.uv_editor.uv_of(at)
        {
            let per_point = self
                .uv_editor
                .uv_of(at + egui::vec2(1.0, 0.0))
                .map_or(0.001, |p| (p - uv).length().max(1e-9));
            self.uv_dab(uv, radius_px * per_point);
        }
        if (response.drag_stopped() || !ui.input(|i| i.pointer.any_down()))
            && let Some(b) = &mut self.uv_brushing
        {
            b.stroke = None;
        }
    }

    /// Whether a brush stroke is under way (keys that change the document
    /// wait for it, as they wait for a gizmo drag).
    pub(crate) fn uv_stroking(&self) -> bool {
        self.uv_brushing
            .as_ref()
            .is_some_and(|b| b.stroke.is_some())
    }

    /// Takes back the stroke under way (Esc), keeping the brush up. The
    /// rest of the drag does nothing.
    pub(super) fn cancel_uv_stroke(&mut self) {
        let Some(stroke) = self.uv_brushing.as_mut().and_then(|b| b.stroke.take()) else {
            return;
        };
        self.document.abandon(stroke.key);
        self.note = Some(tr("Stroke cancelled: the UVs are back as they were.").into());
    }

    /// Begins a stroke at `uv` (`other_way`: Ctrl held).
    pub(super) fn begin_uv_stroke(&mut self, uv: Vec2, other_way: bool) {
        let key = self.new_step_key();
        // Only what the editor shows: while it isolates, the pieces it
        // hides stay put.
        let shapes = self.own_shapes();
        let parts: Vec<(ObjectId, Spots)> = shapes
            .into_iter()
            .map(|(id, mesh)| (id, self.uv_editor.shown_mesh(&self.document, id, mesh)))
            .map(|(id, mesh)| (id, Spots::of(&mesh)))
            .filter(|(_, s)| !s.is_empty())
            .collect();
        if parts.is_empty() {
            self.note = Some(tr("UV brush: select objects with UVs first.").into());
            return;
        }
        if let Some(b) = &mut self.uv_brushing {
            b.stroke = Some(BrushStroke {
                key,
                last: uv,
                other_way,
                parts,
            });
        }
    }

    /// One dab at `uv` with a radius of `radius` texture widths, recorded
    /// in place of the stroke's last.
    pub(super) fn uv_dab(&mut self, uv: Vec2, radius: f32) {
        let Some(b) = &mut self.uv_brushing else {
            return;
        };
        let kind = b.kind;
        let Some(stroke) = &mut b.stroke else {
            return;
        };
        let pull = uv - stroke.last;
        stroke.last = uv;
        // Ctrl: Pinch pushes; Grab and Relax relax gently.
        let (kind, out) = match (kind, stroke.other_way) {
            (UvBrushKind::Pinch, out) => (UvBrushKind::Pinch, out),
            (_, true) => (UvBrushKind::Relax, false),
            (k, false) => (k, false),
        };
        let mut any = 0;
        for (_, spots) in &mut stroke.parts {
            any += spots.dab(kind, uv, radius, pull, STRENGTH, out);
        }
        if any == 0 {
            return;
        }
        let key = stroke.key;
        let ops: Vec<(ObjectId, Operation)> = stroke
            .parts
            .iter()
            .map(|(id, spots)| (*id, spots.moved_corners()))
            .filter(|(_, c)| !c.is_empty())
            .map(|(id, corners)| (id, Operation::SetUvs { corners }))
            .collect();
        self.document.abandon(key);
        let label = crate::trf!("{} stroke", kind.name());
        self.record_each_in(&label, ops, key);
    }
}
