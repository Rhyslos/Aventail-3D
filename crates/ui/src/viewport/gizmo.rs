//! The move, rotate and scale gizmo in a view: drawing it, lighting the
//! handle under the mouse, and turning a left drag on a handle into
//! [`GizmoEvent`]s for the workbench ([decision 0030](../../../../docs/decisions/0030-transform-gizmos.md)).
//!
//! A press on a handle belongs to the gizmo: it neither selects nor (if
//! the mouse doesn't move) does anything. A press anywhere else selects,
//! as with the Select tool. While dragging, Shift makes the drag ten times
//! finer and a right click cancels it (Esc too, handled by the panel).

use super::View;
use crate::components::ViewPointer;
use crate::theme::tint;
use egui::{Color32, PointerButton, Pos2, Rect, Response, Shape as Paint, Stroke, Ui, pos2};
use glam::Vec2;
use tools::gizmo::Shape;
use tools::{Change, Drag, Gizmo, Handle};

/// The colour of the handle under the mouse or being dragged.
const HIGHLIGHT: Color32 = Color32::from_rgb(0xff, 0xd8, 0x5a);
/// The colour of handles that belong to no one axis.
const NEUTRAL: Color32 = Color32::from_rgb(0xd8, 0xd8, 0xd8);
/// Line widths, in points.
const WIDTH: f32 = 2.0;
const WIDTH_HIGHLIGHT: f32 = 3.0;
/// The arrowhead's length and half width, in points.
const ARROW_LENGTH: f32 = 12.0;
const ARROW_HALF_WIDTH: f32 = 5.0;

/// Something done with the gizmo in a view, for the workbench.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum GizmoEvent {
    /// A drag of `handle` began; `slide` when Alt was held (Alt + drag
    /// on the Move gizmo slides points along their edges, decision 0055).
    Begin { handle: Handle, slide: bool },
    /// The drag so far, with the mouse at `pointer`.
    Update {
        change: Change,
        handle: Handle,
        pointer: ViewPointer,
    },
    /// The drag ended: keep what it did.
    End,
    /// The drag was cancelled: put everything back.
    Cancel,
}

/// What a view needs to handle the gizmo this frame.
pub(super) struct GizmoInput<'a> {
    pub(super) gizmo: Option<Gizmo>,
    pub(super) locked: [bool; 3],
    pub(super) out: &'a mut Vec<GizmoEvent>,
}

impl View {
    /// Handles the gizmo for this frame's input over the view at `rect`.
    /// Returns whether the gizmo has the left button (so selecting
    /// should leave this input alone).
    pub(super) fn capture_gizmo(
        &mut self,
        ui: &Ui,
        response: &Response,
        rect: Rect,
        input: GizmoInput<'_>,
    ) -> bool {
        let size = Vec2::new(rect.width(), rect.height());
        let local = |p: Pos2| Vec2::new(p.x - rect.min.x, p.y - rect.min.y);
        let camera = *self.camera();
        let (shift, primary_down, primary_pressed, secondary_pressed) = ui.input(|i| {
            (
                i.modifiers.shift,
                i.pointer.primary_down(),
                i.pointer.primary_pressed(),
                i.pointer.secondary_pressed(),
            )
        });

        // A drag in progress.
        if let Some(drag) = &mut self.gizmo_drag {
            if secondary_pressed {
                self.gizmo_drag = None;
                self.pressed_handle = None;
                input.out.push(GizmoEvent::Cancel);
                return true;
            }
            if let Some(at) = response.interact_pointer_pos().or(response.hover_pos()) {
                let change = drag.update(local(at), shift, input.locked);
                input.out.push(GizmoEvent::Update {
                    change,
                    handle: drag.handle,
                    pointer: ViewPointer {
                        camera,
                        size,
                        at: local(at),
                    },
                });
            }
            if !primary_down || response.drag_stopped() {
                self.gizmo_drag = None;
                self.pressed_handle = None;
                input.out.push(GizmoEvent::End);
            }
            return true;
        }

        let Some(gizmo) = input.gizmo else {
            self.hovered_handle = None;
            self.pressed_handle = None;
            return false;
        };
        self.hovered_handle = response
            .hover_pos()
            .filter(|_| self.drag.is_none())
            .and_then(|at| gizmo.handle_at(&camera, size, local(at), input.locked));
        if primary_pressed && response.contains_pointer() {
            self.pressed_handle = ui
                .input(|i| i.pointer.press_origin())
                .and_then(|at| gizmo.handle_at(&camera, size, local(at), input.locked));
        }
        let Some(handle) = self.pressed_handle else {
            return false;
        };
        if response.drag_started_by(PointerButton::Primary)
            && self.drag.is_none()
            && let Some(start) = ui.input(|i| i.pointer.press_origin())
        {
            let mut drag = Drag::new(gizmo, handle, &camera, size, local(start));
            let slide = ui.input(|i| i.modifiers.alt);
            input.out.push(GizmoEvent::Begin { handle, slide });
            if let Some(at) = response.interact_pointer_pos() {
                let change = drag.update(local(at), shift, input.locked);
                input.out.push(GizmoEvent::Update {
                    change,
                    handle,
                    pointer: ViewPointer {
                        camera,
                        size,
                        at: local(at),
                    },
                });
            }
            self.gizmo_drag = Some(drag);
        } else if !primary_down {
            // Pressed on a handle and let go without moving: nothing.
            self.pressed_handle = None;
        }
        true
    }

    /// Whether the gizmo is being dragged in this view.
    pub(super) fn dragging_gizmo(&self) -> bool {
        self.gizmo_drag.is_some()
    }

    /// Draws the gizmo over the view at `rect`.
    pub(super) fn paint_gizmo(&self, ui: &Ui, rect: Rect, gizmo: Option<Gizmo>, locked: [bool; 3]) {
        let Some(gizmo) = gizmo else {
            return;
        };
        let size = Vec2::new(rect.width(), rect.height());
        let painter = ui.painter_at(rect);
        let screen = |p: Vec2| pos2(rect.min.x + p.x, rect.min.y + p.y);
        let active = self
            .gizmo_drag
            .as_ref()
            .map(|d| d.handle)
            .or(self.hovered_handle);
        for (handle, shape) in gizmo.shapes(self.camera(), size, locked) {
            let lit = active == Some(handle);
            let color = if lit {
                HIGHLIGHT
            } else {
                match handle.axis() {
                    Some(0) => tint::AXIS_X,
                    Some(1) => tint::AXIS_Y,
                    Some(2) => tint::AXIS_Z,
                    _ => NEUTRAL,
                }
            };
            let stroke = Stroke::new(if lit { WIDTH_HIGHLIGHT } else { WIDTH }, color);
            match shape {
                Shape::Line { a, b, arrow } => {
                    let (a, b) = (screen(a), screen(b));
                    painter.line_segment([a, b], stroke);
                    if arrow {
                        let along = (b - a).normalized();
                        let side = egui::vec2(-along.y, along.x);
                        let base = b - along * ARROW_LENGTH;
                        painter.add(Paint::convex_polygon(
                            vec![
                                b,
                                base + side * ARROW_HALF_WIDTH,
                                base - side * ARROW_HALF_WIDTH,
                            ],
                            color,
                            Stroke::NONE,
                        ));
                    }
                }
                Shape::Ring(points) => {
                    let points: Vec<Pos2> = points.into_iter().map(screen).collect();
                    painter.add(Paint::closed_line(points, stroke));
                }
                Shape::Quad(corners) => {
                    let corners: Vec<Pos2> = corners.into_iter().map(screen).collect();
                    let fill = color.gamma_multiply(if lit { 0.6 } else { 0.35 });
                    painter.add(Paint::convex_polygon(corners, fill, stroke));
                }
                Shape::Dot { centre, radius } => {
                    let fill = color.gamma_multiply(if lit { 0.9 } else { 0.55 });
                    painter.circle(screen(centre), radius, fill, Stroke::new(1.0, color));
                }
            }
        }
    }
}
