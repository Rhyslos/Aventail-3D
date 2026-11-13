//! Drawing the shape canvas on screen: the texture square and the copies
//! round it (a shape crossing an edge comes back on the other side), the
//! reference image underneath, the filled shape, each piece's outline,
//! the mirror line, the gizmo, the pen line and the points being edited.

use super::geometry::{gizmo_parts, middle_of};
use super::pointer::{ARROW, Gizmo, Part, RING, ShapeDrag};
use super::{ShapeTool, ShapeView};
use crate::material::MaterialEditor;
use crate::material::look::REFUSED;
use crate::theme::{self, tint};
use egui::{
    Color32, ColorImage, Context, Painter, Pos2, Rect, Stroke, TextureHandle, TextureOptions,
};
use glam::Vec2;
use shapes::{Drawing, Knot, Path, PieceKind};
use std::sync::Arc;

/// The filled shape's picture on the canvas is this many pixels across.
const MASK_SIDE: u32 = 256;
/// How many straight steps each curved side is drawn with.
const STEPS: usize = 24;
/// The colour of chosen pieces and points.
const CHOSEN: Color32 = theme::WARNING;

/// The canvas's textures, each with what it was made from.
#[derive(Default)]
pub(crate) struct Textures {
    mask: Option<(u64, TextureHandle)>,
    reference: Option<(usize, TextureHandle)>,
    pub(crate) strip: Option<(u64, [TextureHandle; 4])>,
}

impl std::fmt::Debug for Textures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Textures")
            .field("mask", &self.mask.as_ref().map(|m| m.0))
            .field("strip", &self.strip.as_ref().map(|s| s.0))
            .finish()
    }
}

/// Greys as a picture: white where 1, black where 0.
pub(crate) fn grey_image(side: u32, pixels: &[f32]) -> ColorImage {
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    let colours = pixels
        .iter()
        .map(|v| Color32::from_gray(byte(*v)))
        .collect();
    ColorImage::new([side as usize, side as usize], colours)
}

/// Colours (sRGB, 0 to 1) as a picture.
pub(crate) fn colour_image(side: u32, pixels: &[[f32; 3]]) -> ColorImage {
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    let colours = pixels
        .iter()
        .map(|[r, g, b]| Color32::from_rgb(byte(*r), byte(*g), byte(*b)))
        .collect();
    ColorImage::new([side as usize, side as usize], colours)
}

/// Points along `path` on screen, closed ones back to their start.
fn screen_line(path: &Path, view: &ShapeView, area: Rect) -> Vec<Pos2> {
    let knots = path.knots();
    let mut out = Vec::new();
    let Some(first) = knots.first() else {
        return out;
    };
    out.push(view.on_screen(area, first.at));
    for i in 0..path.sides() {
        let (a, b) = (knots[i], knots[(i + 1) % knots.len()]);
        if a.out == Vec2::ZERO && b.inn == Vec2::ZERO {
            out.push(view.on_screen(area, b.at));
            continue;
        }
        let (p0, p1, p2, p3) = (a.at, a.at + a.out, b.at + b.inn, b.at);
        for s in 1..=STEPS {
            let t = s as f32 / STEPS as f32;
            let u = 1.0 - t;
            let p = p0 * (u * u * u)
                + p1 * (3.0 * u * u * t)
                + p2 * (3.0 * u * t * t)
                + p3 * (t * t * t);
            out.push(view.on_screen(area, p));
        }
    }
    out
}

impl MaterialEditor {
    /// Draws the shape canvas's drawing area.
    pub(crate) fn paint_shape_canvas(
        &mut self,
        ctx: &Context,
        painter: &Painter,
        area: Rect,
        drawing: &Arc<Drawing>,
        pointer: Option<Pos2>,
    ) {
        painter.rect_filled(area, 0.0, crate::material::canvas::BACKGROUND);
        let Some(s) = self.shape.as_mut() else { return };
        let view = s.view;
        let square = Rect::from_two_pos(
            view.on_screen(area, Vec2::ZERO),
            view.on_screen(area, Vec2::ONE),
        );
        let side = square.width();
        // The copies round the square, faintly: what wraps round shows.
        for dy in -1..=1 {
            for dx in -1..=1 {
                let r = square.translate(egui::vec2(dx as f32 * side, dy as f32 * side));
                painter.rect_filled(
                    r,
                    0.0,
                    theme::UV_SQUARE.gamma_multiply(if dx == 0 && dy == 0 { 1.0 } else { 0.45 }),
                );
            }
        }
        let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        if let Some(r) = drawing.reference() {
            let ptr = Arc::as_ptr(r) as usize;
            if s.textures.reference.as_ref().is_none_or(|t| t.0 != ptr) {
                let image = ColorImage::from_rgba_unmultiplied(
                    [r.width() as usize, r.height() as usize],
                    r.rgba(),
                );
                let handle = ctx.load_texture("shape reference", image, TextureOptions::LINEAR);
                s.textures.reference = Some((ptr, handle));
            }
            if let Some((_, t)) = &s.textures.reference {
                painter.image(
                    t.id(),
                    square,
                    uv,
                    Color32::from_white_alpha((r.opacity * 255.0) as u8),
                );
            }
        }
        let key = drawing.key();
        if s.textures.mask.as_ref().is_none_or(|t| t.0 != key) {
            // Each piece in its colour, seen through the shape's cover.
            let layers = shapes::raster::layers(drawing, MASK_SIDE);
            let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
            let colours = layers
                .mask
                .iter()
                .zip(&layers.colours)
                .map(|(cover, rgb)| {
                    let un = |v: f32| if *cover > 1e-4 { v / cover } else { 0.0 };
                    Color32::from_rgba_unmultiplied(
                        byte(un(rgb[0])),
                        byte(un(rgb[1])),
                        byte(un(rgb[2])),
                        byte(*cover),
                    )
                })
                .collect();
            let image = ColorImage::new([MASK_SIDE as usize; 2], colours);
            let handle = ctx.load_texture("shape mask", image, TextureOptions::LINEAR);
            s.textures.mask = Some((key, handle));
        }
        if let Some((_, t)) = &s.textures.mask {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let r = square.translate(egui::vec2(dx as f32 * side, dy as f32 * side));
                    let tint = if dx == 0 && dy == 0 { 150 } else { 45 };
                    painter.image(t.id(), r, uv, Color32::from_white_alpha(tint));
                }
            }
        }
        painter.rect_stroke(
            square,
            0.0,
            Stroke::new(1.0, theme::UV_SQUARE_EDGE),
            egui::StrokeKind::Outside,
        );
        self.paint_pieces(painter, area, drawing);
        self.paint_helpers(painter, area, drawing, pointer);
    }

    /// Each piece's outline: added pieces light, cut-outs red, lines blue;
    /// chosen ones amber; mirror images fainter.
    fn paint_pieces(&self, painter: &Painter, area: Rect, drawing: &Drawing) {
        let Some(s) = self.shape.as_ref() else { return };
        for piece in drawing.pieces() {
            let colour = match piece.kind {
                PieceKind::Add => theme::TEXT_STRONG,
                PieceKind::Cut => REFUSED,
                PieceKind::Line => tint::AXIS_Y,
            };
            let chosen = s.selected.contains(&piece.id);
            let (colour, width) = if chosen { (CHOSEN, 2.0) } else { (colour, 1.2) };
            for (i, path) in drawing.drawn_paths(piece).iter().enumerate() {
                let mut line = screen_line(path, &s.view, area);
                if path.is_closed()
                    && let Some(first) = line.first().copied()
                {
                    line.push(first);
                }
                let faint = if i == 0 { 1.0 } else { 0.45 };
                painter.add(egui::Shape::line(
                    line,
                    Stroke::new(width, colour.gamma_multiply(faint)),
                ));
            }
        }
    }

    /// The mirror line, the points being edited, the pen line, the gizmo
    /// and a box being drawn round pieces.
    fn paint_helpers(
        &self,
        painter: &Painter,
        area: Rect,
        drawing: &Drawing,
        pointer: Option<Pos2>,
    ) {
        let Some(s) = self.shape.as_ref() else { return };
        let view = s.view;
        let screen = |p: Vec2| view.on_screen(area, p);
        let mirror = drawing.mirror();
        if mirror.on {
            let (top, bottom) = (
                screen(Vec2::new(mirror.x, 0.0)),
                screen(Vec2::new(mirror.x, 1.0)),
            );
            let ends = [top - egui::vec2(0.0, 20.0), bottom + egui::vec2(0.0, 20.0)];
            painter.extend(egui::Shape::dashed_line(
                &ends,
                Stroke::new(1.5, tint::AXIS_Z),
                8.0,
                5.0,
            ));
            painter.circle_filled(ends[0], 5.0, tint::AXIS_Z);
        }
        if let Some(piece) = s.editing.and_then(|id| drawing.piece(id)) {
            for (i, k) in piece.placed().knots().iter().enumerate() {
                paint_knot(painter, &screen, *k, s.point == Some(i));
            }
        }
        if let Some(pen) = &s.pen {
            let colour = if pen.cut { REFUSED } else { CHOSEN };
            let path = Path::new(pen.knots.clone(), false);
            let mut line = screen_line(&path, &view, area);
            if pen.shaping.is_none()
                && let Some(at) = pointer.filter(|p| area.contains(*p))
            {
                line.push(at);
            }
            painter.add(egui::Shape::line(line, Stroke::new(1.5, colour)));
            for (i, k) in pen.knots.iter().enumerate() {
                paint_knot(painter, &screen, *k, false);
                if i == 0 && pen.knots.len() >= 2 {
                    // Click here to close.
                    painter.circle_stroke(
                        screen(k.at),
                        super::pointer::REACH,
                        Stroke::new(1.5, colour),
                    );
                }
            }
        }
        if let Some(gizmo) = s.tool.gizmo()
            && s.editing.is_none()
            && let Some(m) = middle_of(drawing, s.selected.iter().copied())
        {
            paint_gizmo(painter, gizmo, screen(m));
        }
        if let Some(ShapeDrag::Select { from, to, .. }) = &s.drag {
            let r = Rect::from_two_pos(*from, *to);
            painter.rect_filled(r, 0.0, CHOSEN.gamma_multiply(0.08));
            painter.rect_stroke(r, 0.0, Stroke::new(1.0, CHOSEN), egui::StrokeKind::Inside);
        }
        if s.tool == ShapeTool::Pen && s.pen.is_none() && s.editing.is_none() && drawing.is_empty()
        {
            painter.text(
                area.center_bottom() - egui::vec2(0.0, 16.0),
                egui::Align2::CENTER_BOTTOM,
                crate::text::tr(
                    "Click for a corner, drag for a curve; click the first point to close.",
                ),
                egui::FontId::proportional(13.0),
                theme::TEXT_WEAK,
            );
        }
    }
}

/// A point (a square, filled when chosen) and its handles.
fn paint_knot(painter: &Painter, screen: &impl Fn(Vec2) -> Pos2, k: Knot, chosen: bool) {
    let at = screen(k.at);
    for h in [k.inn, k.out] {
        if h != Vec2::ZERO {
            let end = screen(k.at + h);
            painter.line_segment([at, end], Stroke::new(1.0, theme::TEXT_WEAK));
            painter.circle_filled(end, 3.5, theme::TEXT_WEAK);
        }
    }
    let r = Rect::from_center_size(at, egui::vec2(8.0, 8.0));
    if chosen {
        painter.rect_filled(r, 0.0, CHOSEN);
    } else {
        painter.rect_filled(r, 0.0, theme::FIELD);
        painter.rect_stroke(r, 0.0, Stroke::new(1.5, CHOSEN), egui::StrokeKind::Inside);
    }
}

/// The gizmo round the middle `m`: arrows (X red, Y blue, the app's
/// colours) to move, squares to size, a ring to turn.
fn paint_gizmo(painter: &Painter, gizmo: Gizmo, m: Pos2) {
    for (part, spot) in gizmo_parts(gizmo, m) {
        let colour = match part {
            Part::MoveX | Part::ScaleX => tint::AXIS_X,
            Part::MoveY | Part::ScaleY => tint::AXIS_Y,
            Part::Ring => theme::TEXT,
        };
        match part {
            Part::Ring => {
                painter.circle_stroke(m, RING, Stroke::new(2.0, colour));
            }
            Part::MoveX | Part::MoveY => {
                painter.line_segment([m, spot], Stroke::new(2.0, colour));
                let dir = (spot - m) / ARROW;
                let side = egui::vec2(-dir.y, dir.x) * 5.0;
                painter.add(egui::Shape::convex_polygon(
                    vec![spot + dir * 9.0, spot + side, spot - side],
                    colour,
                    Stroke::NONE,
                ));
            }
            Part::ScaleX | Part::ScaleY => {
                painter.line_segment([m, spot], Stroke::new(1.0, colour.gamma_multiply(0.6)));
                painter.rect_filled(
                    Rect::from_center_size(spot, egui::vec2(9.0, 9.0)),
                    0.0,
                    colour,
                );
            }
        }
    }
    painter.circle_filled(m, 3.0, theme::TEXT_STRONG);
}
