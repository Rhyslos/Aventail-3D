//! The pen the icons are drawn with: points on the 24 × 24 design grid
//! scaled into the icon's rectangle, and lines, outlines, fills, dots and
//! rings in the theme's colour.

use egui::epaint::PathStroke;
use egui::{Color32, Pos2, Rect, Shape, Stroke, StrokeKind, vec2};
use std::f32::consts::TAU;

/// How a line is drawn. Every line has the same width; they differ only
/// in tone, dashes and colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Ink {
    /// The icon's colour.
    Line,
    /// The icon's colour, faint: a part in the background.
    Faint,
    /// Dashed: something about to be, or no longer, there.
    Dashed,
    /// The X axis's red.
    AxisX,
    /// The Y axis's blue.
    AxisY,
    /// The Z axis's green.
    AxisZ,
}

/// How a shape is filled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Paint {
    /// The icon's colour.
    Solid,
    /// A light tone of it: the part a tool changes.
    Tone,
    /// A stronger tone: the darker side of a flat-shaded shape.
    Shade,
    /// Smooth shading, faint on the left to strong on the right.
    Shading,
    /// The X axis's red.
    AxisX,
    /// The Z axis's green.
    AxisZ,
}

/// Collects shapes on the 24 × 24 design grid, scaled into a rectangle.
pub(super) struct Pen {
    origin: Pos2,
    scale: f32,
    color: Color32,
    pub(super) shapes: Vec<Shape>,
}

impl Pen {
    pub(super) fn new(rect: Rect, color: Color32) -> Self {
        let scale = rect.width().min(rect.height()) / 24.0;
        Self {
            origin: rect.center() - vec2(12.0 * scale, 12.0 * scale),
            scale,
            color,
            shapes: Vec::new(),
        }
    }

    pub(super) fn p(&self, x: f32, y: f32) -> Pos2 {
        self.origin + vec2(x * self.scale, y * self.scale)
    }

    fn width(&self) -> f32 {
        1.6 * self.scale
    }

    pub(super) fn points(&self, list: &[(f32, f32)]) -> Vec<Pos2> {
        list.iter().map(|&(x, y)| self.p(x, y)).collect()
    }

    /// Points along an ellipse arc. Angles in radians: 0 points right, and
    /// angles grow clockwise on screen because y points down.
    pub(super) fn arc(
        &self,
        centre: (f32, f32),
        radii: (f32, f32),
        from: f32,
        to: f32,
    ) -> Vec<Pos2> {
        let steps = 24;
        (0..=steps)
            .map(|i| {
                let t = from + (to - from) * i as f32 / steps as f32;
                self.p(centre.0 + radii.0 * t.cos(), centre.1 + radii.1 * t.sin())
            })
            .collect()
    }

    pub(super) fn line(&mut self, points: Vec<Pos2>) {
        let stroke = PathStroke::new(self.width(), self.color);
        self.shapes.push(Shape::line(points, stroke));
    }

    pub(super) fn polyline(&mut self, list: &[(f32, f32)]) {
        let points = self.points(list);
        self.line(points);
    }

    pub(super) fn closed(&mut self, points: Vec<Pos2>) {
        let stroke = PathStroke::new(self.width(), self.color);
        self.shapes.push(Shape::closed_line(points, stroke));
    }

    pub(super) fn polygon(&mut self, list: &[(f32, f32)]) {
        let points = self.points(list);
        self.closed(points);
    }

    pub(super) fn ellipse(&mut self, centre: (f32, f32), radii: (f32, f32)) {
        let points = self.arc(centre, radii, 0.0, TAU);
        self.closed(points);
    }

    pub(super) fn dot(&mut self, x: f32, y: f32, radius: f32) {
        let centre = self.p(x, y);
        self.shapes.push(Shape::circle_filled(
            centre,
            radius * self.scale,
            self.color,
        ));
    }

    pub(super) fn square(&mut self, min: (f32, f32), max: (f32, f32), faint: bool) {
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

    pub(super) fn fill(&mut self, min: (f32, f32), max: (f32, f32), strength: f32) {
        let rect = Rect::from_min_max(self.p(min.0, min.1), self.p(max.0, max.1));
        let color = self.color.gamma_multiply(strength);
        self.shapes.push(Shape::rect_filled(rect, 0.0, color));
    }

    /// A line through `list`, closed back to its start if `closed`.
    pub(super) fn stroke(&mut self, list: &[(f32, f32)], closed: bool, ink: Ink) {
        let color = match ink {
            Ink::Line => self.color,
            Ink::Faint => self.color.gamma_multiply(0.4),
            Ink::Dashed => self.color.gamma_multiply(0.75),
            Ink::AxisX => crate::theme::tint::AXIS_X,
            Ink::AxisY => crate::theme::tint::AXIS_Y,
            Ink::AxisZ => crate::theme::tint::AXIS_Z,
        };
        let mut points = self.points(list);
        if ink == Ink::Dashed {
            if closed && let Some(&first) = points.first() {
                points.push(first);
            }
            let stroke = Stroke::new(self.width(), color);
            let (dash, gap) = (1.6 * self.scale, 2.6 * self.scale);
            self.shapes
                .extend(Shape::dashed_line(&points, stroke, dash, gap));
            return;
        }
        let stroke = PathStroke::new(self.width(), color);
        self.shapes.push(if closed {
            Shape::closed_line(points, stroke)
        } else {
            Shape::line(points, stroke)
        });
    }

    /// A filled convex shape through `list`.
    pub(super) fn shape(&mut self, list: &[(f32, f32)], paint: Paint) {
        let points = self.points(list);
        let flat = |color: Color32| Shape::convex_polygon(points.clone(), color, Stroke::NONE);
        let shape = match paint {
            Paint::Solid => flat(self.color),
            Paint::Tone => flat(self.color.gamma_multiply(0.3)),
            Paint::Shade => flat(self.color.gamma_multiply(0.62)),
            Paint::AxisX => flat(crate::theme::tint::AXIS_X),
            Paint::AxisZ => flat(crate::theme::tint::AXIS_Z),
            Paint::Shading => {
                // A fan of triangles whose corners fade from left to right.
                let (left, right) = points
                    .iter()
                    .fold((f32::MAX, f32::MIN), |(l, r), p| (l.min(p.x), r.max(p.x)));
                let alpha = |x: f32| {
                    let t = ((x - left) / (right - left).max(1e-3)).clamp(0.0, 1.0);
                    Color32::from_white_alpha((255.0 * (0.12 + 0.48 * t)) as u8)
                };
                let mut mesh = egui::Mesh::default();
                for p in &points {
                    mesh.colored_vertex(*p, alpha(p.x));
                }
                for i in 1..points.len().saturating_sub(1) as u32 {
                    mesh.add_triangle(0, i, i + 1);
                }
                Shape::mesh(mesh)
            }
        };
        self.shapes.push(shape);
    }

    /// A filled circle.
    pub(super) fn disc(&mut self, centre: (f32, f32), radius: f32, paint: Paint) {
        let color = match paint {
            Paint::Tone => self.color.gamma_multiply(0.3),
            _ => self.color,
        };
        let centre = self.p(centre.0, centre.1);
        self.shapes
            .push(Shape::circle_filled(centre, radius * self.scale, color));
    }

    /// A circle or ellipse's outline.
    pub(super) fn ring(&mut self, centre: (f32, f32), radii: (f32, f32), ink: Ink) {
        let list: Vec<(f32, f32)> = (0..48)
            .map(|i| {
                let a = i as f32 / 48.0 * TAU;
                (centre.0 + radii.0 * a.cos(), centre.1 + radii.1 * a.sin())
            })
            .collect();
        self.stroke(&list, true, ink);
    }

    pub(super) fn segment(&mut self, from: (f32, f32), to: (f32, f32), width: f32, strength: f32) {
        let stroke = Stroke::new(self.width() * width, self.color.gamma_multiply(strength));
        let points = [self.p(from.0, from.1), self.p(to.0, to.1)];
        self.shapes.push(Shape::line_segment(points, stroke));
    }
}
