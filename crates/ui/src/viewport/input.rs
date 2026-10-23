//! The viewport's keys and wheel outside navigation drags: Ctrl + wheel
//! for a tool's count, the view keys (F1 to F5, P), framing (H and F),
//! and asking the renderer what is under the mouse.

use super::renderer::{PickAsk, ViewportRenderer, ViewportView};
use super::view::{Frame, UnderMouse};
use super::{Asked, FOUR_VIEWS_KEY, PROJECTION_KEY, VIEW_KEYS, ViewportPanel, pixels_per_point};
use crate::drawing::SceneSummary;
use crate::selecting::{BRUSH_MAX, BRUSH_MIN, BRUSH_STEP, Gesture, SelectTool};
use egui::{CursorIcon, Rect, Ui};
use tools::DragKind;

impl ViewportPanel {
    /// Ctrl + wheel over a view: with the brush, changes its size;
    /// otherwise grows or shrinks the selection, a step for each notch of
    /// the wheel.
    pub(super) fn ctrl_wheel(&mut self, ui: &Ui) {
        if !ui.input(|i| i.modifiers.command) {
            self.ctrl_scroll = 0.0;
            return;
        }
        let scroll = wheel_scroll(ui, 400.0);
        if scroll == 0.0 {
            return;
        }
        // One notch of a mouse wheel; touchpads send many small turns.
        let notch = ui
            .ctx()
            .options(|o| o.input_options.line_scroll_speed)
            .max(1.0);
        self.ctrl_scroll += scroll;
        let steps = (self.ctrl_scroll / notch).trunc();
        if steps == 0.0 {
            return;
        }
        self.ctrl_scroll -= steps * notch;
        if self.wheel_tool {
            // A tool with a count (Bevel's segments, Loop cut's cuts) takes
            // the turns while it is in use (decision 0048).
            self.tool_steps += steps as i32;
        } else if self.select_tool == SelectTool::Brush {
            self.brush_radius =
                (self.brush_radius * BRUSH_STEP.powf(steps)).clamp(BRUSH_MIN, BRUSH_MAX);
        } else {
            self.gestures.push(Gesture::Grow(steps as i32));
        }
    }

    /// Shows one view or four, for keys `F1` to `F5` (unless typing in a
    /// field).
    pub(super) fn views_from_keys(&mut self, ui: &Ui) {
        if ui.ctx().memory(|m| m.focused().is_some()) {
            return;
        }
        for (key, kind) in VIEW_KEYS {
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key)) {
                self.show_alone(kind);
            }
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, FOUR_VIEWS_KEY)) {
            self.options.four_views = true;
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, PROJECTION_KEY)) {
            self.toggle_projection();
        }
    }

    /// Collects the renderer's latest answer about the point under the
    /// mouse, and asks again while the mouse is over a view, so an answer
    /// is ready when an orbit or zoom starts. `hovered` is the view under
    /// the mouse, if any: its slot, place, the mouse position and what was
    /// drawn.
    pub(super) fn ask_what_is_under_the_mouse(
        &mut self,
        ui: &Ui,
        hovered: Option<(usize, Rect, Option<egui::Pos2>, ViewportView)>,
        renderer: &mut dyn ViewportRenderer,
    ) {
        let ppp = pixels_per_point(ui);
        let around_cursor = self.options.navigation.around_cursor;
        let mut ask = None;
        if let Some((slot, rect, Some(at), view)) = hovered
            && around_cursor
        {
            let it = &self.views[slot];
            // One question at a time, and only when the mouse has moved
            // since the last answer, so a resting mouse costs nothing.
            let fresh_needed = it
                .under_mouse
                .is_none_or(|u| u.at.distance(at) > 0.5 || u.camera != *it.camera());
            if it.drag.is_none() && !self.asking && fresh_needed {
                ask = Some(PickAsk {
                    view_projection: view.view_projection,
                    pixel: [(at.x - rect.min.x) * ppp, (at.y - rect.min.y) * ppp],
                    size_px: view.size_px,
                });
                self.asked = Some(Asked {
                    slot,
                    camera: *it.camera(),
                    corner: rect.min,
                });
            }
        }
        let reply = renderer.pick(ask);
        if let (Some(answer), Some(asked)) = (reply.answer, self.asked)
            && let Some(view) = self.views.get_mut(asked.slot)
        {
            view.under_mouse = Some(UnderMouse {
                at: asked.corner + egui::vec2(answer.pixel[0], answer.pixel[1]) / ppp,
                point: answer.point,
                camera: asked.camera,
            });
        }
        self.asking = reply.waiting;
        if reply.waiting {
            ui.ctx().request_repaint();
        }
    }

    /// Starts framing the active view for the `H` and `Z` keys (unless
    /// typing in a field) or a menu entry, and moves every view along a
    /// glide in progress.
    pub(super) fn frame_from_keys_and_menus(&mut self, ui: &Ui, scene: &SceneSummary) {
        let typing = ui.ctx().memory(|m| m.focused().is_some());
        let now = ui.input(|i| i.time);
        let mut wanted = self.frame_request.take();
        if !typing {
            for what in [Frame::All, Frame::Selected] {
                if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, what.key())) {
                    wanted = Some(what);
                }
            }
        }
        if let Some(what) = wanted {
            self.active_view_mut()
                .start_frame(scene.bounds_for(what), now);
        }
        let mut gliding = false;
        for view in &mut self.views {
            gliding |= view.follow_glide(now);
        }
        if gliding {
            ui.ctx().request_repaint();
        }
    }

    /// Applies a drag on one of the menu line's navigation buttons to the
    /// active view.
    pub(super) fn button_drag(&mut self, kind: DragKind, delta: egui::Vec2) {
        let settings = self.options.navigation;
        self.active_view_mut().button_drag(kind, delta, settings);
    }
}

/// The mouse pointer shown while navigating.
pub(super) fn cursor_for(kind: DragKind) -> CursorIcon {
    match kind {
        DragKind::Orbit => CursorIcon::Grabbing,
        DragKind::Pan => CursorIcon::Move,
        DragKind::Dolly => CursorIcon::ZoomIn,
    }
}

/// How far the mouse wheel turned this frame, in points; positive when
/// turned away from you. Read straight from the wheel events rather than
/// egui's smoothed scrolling, so zooming responds at once (DESIGN.md §5.2).
pub(super) fn wheel_scroll(ui: &Ui, page_height: f32) -> f32 {
    let line = ui.ctx().options(|o| o.input_options.line_scroll_speed);
    ui.input(|input| {
        input
            .events
            .iter()
            .map(|event| match event {
                egui::Event::MouseWheel { unit, delta, .. } => match unit {
                    egui::MouseWheelUnit::Point => delta.y,
                    egui::MouseWheelUnit::Line => delta.y * line,
                    egui::MouseWheelUnit::Page => delta.y * page_height,
                },
                _ => 0.0,
            })
            .sum::<f32>()
    })
}
