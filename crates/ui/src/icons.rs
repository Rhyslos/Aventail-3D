//! Small vector icons for the palettes, drawn with egui's shapes.
//!
//! Drawn in code rather than shipped as images: nothing to license, sharp
//! at any interface scale, and coloured by the theme. Each icon is
//! designed on a 24 × 24 grid and scaled into the rectangle it is given.
//! These are original drawings; no icons are copied from other software.

use egui::epaint::PathStroke;
use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, vec2};
use std::f32::consts::{PI, TAU};

/// Every icon the palettes use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// Edit → Undo.
    Undo,
    /// Edit → Redo.
    Redo,
    /// Live selection: paint to select.
    LiveSelection,
    /// Move tool.
    Move,
    /// Scale tool.
    Scale,
    /// Rotate tool.
    Rotate,
    /// World or object coordinates.
    World,
    /// Cube primitive, and Model mode.
    Cube,
    /// Cylinder primitive.
    Cylinder,
    /// Plane primitive.
    Plane,
    /// Sphere primitive.
    Sphere,
    /// Null object (an empty group).
    Null,
    /// Snapping.
    Snap,
    /// Symmetry.
    Symmetry,
    /// Command search.
    Search,
    /// Object mode: work on whole objects and their axes.
    ObjectMode,
    /// Points mode.
    Points,
    /// Edges mode.
    Edges,
    /// Polygons mode.
    Polygons,
    /// UV mode.
    Uv,
    /// Workplane.
    Workplane,
    /// Solo: show only the selection.
    Solo,
}

/// Every icon, for tests and previews.
pub const ALL: [Icon; 22] = [
    Icon::Undo,
    Icon::Redo,
    Icon::LiveSelection,
    Icon::Move,
    Icon::Scale,
    Icon::Rotate,
    Icon::World,
    Icon::Cube,
    Icon::Cylinder,
    Icon::Plane,
    Icon::Sphere,
    Icon::Null,
    Icon::Snap,
    Icon::Symmetry,
    Icon::Search,
    Icon::ObjectMode,
    Icon::Points,
    Icon::Edges,
    Icon::Polygons,
    Icon::Uv,
    Icon::Workplane,
    Icon::Solo,
];

/// Draws `icon` into `rect` in `color`.
pub fn paint(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    painter.extend(shapes(rect, icon, color));
}

/// Collects shapes on the 24 × 24 design grid, scaled into a rectangle.
struct Pen {
    origin: Pos2,
    scale: f32,
    color: Color32,
    shapes: Vec<Shape>,
}

impl Pen {
    fn new(rect: Rect, color: Color32) -> Self {
        let scale = rect.width().min(rect.height()) / 24.0;
        Self {
            origin: rect.center() - vec2(12.0 * scale, 12.0 * scale),
            scale,
            color,
            shapes: Vec::new(),
        }
    }

    fn p(&self, x: f32, y: f32) -> Pos2 {
        self.origin + vec2(x * self.scale, y * self.scale)
    }

    fn width(&self) -> f32 {
        1.6 * self.scale
    }

    fn points(&self, list: &[(f32, f32)]) -> Vec<Pos2> {
        list.iter().map(|&(x, y)| self.p(x, y)).collect()
    }

    /// Points along an ellipse arc. Angles in radians: 0 points right, and
    /// angles grow clockwise on screen because y points down.
    fn arc(&self, centre: (f32, f32), radii: (f32, f32), from: f32, to: f32) -> Vec<Pos2> {
        let steps = 24;
        (0..=steps)
            .map(|i| {
                let t = from + (to - from) * i as f32 / steps as f32;
                self.p(centre.0 + radii.0 * t.cos(), centre.1 + radii.1 * t.sin())
            })
            .collect()
    }

    fn line(&mut self, points: Vec<Pos2>) {
        let stroke = PathStroke::new(self.width(), self.color);
        self.shapes.push(Shape::line(points, stroke));
    }

    fn polyline(&mut self, list: &[(f32, f32)]) {
        let points = self.points(list);
        self.line(points);
    }

    fn closed(&mut self, points: Vec<Pos2>) {
        let stroke = PathStroke::new(self.width(), self.color);
        self.shapes.push(Shape::closed_line(points, stroke));
    }

    fn polygon(&mut self, list: &[(f32, f32)]) {
        let points = self.points(list);
        self.closed(points);
    }

    fn ellipse(&mut self, centre: (f32, f32), radii: (f32, f32)) {
        let points = self.arc(centre, radii, 0.0, TAU);
        self.closed(points);
    }

    fn dot(&mut self, x: f32, y: f32, radius: f32) {
        let centre = self.p(x, y);
        self.shapes.push(Shape::circle_filled(
            centre,
            radius * self.scale,
            self.color,
        ));
    }

    fn square(&mut self, min: (f32, f32), max: (f32, f32), faint: bool) {
        let rect = Rect::from_min_max(self.p(min.0, min.1), self.p(max.0, max.1));
        let color = if faint {
            self.color.gamma_multiply(0.45)
        } else {
            self.color
        };
        self.shapes.push(Shape::rect_stroke(
            rect,
            0.0,
            Stroke::new(self.width(), color),
            StrokeKind::Middle,
        ));
    }

    fn fill(&mut self, min: (f32, f32), max: (f32, f32), strength: f32) {
        let rect = Rect::from_min_max(self.p(min.0, min.1), self.p(max.0, max.1));
        let color = self.color.gamma_multiply(strength);
        self.shapes.push(Shape::rect_filled(rect, 0.0, color));
    }

    fn segment(&mut self, from: (f32, f32), to: (f32, f32), width: f32, strength: f32) {
        let stroke = Stroke::new(self.width() * width, self.color.gamma_multiply(strength));
        let points = [self.p(from.0, from.1), self.p(to.0, to.1)];
        self.shapes.push(Shape::line_segment(points, stroke));
    }
}

/// The shapes that make up `icon`, scaled into `rect`.
pub fn shapes(rect: Rect, icon: Icon, color: Color32) -> Vec<Shape> {
    let mut pen = Pen::new(rect, color);
    match icon {
        Icon::Undo => {
            pen.polyline(&[(9.0, 14.0), (4.0, 9.0), (9.0, 4.0)]);
            let mut curve = pen.points(&[(4.0, 9.0)]);
            curve.extend(pen.arc((14.5, 14.5), (5.5, 5.5), -PI / 2.0, PI / 2.0));
            curve.push(pen.p(11.0, 20.0));
            pen.line(curve);
        }
        Icon::Redo => {
            pen.polyline(&[(15.0, 14.0), (20.0, 9.0), (15.0, 4.0)]);
            let mut curve = pen.points(&[(20.0, 9.0)]);
            curve.extend(pen.arc((9.5, 14.5), (5.5, 5.5), -PI / 2.0, -1.5 * PI));
            curve.push(pen.p(13.0, 20.0));
            pen.line(curve);
        }
        Icon::LiveSelection => {
            // A dashed circle with a plus in it.
            for i in 0..12 {
                let a = i as f32 / 12.0 * TAU;
                let dash = pen.arc((12.0, 12.0), (8.0, 8.0), a, a + TAU / 24.0);
                pen.line(dash);
            }
            pen.polyline(&[(12.0, 9.0), (12.0, 15.0)]);
            pen.polyline(&[(9.0, 12.0), (15.0, 12.0)]);
        }
        Icon::Move => {
            pen.polyline(&[(12.0, 3.0), (12.0, 21.0)]);
            pen.polyline(&[(3.0, 12.0), (21.0, 12.0)]);
            pen.polyline(&[(9.0, 6.0), (12.0, 3.0), (15.0, 6.0)]);
            pen.polyline(&[(9.0, 18.0), (12.0, 21.0), (15.0, 18.0)]);
            pen.polyline(&[(6.0, 9.0), (3.0, 12.0), (6.0, 15.0)]);
            pen.polyline(&[(18.0, 9.0), (21.0, 12.0), (18.0, 15.0)]);
        }
        Icon::Scale => {
            pen.polygon(&[(3.0, 11.0), (13.0, 11.0), (13.0, 21.0), (3.0, 21.0)]);
            pen.polyline(&[(13.0, 11.0), (21.0, 3.0)]);
            pen.polyline(&[(15.0, 3.0), (21.0, 3.0), (21.0, 9.0)]);
        }
        Icon::Rotate => {
            let ring = pen.arc((12.0, 12.0), (8.5, 8.5), -PI / 4.0, 1.5 * PI);
            pen.line(ring);
            pen.polyline(&[(20.5, 3.5), (20.5, 9.0), (15.0, 9.0)]);
        }
        Icon::World => {
            pen.ellipse((12.0, 12.0), (9.0, 9.0));
            pen.polyline(&[(3.0, 12.0), (21.0, 12.0)]);
            pen.ellipse((12.0, 12.0), (3.8, 9.0));
        }
        Icon::Cube => {
            pen.polygon(&[
                (12.0, 2.5),
                (20.5, 7.0),
                (20.5, 17.0),
                (12.0, 21.5),
                (3.5, 17.0),
                (3.5, 7.0),
            ]);
            pen.polyline(&[(3.5, 7.0), (12.0, 11.5), (20.5, 7.0)]);
            pen.polyline(&[(12.0, 11.5), (12.0, 21.5)]);
        }
        Icon::Cylinder => {
            pen.ellipse((12.0, 5.5), (8.0, 3.0));
            pen.polyline(&[(4.0, 5.5), (4.0, 18.5)]);
            pen.polyline(&[(20.0, 5.5), (20.0, 18.5)]);
            let bottom = pen.arc((12.0, 18.5), (8.0, 3.0), 0.0, PI);
            pen.line(bottom);
        }
        Icon::Plane => {
            pen.polygon(&[(2.0, 17.0), (8.0, 8.0), (22.0, 8.0), (16.0, 17.0)]);
        }
        Icon::Sphere => {
            pen.ellipse((12.0, 12.0), (9.0, 9.0));
            pen.ellipse((12.0, 12.0), (9.0, 3.5));
        }
        Icon::Null => {
            pen.polyline(&[(12.0, 3.0), (12.0, 21.0)]);
            pen.polyline(&[(3.0, 12.0), (21.0, 12.0)]);
            pen.ellipse((12.0, 12.0), (3.0, 3.0));
        }
        Icon::Snap => {
            // A horseshoe magnet: down the left side, round the bottom
            // (from the left, through the bottom, to the right), and up.
            let mut magnet = pen.points(&[(6.0, 3.0), (6.0, 11.0)]);
            magnet.extend(pen.arc((12.0, 11.0), (6.0, 6.0), PI, 0.0));
            magnet.push(pen.p(18.0, 3.0));
            pen.line(magnet);
            pen.polyline(&[(6.0, 7.0), (10.0, 7.0)]);
            pen.polyline(&[(14.0, 7.0), (18.0, 7.0)]);
        }
        Icon::Symmetry => {
            for i in 0..6 {
                let y = 3.0 + i as f32 * 3.0;
                pen.polyline(&[(12.0, y), (12.0, y + 1.5)]);
            }
            pen.polygon(&[(9.0, 6.0), (3.0, 12.0), (9.0, 18.0)]);
            pen.polygon(&[(15.0, 6.0), (21.0, 12.0), (15.0, 18.0)]);
        }
        Icon::Search => {
            pen.ellipse((11.0, 11.0), (7.0, 7.0));
            pen.polyline(&[(16.0, 16.0), (20.5, 20.5)]);
        }
        Icon::ObjectMode => {
            pen.polyline(&[(12.0, 12.0), (12.0, 3.0)]);
            pen.polyline(&[(12.0, 12.0), (20.0, 17.0)]);
            pen.polyline(&[(12.0, 12.0), (4.0, 17.0)]);
            pen.dot(12.0, 12.0, 2.2);
        }
        Icon::Points => {
            pen.square((5.0, 5.0), (19.0, 19.0), true);
            for &(x, y) in &[(5.0, 5.0), (19.0, 5.0), (5.0, 19.0), (19.0, 19.0)] {
                pen.dot(x, y, 2.4);
            }
        }
        Icon::Edges => {
            pen.square((5.0, 5.0), (19.0, 19.0), true);
            pen.segment((5.0, 19.0), (19.0, 19.0), 2.0, 1.0);
        }
        Icon::Polygons => {
            pen.fill((5.0, 5.0), (19.0, 19.0), 0.6);
        }
        Icon::Uv => {
            pen.square((4.0, 4.0), (20.0, 20.0), false);
            pen.segment((4.0, 12.0), (20.0, 12.0), 1.0, 0.5);
            pen.segment((12.0, 4.0), (12.0, 20.0), 1.0, 0.5);
            pen.fill((6.5, 6.5), (10.0, 10.0), 1.0);
        }
        Icon::Workplane => {
            pen.polygon(&[(3.0, 16.0), (8.0, 8.0), (21.0, 8.0), (16.0, 16.0)]);
            pen.segment((10.0, 12.0), (18.0, 12.0), 1.0, 0.5);
        }
        Icon::Solo => {
            pen.ellipse((12.0, 12.0), (10.0, 6.0));
            pen.ellipse((12.0, 12.0), (3.0, 3.0));
        }
    }
    pen.shapes
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::pos2;

    #[test]
    fn every_icon_draws_inside_its_box() {
        let rect = Rect::from_min_size(pos2(10.0, 10.0), vec2(24.0, 24.0));
        for icon in ALL {
            let drawn = shapes(rect, icon, Color32::WHITE);
            assert!(!drawn.is_empty(), "{icon:?} draws nothing");
            for shape in &drawn {
                // Allow for half a stroke width over the edge.
                assert!(
                    rect.expand(2.0).contains_rect(shape.visual_bounding_rect()),
                    "{icon:?} draws outside its box"
                );
            }
        }
    }

    #[test]
    fn icons_scale_with_their_box() {
        let small = Rect::from_min_size(pos2(0.0, 0.0), vec2(12.0, 12.0));
        let large = Rect::from_min_size(pos2(0.0, 0.0), vec2(48.0, 48.0));
        let width = |rect| {
            shapes(rect, Icon::Cube, Color32::WHITE)
                .iter()
                .map(Shape::visual_bounding_rect)
                .fold(Rect::NOTHING, |a, b| a.union(b))
                .width()
        };
        assert!(width(large) > 3.0 * width(small));
    }

    #[test]
    fn a_non_square_box_keeps_icons_square_and_centred() {
        let wide = Rect::from_min_size(pos2(0.0, 0.0), vec2(48.0, 24.0));
        let bounds = shapes(wide, Icon::Polygons, Color32::WHITE)
            .iter()
            .map(Shape::visual_bounding_rect)
            .fold(Rect::NOTHING, |a, b| a.union(b));
        assert!((bounds.center().x - 24.0).abs() < 0.5);
        assert!((bounds.width() - bounds.height()).abs() < 0.5);
    }
}
