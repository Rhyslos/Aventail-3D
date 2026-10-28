//! The paint brush in the UV Editor ([decision 0180](../../../../docs/decisions/0180-painting-in-the-uv-editor.md)):
//! in Paint mode the editor takes the same brush as the 3D view, with the
//! same controls (a left drag paints, Ctrl + right drag changes the size
//! and hardness, Ctrl + left drag the flow). Its strokes go to the
//! workbench as places on the texture; the workbench paints them flat.

use super::UvEditor;
use crate::viewport::{BrushCapture, BrushEvent};
use egui::{Pos2, Rect, Response, Ui};
use glam::Vec2;

/// Where the mouse was in the UV Editor, for a brush stroke.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct UvPointer {
    /// The place on the texture (0 to 1 across, V up).
    pub(crate) uv: Vec2,
    /// Screen points per texture width, so the brush's size on screen
    /// can be turned into a size on the texture.
    pub(crate) zoom: f32,
}

/// The brush's state in the UV Editor.
#[derive(Debug, Clone, Default)]
pub(super) struct UvBrush {
    /// The brush's radius in points and its hardness, in Paint mode.
    pub(super) shown: Option<(f32, f32)>,
    pub(super) capture: BrushCapture,
    pub(super) events: Vec<BrushEvent<UvPointer>>,
    /// The place on the texture under the mouse, last frame.
    pub(super) hover: Option<Vec2>,
}

impl UvEditor {
    /// Shows the paint brush (Paint mode) with `radius` points and
    /// `hardness`, or hides it.
    pub(crate) fn set_paint_brush(&mut self, brush: Option<(f32, f32)>) {
        self.brush.shown = brush;
        if brush.is_none() {
            self.brush.hover = None;
        }
    }

    /// The brush's events since last asked.
    pub(crate) fn take_brush_events(&mut self) -> Vec<BrushEvent<UvPointer>> {
        std::mem::take(&mut self.brush.events)
    }

    /// The place on the texture under the mouse while painting, if the
    /// mouse is over the editor.
    pub(crate) fn brush_hover(&self) -> Option<UvPointer> {
        self.brush.hover.map(|uv| UvPointer {
            uv,
            zoom: self.zoom,
        })
    }

    /// Whether a Ctrl + drag is about to change the brush or is changing
    /// it (the editor mustn't take it for moving the view).
    pub(super) fn brush_wants(&self, ui: &Ui, response: &Response) -> bool {
        self.brush.shown.is_some()
            && (self.brush.capture.adjusting() || BrushCapture::wants(ui, response))
    }

    /// Turns the mouse into brush events, in Paint mode.
    pub(super) fn use_brush(&mut self, ui: &Ui, response: &Response, rect: Rect) {
        if self.brush.shown.is_none() {
            self.brush.hover = None;
            return;
        }
        let (centre, zoom) = (self.centre, self.zoom);
        let to_uv = move |at: Pos2| {
            let c = rect.center();
            Vec2::new(
                centre.x + (at.x - c.x) / zoom,
                centre.y - (at.y - c.y) / zoom,
            )
        };
        self.brush.hover = response.hover_pos().map(to_uv);
        if self.drag.is_none() || self.brush.capture.adjusting() {
            self.brush.capture.capture(
                ui,
                response,
                |at| UvPointer {
                    uv: to_uv(at),
                    zoom,
                },
                &mut self.brush.events,
            );
        }
    }

    /// Draws the brush's circle under the mouse, in Paint mode.
    pub(super) fn paint_brush(&self, ui: &Ui, rect: Rect, response: &Response) {
        if let Some((radius, hardness)) = self.brush.shown
            && self.drag.is_none()
        {
            self.brush
                .capture
                .paint(ui, rect, response.hover_pos(), radius, hardness);
        }
    }
}
