//! Drawing the canvas, back to front: the background, comment frames
//! (biggest first, so frames inside frames show), wires coloured by what
//! they carry, boxes with their live pictures and plugs, then whatever is
//! being dragged (decision 0258, after the mockups the project lead
//! chose).

use super::MaterialEditor;
use super::canvas::{BACKGROUND, BoxPlace, Places, badge_rect, title_height};
use super::drags::{Drag, Target};
use super::look::{self, FITS, REFUSED, kind_colour};
use crate::theme;
use egui::epaint::CubicBezierShape;
use egui::{Align2, Color32, Context, FontId, Painter, Pos2, Rect, Stroke, StrokeKind};
use matgraph::{BoxId, Graph, PlugRef};
use std::collections::BTreeSet;

/// A box's body.
const BODY: Color32 = Color32::from_rgb(0x2b, 0x2b, 0x2b);
/// A box's title band.
const BODY_HEADER: Color32 = Color32::from_rgb(0x34, 0x34, 0x34);
/// The background's faint lines.
const GRID: Color32 = Color32::from_rgb(0x26, 0x26, 0x26);
/// How far apart the background's lines are, on the canvas.
const GRID_STEP: f32 = 80.0;

/// A wire from an output at `from` to an input at `to`, curving out of
/// the one and into the other.
pub(crate) fn wire_shape(from: Pos2, to: Pos2, zoom: f32, colour: Color32) -> CubicBezierShape {
    let reach = ((to.x - from.x).abs() * 0.5).max(40.0 * zoom);
    let width = (2.5 * zoom).clamp(1.2, 4.0);
    CubicBezierShape::from_points_stroke(
        [
            from,
            from + egui::vec2(reach, 0.0),
            to - egui::vec2(reach, 0.0),
            to,
        ],
        false,
        Color32::TRANSPARENT,
        Stroke::new(width, colour),
    )
}

impl MaterialEditor {
    /// Draws the canvas.
    pub(super) fn draw(
        &mut self,
        ctx: &Context,
        painter: &Painter,
        rect: Rect,
        graph: &Graph,
        places: &Places,
    ) {
        painter.rect_filled(rect, 0.0, BACKGROUND);
        self.draw_grid(painter, rect);
        self.draw_frames(painter, graph, places, ctx);
        let detached = match &self.canvas.drag {
            Some(Drag::Wire { detached, .. }) => detached.clone(),
            _ => None,
        };
        let zoom = self.canvas.view.zoom;
        for wire in graph.wires() {
            if detached.as_ref() == Some(wire) {
                continue;
            }
            let (Some(from), Some(to)) =
                (places.plug(&wire.from, true), places.plug(&wire.to, false))
            else {
                continue;
            };
            painter.add(wire_shape(from.at, to.at, zoom, kind_colour(from.kind)));
        }
        let problems: BTreeSet<BoxId> = self.problems(graph).into_keys().collect();
        let accent = theme::accent(ctx).colors();
        for b in &places.boxes {
            let texture = self.pictures.texture(ctx, b.id);
            let state = BoxState {
                selected: self.canvas.selected.contains(&b.id),
                problem: problems.contains(&b.id),
                looked_at: self.canvas.looking_at == Some(b.id),
                accent: accent.light,
            };
            draw_box(
                painter,
                b,
                graph,
                texture,
                state,
                zoom,
                self.pictures.waiting(),
            );
        }
        self.draw_drag(painter, places, accent.light);
    }

    /// Faint lines across the background, when not zoomed far out.
    fn draw_grid(&self, painter: &Painter, rect: Rect) {
        let view = &self.canvas.view;
        let step = GRID_STEP * view.zoom;
        if step < 16.0 {
            return;
        }
        let origin = view.on_screen(rect, glam::Vec2::ZERO);
        let first_x = origin.x - ((origin.x - rect.left()) / step).floor() * step;
        let first_y = origin.y - ((origin.y - rect.top()) / step).floor() * step;
        let stroke = Stroke::new(1.0, GRID);
        let mut x = first_x;
        while x < rect.right() {
            painter.vline(x, rect.y_range(), stroke);
            x += step;
        }
        let mut y = first_y;
        while y < rect.bottom() {
            painter.hline(rect.x_range(), y, stroke);
            y += step;
        }
    }

    /// Comment frames, biggest first; their titles stay readable however
    /// far out the canvas is zoomed.
    fn draw_frames(&self, painter: &Painter, graph: &Graph, places: &Places, ctx: &Context) {
        let zoom = self.canvas.view.zoom;
        let accent = theme::accent(ctx).colors();
        for place in &places.frames {
            let Some(frame) = graph.frame(place.id) else {
                continue;
            };
            let [r, g, b] = frame.colour;
            let corner = 6.0 * zoom.min(1.0);
            painter.rect_filled(
                place.rect,
                corner,
                Color32::from_rgba_unmultiplied(r, g, b, 34),
            );
            let title_rect = Rect::from_min_size(
                place.rect.min,
                egui::vec2(place.rect.width(), title_height(zoom)),
            );
            painter.rect_filled(
                title_rect,
                egui::CornerRadius {
                    nw: corner as u8,
                    ne: corner as u8,
                    sw: 0,
                    se: 0,
                },
                Color32::from_rgba_unmultiplied(r, g, b, 120),
            );
            let selected = self.canvas.frame == Some(place.id);
            let stroke = if selected {
                Stroke::new(2.0, accent.light)
            } else {
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(r, g, b, 170))
            };
            painter.rect_stroke(place.rect, corner, stroke, StrokeKind::Inside);
            let renaming = self
                .canvas
                .renaming
                .as_ref()
                .is_some_and(|(id, ..)| *id == place.id);
            if !renaming {
                // Readable zoomed out: never smaller than 13 points.
                let size = (14.0 * zoom).clamp(13.0, 26.0);
                painter.text(
                    title_rect.left_center() + egui::vec2(8.0, 0.0),
                    Align2::LEFT_CENTER,
                    &frame.title,
                    FontId::proportional(size),
                    theme::TEXT_STRONG,
                );
            }
        }
    }

    /// The wire or box being drawn by the pointer.
    fn draw_drag(&self, painter: &Painter, places: &Places, accent: Color32) {
        let zoom = self.canvas.view.zoom;
        match &self.canvas.drag {
            Some(Drag::Wire {
                end,
                target,
                pointer,
                ..
            }) => {
                let (plug, kind, output) = end.plug();
                let Some(fixed) = places.plug(plug, output) else {
                    return;
                };
                let refused = matches!(target, Some(Target { fits: Err(_), .. }));
                let colour = if refused { REFUSED } else { kind_colour(kind) };
                // Snapped onto the plug it is over.
                let loose = target
                    .as_ref()
                    .and_then(|t| {
                        let other: &PlugRef = if output { &t.to } else { &t.from };
                        places.plug(other, !output)
                    })
                    .map_or(*pointer, |p| p.at);
                let (from, to) = if output {
                    (fixed.at, loose)
                } else {
                    (loose, fixed.at)
                };
                painter.add(wire_shape(from, to, zoom, colour));
                if target.is_some() {
                    let radius = (look::PLUG_RADIUS * zoom).clamp(3.0, 8.0) + 3.0;
                    let ring = if refused { REFUSED } else { FITS };
                    if refused {
                        painter.circle_filled(loose, radius, REFUSED);
                    }
                    painter.circle_stroke(loose, radius + 1.0, Stroke::new(2.0, ring));
                }
            }
            Some(Drag::Select { from, to, .. }) => {
                let band = Rect::from_two_pos(*from, *to);
                painter.rect_filled(band, 2.0, accent.gamma_multiply(0.12));
                painter.rect_stroke(band, 2.0, Stroke::new(1.0, accent), StrokeKind::Inside);
            }
            _ => {}
        }
    }
}

/// How a box is marked.
#[derive(Debug, Clone, Copy)]
struct BoxState {
    selected: bool,
    problem: bool,
    looked_at: bool,
    accent: Color32,
}

/// One box: its body, title, live picture, plugs and marks.
fn draw_box(
    painter: &Painter,
    b: &BoxPlace,
    graph: &Graph,
    texture: Option<egui::TextureId>,
    state: BoxState,
    zoom: f32,
    working: bool,
) {
    let corner = (6.0 * zoom).clamp(2.0, 6.0);
    painter.rect_filled(b.rect, corner, BODY);
    let header = Rect::from_min_size(b.rect.min, egui::vec2(b.rect.width(), look::HEADER * zoom));
    painter.rect_filled(
        header,
        egui::CornerRadius {
            nw: corner as u8,
            ne: corner as u8,
            sw: 0,
            se: 0,
        },
        BODY_HEADER,
    );
    if let Some(kind) = b.makes {
        painter.hline(
            header.x_range(),
            header.bottom(),
            Stroke::new((2.0 * zoom).max(1.0), kind_colour(kind)),
        );
    }
    if zoom >= 0.3 {
        painter.with_clip_rect(header.shrink(2.0)).text(
            header.left_center() + egui::vec2(6.0 * zoom, 0.0),
            Align2::LEFT_CENTER,
            &b.name,
            FontId::proportional((12.0 * zoom).max(6.0)),
            theme::TEXT_STRONG,
        );
    }
    match texture {
        Some(texture) => {
            let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
            painter.image(texture, b.picture, uv, Color32::WHITE);
        }
        None => {
            painter.rect_filled(b.picture, 2.0, Color32::from_rgb(0x1a, 0x1a, 0x1a));
            if working && !state.problem && zoom >= 0.3 {
                painter.text(
                    b.picture.center(),
                    Align2::CENTER_CENTER,
                    "…",
                    FontId::proportional(16.0 * zoom),
                    theme::TEXT_WEAK,
                );
            }
        }
    }
    if state.looked_at {
        painter.rect_stroke(
            b.picture,
            2.0,
            Stroke::new(2.0, state.accent),
            StrokeKind::Outside,
        );
    }
    for p in &b.plugs {
        let connected = if p.output {
            graph.wires().iter().any(|w| w.from == p.plug)
        } else {
            graph.wire_into(&p.plug).is_some()
        };
        let radius = (look::PLUG_RADIUS * zoom).clamp(2.5, 7.0);
        let colour = kind_colour(p.kind);
        if connected {
            painter.circle_filled(p.at, radius, colour);
        } else {
            painter.circle_filled(p.at, radius, BACKGROUND);
            painter.circle_stroke(p.at, radius, Stroke::new(1.5, colour));
        }
        if zoom >= 0.55 {
            let (anchor, offset) = if p.output {
                (Align2::RIGHT_CENTER, -8.0 * zoom)
            } else {
                (Align2::LEFT_CENTER, 8.0 * zoom)
            };
            let colour = if p.needed && !connected {
                theme::TEXT
            } else {
                theme::TEXT_WEAK
            };
            painter.text(
                p.at + egui::vec2(offset, 0.0),
                anchor,
                &p.name,
                FontId::proportional(10.5 * zoom),
                colour,
            );
        }
    }
    let border = if state.selected {
        Stroke::new(2.0, state.accent)
    } else if state.problem {
        Stroke::new(1.5, REFUSED)
    } else {
        Stroke::new(1.0, theme::BORDER)
    };
    painter.rect_stroke(b.rect, corner, border, StrokeKind::Outside);
    if state.problem {
        let badge = badge_rect(b.rect);
        painter.circle_filled(badge.center(), badge.width() * 0.32, REFUSED);
        painter.text(
            badge.center(),
            Align2::CENTER_CENTER,
            "!",
            FontId::proportional((12.0 * zoom).max(7.0)),
            Color32::WHITE,
        );
    }
}
