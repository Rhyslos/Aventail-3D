//! Moving around the editor like the viewport: panning with the
//! navigation controls, zooming at the mouse, and framing the texture
//! square (H) or the selection (Z).

use super::shape::UvShape;
use super::{EditorDrag, UvEditor};
use crate::text::tr;
use egui::{Key, Pos2, Rect, Response, Ui, pos2};
use glam::Vec2;
use scene::{Document, Element, ObjectId};
use tools::{DragKind, Modifiers, MouseButton, NavigationSettings};

/// Zoom limits, in screen points per texture width.
const MIN_ZOOM: f32 = 20.0;

const MAX_ZOOM: f32 = 200_000.0;

/// What to fit in the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UvFrame {
    /// The texture square (H, as Frame all in the viewport).
    Square,
    /// The selection's UVs (Z, as Frame selected).
    Selected,
}

impl UvFrame {
    /// The menu entry's name.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Square => tr("Frame the texture square"),
            Self::Selected => tr("Frame selected"),
        }
    }

    /// Its key, the same as the viewport's framing key.
    pub(crate) fn key(self) -> Key {
        match self {
            Self::Square => crate::Frame::All.key(),
            Self::Selected => crate::Frame::Selected.key(),
        }
    }
}

impl UvEditor {
    /// Asks for a frame on the next frame (from a menu).
    pub(crate) fn frame(&mut self, what: UvFrame) {
        self.frame_request = Some(what);
    }

    /// Takes H and Z while the mouse is over the editor (before the
    /// viewport, which would take them otherwise), so they frame here.
    pub(crate) fn take_keys(&mut self, ctx: &egui::Context) {
        if !self.hovered(ctx) || ctx.egui_wants_keyboard_input() {
            return;
        }
        for what in [UvFrame::Square, UvFrame::Selected] {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, what.key())) {
                self.frame_request = Some(what);
            }
        }
    }

    pub(super) fn to_screen(&self, rect: Rect, uv: Vec2) -> Pos2 {
        let c = rect.center();
        pos2(
            c.x + (uv.x - self.centre.x) * self.zoom,
            c.y - (uv.y - self.centre.y) * self.zoom,
        )
    }

    pub(super) fn to_uv(&self, rect: Rect, at: Pos2) -> Vec2 {
        let c = rect.center();
        Vec2::new(
            self.centre.x + (at.x - c.x) / self.zoom,
            self.centre.y - (at.y - c.y) / self.zoom,
        )
    }

    /// Fits `min`..`max` (UVs) in the panel, with a margin.
    pub(super) fn fit(&mut self, rect: Rect, min: Vec2, max: Vec2) {
        let size = (max - min).max(Vec2::splat(1e-3));
        let room = (rect.width().min(rect.height()) * 0.85).max(10.0);
        self.zoom = (room / size.x.max(size.y)).clamp(MIN_ZOOM, MAX_ZOOM);
        self.centre = (min + max) / 2.0;
    }

    pub(super) fn frame_from_request(
        &mut self,
        rect: Rect,
        shapes: &[(ObjectId, UvShape)],
        doc: &Document,
    ) {
        let Some(what) = self.frame_request.take() else {
            return;
        };
        match what {
            UvFrame::Square => self.fit(rect, Vec2::ZERO, Vec2::ONE),
            UvFrame::Selected => {
                let mut min = Vec2::splat(f32::INFINITY);
                let mut max = Vec2::splat(f32::NEG_INFINITY);
                let mut any_selected = false;
                for (id, shape) in shapes {
                    let selected = doc.elements(*id).cloned().unwrap_or_default();
                    for (face, points) in &shape.polygons {
                        let chosen = selected.contains(Element::Polygon(*face))
                            || points
                                .iter()
                                .any(|(v, _)| selected.contains(Element::Point(*v)));
                        if chosen {
                            any_selected = true;
                            for (_, uv) in points {
                                min = min.min(*uv);
                                max = max.max(*uv);
                            }
                        }
                    }
                }
                if !any_selected {
                    for (_, shape) in shapes {
                        for (_, points) in &shape.polygons {
                            for (_, uv) in points {
                                min = min.min(*uv);
                                max = max.max(*uv);
                            }
                        }
                    }
                }
                if min.is_finite() {
                    self.fit(rect, min, max);
                } else {
                    self.fit(rect, Vec2::ZERO, Vec2::ONE);
                }
            }
        }
    }

    /// Panning and zooming, with the viewport's controls (orbiting pans).
    pub(super) fn navigate(
        &mut self,
        ui: &Ui,
        response: &Response,
        rect: Rect,
        settings: &NavigationSettings,
    ) {
        if response.drag_started() && self.drag.is_none() {
            let keys = ui.input(|i| Modifiers {
                alt: i.modifiers.alt,
                shift: i.modifiers.shift,
            });
            let button = [
                (egui::PointerButton::Primary, MouseButton::Primary),
                (egui::PointerButton::Middle, MouseButton::Middle),
                (egui::PointerButton::Secondary, MouseButton::Secondary),
            ]
            .into_iter()
            .find(|(b, _)| response.drag_started_by(*b))
            .map(|(_, b)| b);
            if let Some(kind) = button.and_then(|b| settings.drag(b, keys)) {
                self.drag = Some(EditorDrag::Navigate(kind));
            }
        }
        if let Some(EditorDrag::Navigate(kind)) = self.drag {
            let delta = response.drag_delta();
            match kind {
                DragKind::Pan | DragKind::Orbit => {
                    self.centre -= Vec2::new(delta.x, -delta.y) / self.zoom;
                }
                DragKind::Dolly => {
                    let factor = (-delta.y * 0.01).exp();
                    self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
                }
            }
            if response.drag_stopped() || !ui.input(|i| i.pointer.any_down()) {
                self.drag = None;
            }
        }
        // The wheel zooms at the mouse (Ctrl + wheel is left alone, as in
        // the viewport, where it grows and shrinks the selection).
        if let Some(at) = response.hover_pos() {
            let (scroll, ctrl) = ui.input(|i| (i.smooth_scroll_delta.y, i.modifiers.command));
            if scroll != 0.0 && !ctrl {
                let before = self.to_uv(rect, at);
                self.zoom = (self.zoom * (scroll * 0.0025).exp()).clamp(MIN_ZOOM, MAX_ZOOM);
                let after = self.to_uv(rect, at);
                self.centre += before - after;
            }
        }
    }
}
