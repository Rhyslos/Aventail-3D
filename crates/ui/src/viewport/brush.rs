//! The paint brush in a view ([decision 0179](../../../../docs/decisions/0179-the-round-brush.md)):
//! in Paint mode a left press paints and a left drag carries on painting,
//! turned into [`BrushEvent`]s for the workbench; the brush's circle is
//! drawn under the mouse. The UV Editor uses the same capture, with places
//! on the texture instead of in a view (decision 0180).
//!
//! The controls are Substance Painter's (the project lead's choice):
//! **Ctrl + right drag** changes the size (sideways) and the hardness (up
//! and down); **Ctrl + left drag** changes the flow (sideways). Navigation
//! keeps its own buttons (Alt + drag, right and middle drag in the
//! schemes that use them).

use crate::components::ViewPointer;
use crate::theme;
use egui::{Color32, PointerButton, Pos2, Rect, Response, Stroke, Ui};

/// Something the brush did in a view, for the workbench: `P` is where
/// the mouse was (in a 3D view, the view and the mouse in it; in the UV
/// Editor, the place on the texture).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum BrushEvent<P = ViewPointer> {
    /// A stroke began with the mouse at `pointer`.
    Begin(P),
    /// The stroke went on to `pointer`.
    Move(P),
    /// The stroke ended.
    End,
    /// Ctrl + drag changed the brush: its size by a factor, its hardness
    /// and flow by amounts (0 to 1 scale).
    Adjust {
        /// The size multiplied by this.
        size: f32,
        /// Added to the hardness.
        hardness: f32,
        /// Added to the flow.
        flow: f32,
    },
}

/// How far a Ctrl + drag must move to change the hardness or flow by all
/// of its range, in points.
const ADJUST_RANGE: f32 = 300.0;

/// The brush's state in the views.
#[derive(Debug, Clone, Default)]
pub(crate) struct BrushCapture {
    /// A stroke is under way.
    painting: bool,
    /// A Ctrl + drag is changing the brush: with which button, and where
    /// the mouse was last.
    adjusting: Option<(PointerButton, Pos2)>,
    /// Where an adjustment began, to draw the brush there.
    adjust_origin: Option<Pos2>,
}

impl BrushCapture {
    /// Whether a Ctrl + drag is changing the brush (navigation keeps out).
    pub(crate) fn adjusting(&self) -> bool {
        self.adjusting.is_some()
    }

    /// Whether a Ctrl + drag is about to start (the view mustn't take it
    /// for navigating).
    pub(crate) fn wants(ui: &Ui, response: &Response) -> bool {
        ui.input(|i| i.modifiers.command)
            && (response.drag_started_by(PointerButton::Secondary)
                || response.drag_started_by(PointerButton::Primary))
    }

    /// Turns the mouse over `response` into brush events, with places
    /// from `pointer_at`.
    pub(crate) fn capture<P>(
        &mut self,
        ui: &Ui,
        response: &Response,
        pointer_at: impl Fn(Pos2) -> P,
        out: &mut Vec<BrushEvent<P>>,
    ) {
        let (ctrl, alt) = ui.input(|i| (i.modifiers.command, i.modifiers.alt));
        // Ctrl + drag: change the brush.
        if Self::wants(ui, response) && self.adjusting.is_none() && !self.painting {
            let button = if response.drag_started_by(PointerButton::Secondary) {
                PointerButton::Secondary
            } else {
                PointerButton::Primary
            };
            if let Some(at) = ui.input(|i| i.pointer.press_origin()) {
                self.adjusting = Some((button, at));
                self.adjust_origin = Some(at);
            }
        }
        if let Some((button, last)) = self.adjusting {
            match response.interact_pointer_pos() {
                Some(now) if response.dragged_by(button) => {
                    let d = now - last;
                    let event = match button {
                        PointerButton::Secondary => BrushEvent::Adjust {
                            size: (d.x / 150.0).exp(),
                            hardness: -d.y / ADJUST_RANGE,
                            flow: 0.0,
                        },
                        _ => BrushEvent::Adjust {
                            size: 1.0,
                            hardness: 0.0,
                            flow: d.x / ADJUST_RANGE,
                        },
                    };
                    if d != egui::Vec2::ZERO {
                        out.push(event);
                    }
                    self.adjusting = Some((button, now));
                }
                _ => {
                    self.adjusting = None;
                    self.adjust_origin = None;
                }
            }
            return;
        }
        // A left press paints at once; holding it paints on.
        let pressed = ui.input(|i| i.pointer.primary_pressed());
        if !self.painting && pressed && !ctrl && !alt && response.contains_pointer() {
            if let Some(at) = ui.input(|i| i.pointer.interact_pos()) {
                self.painting = true;
                out.push(BrushEvent::Begin(pointer_at(at)));
            }
            return;
        }
        if self.painting {
            let down = ui.input(|i| i.pointer.primary_down());
            if let Some(at) = ui.input(|i| i.pointer.latest_pos())
                && ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO)
            {
                out.push(BrushEvent::Move(pointer_at(at)));
            }
            if !down {
                self.painting = false;
                out.push(BrushEvent::End);
            }
        }
    }

    /// Draws the brush's circle of `radius` points under the mouse (or
    /// where a Ctrl + drag began), with its hard part inside.
    pub(crate) fn paint(
        &self,
        ui: &Ui,
        rect: Rect,
        hover: Option<Pos2>,
        radius: f32,
        hardness: f32,
    ) {
        let Some(at) = self.adjust_origin.or(hover) else {
            return;
        };
        if !rect.contains(at) {
            return;
        }
        let painter = ui.painter_at(rect);
        let light = Color32::from_white_alpha(220);
        painter.circle_stroke(
            at,
            radius + 1.0,
            Stroke::new(1.0, Color32::from_black_alpha(140)),
        );
        painter.circle_stroke(at, radius, Stroke::new(1.0, light));
        let inner = radius * hardness.clamp(0.0, 1.0);
        if inner > 2.0 && inner < radius - 2.0 {
            painter.circle_stroke(
                at,
                inner,
                Stroke::new(1.0, theme::TEXT_WEAK.gamma_multiply(0.8)),
            );
        }
        painter.circle_filled(at, 1.0, light);
    }
}

impl super::View {
    /// Whether the view is being navigated (orbit, pan, dolly).
    pub(super) fn navigating(&self) -> bool {
        self.drag.is_some()
    }
}
