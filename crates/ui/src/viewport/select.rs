//! Selecting in a view with the left mouse button: clicks, double-clicks,
//! and drags with the box, lasso or brush, turned into
//! [`Gesture`]s for the workbench; and the box, lasso or brush drawn
//! while it happens ([decision 0029](../../../../docs/decisions/0029-selection.md)).
//!
//! A drag with the left button selects unless the navigation controls
//! use it (Alt + left drag orbits in some schemes).

use super::View;
use crate::components::ViewPointer;
use crate::selecting::{Gesture, SelectTool, click_combine, drag_combine};
use crate::theme;
use egui::{Color32, PointerButton, Pos2, Rect, Response, Shape, Stroke, Ui, pos2};
use glam::Vec2;
use scene::Combine;
use tools::Region;

/// How far apart (in screen points) the lasso's outline points are kept.
const LASSO_SPACING: f32 = 3.0;

/// A selection drag in progress in a view.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct SelectDrag {
    tool: SelectTool,
    how: Combine,
    /// Where the drag started and where it has been, in the view's own
    /// screen points: the box's corners, the lasso's outline, or the
    /// brush's last dab.
    points: Vec<Vec2>,
    /// This drag's number, so a brush stroke undoes in one go.
    stroke: u64,
    /// Whether the brush has already dabbed (a replacing stroke only
    /// replaces on its first dab, then adds).
    dabbed: bool,
}

/// What a view needs to capture selection gestures this frame.
pub(super) struct Capture<'a> {
    pub(super) tool: SelectTool,
    pub(super) brush_radius: f32,
    /// The last stroke number given out.
    pub(super) strokes: &'a mut u64,
    pub(super) out: &'a mut Vec<Gesture>,
}

impl View {
    /// Turns this frame's left-button input over the view at `rect` into
    /// selection gestures.
    pub(super) fn capture_selection(
        &mut self,
        ui: &Ui,
        response: &Response,
        rect: Rect,
        capture: Capture<'_>,
    ) {
        let keys = ui.input(|i| i.modifiers);
        let size = Vec2::new(rect.width(), rect.height());
        let local = |p: Pos2| Vec2::new(p.x - rect.min.x, p.y - rect.min.y);
        let camera = *self.camera();
        let pointer_at = |p: Pos2| ViewPointer {
            camera,
            size,
            at: local(p),
        };

        if response.drag_started_by(PointerButton::Primary)
            && self.drag.is_none()
            && let Some(start) = ui.input(|i| i.pointer.press_origin())
        {
            *capture.strokes += 1;
            self.select_drag = Some(SelectDrag {
                tool: capture.tool,
                how: drag_combine(keys),
                points: vec![local(start)],
                stroke: *capture.strokes,
                dabbed: false,
            });
        }

        if let Some(drag) = &mut self.select_drag {
            let now = response
                .interact_pointer_pos()
                .or(response.hover_pos())
                .map(local);
            if let Some(now) = now {
                match drag.tool {
                    SelectTool::Box => {
                        drag.points.truncate(1);
                        drag.points.push(now);
                    }
                    SelectTool::Lasso => {
                        if drag
                            .points
                            .last()
                            .is_none_or(|last| last.distance(now) >= LASSO_SPACING)
                        {
                            drag.points.push(now);
                        }
                    }
                    SelectTool::Brush => {
                        let from = *drag.points.last().unwrap_or(&now);
                        let dabs = brush_dabs(from, now, capture.brush_radius, !drag.dabbed);
                        if !dabs.is_empty() {
                            let how = match (drag.how, drag.dabbed) {
                                (Combine::Replace, true) => Combine::Add,
                                (how, _) => how,
                            };
                            capture.out.push(Gesture::Region {
                                camera,
                                size,
                                regions: dabs,
                                how,
                                stroke: Some(drag.stroke),
                            });
                            drag.dabbed = true;
                            drag.points = vec![now];
                        }
                    }
                }
            }
            let released = response.drag_stopped() || !ui.input(|i| i.pointer.primary_down());
            if released {
                let region = match drag.tool {
                    SelectTool::Box if drag.points.len() == 2 => Some(Region::Rect {
                        a: drag.points[0],
                        b: drag.points[1],
                    }),
                    SelectTool::Lasso if drag.points.len() >= 3 => {
                        Some(Region::Lasso(drag.points.clone()))
                    }
                    _ => None,
                };
                if let Some(region) = region {
                    capture.out.push(Gesture::Region {
                        camera,
                        size,
                        regions: vec![region],
                        how: drag.how,
                        stroke: None,
                    });
                }
                self.select_drag = None;
            }
        }

        let clicked_at = response
            .interact_pointer_pos()
            .or_else(|| ui.input(|i| i.pointer.interact_pos()));
        if response.double_clicked_by(PointerButton::Primary) {
            if let Some(at) = clicked_at {
                capture.out.push(Gesture::DoubleClick {
                    pointer: pointer_at(at),
                    ring: keys.shift,
                    how: if keys.command {
                        Combine::Add
                    } else {
                        Combine::Replace
                    },
                });
            }
        } else if response.clicked_by(PointerButton::Primary)
            && let Some(at) = clicked_at
        {
            capture.out.push(Gesture::Click {
                pointer: pointer_at(at),
                how: click_combine(keys),
            });
        }
    }

    /// Draws the box, lasso or brush over the view at `rect`: the drag in
    /// progress, or the brush's circle around the mouse.
    pub(super) fn paint_selection(
        &self,
        ui: &Ui,
        rect: Rect,
        tool: SelectTool,
        brush_radius: f32,
        mouse: Option<Pos2>,
    ) {
        let accent = theme::accent(ui.ctx()).colors();
        let line = Stroke::new(1.0, accent.light);
        let fill = accent.fill.gamma_multiply(0.12);
        let painter = ui.painter_at(rect);
        let screen = |p: Vec2| pos2(rect.min.x + p.x, rect.min.y + p.y);
        match &self.select_drag {
            Some(drag) if drag.tool == SelectTool::Box && drag.points.len() == 2 => {
                let area = Rect::from_two_pos(screen(drag.points[0]), screen(drag.points[1]));
                painter.rect(area, 0.0, fill, line, egui::StrokeKind::Inside);
            }
            Some(drag) if drag.tool == SelectTool::Lasso && drag.points.len() >= 2 => {
                let outline: Vec<Pos2> = drag.points.iter().map(|p| screen(*p)).collect();
                painter.add(Shape::closed_line(outline, line));
            }
            _ => {}
        }
        if tool == SelectTool::Brush
            && self.drag.is_none()
            && let Some(at) = mouse
        {
            painter.circle(at, brush_radius, Color32::TRANSPARENT, line);
        }
    }
}

/// Brush dabs from `from` to `to`, close enough together that a quick
/// stroke leaves no gaps. With `first`, the dab at `from` too. Empty when
/// the mouse hasn't moved far enough for a new dab.
fn brush_dabs(from: Vec2, to: Vec2, radius: f32, first: bool) -> Vec<Region> {
    let radius = radius.max(1.0);
    let spacing = radius / 2.0;
    let distance = from.distance(to);
    let mut dabs = Vec::new();
    if first {
        dabs.push(Region::Circle {
            centre: from,
            radius,
        });
    }
    if distance >= spacing {
        let steps = (distance / spacing).ceil() as usize;
        for i in 1..=steps {
            dabs.push(Region::Circle {
                centre: from.lerp(to, i as f32 / steps as f32),
                radius,
            });
        }
    }
    dabs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brush_dabs_fill_the_gap_between_mouse_positions() {
        assert!(brush_dabs(Vec2::ZERO, Vec2::new(5.0, 0.0), 20.0, false).is_empty());
        let first = brush_dabs(Vec2::ZERO, Vec2::ZERO, 20.0, true);
        assert_eq!(first.len(), 1);
        let long = brush_dabs(Vec2::ZERO, Vec2::new(100.0, 0.0), 20.0, false);
        assert_eq!(long.len(), 10, "every half radius");
        assert_eq!(
            long.last(),
            Some(&Region::Circle {
                centre: Vec2::new(100.0, 0.0),
                radius: 20.0
            })
        );
        // A broken radius still makes sane dabs.
        assert!(!brush_dabs(Vec2::ZERO, Vec2::new(10.0, 0.0), 0.0, true).is_empty());
    }
}
