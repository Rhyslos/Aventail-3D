//! Dragging anywhere in a view to set a modelling tool's amount (extrude
//! and inset, [decision 0038](../../../../docs/decisions/0038-extrude-and-inset.md)).
//!
//! The controls, chosen by the project lead: with Extrude or Inset in use,
//! a left drag anywhere in a view sets the distance, right for more and
//! left for less (or the other way). A plain click still selects, so
//! polygons can be picked without leaving the tool. Shift makes the drag
//! ten times finer; a right click (or Esc, handled by the panel) cancels.

use super::View;
use crate::components::ViewPointer;
use crate::theme;
use egui::{FontId, PointerButton, Pos2, Rect, Response, Ui, vec2};
use glam::Vec2;

/// How much finer a drag is with Shift held.
const FINE: f32 = 0.1;

/// Something done with a modelling tool's drag in a view, for the
/// workbench.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ToolDragEvent {
    /// A drag began, with the mouse at `pointer`.
    Begin { pointer: ViewPointer },
    /// The drag so far: how far it has gone sideways, in screen points
    /// (right is positive; Shift counts a tenth).
    Update { amount: f32, pointer: ViewPointer },
    /// The drag ended: keep what it did.
    End,
    /// The drag was cancelled: put everything back.
    Cancel,
}

/// A modelling tool's drag in progress in a view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ToolDrag {
    /// How far it has gone, in screen points.
    amount: f32,
    /// Where the mouse was last frame, across the view.
    last_x: f32,
}

impl View {
    /// Handles a modelling tool's drag for this frame's input over the
    /// view at `rect`. `active`: whether a tool that drags is in use.
    /// Returns whether the drag has the left button (so selecting should
    /// leave this input alone).
    pub(super) fn capture_tool_drag(
        &mut self,
        ui: &Ui,
        response: &Response,
        rect: Rect,
        active: bool,
        out: &mut Vec<ToolDragEvent>,
    ) -> bool {
        let size = Vec2::new(rect.width(), rect.height());
        let camera = *self.camera();
        let pointer = |p: Pos2| ViewPointer {
            camera,
            size,
            at: Vec2::new(p.x - rect.min.x, p.y - rect.min.y),
        };
        let (shift, primary_down, secondary_pressed) = ui.input(|i| {
            (
                i.modifiers.shift,
                i.pointer.primary_down(),
                i.pointer.secondary_pressed(),
            )
        });
        if let Some(drag) = &mut self.tool_drag {
            if secondary_pressed {
                self.tool_drag = None;
                out.push(ToolDragEvent::Cancel);
                return true;
            }
            if let Some(at) = response.interact_pointer_pos().or(response.hover_pos()) {
                let moved = at.x - drag.last_x;
                drag.last_x = at.x;
                drag.amount += if shift { moved * FINE } else { moved };
                out.push(ToolDragEvent::Update {
                    amount: drag.amount,
                    pointer: pointer(at),
                });
            }
            if !primary_down || response.drag_stopped() {
                self.tool_drag = None;
                out.push(ToolDragEvent::End);
            }
            return true;
        }
        if !active {
            return false;
        }
        if response.drag_started_by(PointerButton::Primary)
            && self.drag.is_none()
            && let Some(start) = ui.input(|i| i.pointer.press_origin())
        {
            out.push(ToolDragEvent::Begin {
                pointer: pointer(start),
            });
            let now = response.interact_pointer_pos().unwrap_or(start);
            let moved = now.x - start.x;
            let amount = if shift { moved * FINE } else { moved };
            out.push(ToolDragEvent::Update {
                amount,
                pointer: pointer(now),
            });
            self.tool_drag = Some(ToolDrag {
                amount,
                last_x: now.x,
            });
            return true;
        }
        false
    }

    /// Whether a modelling tool is being dragged in this view.
    pub(super) fn dragging_tool(&self) -> bool {
        self.tool_drag.is_some()
    }

    /// Writes `label` (the tool and its amount) beside the mouse while a
    /// modelling tool is dragged in this view.
    pub(super) fn paint_tool_label(&self, ui: &Ui, rect: Rect, label: Option<&str>) {
        let (Some(label), true) = (label, self.dragging_tool()) else {
            return;
        };
        let Some(at) = ui.input(|i| i.pointer.latest_pos()) else {
            return;
        };
        let painter = ui.painter_at(rect);
        let font = FontId::proportional(13.0);
        let galley = painter.layout_no_wrap(label.to_string(), font, theme::TEXT_STRONG);
        let place = at + vec2(16.0, 12.0);
        let back = Rect::from_min_size(place, galley.size()).expand2(vec2(6.0, 3.0));
        painter.rect_filled(back, 4.0, theme::PANEL.gamma_multiply(0.92));
        painter.galley(place, galley, theme::TEXT_STRONG);
    }
}
